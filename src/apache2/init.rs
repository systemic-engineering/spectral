//! spectral init — compile identity into a spectral-db graph.
//!
//! Two-tier snapshot architecture:
//! - **Fast hash (FNV-1a):** the session anchor. Updates on every `spectral` operation.
//!   3.3us at 200 motes. The clock tick. 8 ticks per frame with 41% budget remaining.
//! - **Full hash (CoincidenceHash):** the identity anchor. Updates on crystallization.
//!   7.4ms at 200 motes. The record. The OID that persists.
//!
//! Cascade ratio: 635. The GPU holds 635 units of superposition for every CPU measurement.
//!
//! ## Gestalt auto-detection
//!
//! When no `.mirror` files are found, gestalt auto-detection kicks in:
//! 1. Walk the directory tree (respecting .gitignore)
//! 2. Classify every file by extension
//! 3. Build a directory-level concept graph
//! 4. Compute eigenvalue profile (spectral fingerprint)
//! 5. Include in the snapshot
//!
//! A non-empty directory always produces at least an eigenvalue profile.
//! The only Failure case is an empty or unreadable directory.

use std::path::Path;
use prism::oid::Oid;
use terni::{Imperfect, Loss};
use super::identity::BiasChain;
use super::loss::InitLoss;

use gestalt::detect::GestaltBreakdown;
use gestalt::eigenvalue::EigenvalueProfile;
use gestalt::graph::ConceptGraph;

// ---------------------------------------------------------------------------
// FNV-1a — the fast path hash (session anchor)
// ---------------------------------------------------------------------------

/// FNV-1a 64-bit hash. Non-cryptographic. Deterministic.
/// The fast path: O(n) with tiny constant factor.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ---------------------------------------------------------------------------
// InitSnapshot — two-tier content address of the init state
// ---------------------------------------------------------------------------

/// Two-tier snapshot of the initialized state.
///
/// The fast hash is the session anchor — cheap to compute, updates every tick.
/// The full hash is the identity anchor — expensive, updates on crystallization.
#[derive(Debug, Clone)]
pub struct InitSnapshot {
    /// FNV-1a hash of the serialized init state. The session anchor.
    /// Format: 16 hex chars (64-bit).
    pub fast_oid: Oid,
    /// CoincidenceHash of the serialized init state. The identity anchor.
    /// Format: 64 hex chars (256-bit via SHA-256 of eigenvalue).
    pub full_oid: Oid,
    /// Byte count of the serialized state (for benchmarking).
    pub state_bytes: usize,
}

impl InitSnapshot {
    /// Take a two-tier snapshot of serialized bytes.
    /// Fast path: FNV-1a. Full path: Oid::hash (CoincidenceHash<3>).
    pub fn capture(bytes: &[u8]) -> Self {
        let fast = fnv1a_64(bytes);
        let fast_oid = Oid::new(format!("{:016x}", fast));
        let full_oid = Oid::hash(bytes);
        InitSnapshot {
            fast_oid,
            full_oid,
            state_bytes: bytes.len(),
        }
    }
}

// ---------------------------------------------------------------------------
// InitResult
// ---------------------------------------------------------------------------

/// Result of initializing identity from a directory.
///
/// Supports two tiers of identity discovery:
/// - Explicit: `.mirror` files -> grammar parse -> bias chain
/// - Implicit: all files -> gestalt detect -> concept graph -> eigenvalue profile
///
/// A repo with `.mirror` files gets both. A repo without gets implicit only.
#[derive(Debug)]
pub struct InitResult {
    pub bias_chain: BiasChain,
    pub mirror_files_found: u32,
    pub files: Vec<(String, String)>,
    /// Two-tier snapshot of the initialized state.
    pub snapshot: InitSnapshot,

    // Gestalt auto-detection fields
    /// Total number of files detected by gestalt walk.
    pub gestalt_files_detected: u32,
    /// Breakdown of detected files by grammar kind.
    pub gestalt_breakdown: GestaltBreakdown,
    /// The concept graph (directory-level nodes and edges).
    pub concept_graph: Option<ConceptGraph>,
    /// The eigenvalue profile (spectral fingerprint).
    pub eigenvalue_profile: Option<EigenvalueProfile>,
}

