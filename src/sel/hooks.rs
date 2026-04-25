//! Hook dispatch system — every event is a measurement that shifts the eigenboard.
//!
//! [Glint] The context window is not a buffer. It's a live stream.
//! Each event (keystroke, file-write, git-commit) produces a frame.
//! The frame is a measurement. The measurement shifts the eigenboard.
//!
//! [Taut] Every hook has a latency budget. The cascade finder's question:
//! which hooks can overlap? which must be sequential? where's the line
//! through the worst case?
//!
//! ## Hook → Action mapping (from eigenboard.spec)
//!
//! | Event | Action | Budget |
//! |-------|--------|--------|
//! | keystroke | focus-shift | <2ms |
//! | prompt-submit | tournament-fan-out | <5ms |
//! | suggestion-arrive | beam-receive | <5ms |
//! | suggestion-accept | collapse | <10ms |
//! | suggestion-reject | dissipate | <5ms |
//! | file-write | observe | <20ms |
//! | git-commit | anchor | <50ms |
//! | test-pass | crystallize | <100ms |
//! | test-fail | destabilize | <50ms |

use std::time::Instant;

use prism::oid::Oid;

// ---------------------------------------------------------------------------
// HookEvent — what happened
// ---------------------------------------------------------------------------

/// Every event that the eigenboard cares about.
/// Each variant maps to exactly one action. No ambiguity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HookEvent {
    /// User typed a character. Sub-frame response required.
    Keystroke {
        /// File URI where the keystroke occurred.
        uri: String,
    },

    /// User submitted a prompt to the AI peer.
    PromptSubmit {
        /// Truncated prompt content for logging (not the full prompt).
        prompt_hash: u64,
    },

    /// A suggestion arrived from the tournament.
    SuggestionArrive {
        /// Which player produced this suggestion.
        player: String,
        /// Content-addressed ID of the suggestion.
        suggestion_oid: Oid,
    },

    /// User accepted a suggestion.
    SuggestionAccept {
        /// The accepted suggestion's OID.
        suggestion_oid: Oid,
    },

    /// User rejected a suggestion.
    SuggestionReject {
        /// The rejected suggestion's OID.
        suggestion_oid: Oid,
    },

    /// A file was written to disk.
    FileWrite {
        /// File URI that changed.
        uri: String,
    },

    /// A git commit was created.
    GitCommit {
        /// The commit hash.
        commit_hash: String,
    },

    /// Tests passed.
    TestPass {
        /// How many tests passed.
        count: u32,
    },

    /// Tests failed.
    TestFail {
        /// How many tests failed.
        count: u32,
        /// Summary of failures.
        summary: String,
    },
}

impl HookEvent {
    /// The latency budget for this event, in microseconds.
    pub fn budget_us(&self) -> u64 {
        match self {
            HookEvent::Keystroke { .. } => 2_000,         // 2ms
            HookEvent::PromptSubmit { .. } => 5_000,      // 5ms
            HookEvent::SuggestionArrive { .. } => 5_000,   // 5ms
            HookEvent::SuggestionAccept { .. } => 10_000,  // 10ms
            HookEvent::SuggestionReject { .. } => 5_000,   // 5ms
            HookEvent::FileWrite { .. } => 20_000,         // 20ms
            HookEvent::GitCommit { .. } => 50_000,         // 50ms
            HookEvent::TestPass { .. } => 100_000,         // 100ms
            HookEvent::TestFail { .. } => 50_000,          // 50ms
        }
    }

    /// Priority level (0 = highest). Determines dispatch order when events collide.
    /// Keystroke is highest priority (must never drop frames).
    pub fn priority(&self) -> u8 {
        match self {
            HookEvent::Keystroke { .. } => 0,
            HookEvent::SuggestionArrive { .. } => 1,
            HookEvent::PromptSubmit { .. } => 2,
            HookEvent::SuggestionAccept { .. } => 3,
            HookEvent::SuggestionReject { .. } => 3,
            HookEvent::FileWrite { .. } => 4,
            HookEvent::GitCommit { .. } => 5,
            HookEvent::TestPass { .. } => 6,
            HookEvent::TestFail { .. } => 6,
        }
    }
}

