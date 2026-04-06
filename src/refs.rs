//! Reference resolution — git-like refs for .spectral paths.
//!
//! Resolves short references to paths within the `.spectral/` directory:
//!
//! ```text
//! .       → .spectral/sessions/current    (current session state)
//! ..      → .spectral/sessions/parent     (previous node in path)
//! ~       → .spectral/gestalt             (gestalt root)
//! ^       → most recent file in .spectral/crystals/  (last crystal)
//! HEAD    → .spectral/HEAD                (current session file)
//! HEAD~N  → .spectral/sessions            (N ticks back — placeholder)
//! ...     → .spectral/garden              (garden paths — placeholder)
//! ```
//!
//! Unknown references return `None`.

use std::path::PathBuf;

use crate::session::Session;

/// Resolve a git-like reference to an absolute path within `.spectral/`.
///
/// Returns `None` for unknown references.
pub fn resolve(_reference: &str, _session: &Session) -> Option<PathBuf> {
    todo!("not yet implemented")
}

/// Return the most recently modified file in `dir`, or `None` if the
/// directory is absent, empty, or unreadable.
fn most_recent_file(_dir: &std::path::Path) -> Option<PathBuf> {
    todo!("not yet implemented")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn init_session(dir: &Path) -> Session {
        Session::init(dir).expect("Session::init should succeed")
    }

    // --- . resolves to sessions/current ---

    #[test]
    fn resolve_dot_returns_current_session() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve(".", &session);

        assert!(result.is_some(), ". should resolve to something");
        let path = result.unwrap();
        assert!(
            path.to_str()
                .unwrap()
                .contains(&format!("{}sessions", std::path::MAIN_SEPARATOR)),
            ". should resolve under sessions/, got: {}",
            path.display()
        );
        assert!(
            path.ends_with("current"),
            ". should resolve to sessions/current, got: {}",
            path.display()
        );
    }

    // --- ~ resolves to gestalt ---

    #[test]
    fn resolve_tilde_returns_gestalt_root() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve("~", &session);

        assert!(result.is_some(), "~ should resolve to something");
        let path = result.unwrap();
        assert!(
            path.ends_with("gestalt"),
            "~ should resolve to gestalt/, got: {}",
            path.display()
        );
        assert!(path.is_dir(), "gestalt path should be a directory");
    }

    // --- HEAD resolves to HEAD file ---

    #[test]
    fn resolve_head_returns_head_file() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve("HEAD", &session);

        assert!(result.is_some(), "HEAD should resolve to something");
        let path = result.unwrap();
        assert!(
            path.ends_with("HEAD"),
            "HEAD should resolve to the HEAD file, got: {}",
            path.display()
        );
        assert!(path.is_file(), "HEAD path should be a file");
    }

    // --- unknown reference returns None ---

    #[test]
    fn resolve_unknown_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        assert!(resolve("notaref", &session).is_none());
        assert!(resolve("", &session).is_none());
        assert!(resolve("MERGE_HEAD", &session).is_none());
        assert!(resolve("@{upstream}", &session).is_none());
    }

    // --- .. resolves to sessions/parent ---

    #[test]
    fn resolve_dotdot_returns_parent_session() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve("..", &session);

        assert!(result.is_some(), ".. should resolve to something");
        let path = result.unwrap();
        assert!(
            path.ends_with("parent"),
            ".. should resolve to sessions/parent, got: {}",
            path.display()
        );
    }

    // --- ^ returns None when crystals/ is empty ---

    #[test]
    fn resolve_caret_returns_none_when_crystals_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve("^", &session);
        assert!(
            result.is_none(),
            "^ should return None when crystals/ is empty"
        );
    }

    // --- ^ returns most recent crystal when one exists ---

    #[test]
    fn resolve_caret_returns_most_recent_crystal() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let crystals_dir = session.root().join("crystals");
        let crystal_path = crystals_dir.join("c001.crystal");
        std::fs::write(&crystal_path, "data").unwrap();

        let result = resolve("^", &session);
        assert!(result.is_some(), "^ should return the crystal file");
        assert_eq!(
            result.unwrap(),
            crystal_path,
            "^ should return the only crystal file"
        );
    }

    // --- HEAD~N resolves to sessions/ ---

    #[test]
    fn resolve_head_tilde_n_returns_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve("HEAD~3", &session);
        assert!(result.is_some(), "HEAD~3 should resolve to something");
        let path = result.unwrap();
        assert!(
            path.ends_with("sessions"),
            "HEAD~3 should resolve to sessions/, got: {}",
            path.display()
        );
    }

    // --- HEAD~ with non-numeric suffix returns None ---

    #[test]
    fn resolve_head_tilde_nonnumeric_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        assert!(
            resolve("HEAD~abc", &session).is_none(),
            "HEAD~abc should return None (non-numeric suffix)"
        );
    }

    // --- ... resolves to garden ---

    #[test]
    fn resolve_ellipsis_returns_garden() {
        let tmp = tempfile::tempdir().unwrap();
        let session = init_session(tmp.path());

        let result = resolve("...", &session);
        assert!(result.is_some(), "... should resolve to something");
        let path = result.unwrap();
        assert!(
            path.ends_with("garden"),
            "... should resolve to garden/, got: {}",
            path.display()
        );
    }
}
