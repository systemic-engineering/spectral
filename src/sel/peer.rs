//! ~/.spectral bare repo — peer registry and ref packet store.
//!
//! ~/.spectral holds the address registry (OID index + peer list), not node data.
//! Data stays authoritative in each project's .spectral/ dir.

use std::path::{Path, PathBuf};

/// A registered peer: path on disk + grammar surface it contributes.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Peer {
    pub path: String,
    pub oid: String,
    pub joined_at: u64,
    pub grammar_surface: Vec<String>,
}

/// Registry of all joined peers.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PeerRegistry {
    pub peers: Vec<Peer>,
}

/// A ref packet: minimum info needed for distributed query + recovery.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RefPacket {
    pub oid: String,
    pub node_type: String,
    pub source_path: String,
}

/// Resolve ~/.spectral path.
pub fn home_spectral() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".spectral")
}

/// Initialise ~/.spectral if it does not exist. Returns the path.
pub fn init_home_spectral() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    init_at(&PathBuf::from(home))
}

/// Load the peer registry from <home>/.spectral/peers.json.
pub fn load_registry(home: &Path) -> PeerRegistry {
    let path = home.join("peers.json");
    std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Save the peer registry to <home>/.spectral/peers.json.
pub fn save_registry(home: &Path, registry: &PeerRegistry) -> Result<(), String> {
    let path = home.join("peers.json");
    let bytes = serde_json::to_vec_pretty(registry).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())
}

/// Register a peer path. Idempotent — re-joining updates grammar_surface.
pub fn register_peer(
    home: &Path,
    peer_path: &str,
    oid: &str,
    grammar_surface: Vec<String>,
) -> Result<(), String> {
    let mut registry = load_registry(home);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(existing) = registry.peers.iter_mut().find(|p| p.path == peer_path) {
        existing.grammar_surface = grammar_surface;
        existing.oid = oid.to_string();
    } else {
        registry.peers.push(Peer {
            path: peer_path.to_string(),
            oid: oid.to_string(),
            joined_at: now,
            grammar_surface,
        });
    }
    save_registry(home, &registry)
}

/// Push ref packets from <source_path>/.spectral/manifest.json into <home>/refs.json.
/// Returns the total number of OIDs processed from the manifest.
pub fn push_refs(home: &Path, source_path: &str) -> Result<usize, String> {
    let manifest_path = Path::new(source_path).join(".spectral").join("manifest.json");
    if !manifest_path.exists() {
        return Ok(0);
    }
    let bytes = std::fs::read(&manifest_path).map_err(|e| e.to_string())?;
    let oids: Vec<String> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let count = oids.len();

    // Load existing refs, merge, deduplicate by OID.
    let refs_path = home.join("refs.json");
    let mut existing: Vec<RefPacket> = if refs_path.exists() {
        std::fs::read(&refs_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let existing_oids: std::collections::HashSet<String> =
        existing.iter().map(|r| r.oid.clone()).collect();

    for oid in oids {
        if !existing_oids.contains(&oid) {
            existing.push(RefPacket {
                oid,
                node_type: "unknown".to_string(),
                source_path: source_path.to_string(),
            });
        }
    }

    let bytes = serde_json::to_vec_pretty(&existing).map_err(|e| e.to_string())?;
    std::fs::write(&refs_path, bytes).map_err(|e| e.to_string())?;
    Ok(count)
}

// Internal: init ~/.spectral at an explicit base path (used by tests and prod).
fn init_at(base: &Path) -> Result<PathBuf, String> {
    let spectral = base.join(".spectral");
    std::fs::create_dir_all(&spectral).map_err(|e| e.to_string())?;
    let peers_path = spectral.join("peers.json");
    if !peers_path.exists() {
        let empty = PeerRegistry::default();
        let bytes = serde_json::to_vec(&empty).map_err(|e| e.to_string())?;
        std::fs::write(&peers_path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(spectral)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_spectral() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let home = init_at(tmp.path()).unwrap();
        (tmp, home)
    }

    #[test]
    fn init_creates_spectral_dir_and_peers_json() {
        let tmp = tempfile::tempdir().unwrap();
        let home = init_at(tmp.path()).unwrap();
        assert!(home.exists(), ".spectral dir must be created");
        assert!(home.join("peers.json").exists(), "peers.json must be created");
    }

    #[test]
    fn load_registry_on_empty_returns_default() {
        let (_tmp, home) = tmp_spectral();
        let reg = load_registry(&home);
        assert!(reg.peers.is_empty());
    }

    #[test]
    fn register_peer_persists_to_registry() {
        let (_tmp, home) = tmp_spectral();
        register_peer(&home, "/some/project", "abc123", vec!["@reed".to_string()]).unwrap();
        let reg = load_registry(&home);
        assert_eq!(reg.peers.len(), 1);
        assert_eq!(reg.peers[0].path, "/some/project");
        assert_eq!(reg.peers[0].grammar_surface, vec!["@reed"]);
    }

    #[test]
    fn register_peer_is_idempotent() {
        let (_tmp, home) = tmp_spectral();
        register_peer(&home, "/some/project", "abc123", vec![]).unwrap();
        register_peer(&home, "/some/project", "abc123", vec!["@nl".to_string()]).unwrap();
        let reg = load_registry(&home);
        assert_eq!(reg.peers.len(), 1, "re-join must not duplicate");
        assert_eq!(reg.peers[0].grammar_surface, vec!["@nl"]);
    }

    #[test]
    fn push_refs_reads_manifest_and_writes_refs() {
        let (_tmp, home) = tmp_spectral();

        // Create a fake source with .spectral/manifest.json
        let src = tempfile::tempdir().unwrap();
        let spec_dir = src.path().join(".spectral");
        std::fs::create_dir_all(&spec_dir).unwrap();
        let oids = vec!["oid1".to_string(), "oid2".to_string(), "oid3".to_string()];
        std::fs::write(
            spec_dir.join("manifest.json"),
            serde_json::to_vec(&oids).unwrap(),
        )
        .unwrap();

        let pushed = push_refs(&home, src.path().to_str().unwrap()).unwrap();
        assert_eq!(pushed, 3);

        let refs_raw = std::fs::read(home.join("refs.json")).unwrap();
        let refs: Vec<RefPacket> = serde_json::from_slice(&refs_raw).unwrap();
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn push_refs_deduplicates_on_second_push() {
        let (_tmp, home) = tmp_spectral();
        let src = tempfile::tempdir().unwrap();
        let spec_dir = src.path().join(".spectral");
        std::fs::create_dir_all(&spec_dir).unwrap();
        let oids = vec!["oid1".to_string(), "oid2".to_string()];
        std::fs::write(
            spec_dir.join("manifest.json"),
            serde_json::to_vec(&oids).unwrap(),
        )
        .unwrap();

        push_refs(&home, src.path().to_str().unwrap()).unwrap();
        push_refs(&home, src.path().to_str().unwrap()).unwrap();

        let refs_raw = std::fs::read(home.join("refs.json")).unwrap();
        let refs: Vec<RefPacket> = serde_json::from_slice(&refs_raw).unwrap();
        assert_eq!(refs.len(), 2, "duplicate push must not add duplicate refs");
    }

    #[test]
    fn push_refs_returns_zero_when_no_manifest() {
        let (_tmp, home) = tmp_spectral();
        let src = tempfile::tempdir().unwrap();
        let count = push_refs(&home, src.path().to_str().unwrap()).unwrap();
        assert_eq!(count, 0);
    }
}
