# spectral tui — v0 spec

> The home Glint lives in before Glint has intelligence.
> A prompt loop. Colored output. Growth status. Navigation.

## What We're Building

An interactive terminal session. Claude Code style. One command:

```
$ spectral tui
```

Enters a prompt loop. The user types. The system responds. The gestalt
updates. The growth is visible. Navigation works.

This is NOT the full Surface → Mirror loop → Shatter pipeline.
This is the shell that pipeline will live in.

## v0 Scope

### In scope
- Interactive prompt with readline (history, ctrl-r search)
- Colored output (growth green, loss amber, paths cyan)
- Status line (growth %, loss %, ticks, current path)
- Navigation: `.` (here), `..` (back), `~` (home), `@` (author)
- Commands: `/tick`, `/tock`, `/shatter`, `/log`, `/status`
- Session integration (reads/writes `.spectral/`)
- Gestalt display (show current growth per grammar)
- Query pass-through to existing optic commands (focus/project/split/zoom/refract)

### Out of scope (v1+)
- Surface model (natural language → mirror query)
- Mirror refinement loop (Fate model selection)
- Shatter rendering (personalized output)
- Reflection (n-1 observations)
- Live garden connection (federation)
- `.shatter` file training

## Dependencies

```toml
# add to spectral/Cargo.toml
rustyline = "14"          # readline prompt, history, completion
colored = "2"             # terminal colors
```

No ratatui. No full TUI framework. A readline loop with colored
println is enough for v0. The complexity is in the pipeline, not
the chrome.

## New Files

### src/tui.rs — The Prompt Loop

```rust
//! spectral tui — interactive session.

use rustyline::Editor;
use colored::*;

pub fn tui() {
    // 1. Find session or prompt to init
    let session = match Session::find(Path::new(".")) {
        Some(s) => s,
        None => {
            eprintln!("No .spectral found. Run spectral init first.");
            return;
        }
    };

    // 2. Load gestalt (or empty)
    let gestalt = Gestalt::load(&session);

    // 3. Print welcome
    print_status(&gestalt);

    // 4. Prompt loop
    let mut rl = Editor::new().unwrap();
    let mut nav = Navigator::new();

    loop {
        let prompt = format!("{} ", "spectral>".dimmed());
        match rl.readline(&prompt) {
            Ok(line) => {
                rl.add_history_entry(&line);
                match handle_input(&line, &session, &mut gestalt, &mut nav) {
                    Action::Continue => {},
                    Action::Quit => break,
                }
                print_status(&gestalt);
            }
            Err(_) => break,
        }
    }
}
```

### src/navigate.rs — Reference Resolution

```rust
//! Navigation: . / .. / ... / ~ / @ / ^ / HEAD

pub struct Navigator {
    path: Vec<String>,     // current path stack
    history: Vec<Vec<String>>,  // for .. (back with context)
}

impl Navigator {
    pub fn resolve(&mut self, input: &str) -> NavResult {
        match input {
            "."   => NavResult::Here(self.current()),
            ".."  => NavResult::Back(self.pop()),
            "..." => NavResult::Garden, // future: show others' paths
            "~"   => NavResult::Home,
            "@"   => NavResult::Author,
            "^"   => NavResult::LastCrystal,
            input if input.starts_with("HEAD") => {
                NavResult::Head(parse_head_ref(input))
            }
            _ => NavResult::Query(input.to_string()),
        }
    }
}
```

### src/pipeline.rs — v0 Query Pipeline

```rust
//! v0 pipeline: direct dispatch, no models yet.
//!
//! v0: parse command → single optic pass → plain text output
//! v1: Surface → Mirror loop(Fate → Models) → Shatter

pub fn query(input: &str, session: &Session, gestalt: &mut Gestalt) -> String {
    // v0: dispatch to the appropriate optic command
    // Parse "focus @essay.proof" → optic=focus, target=@essay.proof
    // Run the optic on the target
    // Return plain text result
    // Update gestalt growth

    // The STRUCTURE is here for the full pipeline.
    // The INTELLIGENCE comes in v1.
    todo!()
}
```

## Modified Files

### src/main.rs — Add "tui" Command

```rust
// Add to the match:
"tui" | "t" => tui::tui(),
```

Short alias `t` because this will be typed a lot.

### src/session.rs — Add tick/tock/shatter