// ---------------------------------------------------------------------------
// HookAction — what the event triggers
// ---------------------------------------------------------------------------

/// The measurement action triggered by a hook event.
/// Each action describes what the eigenboard should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HookAction {
    /// Focus shifted to a new region. Update gutter, recompute local eigenvalues.
    FocusShift {
        uri: String,
    },

    /// Fan out to tournament players. Show superposition state.
    TournamentFanOut {
        prompt_hash: u64,
    },

    /// A beam arrived from a tournament player. Draw the arc.
    BeamReceive {
        player: String,
        suggestion_oid: Oid,
    },

    /// Collapse: suggestion accepted. Weight shift, gestalt reparse, fast snapshot.
    Collapse {
        suggestion_oid: Oid,
    },

    /// Dissipate: suggestion rejected. Weight hold, fade animation.
    Dissipate {
        suggestion_oid: Oid,
    },

    /// Observe: file changed. Gestalt reparse, dirty eigen cascade, fast snapshot.
    Observe {
        uri: String,
    },

    /// Anchor: git commit. Full CoincidenceHash snapshot, crystallize check.
    Anchor {
        commit_hash: String,
    },

    /// Crystallize: tests passed. Crystal check, conditional deploy.
    Crystallize {
        test_count: u32,
    },

    /// Destabilize: tests failed. Weight penalty, flicker animation.
    Destabilize {
        fail_count: u32,
        summary: String,
    },
}

// ---------------------------------------------------------------------------
// RenderHint — how the UI should animate this frame
// ---------------------------------------------------------------------------

/// Visual hint for the eigenboard renderer.
/// Maps to spectral-ui animation states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderHint {
    /// Eigenvalues settling toward equilibrium. Gentle glow changes.
    Settle,
    /// Energy pulse from a new event. Brief brightness increase.
    Pulse,
    /// A node crystallized. Solid, bright, stable.
    Solidify,
    /// Something destabilized. Rapid brightness oscillation.
    Flicker,
    /// A tournament beam connecting two nodes. Draw arc.
    Beam,
    /// A suggestion fading away. Opacity decrease.
    Fade,
}

// ---------------------------------------------------------------------------
// SnapshotKind — fast (FNV-1a) or full (CoincidenceHash)
// ---------------------------------------------------------------------------

/// Which snapshot to take after this action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotKind {
    /// FNV-1a: 650ns. The hot-path tick.
    Fast,
    /// CoincidenceHash: ~1.755ms. The persistent record.
    Full,
    /// No snapshot needed for this action.
    None,
}

// ---------------------------------------------------------------------------
// TopologyDelta — what changed in the eigenboard
// ---------------------------------------------------------------------------

/// Describes what changed in the eigenboard topology.
#[derive(Clone, Debug, PartialEq)]
pub enum TopologyDelta {
    /// Nothing changed in the topology.
    None,
    /// A node's weight shifted.
    WeightShift { node_oid: Oid, delta: f64 },
    /// A new edge appeared (beam).
    EdgeAdded { from: Oid, to: Oid, strength: f64 },
    /// An edge faded away.
    EdgeRemoved { from: Oid, to: Oid },
    /// Multiple changes (batch).
    Batch(Vec<TopologyDelta>),
}

// ---------------------------------------------------------------------------
// EigenboardFrame — a single measurement
// ---------------------------------------------------------------------------

/// A single frame in the eigenboard's context stream.
/// Each hook event produces exactly one frame.
#[derive(Clone, Debug)]
pub struct EigenboardFrame {
    /// Monotonic tick counter.
    pub tick: u64,
    /// The event that produced this frame.
    pub event: HookEvent,
    /// The action dispatched.
    pub action: HookAction,
    /// What changed in the topology.
    pub topology_delta: TopologyDelta,
    /// The snapshot OID (fast or full, depending on action).
    pub snapshot_oid: Oid,
    /// Which kind of snapshot was taken.
    pub snapshot_kind: SnapshotKind,
    /// How the UI should render this frame.
    pub render_hint: RenderHint,
    /// Time elapsed for this hook dispatch (for budget tracking).
    pub elapsed_us: u64,
}

