// hook_latency.rs -- [Taut] benchmark suite for hook dispatch latency.
//
// Every hook has a latency budget. This suite measures them.
//
// | Hook                      | Budget  | What's measured                          |
// |---------------------------|---------|------------------------------------------|
// | keystroke -> focus-shift   | <2ms    | eigenvalue lookup + gutter update         |
// | prompt-submit -> fan-out   | <5ms    | dispatcher overhead (spawn is async)      |
// | suggestion-accept -> collapse | <10ms | weight shift + snapshot                |
// | file-write -> observe      | <20ms   | gestalt reparse + dirty cascade + snapshot|
// | git-commit -> anchor       | <50ms   | full CoincidenceHash + crystallize check  |
// | test-pass -> crystallize   | <100ms  | crystal check + conditional deploy        |
//
// The cascade question: which hooks can overlap? Which must be sequential?
// The worst case: prompt-submit -> 5x arrive -> accept -> file-write -> test-pass.
// That's 140ms sequential. Multiple frames. The line is: does it complete
// before the NEXT prompt-submit?
//
// Apache-2.0

// We need the `sel` feature for the hooks module.
// When running without `sel`, these benchmarks are no-ops.
#[cfg(feature = "sel")]
mod with_sel {
    use spectral::sel::hooks::{HookDispatcher, HookEvent};
    use prism::oid::Oid;

    // =========================================================================
    // Part 1: Individual hook dispatch latency
    // =========================================================================