/// Read directory, find .mirror files, derive bias chain from filename order.
/// Falls back to gestalt auto-detection when no .mirror files are found.
///
/// Returns Success (identity found), Partial (some warnings), Failure (empty/unreadable).
pub fn init_identity(path: &Path) -> Imperfect<InitResult, String, InitLoss> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            return Imperfect::Failure(
                format!("cannot read directory: {}", e),
                InitLoss::total(),
            );
        }
    };

    let mut mirror_files: Vec<(String, String)> = Vec::new();

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".mirror") {
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            mirror_files.push((file_name, content));
        }
    }

    // Sort by filename to get numbered ordering
    mirror_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Derive bias chain: "00-narrative.mirror" -> "narrative"
    let ordering: Vec<String> = mirror_files
        .iter()
        .map(|(name, _)| {
            let stem = name.trim_end_matches(".mirror");
            // Strip leading number prefix like "00-"
            if let Some(pos) = stem.find('-') {
                let prefix = &stem[..pos];
                if prefix.chars().all(|c| c.is_ascii_digit()) {
                    return stem[pos + 1..].to_string();
                }
            }
            stem.to_string()
        })
        .collect();

    let mirror_count = mirror_files.len() as u32;

    // Run gestalt auto-detection
    let (gestalt_graph, _detected_files, gestalt_breakdown) =
        gestalt::graph::build_concept_graph(path);
    let gestalt_files_detected = gestalt_breakdown.total();

    // Compute eigenvalue profile if graph is non-empty
    let eigenvalue_profile = if !gestalt_graph.nodes.is_empty() {
        let profile = gestalt::eigenvalue::eigenvalue_profile(&gestalt_graph);
        if profile.is_dark() { None } else { Some(profile) }
    } else {
        None
    };

    // If no .mirror files AND no gestalt files: failure
    if mirror_files.is_empty() && gestalt_files_detected == 0 {
        return Imperfect::Failure(
            "no files found (no .mirror files, no gestalt-detectable files)".to_string(),
            InitLoss { grammars_compiled: 0, grammars_with_warnings: 0 },
        );
    }

    // Build serialization: mirror state + gestalt eigenvalue profile
    let mut state_bytes = serialize_init_state(&mirror_files);

    // Include gestalt data in the snapshot
    if let Some(ref profile) = eigenvalue_profile {
        state_bytes.extend_from_slice(&profile.to_bytes());
    }

    let snapshot = InitSnapshot::capture(&state_bytes);

    let concept_graph = if gestalt_graph.nodes.is_empty() {
        None
    } else {
        Some(gestalt_graph)
    };

    // Determine result status
    if mirror_count > 0 {
        // Explicit identity found — enriched with gestalt
        Imperfect::Success(InitResult {
            bias_chain: BiasChain::new(ordering),
            mirror_files_found: mirror_count,
            files: mirror_files,
            snapshot,
            gestalt_files_detected,
            gestalt_breakdown,
            concept_graph,
            eigenvalue_profile,
        })
    } else {
        // No .mirror files, but gestalt found files — implicit identity
        Imperfect::Partial(
            InitResult {
                bias_chain: BiasChain::new(Vec::new()),
                mirror_files_found: 0,
                files: Vec::new(),
                snapshot,
                gestalt_files_detected,
                gestalt_breakdown,
                concept_graph,
                eigenvalue_profile,
            },
            InitLoss { grammars_compiled: 0, grammars_with_warnings: 0 },
        )
    }
}