// ---------------------------------------------------------------------------
// HookDispatcher — receives events, dispatches actions
// ---------------------------------------------------------------------------

/// The hook dispatcher: transforms events into measurements.
///
/// [Glint] This is the event→measurement pipeline. Each event enters,
/// an action is computed, a frame is produced. The frame IS the measurement.
///
/// [Taut] The dispatcher tracks latency per-hook-type. If any hook
/// exceeds its budget, the dispatcher records the overrun for analysis.
pub struct HookDispatcher {
    /// Current tick (monotonic, advances with each dispatched event).
    tick: u64,
    /// Budget overruns: (event_name, elapsed_us, budget_us).
    overruns: Vec<(String, u64, u64)>,
    /// Total frames dispatched.
    frame_count: u64,
}

impl HookDispatcher {
    /// Create a new dispatcher at tick 0.
    pub fn new() -> Self {
        HookDispatcher {
            tick: 0,
            overruns: Vec::new(),
            frame_count: 0,
        }
    }

    /// Current tick value.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Total frames dispatched.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Budget overruns recorded so far.
    pub fn overruns(&self) -> &[(String, u64, u64)] {
        &self.overruns
    }

    /// Dispatch a hook event → produce an EigenboardFrame.
    ///
    /// This is the core pipeline:
    /// 1. Map event → action
    /// 2. Determine render hint
    /// 3. Determine snapshot kind
    /// 4. Execute the action (topology delta)
    /// 5. Take snapshot
    /// 6. Package into frame
    /// 7. Track latency against budget
    pub fn dispatch(&mut self, event: HookEvent) -> EigenboardFrame {
        let start = Instant::now();
        self.tick += 1;

        let action = Self::map_action(&event);
        let render_hint = Self::map_render_hint(&event);
        let snapshot_kind = Self::map_snapshot_kind(&event);
        let topology_delta = TopologyDelta::None; // Placeholder — real delta comes from actor responses

        // Snapshot OID — in production this comes from the actual field state.
        // Here we content-address the event itself for deterministic testing.
        let snapshot_oid = Self::event_oid(&event, self.tick);

        let elapsed_us = start.elapsed().as_micros() as u64;

        // Track budget overruns
        let budget = event.budget_us();
        if elapsed_us > budget {
            self.overruns.push((
                format!("{:?}", event).chars().take(50).collect(),
                elapsed_us,
                budget,
            ));
        }

        self.frame_count += 1;

        EigenboardFrame {
            tick: self.tick,
            event,
            action,
            topology_delta,
            snapshot_oid,
            snapshot_kind,
            render_hint,
            elapsed_us,
        }
    }

    /// Map an event to its action. Pure function — no side effects.
    fn map_action(event: &HookEvent) -> HookAction {
        match event {
            HookEvent::Keystroke { uri } => HookAction::FocusShift {
                uri: uri.clone(),
            },
            HookEvent::PromptSubmit { prompt_hash } => HookAction::TournamentFanOut {
                prompt_hash: *prompt_hash,
            },
            HookEvent::SuggestionArrive { player, suggestion_oid } => HookAction::BeamReceive {
                player: player.clone(),
                suggestion_oid: suggestion_oid.clone(),
            },
            HookEvent::SuggestionAccept { suggestion_oid } => HookAction::Collapse {
                suggestion_oid: suggestion_oid.clone(),
            },
            HookEvent::SuggestionReject { suggestion_oid } => HookAction::Dissipate {
                suggestion_oid: suggestion_oid.clone(),
            },
            HookEvent::FileWrite { uri } => HookAction::Observe {
                uri: uri.clone(),
            },
            HookEvent::GitCommit { commit_hash } => HookAction::Anchor {
                commit_hash: commit_hash.clone(),
            },
            HookEvent::TestPass { count } => HookAction::Crystallize {
                test_count: *count,
            },
            HookEvent::TestFail { count, summary } => HookAction::Destabilize {
                fail_count: *count,
                summary: summary.clone(),
            },
        }
    }

