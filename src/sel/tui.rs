//! TUI for `spectral join` — GPU-rendered eigenboard in the terminal.
//!
//! Architecture: ratatui for text panels, spectral-ui for headless GPU
//! rendering of the eigenboard field. The two meet via the Kitty graphics
//! protocol (or text-mode fallback for unsupported terminals).
//!
//! The event loop: `tokio::select!` over hook events (EigenboardFrame),
//! crossterm input, and idle tick timer.

use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use gestalt::eigenvalue::{spectral_embedding_2d, EigenvalueProfile};
use gestalt::graph::ConceptGraph;

use crate::apache2::init::init_identity;
use crate::apache2::views::{SavingsView, StatusView};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// The TUI application state.
pub struct App {
    /// Paths that were --add'd into the session.
    pub paths: Vec<String>,
    /// Context name (from @context argument).
    pub context_name: String,
    /// Status view (refreshed periodically).
    pub status: StatusView,
    /// Savings view.
    pub savings: SavingsView,
    /// Mote positions from spectral embedding.
    pub mote_positions: Vec<[f32; 2]>,
    /// Mote names (directory/repo names).
    pub mote_names: Vec<String>,
    /// Prompt input buffer.
    pub input: String,
    /// Whether to quit.
    pub should_quit: bool,
    /// Current tick (for idle animation).
    pub tick: u64,
}

impl App {
    /// Build a new App from parsed arguments.
    pub fn new(context_name: &str, add_paths: &[&str]) -> Self {
        let mut mote_positions = Vec::new();
        let mut mote_names = Vec::new();

        // Initialize each path and collect eigenvalue profiles
        for path_str in add_paths {
            let path = Path::new(path_str);
            // Run init to ensure .spectral/ exists
            let _ = init_identity(path);

            // Build concept graph for this path
            let (graph, _, _) = gestalt::graph::build_concept_graph(path);
            let positions = spectral_embedding_2d(&graph);

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path_str.to_string());

            // Use center of mass of this graph's embedding as the mote position
            if !positions.is_empty() {
                let cx: f32 = positions.iter().map(|p| p[0]).sum::<f32>() / positions.len() as f32;
                let cy: f32 = positions.iter().map(|p| p[1]).sum::<f32>() / positions.len() as f32;
                mote_positions.push([cx, cy]);
            } else {
                mote_positions.push([0.0, 0.0]);
            }
            mote_names.push(name);
        }

        // Build status from the first path (or cwd)
        let status_path = add_paths.first().map(Path::new).unwrap_or(Path::new("."));
        let status = StatusView::from_session(status_path);
        let savings = SavingsView::from_session(status_path);

        App {
            paths: add_paths.iter().map(|s| s.to_string()).collect(),
            context_name: context_name.to_string(),
            status,
            savings,
            mote_positions,
            mote_names,
            input: String::new(),
            should_quit: false,
            tick: 0,
        }
    }

    /// Advance the idle tick (for breathing animation).
    pub fn tick(&mut self) {
        self.tick += 1;
    }

    /// Handle a crossterm key event.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Esc, _) => self.should_quit = true,
            (KeyCode::Char(c), _) => self.input.push(c),
            (KeyCode::Backspace, _) => { self.input.pop(); }
            (KeyCode::Enter, _) => {
                let cmd = self.input.trim().to_string();
                if cmd == "/quit" || cmd == "/q" {
                    self.should_quit = true;
                }
                self.input.clear();
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Layout rendering
// ---------------------------------------------------------------------------

/// Render the TUI layout into a ratatui frame.
fn render_ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),        // eigenboard field (top)
            Constraint::Length(6),      // savings panel (middle)
            Constraint::Length(3),      // prompt input (bottom)
        ])
        .split(frame.area());

    // Top: eigenboard field (text-mode fallback)
    render_eigenboard_text(frame, chunks[0], app);

    // Middle: savings panel
    render_savings_panel(frame, chunks[1], app);

    // Bottom: prompt input
    render_prompt(frame, chunks[2], app);
}

