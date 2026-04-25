//! Tick history log — `.spectral/log`.
//!
//! Tab-delimited format, one entry per line:
//! ```text
//! {timestamp}\t{operation}\t{description}\t{growth_delta}
//! ```
//!
//! Example:
//! ```text
//! 1712390400	init	Garden planted	0.0
//! 1712390460	tick	focus on @systems	0.03
//! ```

use std::io;
use std::path::Path;

use crate::session::Session;

/// One entry in the tick history log.
pub struct LogEntry {
    pub timestamp: u64,
    pub operation: String,
    pub description: String,
    pub growth_delta: f64,
}

/// Append a single entry to `.spectral/log`.
///
/// Creates the file if it does not exist.
pub fn append(session: &Session, operation: &str, description: &str, growth_delta: f64) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("{}\t{}\t{}\t{}\n", ts, operation, description, growth_delta);
    if let Err(e) = append_line(session.root().join("log").as_path(), &line) {
        eprintln!("spectral log: failed to write log entry: {}", e);
    }
}

/// Read all log entries from `.spectral/log`.
///
/// Returns an empty `Vec` if the file does not exist or cannot be read.
pub fn read_log(session: &Session) -> Vec<LogEntry> {
    let path = session.root().join("log");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    parse_log(&content)
}

/// Parse log file content into `LogEntry` values.
///
/// Lines that do not conform to the four-field tab-delimited format are
/// silently skipped so that partial or future-format files degrade
/// gracefully.
fn parse_log(content: &str) -> Vec<LogEntry> {
    content
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            let ts: u64 = parts.next()?.trim().parse().ok()?;
            let operation = parts.next()?.to_string();
            let description = parts.next()?.to_string();
            let growth_delta: f64 = parts.next()?.trim().parse().ok()?;
            Some(LogEntry {
                timestamp: ts,
                operation,
                description,
                growth_delta,
            })
        })
        .collect()
}

/// Format log entries for display.
///
/// `oneline = true`  — one line per entry: `{timestamp}\t{operation}\t{description}\t+{growth}`
/// `oneline = false` — multi-line block per entry with a header timestamp line
pub fn format_log(entries: &[LogEntry], oneline: bool) -> String {
    if entries.is_empty() {
        return String::from("(no log entries)\n");
    }

    let mut out = String::new();
    for entry in entries {
        if oneline {
            out.push_str(&format!(
                "{}\t{}\t{}\t+{}\n",
                entry.timestamp, entry.operation, entry.description, entry.growth_delta
            ));
        } else {
            out.push_str(&format!(
                "tick   {}\nop     {}\ndesc   {}\ngrowth +{}\n\n",
                entry.timestamp, entry.operation, entry.description, entry.growth_delta
            ));
        }
    }
    out
}

/// Append a raw line to a file, creating it if needed.
fn append_line(path: &Path, line: &str) -> io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Build a minimal Session we can use in tests by creating a `.spectral`
    /// directory and calling `Session::find`.
    fn make_session() -> (TempDir, Session) {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".spectral")).unwrap();
        let session = Session::find(tmp.path()).expect("session should be found after mkdir");
        (tmp, session)
    }

    #[test]
    fn read_empty_log_returns_empty() {
        let (_tmp, session) = make_session();
        // No log file written — should return empty vec.
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
        // Multi-line: should have more than one non-empty line for the single entry.
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
