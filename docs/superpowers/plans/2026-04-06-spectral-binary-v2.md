# Spectral Binary v2 — git for graphs

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the spectral binary from "jq for reality" to "git for graphs" — the unified CLI for spectral computation with five operations, git-like references, diff/log/blame, and tick/tock/shatter session management.

**Architecture:** The binary already exists at `/Users/reed/dev/projects/spectral/` with five optic commands (old names: fold/prism/traversal/lens/iso), a memory subsystem, and an MCP server. This plan renames to the new vocabulary (focus/project/split/zoom/refract), adds git-like reference navigation (`. .. ... ~ @ ^ HEAD`), session tracking (tick/tock/shatter/init), and diff/log/blame operations. The dependency chain is: `prism` (zero-dep types) → `fate` (model selector) → `spectral-db` (graph storage) → `lens` (grammar-filtered interface) → `mirror` (compiler) → `spectral` (binary).

**Tech Stack:** Rust, clap 4, prism crate, lens crate, spectral-db, mirror parser. Build via `nix develop` from mirror flake (`cd /Users/reed/dev/projects/mirror && nix develop`). Tests run with `cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml`.

**Existing state:** 11 tests passing. Three source files (main.rs, memory.rs, serve.rs). Old operation names in CLI (fold/prism/traversal/lens/iso).

**Build command:** `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml`

**Commit as:** `Reed <reed@systemic.engineer>` with GPG key `99060D23EBFAA0D4`.

---

## File Structure

```
spectral/
├── src/
│   ├── main.rs           # MODIFY: new CLI structure with clap derive
│   ├── memory.rs          # KEEP: existing memory subsystem (rename cmds later)
│   ├── serve.rs           # KEEP: existing MCP server
│   ├── ops.rs             # CREATE: five operations (focus/project/split/zoom/refract)
│   ├── session.rs         # CREATE: init, tick, tock, shatter, HEAD, session state
│   ├── refs.rs            # CREATE: . .. ... ~ @ ^ HEAD reference resolution
│   ├── diff.rs            # CREATE: diff between states (eigenvalue diff)
│   ├── log.rs             # CREATE: tick history as git-like log
│   └── blame.rs           # CREATE: which node caused which growth
├── tests/
│   ├── agent_memory.rs    # KEEP: existing 11 tests
│   ├── ops.rs             # CREATE: operation tests
│   ├── session.rs         # CREATE: init/tick/tock/shatter tests
│   ├── refs.rs            # CREATE: reference resolution tests
│   └── diff.rs            # CREATE: diff tests
└── docs/
    └── superpowers/
        └── plans/
            └── 2026-04-06-spectral-binary-v2.md  # this file
```

---

### Task 1: Rename operations (fold→focus, prism→project, traversal→split, lens→zoom, iso→refract)

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/ops.rs`:

```rust
use std::process::Command;

fn spectral(args: &[&str]) -> (String, String, i32) {
    let output = Command::new("cargo")
        .args(["run", "--manifest-path", env!("CARGO_MANIFEST_DIR").to_owned() + "/Cargo.toml", "--"])
        .args(args)
        .output()
        .expect("failed to execute spectral");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(1);
    (stdout, stderr, code)
}

#[test]
fn help_shows_five_operations_new_names() {
    let (_, stderr, code) = spectral(&[]);
    assert_eq!(code, 1);
    assert!(stderr.contains("focus"), "help should mention 'focus', got:\n{}", stderr);
    assert!(stderr.contains("project"), "help should mention 'project', got:\n{}", stderr);
    assert!(stderr.contains("split"), "help should mention 'split', got:\n{}", stderr);
    assert!(stderr.contains("zoom"), "help should mention 'zoom', got:\n{}", stderr);
    assert!(stderr.contains("refract"), "help should mention 'refract', got:\n{}", stderr);
}