    #[divan::bench]
    fn hook_keystroke_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let event = HookEvent::Keystroke {
                uri: "file:///src/lib.rs".into(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_prompt_submit_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let event = HookEvent::PromptSubmit { prompt_hash: 42 };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_suggestion_arrive_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        let oid = Oid::hash(b"suggestion-content");
        bencher.bench_local(|| {
            let event = HookEvent::SuggestionArrive {
                player: "fate".into(),
                suggestion_oid: oid.clone(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_suggestion_accept_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        let oid = Oid::hash(b"accepted-suggestion");
        bencher.bench_local(|| {
            let event = HookEvent::SuggestionAccept {
                suggestion_oid: oid.clone(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_suggestion_reject_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        let oid = Oid::hash(b"rejected-suggestion");
        bencher.bench_local(|| {
            let event = HookEvent::SuggestionReject {
                suggestion_oid: oid.clone(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_file_write_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let event = HookEvent::FileWrite {
                uri: "file:///src/sel/hooks.rs".into(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_git_commit_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let event = HookEvent::GitCommit {
                commit_hash: "abc123def456".into(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_test_pass_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let event = HookEvent::TestPass { count: 48 };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    #[divan::bench]
    fn hook_test_fail_dispatch(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let event = HookEvent::TestFail {
                count: 3,
                summary: "3 assertions failed in sel::hooks".into(),
            };
            divan::black_box(dispatcher.dispatch(event));
        });
    }

    // =========================================================================
    // Part 2: Cascade sequences — the worst-case paths
    // =========================================================================

    /// The happy path: keystroke -> file-write (save on type).
    /// Budget: 2ms + 20ms = 22ms sequential. Should overlap.
    #[divan::bench]
    fn cascade_keystroke_then_file_write(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let k = HookEvent::Keystroke { uri: "file:///src/lib.rs".into() };
            let f = HookEvent::FileWrite { uri: "file:///src/lib.rs".into() };
            divan::black_box(dispatcher.dispatch(k));
            divan::black_box(dispatcher.dispatch(f));
        });
    }

    /// The commit path: file-write -> git-commit -> test-pass.
    /// Budget: 20ms + 50ms + 100ms = 170ms sequential. Must be sequential.
    #[divan::bench]
    fn cascade_write_commit_test(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            let f = HookEvent::FileWrite { uri: "file:///src/lib.rs".into() };
            let g = HookEvent::GitCommit { commit_hash: "abc123".into() };
            let t = HookEvent::TestPass { count: 48 };
            divan::black_box(dispatcher.dispatch(f));
            divan::black_box(dispatcher.dispatch(g));
            divan::black_box(dispatcher.dispatch(t));
        });
    }

    /// The full worst case: prompt-submit -> 5x arrive -> accept -> file-write -> test-pass.
    /// Budget: 5 + 5 + 10 + 20 + 100 = 140ms. The line through the worst case.
    #[divan::bench]
    fn cascade_worst_case_full(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        let oid = Oid::hash(b"suggestion");
        bencher.bench_local(|| {
            // prompt-submit
            divan::black_box(dispatcher.dispatch(
                HookEvent::PromptSubmit { prompt_hash: 99 },
            ));
            // 5x suggestion-arrive (these can overlap in production)
            for player in &["abyss", "introject", "cartographer", "explorer", "fate"] {
                divan::black_box(dispatcher.dispatch(
                    HookEvent::SuggestionArrive {
                        player: (*player).into(),
                        suggestion_oid: oid.clone(),
                    },
                ));
            }
            // accept
            divan::black_box(dispatcher.dispatch(
                HookEvent::SuggestionAccept { suggestion_oid: oid.clone() },
            ));
            // file-write
            divan::black_box(dispatcher.dispatch(
                HookEvent::FileWrite { uri: "file:///src/lib.rs".into() },
            ));
            // test-pass (crystallize)
            divan::black_box(dispatcher.dispatch(
                HookEvent::TestPass { count: 48 },
            ));
        });
    }

    /// Tournament fan-out: prompt-submit followed by 5 suggestion arrivals.
    /// The arrives can overlap (different players). Measures the fan-out cost.
    #[divan::bench]
    fn cascade_tournament_fan_out(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        let oid = Oid::hash(b"tournament-suggestion");
        bencher.bench_local(|| {
            divan::black_box(dispatcher.dispatch(
                HookEvent::PromptSubmit { prompt_hash: 1 },
            ));
            for player in &["abyss", "introject", "cartographer", "explorer", "fate"] {
                divan::black_box(dispatcher.dispatch(
                    HookEvent::SuggestionArrive {
                        player: (*player).into(),
                        suggestion_oid: oid.clone(),
                    },
                ));
            }
        });
    }

    // =========================================================================
    // Part 3: Throughput — how many events per frame?
    // =========================================================================

    /// Dispatch N keystrokes and measure total time.
    /// At 60fps, we have 16.67ms per frame. How many keystrokes fit?
    #[divan::bench(args = [1, 5, 10, 50, 100])]
    fn throughput_keystrokes(bencher: divan::Bencher, n: usize) {
        let mut dispatcher = HookDispatcher::new();
        bencher.bench_local(|| {
            for i in 0..n {
                let event = HookEvent::Keystroke {
                    uri: format!("file:///src/file_{}.rs", i % 10),
                };
                divan::black_box(dispatcher.dispatch(event));
            }
        });
    }

    /// Mixed event burst: keystrokes + file-writes + suggestions.
    /// Simulates a typical editing burst.
    #[divan::bench]
    fn throughput_mixed_burst(bencher: divan::Bencher) {
        let mut dispatcher = HookDispatcher::new();
        let oid = Oid::hash(b"burst");
        bencher.bench_local(|| {
            // 10 keystrokes
            for _ in 0..10 {
                divan::black_box(dispatcher.dispatch(
                    HookEvent::Keystroke { uri: "file:///src/lib.rs".into() },
                ));
            }
            // 1 file-write (save)
            divan::black_box(dispatcher.dispatch(
                HookEvent::FileWrite { uri: "file:///src/lib.rs".into() },
            ));
            // 2 suggestion arrives
            divan::black_box(dispatcher.dispatch(
                HookEvent::SuggestionArrive {
                    player: "fate".into(),
                    suggestion_oid: oid.clone(),
                },
            ));
            divan::black_box(dispatcher.dispatch(
                HookEvent::SuggestionArrive {
                    player: "abyss".into(),
                    suggestion_oid: oid.clone(),
                },
            ));
            // 1 accept
            divan::black_box(dispatcher.dispatch(
                HookEvent::SuggestionAccept { suggestion_oid: oid.clone() },
            ));
        });
    }

    // =========================================================================
    // Part 4: Overlap analysis — can_overlap checks
    // =========================================================================

    /// Benchmark the overlap check itself (should be sub-nanosecond).
    #[divan::bench]
    fn overlap_check_keystroke_file_write(bencher: divan::Bencher) {
        let k = HookEvent::Keystroke { uri: "file:///a.rs".into() };
        let f = HookEvent::FileWrite { uri: "file:///b.rs".into() };
        bencher.bench_local(|| {
            divan::black_box(HookDispatcher::can_overlap(&k, &f));
        });
    }

    #[divan::bench]
    fn overlap_check_accept_file_write(bencher: divan::Bencher) {
        let a = HookEvent::SuggestionAccept { suggestion_oid: Oid::dark() };
        let f = HookEvent::FileWrite { uri: "file:///a.rs".into() };
        bencher.bench_local(|| {
            divan::black_box(HookDispatcher::can_overlap(&a, &f));
        });
    }
}

fn main() {
    divan::main();
}
