//! spectral observe — inbox writer.
//!
//! Fast (<5ms) file I/O only. No actor system, no DB.
//! Writes JSON observations to `.git/spectral/inbox/{nanos}.json`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the inbox directory: `project_root/.git/spectral/inbox/`
pub fn inbox_dir(project_root: &Path) -> PathBuf {
    project_root.join(".git").join("spectral").join("inbox")
}

/// Write a single observation as JSON to the inbox.
///
/// Returns the path of the file created, or an error string.
/// Filename is nanosecond timestamp: `{nanos}.json`.
pub fn write_observation(
    project_root: &Path,
    tool_name: &str,
    input_summary: &str,
    output_summary: &str,
) -> Result<PathBuf, String> {
    let dir = inbox_dir(project_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("observe: failed to create inbox dir: {}", e))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = now.as_nanos();
    let timestamp_secs = now.as_secs();

    let payload = serde_json::json!({
        "tool": tool_name,
        "input": input_summary,
        "output": output_summary,
        "timestamp": timestamp_secs,
    });

    let filename = format!("{}.json", nanos);
    let path = dir.join(&filename);

    let content = serde_json::to_string(&payload)
        .map_err(|e| format!("observe: failed to serialize: {}", e))?;

    std::fs::write(&path, content)
        .map_err(|e| format!("observe: failed to write {}: {}", path.display(), e))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_observation_creates_inbox_file() {
        let tmp = TempDir::new().unwrap();
        // create a .git dir so inbox_dir resolves correctly
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();

        let path = write_observation(
            tmp.path(),
            "Bash",
            "ls -la",
            "total 42",
        )
        .expect("write_observation failed");

        assert!(path.exists(), "file should exist at {:?}", path);

        let content = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["tool"], "Bash");
    }

    #[test]
    fn write_observation_creates_inbox_dir_if_missing() {
        let tmp = TempDir::new().unwrap();
        // Do NOT pre-create .git or inbox — the function must create them
        let inbox = inbox_dir(tmp.path());
        assert!(!inbox.exists(), "inbox should not exist before write");

        let path = write_observation(tmp.path(), "Read", "src/main.rs", "fn main")
            .expect("write_observation failed");

        assert!(inbox.exists(), "inbox dir should be created");
        assert!(path.exists(), "file should exist");
    }

    #[test]
    fn write_observation_unique_filenames() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();

        let p1 = write_observation(tmp.path(), "Bash", "a", "b").unwrap();
        // 1ms sleep to guarantee distinct nanosecond timestamps
        std::thread::sleep(std::time::Duration::from_millis(1));
        let p2 = write_observation(tmp.path(), "Bash", "c", "d").unwrap();

        assert_ne!(p1, p2, "filenames must be unique");
    }
}