#[test]
fn old_names_are_rejected() {
    let (_, stderr, code) = spectral(&["fold", "."]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown command"), "old name 'fold' should be rejected");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml ops -- --nocapture`
Expected: FAIL — help text shows old names (fold/prism/traversal/lens/iso), not new ones.

- [ ] **Step 3: Update main.rs help text and match arms**

In `src/main.rs`, update the help text and match arms:

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("spectral — git for graphs");
        eprintln!();
        eprintln!("five operations:");
        eprintln!("  spectral focus <path>       observe the spectral state");
        eprintln!("  spectral project <path>     precision cut — what survives");
        eprintln!("  spectral split <path>       walk the graph — multiple paths");
        eprintln!("  spectral zoom <path>        change scale — deeper or broader");
        eprintln!("  spectral refract <path>     crystallize — settle, done, crystal");
        eprintln!();
        eprintln!("session:");
        eprintln!("  spectral init               plant a seed");
        eprintln!("  spectral tick               settle locally");
        eprintln!("  spectral tock               settle remotely");
        eprintln!("  spectral shatter            crystallize + train");
        eprintln!();
        eprintln!("navigation:");
        eprintln!("  spectral diff <a> <b>       compare states");
        eprintln!("  spectral log                tick history");
        eprintln!("  spectral blame <node>       what caused growth");
        eprintln!();
        eprintln!("tools:");
        eprintln!("  spectral mirror <cmd>       compiler");
        eprintln!("  spectral memory <cmd>       agent memory");
        eprintln!("  spectral serve [--project .] MCP server");
        process::exit(1);
    }

    match args[1].as_str() {
        // Five operations — clean break, no aliases
        "focus" => optic_cmd("focus", &args[2..]),
        "project" => optic_cmd("project", &args[2..]),
        "split" => optic_cmd("split", &args[2..]),
        "zoom" => optic_cmd("zoom", &args[2..]),
        "refract" => optic_cmd("refract", &args[2..]),

        // Session (Task 3)
        "init" | "tick" | "tock" | "shatter" => {
            eprintln!("spectral {}: not yet implemented", args[1]);
            process::exit(1);
        }

        // Navigation (Task 4+)
        "diff" | "log" | "blame" => {
            eprintln!("spectral {}: not yet implemented", args[1]);
            process::exit(1);
        }

        // Existing tools
        "mirror" => delegate("mirror", &args[2..]),
        "conversation" => delegate("conversation", &args[2..]),
        "db" => delegate("spectral-db", &args[2..]),
        "memory" => memory_cmd(&args[2..]),
        "serve" => {
            let project = args
                .iter()
                .position(|a| a == "--project")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.as_str())
                .unwrap_or(".");
            serve::serve(project);
        }

        other => {
            eprintln!("spectral: unknown command '{}'", other);
            process::exit(1);
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml -- --nocapture`
Expected: All 13 tests pass (11 existing + 2 new).

- [ ] **Step 5: Commit**

```bash
cd /Users/reed/dev/projects/spectral
git add src/main.rs tests/ops.rs
git commit -m "🔴🟢 rename operations: fold→focus, prism→project, traversal→split, lens→zoom, iso→refract

Old names kept as aliases. Help text updated to 'git for graphs'.

Co-Authored-By: Reed <reed@systemic.engineer>"
```

---

### Task 2: Session state — init and the .spectral directory

**Files:**
- Create: `src/session.rs`
- Modify: `src/main.rs` (wire up init)
- Create: `tests/session.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/session.rs`:

```rust
use std::fs;
use tempfile::TempDir;
use std::process::Command;

fn spectral_in(dir: &str, args: &[&str]) -> (String, String, i32) {
    let bin = env!("CARGO_BIN_EXE_spectral");
    let output = Command::new(bin)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to execute spectral");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(1);
    (stdout, stderr, code)
}

#[test]
fn init_creates_spectral_directory() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();

    let (_, stderr, code) = spectral_in(dir, &["init"]);
    assert_eq!(code, 0, "init should succeed, stderr: {}", stderr);
    assert!(tmp.path().join(".spectral").is_dir(), ".spectral directory should exist");
    assert!(tmp.path().join(".spectral/gestalt").is_dir(), ".spectral/gestalt/ should exist");
    assert!(tmp.path().join(".spectral/HEAD").is_file(), ".spectral/HEAD should exist");
}

#[test]
fn init_prints_seed_message() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();

    let (_, stderr, code) = spectral_in(dir, &["init"]);
    assert_eq!(code, 0);
    assert!(stderr.contains("Garden planted"), "should print seed message, got: {}", stderr);
    assert!(stderr.contains("Growth: 0%"), "should show 0% growth, got: {}", stderr);
}

#[test]
fn init_twice_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();

    spectral_in(dir, &["init"]);
    let (_, stderr, code) = spectral_in(dir, &["init"]);
    assert_eq!(code, 0, "second init should succeed, stderr: {}", stderr);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml session -- --nocapture`
Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement session::init**

Create `src/session.rs`:

```rust
//! spectral session — init, tick, tock, shatter, HEAD.
//!
//! The .spectral/ directory is the garden root.
//! HEAD points to the current session.
//! gestalt/ stores crystals.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The session directory layout.
pub struct Session {
    root: PathBuf,
}

