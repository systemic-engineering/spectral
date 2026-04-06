//! Tick history log — `.spectral/log`.
//!
//! Tab-delimited format, one entry per line:
//! ```text
//! {timestamp}\t{operation}\t{description}\t{growth_delta}
//! ```

use std::path::Path;
use std::io;

use crate::session::Session;

/// One entry in the tick history log.
pub struct LogEntry {
    pub timestamp: u64,
    pub operation: String,
    pub description: String,
    pub growth_delta: f64,
}

/// Append a single entry to `.spectral/log`.
pub fn append(_session: &Session, _operation: &str, _description: &str, _growth_delta: f64) {
    todo!("log::append not yet implemented")
}

/// Read all log entries from `.spectral/log`.
pub fn read_log(_session: &Session) -> Vec<LogEntry> {
    todo!("log::read_log not yet implemented")
}

/// Format log entries for display.
pub fn format_log(_entries: &[LogEntry], _oneline: bool) -> String {
    todo!("log::format_log not yet implemented")
}

fn append_line(_path: &Path, _line: &str) -> io::Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_session() -> (TempDir, Session) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".spectral")).unwrap();
        let session = Session::find(tmp.path()).expect("session should be found after mkdir");
        (tmp, session)
    }

    #[test]
    fn read_empty_log_returns_empty() {
        let (_tmp, session) = make_session();
        let entries = read_log(&session);
        assert!(entries.is_empty(), "expected empty vec, got {} entries", entries.len());
    }

    #[test]
    fn append_and_read_roundtrip() {
        let (_tmp, session) = make_session();
        append(&session, "tick", "focus on @systems", 0.03);
        let entries = read_log(&session);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].operation, "tick");
        assert_eq!(entries[0].description, "focus on @systems");
        assert!((entries[0].growth_delta - 0.03).abs() < 1e-9);
        assert!(entries[0].timestamp > 0);
    }

    #[test]
    fn format_oneline() {
        let entries = vec![
            LogEntry {
                timestamp: 1712390400,
                operation: "init".to_string(),
                description: "Garden planted".to_string(),
                growth_delta: 0.0,
            },
            LogEntry {
                timestamp: 1712390460,
                operation: "tick".to_string(),
                description: "focus on @systems".to_string(),
                growth_delta: 0.03,
            },
        ];
        let output = format_log(&entries, true);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2, "oneline format should have one line per entry");
        assert!(lines[0].contains("init"), "first line should mention operation");
        assert!(lines[0].contains("Garden planted"), "first line should mention description");
        assert!(lines[1].contains("tick"));
        assert!(lines[1].contains("focus on @systems"));
    }

    #[test]
    fn format_full() {
        let entries = vec![LogEntry {
            timestamp: 1712390400,
            operation: "shatter".to_string(),
            description: "crystal formed at 77%".to_string(),
            growth_delta: 0.05,
        }];
        let output = format_log(&entries, false);
        let non_empty_lines: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            non_empty_lines.len() >= 3,
            "full format should have multiple lines per entry, got:\n{output}"
        );
        assert!(output.contains("1712390400"), "should show timestamp");
        assert!(output.contains("shatter"), "should show operation");
        assert!(output.contains("crystal formed at 77%"), "should show description");
    }

    #[test]
    fn multiple_appends_preserve_order() {
        let (_tmp, session) = make_session();
        append(&session, "tick",    "first",  0.01);
        append(&session, "tock",    "second", 0.02);
        append(&session, "shatter", "third",  0.05);
        let entries = read_log(&session);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].operation, "tick");
        assert_eq!(entries[1].operation, "tock");
        assert_eq!(entries[2].operation, "shatter");
        assert_eq!(entries[0].description, "first");
        assert_eq!(entries[1].description, "second");
        assert_eq!(entries[2].description, "third");
    }
}
