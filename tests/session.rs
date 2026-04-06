//! Integration tests for `spectral init` and the .spectral directory.
//!
//! TDD: These tests are written BEFORE the implementation.
//! They should FAIL until src/session.rs is implemented.

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

    // .spectral/ exists
    assert!(
        tmp.path().join(".spectral").is_dir(),
        ".spectral directory should exist"
    );

    // Subdirectories exist
    assert!(
        tmp.path().join(".spectral/gestalt").is_dir(),
        ".spectral/gestalt should exist"
    );
    assert!(
        tmp.path().join(".spectral/sessions").is_dir(),
        ".spectral/sessions should exist"
    );
    assert!(
        tmp.path().join(".spectral/crystals").is_dir(),
        ".spectral/crystals should exist"
    );

    // HEAD file exists and is non-empty
    let head_path = tmp.path().join(".spectral/HEAD");
    assert!(head_path.exists(), ".spectral/HEAD should exist");
    let head = std::fs::read_to_string(&head_path).unwrap();
    assert!(!head.trim().is_empty(), "HEAD should contain a timestamp");

    // HEAD should be parseable as a u64 (unix timestamp)
    let ts: u64 = head.trim().parse().expect("HEAD should contain a unix timestamp");
    assert!(ts > 0, "timestamp should be positive");

    // log file exists and has the init entry
    let log_path = tmp.path().join(".spectral/log");
    assert!(log_path.exists(), ".spectral/log should exist");
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        log.contains("init"),
        ".spectral/log should contain 'init', got:\n{log}"
    );
    assert!(
        log.contains("Garden planted"),
        ".spectral/log should contain 'Garden planted', got:\n{log}"
    );
    assert!(
        log.contains("0.0"),
        ".spectral/log should contain '0.0' (growth), got:\n{log}"
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

    // .spectral/ should still exist with subdirs
    assert!(tmp.path().join(".spectral").is_dir());
    assert!(tmp.path().join(".spectral/gestalt").is_dir());
    assert!(tmp.path().join(".spectral/sessions").is_dir());
    assert!(tmp.path().join(".spectral/crystals").is_dir());
}