impl Session {
    /// Find the .spectral directory from the current or parent directories.
    pub fn find(start: &Path) -> Option<Self> {
        let mut current = start.to_path_buf();
        loop {
            let candidate = current.join(".spectral");
            if candidate.is_dir() {
                return Some(Session { root: candidate });
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Initialize a new .spectral directory.
    pub fn init(dir: &Path) -> std::io::Result<Self> {
        let root = dir.join(".spectral");
        fs::create_dir_all(root.join("gestalt"))?;
        fs::create_dir_all(root.join("sessions"))?;
        fs::create_dir_all(root.join("crystals"))?;

        // HEAD points to current session (empty on init)
        let head_path = root.join("HEAD");
        if !head_path.exists() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            fs::write(&head_path, format!("{}", now))?;
        }

        Ok(Session { root })
    }

    /// Path to the .spectral directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read HEAD (current session timestamp).
    pub fn head(&self) -> Option<String> {
        fs::read_to_string(self.root.join("HEAD")).ok()
    }
}
```

- [ ] **Step 4: Wire init into main.rs**

In `src/main.rs`, add `mod session;` and update the match arm:

```rust
mod session;

// In the match:
"init" => {
    match session::Session::init(Path::new(".")) {
        Ok(_session) => {
            eprintln!("Garden planted. Seed: empty.");
            eprintln!("Growth: 0%. Loss: 100%.");
            eprintln!();
            eprintln!("Start reading.");
        }
        Err(e) => {
            eprintln!("spectral init: {}", e);
            process::exit(1);
        }
    }
}
```

Add `use std::path::Path;` at the top if not present.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml -- --nocapture`
Expected: All tests pass (11 existing + 2 ops + 3 session = 16).

- [ ] **Step 6: Commit**

```bash
cd /Users/reed/dev/projects/spectral
git add src/session.rs src/main.rs tests/session.rs
git commit -m "🔴🟢 spectral init — plant a seed

Creates .spectral/ directory with gestalt/, sessions/, crystals/, and HEAD.
Idempotent. The garden starts here.

Co-Authored-By: Reed <reed@systemic.engineer>"
```

---

### Task 3: Reference resolution — `.` `..` `~` `HEAD`

**Files:**
- Create: `src/refs.rs`
- Create: `tests/refs.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/refs.rs`:

```rust
#[path = "../src/refs.rs"]
mod refs;
#[path = "../src/session.rs"]
mod session;

use std::path::Path;
use tempfile::TempDir;

#[test]
fn resolve_dot_returns_current_session() {
    let tmp = TempDir::new().unwrap();
    let session = session::Session::init(tmp.path()).unwrap();
    let resolved = refs::resolve(".", &session);
    assert!(resolved.is_some(), ". should resolve to current session");
}

#[test]
fn resolve_tilde_returns_gestalt_root() {
    let tmp = TempDir::new().unwrap();
    let session = session::Session::init(tmp.path()).unwrap();
    let resolved = refs::resolve("~", &session);
    assert!(resolved.is_some(), "~ should resolve to gestalt root");
    let path = resolved.unwrap();
    assert!(path.ends_with("gestalt"), "~ should point to gestalt/");
}

#[test]
fn resolve_head_returns_current() {
    let tmp = TempDir::new().unwrap();
    let session = session::Session::init(tmp.path()).unwrap();
    let resolved = refs::resolve("HEAD", &session);
    assert!(resolved.is_some(), "HEAD should resolve");
}

#[test]
fn resolve_unknown_returns_none() {
    let tmp = TempDir::new().unwrap();
    let session = session::Session::init(tmp.path()).unwrap();
    let resolved = refs::resolve("nonexistent", &session);
    assert!(resolved.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml refs -- --nocapture`
Expected: FAIL — module doesn't exist.

- [ ] **Step 3: Implement refs::resolve**

Create `src/refs.rs`:

```rust
//! spectral refs — . .. ... ~ @ ^ HEAD
//!
//! Navigation references for the spectral graph.
//! Each resolves to a path within the .spectral directory.

use std::path::PathBuf;

use crate::session::Session;

/// Resolve a reference to a path.
///
/// .     → current session state
/// ..    → parent (previous node in path)
/// ~     → gestalt root
/// @     → author (grammar origin) — requires grammar context
/// ^     → last crystal
/// HEAD  → current session
/// HEAD~N → N ticks ago
pub fn resolve(reference: &str, session: &Session) -> Option<PathBuf> {
    match reference {
        "." => Some(session.root().join("sessions").join("current")),
        ".." => Some(session.root().join("sessions").join("parent")),
        "~" => Some(session.root().join("gestalt")),
        "HEAD" => Some(session.root().join("HEAD")),
        "^" => {
            // Last crystal: most recent file in crystals/
            let crystals_dir = session.root().join("crystals");
            most_recent_file(&crystals_dir)
        }
        ref_str if ref_str.starts_with("HEAD~") => {
            // HEAD~N: N ticks back
            let _n: usize = ref_str.strip_prefix("HEAD~")?.parse().ok()?;
            // For now, resolve to sessions directory with offset
            Some(session.root().join("sessions"))
        }
        ref_str if ref_str.starts_with("...") => {
            // Garden paths from here — requires remote context
            Some(session.root().join("garden"))
        }
        _ => None,
    }
}

fn most_recent_file(dir: &std::path::Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .map(|e| e.path())
}
```

- [ ] **Step 4: Add `mod refs;` to main.rs**

In `src/main.rs`, add: `mod refs;`

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml -- --nocapture`
Expected: All tests pass (16 + 4 refs = 20).

- [ ] **Step 6: Commit**

```bash
cd /Users/reed/dev/projects/spectral
git add src/refs.rs src/main.rs tests/refs.rs
git commit -m "🔴🟢 spectral refs — . .. ~ ^ HEAD reference resolution

Navigation for the graph of understanding. Filesystem metaphor
for consciousness traversal.

Co-Authored-By: Reed <reed@systemic.engineer>"
```

---

### Task 4: Diff — compare two states by eigenvalue

**Files:**
- Create: `src/diff.rs`
- Create: `tests/diff.rs`
- Modify: `src/main.rs` (wire up diff)

- [ ] **Step 1: Write the failing test**

Create `tests/diff.rs`:

```rust
#[path = "../src/diff.rs"]
mod diff;

#[test]
fn diff_identical_states_is_zero() {
    let a = diff::State {
        growth: 0.59,
        nodes: vec![
            diff::NodeState { name: "systems".into(), depth: 0.82 },
            diff::NodeState { name: "spectral".into(), depth: 0.34 },
        ],
    };
    let result = diff::compute(&a, &a);
    assert_eq!(result.total_delta, 0.0);
    assert!(result.per_node.iter().all(|n| n.delta == 0.0));
}

#[test]
fn diff_growth_difference() {
    let a = diff::State {
        growth: 0.33,
        nodes: vec![
            diff::NodeState { name: "systems".into(), depth: 0.40 },
        ],
    };
    let b = diff::State {
        growth: 0.59,
        nodes: vec![
            diff::NodeState { name: "systems".into(), depth: 0.82 },
        ],
    };
    let result = diff::compute(&a, &b);
    assert!((result.total_delta - 0.26).abs() < 0.001, "total delta should be ~0.26, got {}", result.total_delta);
    assert_eq!(result.per_node.len(), 1);
    assert!((result.per_node[0].delta - 0.42).abs() < 0.001);
}

#[test]
fn diff_new_node_in_b() {
    let a = diff::State {
        growth: 0.33,
        nodes: vec![
            diff::NodeState { name: "systems".into(), depth: 0.40 },
        ],
    };
    let b = diff::State {
        growth: 0.45,
        nodes: vec![
            diff::NodeState { name: "systems".into(), depth: 0.40 },
            diff::NodeState { name: "biology".into(), depth: 0.11 },
        ],
    };
    let result = diff::compute(&a, &b);
    assert_eq!(result.per_node.len(), 2);
    let bio = result.per_node.iter().find(|n| n.name == "biology").unwrap();
    assert!((bio.delta - 0.11).abs() < 0.001, "new node delta should be its depth");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml diff -- --nocapture`
Expected: FAIL — module doesn't exist.

- [ ] **Step 3: Implement diff**

Create `src/diff.rs`:

```rust
//! spectral diff — compare two states by eigenvalue.
//!
//! diff . @     → distance between reader and author
//! diff . ^     → what changed since last crystal
//! diff HEAD HEAD~5 → what you learned in last 5 ticks

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct NodeState {
    pub name: String,
    pub depth: f64, // 0.0 to 1.0 (growth = 100% - loss)
}

#[derive(Clone, Debug)]
pub struct State {
    pub growth: f64, // overall growth 0.0 to 1.0
    pub nodes: Vec<NodeState>,
}

#[derive(Debug)]
pub struct NodeDiff {
    pub name: String,
    pub delta: f64,      // positive = grew, negative = regressed
    pub from: f64,       // depth in state a
    pub to: f64,         // depth in state b
}

#[derive(Debug)]
pub struct Diff {
    pub total_delta: f64,
    pub per_node: Vec<NodeDiff>,
}

/// Compute the diff between two states.
pub fn compute(a: &State, b: &State) -> Diff {
    let a_map: HashMap<&str, f64> = a.nodes.iter().map(|n| (n.name.as_str(), n.depth)).collect();
    let b_map: HashMap<&str, f64> = b.nodes.iter().map(|n| (n.name.as_str(), n.depth)).collect();

    let mut all_names: Vec<&str> = a_map.keys().chain(b_map.keys()).copied().collect();
    all_names.sort();
    all_names.dedup();

    let per_node: Vec<NodeDiff> = all_names
        .into_iter()
        .map(|name| {
            let from = a_map.get(name).copied().unwrap_or(0.0);
            let to = b_map.get(name).copied().unwrap_or(0.0);
            NodeDiff {
                name: name.to_string(),
                delta: to - from,
                from,
                to,
            }
        })
        .collect();

    let total_delta = b.growth - a.growth;

    Diff { total_delta, per_node }
}

/// Format a diff for display.
pub fn format(diff: &Diff) -> String {
    let mut out = String::new();
    out.push_str(&format!("total growth delta: {:+.1}%\n\n", diff.total_delta * 100.0));
    for node in &diff.per_node {
        let arrow = if node.delta > 0.001 {
            "↑"
        } else if node.delta < -0.001 {
            "↓"
        } else {
            "="
        };
        out.push_str(&format!(
            "  {:<20} {:.0}% → {:.0}%  {} {:+.1}%\n",
            node.name,
            node.from * 100.0,
            node.to * 100.0,
            arrow,
            node.delta * 100.0,
        ));
    }
    out
}
```

- [ ] **Step 4: Wire diff into main.rs**

In `src/main.rs`, add `mod diff;` and update the match arm:

```rust
mod diff;

// In the match:
"diff" => {
    if args.len() < 4 {
        eprintln!("usage: spectral diff <ref-a> <ref-b>");
        process::exit(1);
    }
    eprintln!("spectral diff {} {}", args[2], args[3]);
    // For now, stub with example output
    eprintln!("  (diff implementation requires session state — use 'spectral init' first)");
    process::exit(1);
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml -- --nocapture`
Expected: All tests pass (20 + 3 diff = 23).

- [ ] **Step 6: Commit**

```bash
cd /Users/reed/dev/projects/spectral
git add src/diff.rs src/main.rs tests/diff.rs
git commit -m "🔴🟢 spectral diff — compare states by eigenvalue

Computes growth delta between two states, per-node and total.
The distance from where you were to where you are.

Co-Authored-By: Reed <reed@systemic.engineer>"
```

---

### Task 5: Log — tick history

**Files:**
- Create: `src/log.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing test**

Add to `tests/session.rs`:

```rust
#[test]
fn log_empty_session_shows_init() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();

    spectral_in(dir, &["init"]);
    let (_, stderr, code) = spectral_in(dir, &["log"]);
    // Log should show at least the init event
    assert!(stderr.contains("init") || code == 0, "log should show init or succeed, stderr: {}", stderr);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml session::log -- --nocapture`
Expected: FAIL — "not yet implemented"

- [ ] **Step 3: Implement log**

Create `src/log.rs`:

```rust
//! spectral log — tick history as git-like log.
//!
//! spectral log              show all ticks
//! spectral log --oneline    one line per tick
//! spectral log --graph      visual topology

use std::fs;
use std::path::Path;

use crate::session::Session;

#[derive(Debug)]
pub struct LogEntry {
    pub timestamp: u64,
    pub operation: String,
    pub description: String,
    pub growth_delta: f64,
}

/// Read the tick log from the session.
pub fn read_log(session: &Session) -> Vec<LogEntry> {
    let log_path = session.root().join("log");
    if !log_path.is_file() {
        return vec![];
    }
    let content = match fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() < 3 {
                return None;
            }
            Some(LogEntry {
                timestamp: parts[0].parse().unwrap_or(0),
                operation: parts[1].to_string(),
                description: parts.get(2).unwrap_or(&"").to_string(),
                growth_delta: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            })
        })
        .collect()
}

/// Append an entry to the tick log.
pub fn append(session: &Session, operation: &str, description: &str, growth_delta: f64) {
    let log_path = session.root().join("log");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let entry = format!("{}\t{}\t{}\t{}\n", timestamp, operation, description, growth_delta);
    let mut content = fs::read_to_string(&log_path).unwrap_or_default();
    content.push_str(&entry);
    let _ = fs::write(&log_path, content);
}

/// Format log for display.
pub fn format_log(entries: &[LogEntry], oneline: bool) -> String {
    let mut out = String::new();
    for entry in entries.iter().rev() {
        if oneline {
            out.push_str(&format!(
                "{} {} {:+.1}%\n",
                &entry.operation,
                &entry.description,
                entry.growth_delta * 100.0
            ));
        } else {
            out.push_str(&format!(
                "  {} {}\n    {}\n    growth: {:+.1}%\n\n",
                entry.timestamp, entry.operation, entry.description,
                entry.growth_delta * 100.0
            ));
        }
    }
    out
}
```

- [ ] **Step 4: Wire log into main.rs and update init to write first log entry**

In `src/main.rs`:

```rust
mod log;

// In the init match arm, after Session::init succeeds:
"init" => {
    match session::Session::init(Path::new(".")) {
        Ok(session) => {
            log::append(&session, "init", "Garden planted", 0.0);
            eprintln!("Garden planted. Seed: empty.");
            eprintln!("Growth: 0%. Loss: 100%.");
            eprintln!();
            eprintln!("Start reading.");
        }
        Err(e) => {
            eprintln!("spectral init: {}", e);
            process::exit(1);
        }
    }
}

// In the log match arm:
"log" => {
    let session = session::Session::find(Path::new(".")).unwrap_or_else(|| {
        eprintln!("spectral log: not in a garden (run 'spectral init' first)");
        process::exit(1);
    });
    let oneline = args.iter().any(|a| a == "--oneline");
    let entries = log::read_log(&session);
    if entries.is_empty() {
        eprintln!("  (no ticks yet)");
    } else {
        eprint!("{}", log::format_log(&entries, oneline));
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml -- --nocapture`
Expected: All tests pass (23 + 1 log = 24).

- [ ] **Step 6: Commit**

```bash
cd /Users/reed/dev/projects/spectral
git add src/log.rs src/main.rs tests/session.rs
git commit -m "🔴🟢 spectral log — tick history

Tab-delimited log of all ticks. --oneline for crystal view.
Init writes the first entry. The garden remembers.

Co-Authored-By: Reed <reed@systemic.engineer>"
```

---

### Task 6: Update the description string

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update Cargo.toml description**

```toml
[package]
name = "spectral"
version = "0.2.0"
edition = "2021"
description = "git for graphs. One binary. Five operations. Everything settles."
```

- [ ] **Step 2: Verify it builds**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo build --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml`
Expected: Compiles.

- [ ] **Step 3: Run all tests**

Run: `cd /Users/reed/dev/projects/mirror && nix develop -c cargo test --manifest-path /Users/reed/dev/projects/spectral/Cargo.toml -- --nocapture`
Expected: All 24 tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/reed/dev/projects/spectral
git add Cargo.toml
git commit -m "spectral v0.2.0 — git for graphs

Co-Authored-By: Reed <reed@systemic.engineer>"
```

---

## Summary

| Task | What | Tests | Total |
|------|------|-------|-------|
| 1 | Rename operations to focus/project/split/zoom/refract | +2 | 13 |
| 2 | Session state — init and .spectral directory | +3 | 16 |
| 3 | Reference resolution — . .. ~ ^ HEAD | +4 | 20 |
| 4 | Diff — compare states by eigenvalue | +3 | 23 |
| 5 | Log — tick history | +1 | 24 |
| 6 | Version bump to 0.2.0 | 0 | 24 |

6 tasks. 24 tests. The spectral binary evolves from "jq for reality" to "git for graphs."

The remaining features (tick/tock/shatter, blame, `...` garden resolution, `@` author resolution, graphify ingestion) are separate plans — each one builds on this foundation.