/// Text-mode eigenboard rendering (fallback for terminals without image protocol).
fn render_eigenboard_text(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" spectral join @{} ", app.context_name))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render mote positions as text
    let mut lines: Vec<Line> = Vec::new();

    if app.mote_positions.is_empty() {
        lines.push(Line::from("  no motes in the field"));
    } else {
        // Header
        lines.push(Line::from(vec![
            Span::styled("  nodes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.status.nodes),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  edges: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.status.edges),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("  tension: {:.4}", app.status.tension),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        lines.push(Line::from(""));

        // Mote list with position indicators
        for (i, name) in app.mote_names.iter().enumerate() {
            let pos = &app.mote_positions[i];
            // Map position to a sparkline character
            let x_bar = position_to_sparkline(pos[0]);
            let y_bar = position_to_sparkline(pos[1]);

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<20}", name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("  x:{} y:{}", x_bar, y_bar),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  ({:.2}, {:.2})", pos[0], pos[1]),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render the savings panel.
fn render_savings_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" savings ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled("  tokens saved: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.savings.tokens_saved),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!(" ({:.0}%)", app.savings.savings_pct()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("  cost: -${:.2}", app.savings.cost_avoided),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  cache: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "eigen {:.0}% / gestalt {:.0}% / vector {:.0}%",
                    app.savings.cache_eigen_pct,
                    app.savings.cache_gestalt_pct,
                    app.savings.cache_vector_pct,
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render the prompt input line.
fn render_prompt(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prompt_line = Line::from(vec![
        Span::styled("spectral> ", Style::default().fg(Color::DarkGray)),
        Span::styled(&app.input, Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(Color::Cyan)), // cursor block
    ]);

    let paragraph = Paragraph::new(prompt_line);
    frame.render_widget(paragraph, inner);
}

/// Map a [-1, 1] position to a sparkline character.
fn position_to_sparkline(v: f32) -> char {
    let chars = ['\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
    let idx = ((v + 1.0) / 2.0 * 7.0).clamp(0.0, 7.0) as usize;
    chars[idx]
}

// ---------------------------------------------------------------------------
// TUI entry point
// ---------------------------------------------------------------------------

/// Run the TUI event loop.
///
/// This is the AffineTraversal: it might fail (no terminal, no GPU),
/// but when it succeeds, it produces exactly one Session.
pub fn run_tui(context_name: &str, add_paths: &[&str]) -> Result<(), String> {
    let mut app = App::new(context_name, add_paths);

    // Set up terminal
    enable_raw_mode().map_err(|e| format!("raw mode: {}", e))?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen).map_err(|e| format!("alt screen: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("terminal: {}", e))?;

    // Event loop
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| render_ui(f, &app)).map_err(|e| format!("draw: {}", e))?;

        if event::poll(tick_rate).map_err(|e| format!("poll: {}", e))? {
            if let Event::Key(key) = event::read().map_err(|e| format!("read: {}", e))? {
                app.handle_key(key.code, key.modifiers);
            }
        } else {
            app.tick();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode().map_err(|e| format!("restore raw mode: {}", e))?;
    io::stdout().execute(LeaveAlternateScreen).map_err(|e| format!("leave alt screen: {}", e))?;

    // Session summary
    eprintln!(
        "session ended. context: @{}  paths: {}  ticks: {}",
        app.context_name,
        app.paths.len(),
        app.tick,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_new_empty_paths() {
        let app = App::new("test", &[]);
        assert_eq!(app.context_name, "test");
        assert!(app.paths.is_empty());
        assert!(app.mote_positions.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn app_handle_quit_ctrl_c() {
        let mut app = App::new("test", &[]);
        app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(app.should_quit);
    }

    #[test]
    fn app_handle_quit_ctrl_d() {
        let mut app = App::new("test", &[]);
        app.handle_key(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(app.should_quit);
    }

    #[test]
    fn app_handle_quit_esc() {
        let mut app = App::new("test", &[]);
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.should_quit);
    }

    #[test]
    fn app_handle_quit_command() {
        let mut app = App::new("test", &[]);
        for c in "/quit".chars() {
            app.handle_key(KeyCode::Char(c), KeyModifiers::NONE);
        }
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.should_quit);
    }

    #[test]
    fn app_handle_typing() {
        let mut app = App::new("test", &[]);
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(app.input, "hi");
    }

    #[test]
    fn app_handle_backspace() {
        let mut app = App::new("test", &[]);
        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('b'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.input, "a");
    }

    #[test]
    fn app_tick_advances() {
        let mut app = App::new("test", &[]);
        assert_eq!(app.tick, 0);
        app.tick();
        assert_eq!(app.tick, 1);
    }

    #[test]
    fn position_to_sparkline_range() {
        // -1.0 should give the lowest bar
        assert_eq!(position_to_sparkline(-1.0), '\u{2581}');
        // 1.0 should give the highest bar
        assert_eq!(position_to_sparkline(1.0), '\u{2588}');
        // 0.0 should be somewhere in the middle
        let mid = position_to_sparkline(0.0);
        assert!(mid >= '\u{2584}' && mid <= '\u{2585}');
    }

    #[test]
    fn app_with_tempdir_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.md"), "# Test\n").unwrap();
        let sub = dir.path().join("src");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("lib.rs"), "fn main() {}\n").unwrap();

        let path_str = dir.path().to_str().unwrap();
        let app = App::new("test", &[path_str]);
        assert_eq!(app.paths.len(), 1);
        assert_eq!(app.mote_positions.len(), 1);
        assert_eq!(app.mote_names.len(), 1);
    }

    #[test]
    fn app_enter_clears_input() {
        let mut app = App::new("test", &[]);
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.input.is_empty());
        assert!(!app.should_quit); // "h" is not /quit
    }
}
