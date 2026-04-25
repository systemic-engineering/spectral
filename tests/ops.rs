//! Integration tests for spectral operation names.
//!
//! Verifies that:
//! 1. Help text shows the new names: focus/project/split/zoom/refract
//! 2. Old names (fold/prism/traversal/lens/iso) are rejected as unknown command

fn spectral(args: &[&str]) -> (String, String, i32) {
    let bin = env!("CARGO_BIN_EXE_spectral");
    let output = std::process::Command::new(bin)
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
fn help_shows_five_operations_new_names() {
    let (_stdout, stderr, _code) = spectral(&[]);

    assert!(
        stderr.contains("focus"),
        "help should contain 'focus', got:\n{stderr}"
    );
    assert!(
        stderr.contains("project"),
        "help should contain 'project', got:\n{stderr}"
    );
    assert!(
        stderr.contains("split"),
        "help should contain 'split', got:\n{stderr}"
    );
    assert!(
        stderr.contains("zoom"),
        "help should contain 'zoom', got:\n{stderr}"
    );
    assert!(
        stderr.contains("refract"),
        "help should contain 'refract', got:\n{stderr}"
    );
}

#[test]
fn old_names_are_rejected() {
    for old in &["fold", "prism", "traversal", "lens", "iso"] {
        let (_stdout, stderr, code) = spectral(&[old, "."]);
        assert_ne!(
            code, 0,
            "old command '{}' should exit non-zero",
            old
        );
        assert!(
            stderr.contains("unknown command"),
            "old command '{}' should print 'unknown command', got:\n{stderr}",
            old
        );
    }
}