    /// Map an event to its render hint.
    fn map_render_hint(event: &HookEvent) -> RenderHint {
        match event {
            HookEvent::Keystroke { .. } => RenderHint::Settle,
            HookEvent::PromptSubmit { .. } => RenderHint::Pulse,
            HookEvent::SuggestionArrive { .. } => RenderHint::Beam,
            HookEvent::SuggestionAccept { .. } => RenderHint::Solidify,
            HookEvent::SuggestionReject { .. } => RenderHint::Fade,
            HookEvent::FileWrite { .. } => RenderHint::Pulse,
            HookEvent::GitCommit { .. } => RenderHint::Solidify,
            HookEvent::TestPass { .. } => RenderHint::Solidify,
            HookEvent::TestFail { .. } => RenderHint::Flicker,
        }
    }

    /// Map an event to its snapshot kind.
    fn map_snapshot_kind(event: &HookEvent) -> SnapshotKind {
        match event {
            HookEvent::Keystroke { .. } => SnapshotKind::None,
            HookEvent::PromptSubmit { .. } => SnapshotKind::None,
            HookEvent::SuggestionArrive { .. } => SnapshotKind::None,
            HookEvent::SuggestionAccept { .. } => SnapshotKind::Fast,
            HookEvent::SuggestionReject { .. } => SnapshotKind::None,
            HookEvent::FileWrite { .. } => SnapshotKind::Fast,
            HookEvent::GitCommit { .. } => SnapshotKind::Full,
            HookEvent::TestPass { .. } => SnapshotKind::Full,
            HookEvent::TestFail { .. } => SnapshotKind::Fast,
        }
    }

    /// Content-address an event for snapshot OID.
    fn event_oid(event: &HookEvent, tick: u64) -> Oid {
        let content = format!("hook:{}:{:?}", tick, event);
        Oid::hash(content.as_bytes())
    }

    /// Check if two events can overlap (execute concurrently) without frame drop.
    ///
    /// [Taut] The cascade question: which hooks can run in parallel?
    /// Rule: events that share no mutable state can overlap.
    /// Events that both write to topology CANNOT overlap.
    pub fn can_overlap(a: &HookEvent, b: &HookEvent) -> bool {
        match (a, b) {
            // Keystroke overlaps with everything except another keystroke to same file
            (HookEvent::Keystroke { uri: u1 }, HookEvent::Keystroke { uri: u2 }) => u1 != u2,
            (HookEvent::Keystroke { .. }, _) | (_, HookEvent::Keystroke { .. }) => true,

            // Suggestion arrive can overlap with other arrives (different players)
            (
                HookEvent::SuggestionArrive { player: p1, .. },
                HookEvent::SuggestionArrive { player: p2, .. },
            ) => p1 != p2,

            // Accept/reject CANNOT overlap with file-write (both mutate topology)
            (HookEvent::SuggestionAccept { .. }, HookEvent::FileWrite { .. })
            | (HookEvent::FileWrite { .. }, HookEvent::SuggestionAccept { .. }) => false,

            // File-write and git-commit must be sequential
            (HookEvent::FileWrite { .. }, HookEvent::GitCommit { .. })
            | (HookEvent::GitCommit { .. }, HookEvent::FileWrite { .. }) => false,

            // Git-commit and test-pass must be sequential (crystallize depends on anchor)
            (HookEvent::GitCommit { .. }, HookEvent::TestPass { .. })
            | (HookEvent::TestPass { .. }, HookEvent::GitCommit { .. }) => false,

            // PromptSubmit overlaps with file-write (different subsystems)
            (HookEvent::PromptSubmit { .. }, HookEvent::FileWrite { .. })
            | (HookEvent::FileWrite { .. }, HookEvent::PromptSubmit { .. }) => true,

            // Default: no overlap for safety
            _ => false,
        }
    }