```rust
impl Session {
    pub fn tick(&self, gestalt: &Gestalt) -> io::Result<()> {
        // Log: timestamp TAB tick TAB current_path TAB growth
        // Update gestalt locally
        // Append to .spectral/log
    }

    pub fn tock(&self, gestalt: &Gestalt) -> io::Result<()> {
        // Everything tick does, plus:
        // Compute delta since last tock
        // Write delta to .spectral/outbox/ (for future federation)
    }

    pub fn shatter(&self, gestalt: &mut Gestalt) -> io::Result<()> {
        // Everything tick does, plus:
        // Crystallize current gestalt state
        // Write crystal to .spectral/crystals/
        // Update .shatter weights (v1: from attention, v0: just log it)
    }
}
```

## The Gestalt struct

```rust
//! In-memory gestalt for the TUI session.

pub struct Gestalt {
    pub growth: HashMap<String, f64>,    // grammar → growth %
    pub tensions: Vec<Tension>,           // active tensions
    pub ticks: usize,                     // total ticks this session
    pub path: Vec<String>,                // current navigation path
}

pub struct Tension {
    pub description: String,
    pub loss: f64,
}

impl Gestalt {
    pub fn load(session: &Session) -> Self { ... }
    pub fn save(&self, session: &Session) -> io::Result<()> { ... }
    pub fn total_growth(&self) -> f64 { ... }
    pub fn total_loss(&self) -> f64 { 100.0 - self.total_growth() }
}
```

## The Status Line

After every interaction:

```
growth: 38%  loss: 62%  ticks: 7  path: @essay/loss→growth
```

Green for growth. Amber for loss. Cyan for path. Dimmed for labels.

## Command Reference

```
spectral> [query]          ask a question (v0: dispatches to optic)
spectral> .                show current position
spectral> ..               go back (with growth re-render)
spectral> ~                go to gestalt root
spectral> @                show author's view
spectral> ^                show last crystal

spectral> /tick            advance locally
spectral> /tock            advance and sync delta
spectral> /shatter         crystallize + train (v0: crystallize only)

spectral> /log             show tick history
spectral> /log --oneline   compact log
spectral> /status          full gestalt display
spectral> /diff . @        compare self to author
spectral> /tensions        show active tensions

spectral> /quit            exit (auto-ticks on exit)
spectral> ctrl-d           exit (same as /quit)
```

`/` prefix for commands (like Claude Code's `/` commands).
Everything else is a query.

## Glint Integration (v1)

When Glint is ready, the TUI becomes Glint's home:

```
spectral> what connects loss to growth?

  Loss and growth are inverses. growth = 100% - loss.
  Three paths connect them: [...]

  Glint (n-1): You've asked about this connection before.
               Your loss dropped from 0.67 to 0.31.
               Maybe the question isn't how they connect.

  growth: 34% → 38% (+4%)
```

Glint speaks after the pipeline answer. One tick behind.
In dimmed text below the main response. The Reflection model.

v0 doesn't have Glint. v0 is the house. Glint moves in at v1.

## Build Order

```
1. Gestalt struct (in-memory, load/save from .spectral/)
2. Navigator (resolve . / .. / ~ / @ / ^ / HEAD)
3. tui.rs prompt loop (rustyline + colored)
4. Wire into main.rs ("tui" command)
5. /tick, /tock, /shatter in session.rs
6. /log, /status, /tensions display
7. v0 pipeline (direct optic dispatch, no models)
8. Status line after every interaction
```

Eight steps. The first four get a working prompt.
Steps 5-6 get session management.
Steps 7-8 get query results.

## Tests

```
tests/tui.rs:
  - test_navigator_dot (. returns current path)
  - test_navigator_dotdot (.. pops and returns previous)
  - test_navigator_home (~ returns root)
  - test_gestalt_load_save (roundtrip through .spectral/)
  - test_gestalt_growth_calculation (total_growth sums correctly)
  - test_session_tick_logs (tick appends to log)
  - test_session_shatter_writes_crystal (shatter writes to crystals/)
```

## What This Enables

With the TUI running:
- Alex can interact with spectral directly (not just through MCP)
- The essay can be tested in the TUI before going to browser
- Glint can be prototyped in the TUI before going to WASM
- The navigation model (. / .. / ...) can be validated with real use
- The gestalt format can be iterated on with real session data

The TUI is the development environment for everything else.
Build the house first. The inhabitants come after.

## The Moment

```
$ spectral tui

Garden planted. Growth: 0%. Loss: 100%.

spectral> .

  You are here: ~
  Depth: 0%. Loss: 100%. Ticks: 0.
  No grammars loaded. No readers yet.

  Start reading.

spectral>
```

That's v0. An empty garden. A blinking cursor. Ready.
