//! Session state — the .git/spectral directory.
//!
//! `spectral init` creates a `.git/spectral/` directory inside the project's git repo.
//! The directory is the anchor for all spectral operations in a project tree.
//!
//! Layout:
//! ```
//! .git/spectral/
//!   gestalt/     — crystals: reader/user understanding state
//!   sessions/    — session data
//!   crystals/    — crystallized subgraphs
//!   HEAD         — unix timestamp of current session
//!   log          — tick log (tab-separated: timestamp TAB event TAB message TAB growth)
//! ```

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A spectral session anchored at a `.git/spectral/` directory.
pub struct Session {
    root: PathBuf,
}

impl Session {
    /// Create the `.git/spectral/` directory structure under `dir`.
    ///
    /// If `.git/` doesn't exist, runs `git init` via subprocess.
    /// Idempotent — calling twice does not error.
    pub fn init(dir: &Path) -> io::Result<Self> {
        // Ensure .git/ exists
        let git_dir = dir.join(".git");
        if !git_dir.exists() {
            let status = std::process::Command::new("git")
                .args(["init"])
                .current_dir(dir)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()?;
            if !status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "git init failed",
                ));
            }
        }

        let root = git_dir.join("spectral");

        // Create all directories (all_ok if they already exist)
        std::fs::create_dir_all(root.join("gestalt"))?;
        std::fs::create_dir_all(root.join("sessions"))?;
        std::fs::create_dir_all(root.join("crystals"))?;

        // Unix timestamp
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Write HEAD (overwrite if re-initializing)
        std::fs::write(root.join("HEAD"), format!("{}\n", ts))?;

        // Append first log entry: timestamp TAB init TAB message TAB growth
        let log_entry = format!("{}\tinit\tGarden planted\t0.0\n", ts);
        append_to_log(&root.join("log"), &log_entry)?;

        // Print seed message to stderr
        eprintln!("Garden planted.");
        eprintln!("Growth: 0%");

        Ok(Session { root })
    }

    /// Walk up from `start` looking for a `.git/spectral/` directory.
    ///
    /// Returns `None` if no `.git/spectral/` is found at or above `start`.
    pub fn find(start: &Path) -> Option<Self> {
        let mut current = start.to_path_buf();
        loop {
            let candidate = current.join(".git").join("spectral");
            if candidate.is_dir() {
                return Some(Session { root: candidate });
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Path to the `.git/spectral/` directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read the HEAD file. Returns `None` if HEAD is missing or unreadable.
    pub fn head(&self) -> Option<String> {
        std::fs::read_to_string(self.root.join("HEAD"))
            .ok()
            .map(|s| s.trim().to_string())
    }
}

/// Append `entry` to the log file at `path`, creating it if it doesn't exist.
fn append_to_log(path: &Path, entry: &str) -> io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(entry.as_bytes())
}