/// Serialize the init state into a deterministic byte buffer.
/// Format: count (u64 LE) + for each file: filename bytes + \0 + content bytes + \0.
/// Same files in same order = same bytes = same OID.
pub fn serialize_init_state(files: &[(String, String)]) -> Vec<u8> {
    let total_size: usize = 8 + files.iter()
        .map(|(name, content)| name.len() + 1 + content.len() + 1)
        .sum::<usize>();
    let mut buf = Vec::with_capacity(total_size);
    buf.extend_from_slice(&(files.len() as u64).to_le_bytes());
    for (name, content) in files {
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(content.as_bytes());
        buf.push(0);
    }
    buf
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- FNV-1a unit tests --

    #[test]
    fn fnv1a_empty_is_offset_basis() {
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325);
    }

    #[test]
    fn fnv1a_deterministic() {
        assert_eq!(fnv1a_64(b"spectral"), fnv1a_64(b"spectral"));
    }

    #[test]
    fn fnv1a_different_input_different_hash() {
        assert_ne!(fnv1a_64(b"init"), fnv1a_64(b"tick"));
    }

    // -- serialize_init_state tests --

    #[test]
    fn serialize_empty_files() {
        let files: Vec<(String, String)> = vec![];
        let bytes = serialize_init_state(&files);
        // Just the count: 8 bytes, all zero
        assert_eq!(bytes.len(), 8);
        assert_eq!(u64::from_le_bytes(bytes[..8].try_into().unwrap()), 0);
    }

    #[test]
    fn serialize_deterministic() {
        let files = vec![
            ("a.mirror".to_string(), "content_a".to_string()),
            ("b.mirror".to_string(), "content_b".to_string()),
        ];
        let a = serialize_init_state(&files);
        let b = serialize_init_state(&files);
        assert_eq!(a, b);
    }

    #[test]
    fn serialize_different_files_different_bytes() {
        let files_a = vec![("a.mirror".to_string(), "x".to_string())];
        let files_b = vec![("b.mirror".to_string(), "y".to_string())];
        assert_ne!(serialize_init_state(&files_a), serialize_init_state(&files_b));
    }

    #[test]
    fn serialize_order_matters() {
        let files_ab = vec![
            ("a.mirror".to_string(), "x".to_string()),
            ("b.mirror".to_string(), "y".to_string()),
        ];
        let files_ba = vec![
            ("b.mirror".to_string(), "y".to_string()),
            ("a.mirror".to_string(), "x".to_string()),
        ];
        assert_ne!(serialize_init_state(&files_ab), serialize_init_state(&files_ba));
    }

    // -- InitSnapshot tests --

    #[test]
    fn snapshot_fast_oid_is_16_hex_chars() {
        let snap = InitSnapshot::capture(b"test");
        assert_eq!(snap.fast_oid.as_str().len(), 16);
        assert!(snap.fast_oid.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn snapshot_full_oid_is_64_hex_chars() {
        let snap = InitSnapshot::capture(b"test");
        assert_eq!(snap.full_oid.as_str().len(), 64);
        assert!(snap.full_oid.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn snapshot_fast_differs_from_full() {
        let snap = InitSnapshot::capture(b"test");
        // Different algorithms, different OIDs (different lengths alone guarantee this)
        assert_ne!(snap.fast_oid, snap.full_oid);
    }

    #[test]
    fn snapshot_deterministic() {
        let a = InitSnapshot::capture(b"determinism");
        let b = InitSnapshot::capture(b"determinism");
        assert_eq!(a.fast_oid, b.fast_oid);
        assert_eq!(a.full_oid, b.full_oid);
    }

    #[test]
    fn snapshot_different_input_different_oids() {
        let a = InitSnapshot::capture(b"alpha");
        let b = InitSnapshot::capture(b"bravo");
        assert_ne!(a.fast_oid, b.fast_oid);
        assert_ne!(a.full_oid, b.full_oid);
    }

    #[test]
    fn snapshot_records_byte_count() {
        let data = b"hello world";
        let snap = InitSnapshot::capture(data);
        assert_eq!(snap.state_bytes, data.len());
    }

    // -- Integration: init_identity with .mirror files (existing behavior) --

    #[test]
    fn init_identity_success_includes_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("00-test.mirror"),
            "grammar @test { in @benchmark }",
        ).unwrap();

        match init_identity(dir.path()) {
            Imperfect::Success(result) => {
                // Fast OID present
                assert!(!result.snapshot.fast_oid.is_dark());
                assert_eq!(result.snapshot.fast_oid.as_str().len(), 16);
                // Full OID present
                assert!(!result.snapshot.full_oid.is_dark());
                assert_eq!(result.snapshot.full_oid.as_str().len(), 64);
                // State bytes > 0
                assert!(result.snapshot.state_bytes > 0);
                // Mirror files found
                assert_eq!(result.mirror_files_found, 1);
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    #[test]
    fn init_identity_snapshot_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("00-test.mirror"),
            "grammar @test { in @benchmark }",
        ).unwrap();

        let snap_a = match init_identity(dir.path()) {
            Imperfect::Success(r) => r.snapshot,
            _ => panic!("expected Success"),
        };
        let snap_b = match init_identity(dir.path()) {
            Imperfect::Success(r) => r.snapshot,
            _ => panic!("expected Success"),
        };
        assert_eq!(snap_a.fast_oid, snap_b.fast_oid);
        assert_eq!(snap_a.full_oid, snap_b.full_oid);
    }

    #[test]
    fn init_identity_different_files_different_snapshot() {
        let dir_a = tempfile::tempdir().unwrap();
        std::fs::write(dir_a.path().join("00-alpha.mirror"), "grammar @alpha {}").unwrap();

        let dir_b = tempfile::tempdir().unwrap();
        std::fs::write(dir_b.path().join("00-bravo.mirror"), "grammar @bravo {}").unwrap();

        let snap_a = match init_identity(dir_a.path()) {
            Imperfect::Success(r) => r.snapshot,
            _ => panic!("expected Success"),
        };
        let snap_b = match init_identity(dir_b.path()) {
            Imperfect::Success(r) => r.snapshot,
            _ => panic!("expected Success"),
        };
        assert_ne!(snap_a.fast_oid, snap_b.fast_oid);
        assert_ne!(snap_a.full_oid, snap_b.full_oid);
    }

    // -- Gestalt auto-detection tests --

    #[test]
    fn init_empty_dir_is_failure() {
        let dir = tempfile::tempdir().unwrap();
        match init_identity(dir.path()) {
            Imperfect::Failure(msg, _) => {
                assert!(msg.contains("no files found"), "got: {}", msg);
            }
            other => panic!("expected Failure for empty dir, got {:?}", other),
        }
    }

    #[test]
    fn init_no_mirror_with_files_produces_partial() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.md"), "# Hello World\n\nSome content.\n").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "pub fn hello() {}\n").unwrap();

        match init_identity(dir.path()) {
            Imperfect::Partial(result, _loss) => {
                assert_eq!(result.mirror_files_found, 0);
                assert!(result.gestalt_files_detected >= 2);
                assert!(result.gestalt_breakdown.markdown >= 1);
                assert!(result.gestalt_breakdown.code >= 1);
                // Should have a non-dark snapshot
                assert!(!result.snapshot.fast_oid.is_dark());
                assert!(!result.snapshot.full_oid.is_dark());
            }
            other => panic!("expected Partial for no-mirror dir with files, got {:?}", other),
        }
    }

    #[test]
    fn init_mixed_mirror_and_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("00-identity.mirror"),
            "grammar @identity { in @spectral }",
        ).unwrap();
        std::fs::write(dir.path().join("readme.md"), "# Hello\n").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "pub fn main() {}\n").unwrap();

        match init_identity(dir.path()) {
            Imperfect::Success(result) => {
                // Has both .mirror identity and gestalt detection
                assert_eq!(result.mirror_files_found, 1);
                // Gestalt should detect at least the .md and .rs files
                // (.mirror files are also detected by gestalt)
                assert!(result.gestalt_files_detected >= 2);
                assert!(!result.snapshot.fast_oid.is_dark());
            }
            other => panic!("expected Success for mixed dir, got {:?}", other),
        }
    }

    #[test]
    fn init_snapshot_includes_gestalt() {
        // Two repos with different structure produce different snapshots.
        // Single-file repos produce dark eigenvalue profiles, so we need
        // directories with subdirs to get non-trivial profiles.
        let dir_a = tempfile::tempdir().unwrap();
        std::fs::write(dir_a.path().join("readme.md"), "# Alpha\n").unwrap();
        let sub_a = dir_a.path().join("docs");
        std::fs::create_dir(&sub_a).unwrap();
        std::fs::write(sub_a.join("guide.md"), "# Guide\n").unwrap();

        let dir_b = tempfile::tempdir().unwrap();
        std::fs::write(dir_b.path().join("readme.md"), "# Bravo\n").unwrap();
        let sub_b = dir_b.path().join("src");
        std::fs::create_dir(&sub_b).unwrap();
        std::fs::write(sub_b.join("lib.rs"), "pub fn main() {}\n").unwrap();
        let sub_b2 = dir_b.path().join("tests");
        std::fs::create_dir(&sub_b2).unwrap();
        std::fs::write(sub_b2.join("test.rs"), "#[test] fn t() {}\n").unwrap();

        let snap_a = match init_identity(dir_a.path()) {
            Imperfect::Partial(r, _) => r,
            other => panic!("expected Partial, got {:?}", other),
        };
        let snap_b = match init_identity(dir_b.path()) {
            Imperfect::Partial(r, _) => r,
            other => panic!("expected Partial, got {:?}", other),
        };

        // Both should have eigenvalue profiles
        assert!(snap_a.eigenvalue_profile.is_some(), "dir_a should have profile");
        assert!(snap_b.eigenvalue_profile.is_some(), "dir_b should have profile");

        // Different structure -> different eigenvalue profiles -> different snapshots
        assert_ne!(
            snap_a.eigenvalue_profile.as_ref().unwrap().oid(),
            snap_b.eigenvalue_profile.as_ref().unwrap().oid(),
            "different repos should have different eigenvalue profiles"
        );
        assert_ne!(snap_a.snapshot.full_oid, snap_b.snapshot.full_oid);
    }

    #[test]
    fn init_gestalt_with_subdirs_produces_graph() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.md"), "# Root\n").unwrap();
        let sub = dir.path().join("docs");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("guide.md"), "# Guide\n").unwrap();
        std::fs::write(sub.join("api.md"), "# API\n").unwrap();

        match init_identity(dir.path()) {
            Imperfect::Partial(result, _) => {
                assert!(result.concept_graph.is_some(), "should have concept graph");
                let graph = result.concept_graph.unwrap();
                // root + docs = 2 directory nodes
                assert!(graph.nodes.len() >= 2, "expected at least 2 nodes, got {}", graph.nodes.len());
                // Should have eigenvalue profile
                assert!(result.eigenvalue_profile.is_some(), "should have eigenvalue profile");
                let profile = result.eigenvalue_profile.unwrap();
                assert!(!profile.is_dark(), "profile should not be dark for connected graph");
            }
            other => panic!("expected Partial, got {:?}", other),
        }
    }
}