    /// Compute the worst-case cascade latency in microseconds.
    ///
    /// [Taut] The worst case: prompt-submit → 5x suggestion-arrive →
    /// accept → file-write → test-pass → crystallize.
    ///
    /// Returns the total sequential budget and whether it fits in one frame.
    pub fn worst_case_cascade_us() -> (u64, bool) {
        let prompt = 5_000;        // prompt-submit
        let arrives = 5_000;       // suggestion-arrive (overlapping — count once)
        let accept = 10_000;       // suggestion-accept
        let file_write = 20_000;   // file-write
        let test_pass = 100_000;   // test-pass (crystallize)

        let total = prompt + arrives + accept + file_write + test_pass;
        // At 8 ticks per frame, frame budget is 8 * 650ns = 5.2us for snapshot ticks.
        // But the real frame budget is the 41% headroom at 200 motes.
        // Total sequential budget: 140ms. That's multiple frames.
        // The question is: does it complete before the NEXT prompt-submit?
        let fits_single_frame = total < 16_667; // 60fps = 16.67ms per frame
        (total, fits_single_frame)
    }

    /// How many hook events can fire per frame before budget is exceeded?
    ///
    /// [Taut] At 8 ticks per frame (from eigenboard.spec @clock):
    /// - Fast snapshot: 650ns per tick
    /// - Frame budget: 8 * 650ns = 5.2us for clock work
    /// - Total frame time at 60fps: 16,667us
    /// - With 41% headroom: ~9,833us available for hooks per frame
    pub fn max_events_per_frame() -> (usize, u64) {
        let frame_budget_us = 16_667; // 60fps
        let headroom_ratio = 0.41;
        let available_us = (frame_budget_us as f64 * (1.0 - headroom_ratio)) as u64;

        // Keystroke is the cheapest: 2ms budget = ~4.9 keystrokes per frame
        let keystrokes_per_frame = available_us / 2_000;
        (keystrokes_per_frame as usize, available_us)
    }
}

