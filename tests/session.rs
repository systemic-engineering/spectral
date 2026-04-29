//! Integration tests for `spectral init` and the `.git/spectral/` directory.
//!
//! Post-Phase-1: session state lives under `.git/spectral/`, not the
//! legacy `.spectral/`. See `src/session.rs::Session::init`.

fn spectral_in(dir: &str, args: &[&str]) -> (String, String, i32) {
    let bin = env!("CARGO_BIN_EXE_spectral");
    let output = std::process::Command::new(bin)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to execute spectral");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(1),
    )
}

#[test]
fn init_creates_spectral_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_str().unwrap();

    let (_stdout, _stderr, code) = spectral_in(dir, &["init"]);
    assert_eq!(code, 0, "spectral init should exit 0");

    let spectral_dir = tmp.path().join(".git").join("spectral");

    // .git/spectral/ exists
    assert!(
        spectral_dir.is_dir(),
        ".git/spectral directory should exist"
    );

    // Subdirectories exist
    assert!(
        spectral_dir.join("gestalt").is_dir(),
        ".git/spectral/gestalt should exist"
    );
    assert!(
        spectral_dir.join("sessions").is_dir(),
        ".git/spectral/sessions should exist"
    );
    assert!(
        spectral_dir.join("crystals").is_dir(),
        ".git/spectral/crystals should exist"
    );

    // HEAD file exists and is non-empty
    let head_path = spectral_dir.join("HEAD");
    assert!(head_path.exists(), ".git/spectral/HEAD should exist");
    let head = std::fs::read_to_string(&head_path).unwrap();
    assert!(!head.trim().is_empty(), "HEAD should contain a timestamp");

    // HEAD should be parseable as a u64 (unix timestamp)
    let ts: u64 = head.trim().parse().expect("HEAD should contain a unix timestamp");
    assert!(ts > 0, "timestamp should be positive");

    // log file exists and has the init entry
    let log_path = spectral_dir.join("log");
    assert!(log_path.exists(), ".git/spectral/log should exist");
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        log.contains("init"),
        ".git/spectral/log should contain 'init', got:\n{log}"
    );
    assert!(
        log.contains("Garden planted"),
        ".git/spectral/log should contain 'Garden planted', got:\n{log}"
    );
    assert!(
        log.contains("0.0"),
        ".git/spectral/log should contain '0.0' (growth), got:\n{log}"
    );
}

#[test]
fn init_prints_seed_message() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_str().unwrap();

    let (_stdout, stderr, code) = spectral_in(dir, &["init"]);
    assert_eq!(code, 0, "spectral init should exit 0");

    assert!(
        stderr.contains("Garden planted"),
        "stderr should contain 'Garden planted', got:\n{stderr}"
    );
    assert!(
        stderr.contains("Growth: 0%"),
        "stderr should contain 'Growth: 0%', got:\n{stderr}"
    );
}

#[test]
fn log_after_init_shows_entry() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    spectral_in(dir, &["init"]);
    let (_, stderr, code) = spectral_in(dir, &["log"]);
    assert_eq!(code, 0, "log should succeed, stderr: {}", stderr);
    assert!(
        stderr.contains("init") || stderr.contains("Garden"),
        "log should show init entry, got: {}",
        stderr
    );
}

#[test]
fn init_twice_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_str().unwrap();

    // First init
    let (_stdout1, _stderr1, code1) = spectral_in(dir, &["init"]);
    assert_eq!(code1, 0, "first spectral init should exit 0");

    // Second init — should not error
    let (_stdout2, stderr2, code2) = spectral_in(dir, &["init"]);
    assert_eq!(
        code2, 0,
        "second spectral init should also exit 0, stderr:\n{stderr2}"
    );

    // .git/spectral/ should still exist with subdirs
    let spectral_dir = tmp.path().join(".git").join("spectral");
    assert!(spectral_dir.is_dir());
    assert!(spectral_dir.join("gestalt").is_dir());
    assert!(spectral_dir.join("sessions").is_dir());
    assert!(spectral_dir.join("crystals").is_dir());
}