impl Default for HookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- HookEvent tests ---

    #[test]
    fn hook_event_budget_keystroke_is_sub_frame() {
        let event = HookEvent::Keystroke { uri: "file:///test.rs".into() };
        assert_eq!(event.budget_us(), 2_000, "keystroke budget should be 2ms");
    }

    #[test]
    fn hook_event_budget_file_write() {
        let event = HookEvent::FileWrite { uri: "file:///test.rs".into() };
        assert_eq!(event.budget_us(), 20_000, "file-write budget should be 20ms");
    }

    #[test]
    fn hook_event_budget_git_commit() {
        let event = HookEvent::GitCommit { commit_hash: "abc123".into() };
        assert_eq!(event.budget_us(), 50_000, "git-commit budget should be 50ms");
    }

    #[test]
    fn hook_event_budget_test_pass() {
        let event = HookEvent::TestPass { count: 42 };
        assert_eq!(event.budget_us(), 100_000, "test-pass budget should be 100ms");
    }

    #[test]
    fn hook_event_priority_keystroke_is_highest() {
        let k = HookEvent::Keystroke { uri: "f".into() };
        let f = HookEvent::FileWrite { uri: "f".into() };
        let g = HookEvent::GitCommit { commit_hash: "x".into() };
        assert!(k.priority() < f.priority());
        assert!(f.priority() < g.priority());
    }

    #[test]
    fn hook_event_priority_all_nine_variants() {
        let events = vec![
            HookEvent::Keystroke { uri: "f".into() },
            HookEvent::PromptSubmit { prompt_hash: 0 },
            HookEvent::SuggestionArrive { player: "p".into(), suggestion_oid: Oid::dark() },
            HookEvent::SuggestionAccept { suggestion_oid: Oid::dark() },
            HookEvent::SuggestionReject { suggestion_oid: Oid::dark() },
            HookEvent::FileWrite { uri: "f".into() },
            HookEvent::GitCommit { commit_hash: "x".into() },
            HookEvent::TestPass { count: 1 },
            HookEvent::TestFail { count: 1, summary: "s".into() },
        ];
        // All 9 variants should have a priority
        assert_eq!(events.len(), 9);
        for e in &events {
            assert!(e.priority() <= 6);
        }
    }

    // --- HookAction mapping tests ---

    #[test]
    fn keystroke_maps_to_focus_shift() {
        let event = HookEvent::Keystroke { uri: "file:///test.rs".into() };
        let action = HookDispatcher::map_action(&event);
        assert!(matches!(action, HookAction::FocusShift { .. }));
    }

    #[test]
    fn prompt_submit_maps_to_tournament_fan_out() {
        let event = HookEvent::PromptSubmit { prompt_hash: 42 };
        let action = HookDispatcher::map_action(&event);
        assert!(matches!(action, HookAction::TournamentFanOut { prompt_hash: 42 }));
    }

    #[test]
    fn suggestion_arrive_maps_to_beam_receive() {
        let oid = Oid::hash(b"test");
        let event = HookEvent::SuggestionArrive {
            player: "fate".into(),
            suggestion_oid: oid.clone(),
        };
        let action = HookDispatcher::map_action(&event);
        match action {
            HookAction::BeamReceive { player, suggestion_oid } => {
                assert_eq!(player, "fate");
                assert_eq!(suggestion_oid, oid);
            }
            _ => panic!("expected BeamReceive"),
        }
    }

    #[test]
    fn suggestion_accept_maps_to_collapse() {
        let oid = Oid::hash(b"accepted");
        let event = HookEvent::SuggestionAccept { suggestion_oid: oid.clone() };
        let action = HookDispatcher::map_action(&event);
        assert!(matches!(action, HookAction::Collapse { .. }));
    }

    #[test]
    fn suggestion_reject_maps_to_dissipate() {
        let oid = Oid::hash(b"rejected");
        let event = HookEvent::SuggestionReject { suggestion_oid: oid.clone() };
        let action = HookDispatcher::map_action(&event);
        assert!(matches!(action, HookAction::Dissipate { .. }));
    }

    #[test]
    fn file_write_maps_to_observe() {
        let event = HookEvent::FileWrite { uri: "file:///lib.rs".into() };
        let action = HookDispatcher::map_action(&event);
        assert!(matches!(action, HookAction::Observe { .. }));
    }

    #[test]
    fn git_commit_maps_to_anchor() {
        let event = HookEvent::GitCommit { commit_hash: "abc123".into() };
        let action = HookDispatcher::map_action(&event);
        match action {
            HookAction::Anchor { commit_hash } => assert_eq!(commit_hash, "abc123"),
            _ => panic!("expected Anchor"),
        }
    }

    #[test]
    fn test_pass_maps_to_crystallize() {
        let event = HookEvent::TestPass { count: 42 };
        let action = HookDispatcher::map_action(&event);
        assert!(matches!(action, HookAction::Crystallize { test_count: 42 }));
    }

    #[test]
    fn test_fail_maps_to_destabilize() {
        let event = HookEvent::TestFail { count: 3, summary: "oops".into() };
        let action = HookDispatcher::map_action(&event);
        match action {
            HookAction::Destabilize { fail_count, summary } => {
                assert_eq!(fail_count, 3);
                assert_eq!(summary, "oops");
            }
            _ => panic!("expected Destabilize"),
        }
    }

    // --- RenderHint mapping tests ---

    #[test]
    fn keystroke_renders_settle() {
        let event = HookEvent::Keystroke { uri: "f".into() };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Settle);
    }

    #[test]
    fn prompt_submit_renders_pulse() {
        let event = HookEvent::PromptSubmit { prompt_hash: 0 };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Pulse);
    }

    #[test]
    fn suggestion_arrive_renders_beam() {
        let event = HookEvent::SuggestionArrive {
            player: "p".into(),
            suggestion_oid: Oid::dark(),
        };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Beam);
    }

    #[test]
    fn suggestion_accept_renders_solidify() {
        let event = HookEvent::SuggestionAccept { suggestion_oid: Oid::dark() };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Solidify);
    }

    #[test]
    fn suggestion_reject_renders_fade() {
        let event = HookEvent::SuggestionReject { suggestion_oid: Oid::dark() };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Fade);
    }

    #[test]
    fn file_write_renders_pulse() {
        let event = HookEvent::FileWrite { uri: "f".into() };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Pulse);
    }

    #[test]
    fn git_commit_renders_solidify() {
        let event = HookEvent::GitCommit { commit_hash: "x".into() };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Solidify);
    }

    #[test]
    fn test_pass_renders_solidify() {
        let event = HookEvent::TestPass { count: 1 };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Solidify);
    }

    #[test]
    fn test_fail_renders_flicker() {
        let event = HookEvent::TestFail { count: 1, summary: "s".into() };
        assert_eq!(HookDispatcher::map_render_hint(&event), RenderHint::Flicker);
    }

    // --- SnapshotKind mapping tests ---

    #[test]
    fn keystroke_no_snapshot() {
        let event = HookEvent::Keystroke { uri: "f".into() };
        assert_eq!(HookDispatcher::map_snapshot_kind(&event), SnapshotKind::None);
    }

    #[test]
    fn suggestion_accept_fast_snapshot() {
        let event = HookEvent::SuggestionAccept { suggestion_oid: Oid::dark() };
        assert_eq!(HookDispatcher::map_snapshot_kind(&event), SnapshotKind::Fast);
    }

    #[test]
    fn file_write_fast_snapshot() {
        let event = HookEvent::FileWrite { uri: "f".into() };
        assert_eq!(HookDispatcher::map_snapshot_kind(&event), SnapshotKind::Fast);
    }

    #[test]
    fn git_commit_full_snapshot() {
        let event = HookEvent::GitCommit { commit_hash: "x".into() };
        assert_eq!(HookDispatcher::map_snapshot_kind(&event), SnapshotKind::Full);
    }

    #[test]
    fn test_pass_full_snapshot() {
        let event = HookEvent::TestPass { count: 1 };
        assert_eq!(HookDispatcher::map_snapshot_kind(&event), SnapshotKind::Full);
    }

    #[test]
    fn test_fail_fast_snapshot() {
        let event = HookEvent::TestFail { count: 1, summary: "s".into() };
        assert_eq!(HookDispatcher::map_snapshot_kind(&event), SnapshotKind::Fast);
    }

    // --- Dispatcher tests ---

    #[test]
    fn dispatcher_starts_at_tick_zero() {
        let d = HookDispatcher::new();
        assert_eq!(d.tick(), 0);
        assert_eq!(d.frame_count(), 0);
    }

    #[test]
    fn dispatcher_advances_tick_on_dispatch() {
        let mut d = HookDispatcher::new();
        let frame = d.dispatch(HookEvent::Keystroke { uri: "f".into() });
        assert_eq!(frame.tick, 1);
        assert_eq!(d.tick(), 1);
        assert_eq!(d.frame_count(), 1);

        let frame2 = d.dispatch(HookEvent::FileWrite { uri: "f".into() });
        assert_eq!(frame2.tick, 2);
        assert_eq!(d.tick(), 2);
        assert_eq!(d.frame_count(), 2);
    }

    #[test]
    fn dispatcher_produces_deterministic_snapshot_oid() {
        let mut d1 = HookDispatcher::new();
        let mut d2 = HookDispatcher::new();
        let event = HookEvent::Keystroke { uri: "file:///test.rs".into() };
        let f1 = d1.dispatch(event.clone());
        let f2 = d2.dispatch(event);
        assert_eq!(f1.snapshot_oid, f2.snapshot_oid, "same event + tick = same OID");
    }

    #[test]
    fn dispatcher_different_ticks_different_oids() {
        let mut d = HookDispatcher::new();
        let event = HookEvent::Keystroke { uri: "f".into() };
        let f1 = d.dispatch(event.clone());
        let f2 = d.dispatch(event);
        assert_ne!(f1.snapshot_oid, f2.snapshot_oid, "different ticks = different OIDs");
    }

    #[test]
    fn frame_contains_correct_action() {
        let mut d = HookDispatcher::new();
        let frame = d.dispatch(HookEvent::GitCommit { commit_hash: "abc".into() });
        match frame.action {
            HookAction::Anchor { commit_hash } => assert_eq!(commit_hash, "abc"),
            _ => panic!("expected Anchor action"),
        }
    }

    #[test]
    fn frame_contains_correct_render_hint() {
        let mut d = HookDispatcher::new();
        let frame = d.dispatch(HookEvent::TestFail { count: 1, summary: "s".into() });
        assert_eq!(frame.render_hint, RenderHint::Flicker);
    }

    // --- Overlap tests (Taut's cascade analysis) ---

    #[test]
    fn keystroke_and_file_write_can_overlap() {
        let k = HookEvent::Keystroke { uri: "file:///a.rs".into() };
        let f = HookEvent::FileWrite { uri: "file:///b.rs".into() };
        assert!(HookDispatcher::can_overlap(&k, &f));
    }

    #[test]
    fn same_file_keystrokes_cannot_overlap() {
        let k1 = HookEvent::Keystroke { uri: "file:///a.rs".into() };
        let k2 = HookEvent::Keystroke { uri: "file:///a.rs".into() };
        assert!(!HookDispatcher::can_overlap(&k1, &k2));
    }

    #[test]
    fn different_file_keystrokes_can_overlap() {
        let k1 = HookEvent::Keystroke { uri: "file:///a.rs".into() };
        let k2 = HookEvent::Keystroke { uri: "file:///b.rs".into() };
        assert!(HookDispatcher::can_overlap(&k1, &k2));
    }

    #[test]
    fn accept_and_file_write_cannot_overlap() {
        let a = HookEvent::SuggestionAccept { suggestion_oid: Oid::dark() };
        let f = HookEvent::FileWrite { uri: "f".into() };
        assert!(!HookDispatcher::can_overlap(&a, &f));
    }

    #[test]
    fn file_write_and_git_commit_cannot_overlap() {
        let f = HookEvent::FileWrite { uri: "f".into() };
        let g = HookEvent::GitCommit { commit_hash: "x".into() };
        assert!(!HookDispatcher::can_overlap(&f, &g));
    }

    #[test]
    fn git_commit_and_test_pass_cannot_overlap() {
        let g = HookEvent::GitCommit { commit_hash: "x".into() };
        let t = HookEvent::TestPass { count: 1 };
        assert!(!HookDispatcher::can_overlap(&g, &t));
    }

    #[test]
    fn different_player_suggestions_can_overlap() {
        let s1 = HookEvent::SuggestionArrive {
            player: "abyss".into(),
            suggestion_oid: Oid::dark(),
        };
        let s2 = HookEvent::SuggestionArrive {
            player: "fate".into(),
            suggestion_oid: Oid::dark(),
        };
        assert!(HookDispatcher::can_overlap(&s1, &s2));
    }

    #[test]
    fn same_player_suggestions_cannot_overlap() {
        let s1 = HookEvent::SuggestionArrive {
            player: "fate".into(),
            suggestion_oid: Oid::hash(b"a"),
        };
        let s2 = HookEvent::SuggestionArrive {
            player: "fate".into(),
            suggestion_oid: Oid::hash(b"b"),
        };
        assert!(!HookDispatcher::can_overlap(&s1, &s2));
    }

    #[test]
    fn prompt_submit_and_file_write_can_overlap() {
        let p = HookEvent::PromptSubmit { prompt_hash: 0 };
        let f = HookEvent::FileWrite { uri: "f".into() };
        assert!(HookDispatcher::can_overlap(&p, &f));
    }

    // --- Worst case cascade (Taut's line) ---

    #[test]
    fn worst_case_cascade_budget() {
        let (total, fits) = HookDispatcher::worst_case_cascade_us();
        // prompt(5) + arrive(5) + accept(10) + file-write(20) + test-pass(100) = 140ms
        assert_eq!(total, 140_000);
        // 140ms does NOT fit in a single 60fps frame (16.67ms)
        assert!(!fits, "worst case cascade spans multiple frames");
    }

    #[test]
    fn max_events_per_frame_is_reasonable() {
        let (max_keystrokes, available) = HookDispatcher::max_events_per_frame();
        // At 60fps with 41% headroom: ~9.8ms available
        // Keystrokes at 2ms budget: ~4-5 per frame
        assert!(max_keystrokes >= 4, "should fit at least 4 keystrokes, got {}", max_keystrokes);
        assert!(max_keystrokes <= 10, "should not exceed 10, got {}", max_keystrokes);
        assert!(available > 5_000, "available budget should be > 5ms");
    }

    // --- Default trait ---

    #[test]
    fn dispatcher_default_equals_new() {
        let d = HookDispatcher::default();
        assert_eq!(d.tick(), 0);
        assert_eq!(d.frame_count(), 0);
    }
}
