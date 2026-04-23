// superposition.rs -- Benchmark suite for the superposition architecture.
//
// [Taut] The GPU holds the superposition. The CPU collapses it.
// This file measures the five numbers that define the system's aperture:
//
// 1. CPU snapshot cost: serialize + content-address + OID stamp. Target: <1ms.
// 2. GPU superposition hold: how many ticks can the GPU hold compute | render
//    branches alive before the frame budget forces a collapse?
// 3. Collapse latency: decision to wgpu render pass begin.
// 4. Cascade ratio: superposition_hold_time / snapshot_time. This IS the aperture.
// 5. N -> N+1 convergence: does superposition width narrow over successive ticks?
//
// The cascade ratio tells you how much uncertainty the system can hold between
// measurements. Higher ratio = wider aperture = more can happen between anchors.
// Not Hz. Aperture.
//
// Apache-2.0

use spectral_ui::{Arc, Context, DeviceState, Field, Mote, Snapshot, SpectralGpu};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate N motes distributed across NDC space.
fn make_motes(n: usize) -> Vec<Mote> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n.max(1) as f32;
            let angle = t * std::f32::consts::TAU;
            let r = 0.6 * t;
            Mote {
                position: [r * angle.cos(), r * angle.sin()],
                radius: 0.02 + 0.01 * (i % 3) as f32,
                color: [0.2 + 0.3 * t, 0.8 - 0.3 * t, 1.0, 1.0],
                glow_radius: 0.04 + 0.02 * (i % 3) as f32,
                energy: 0.5 + 0.5 * t,
            }
        })
        .collect()
}

/// Generate arcs connecting sequential motes in a ring.
fn make_arcs(mote_count: usize, arc_count: usize) -> Vec<Arc> {
    (0..arc_count)
        .map(|i| Arc {
            from: i % mote_count,
            to: (i + 1) % mote_count,
            strength: 0.3 + 0.4 * (i % 5) as f32 / 5.0,
        })
        .collect()
}

fn make_field(n: usize) -> Field {
    let motes = make_motes(n);
    Field {
        motes,
        arcs: vec![],
        viewer_idx: 0,
    }
}

fn make_full_field(n: usize) -> Field {
    let motes = make_motes(n);
    let arcs = make_arcs(n, n);
    Field {
        motes,
        arcs,
        viewer_idx: 0,
    }
}

// ---------------------------------------------------------------------------
// [Taut] Benchmark 1: CPU snapshot cost
// ---------------------------------------------------------------------------
// serialize field state + content-address + OID stamp.
// Two paths: full (CoincidenceHash eigenvalue) and fast (FNV-1a).
// The full path is the truth. The fast path is the clock tick.

#[divan::bench(args = [50, 100, 200])]
fn bench_cpu_snapshot_full(bencher: divan::Bencher, n: usize) {
    let field = make_field(n);
    let mut tick = 0u64;
    bencher.bench_local(|| {
        tick += 1;
        divan::black_box(Snapshot::capture(&field, tick, DeviceState::Render));
    });
}

#[divan::bench(args = [50, 100, 200])]
fn bench_cpu_snapshot_fast(bencher: divan::Bencher, n: usize) {
    let field = make_field(n);
    let mut tick = 0u64;
    bencher.bench_local(|| {
        tick += 1;
        divan::black_box(Snapshot::capture_fast(&field, tick, DeviceState::Render));
    });
}

// ---------------------------------------------------------------------------
// [Taut] Benchmark 2: GPU superposition hold time
// ---------------------------------------------------------------------------
// How many ticks can the GPU hold compute | render branches alive
// simultaneously before the frame budget (16.7ms @ 60fps) is exceeded?
// We measure one full render pass per tick to find the maximum tick count
// within the budget.

#[divan::bench]
fn bench_gpu_superposition_hold(bencher: divan::Bencher) {
    let mut gpu = SpectralGpu::new();
    let field = make_full_field(200);
    bencher.bench_local(|| {
        // One render dispatch + one snapshot = one tick of the superposition.
        // The benchmark framework measures per-iteration time.
        // Divide 16.7ms by per-iteration time to get max ticks in frame budget.
        let _pixels = gpu.dispatch_render(&field);
        let _snap = gpu.snapshot_fast(&field, DeviceState::Render);
    });
}

// ---------------------------------------------------------------------------
// [Taut] Benchmark 3: Collapse latency (wgpu render path)
// ---------------------------------------------------------------------------
// From "measurement decided" to "GPU dispatches the correct branch."
// The | resolves to one path. How fast does wgpu dispatch happen
// after the decision?

#[divan::bench(args = [50, 100, 200])]
fn bench_collapse_wgpu(bencher: divan::Bencher, n: usize) {
    let mut ctx = Context::new();
    let field = make_full_field(n);
    bencher.bench_local(|| {
        // The collapse: field.render() IS the wgpu dispatch.
        // We measure from the call to first pixels returned.
        divan::black_box(field.render(&mut ctx));
    });
}

// ---------------------------------------------------------------------------
// [Taut] Benchmark 4: Cascade ratio at N = 50, 100, 200 motes
// ---------------------------------------------------------------------------
// superposition_hold_time / snapshot_time at different mote counts.
// Higher ratio = wider aperture = more uncertainty held between anchors.
// Uses the fast snapshot (FNV-1a) -- the real-time clock tick.

#[divan::bench(args = [50, 100, 200])]
fn bench_cascade_ratio(bencher: divan::Bencher, n: usize) {
    let mut gpu = SpectralGpu::new();
    let field = make_full_field(n);

    bencher.bench_local(|| {
        // Measure snapshot cost (the denominator) -- fast path.
        let snap_start = std::time::Instant::now();
        let _snap = gpu.snapshot_fast(&field, DeviceState::Render);
        let snap_ns = snap_start.elapsed().as_nanos() as u64;

        // Measure render cost (the numerator -- one superposition tick).
        let hold_start = std::time::Instant::now();
        let _pixels = gpu.dispatch_render(&field);
        let hold_ns = hold_start.elapsed().as_nanos() as u64;

        let ratio = gpu.cascade_ratio(hold_ns, snap_ns);
        divan::black_box(ratio);
    });
}

// ---------------------------------------------------------------------------
// [Taut] Benchmark 5: N -> N+1 convergence rate
// ---------------------------------------------------------------------------
// Run 100 ticks with alternating state changes, measure whether
// superposition width narrows. The convergence rate IS the system's
// self-organization velocity. Uses fast snapshot for the hot path.

#[divan::bench]
fn bench_convergence_rate(bencher: divan::Bencher) {
    bencher.bench_local(|| {
        let mut gpu = SpectralGpu::new();
        let field = make_field(200);

        // Simulate 100 ticks with a pattern that should produce convergence:
        // start with long holds, decrease over time (system learning its rhythm).
        let mut hold_length = 10u64;
        let mut current_state = DeviceState::Render;

        for tick in 0..100u64 {
            // Switch state every `hold_length` ticks, with hold_length
            // decreasing as the system "settles."
            if tick > 0 && tick % hold_length == 0 {
                current_state = match current_state {
                    DeviceState::Render => DeviceState::Compute,
                    DeviceState::Compute => DeviceState::Render,
                    DeviceState::Idle => DeviceState::Render,
                };
                // Narrow the hold length (convergence).
                // Floor at 2 to avoid division by zero.
                if hold_length > 2 {
                    hold_length -= 1;
                }
            }
            gpu.snapshot_fast(&field, current_state);
        }

        let rate = gpu.convergence_rate();
        divan::black_box(rate);
    });
}

// ---------------------------------------------------------------------------
// [Taut] Snapshot scaling: does cost grow linearly with mote count?
// ---------------------------------------------------------------------------
// Full (CoincidenceHash) path -- measures the content-address hash cost.

#[divan::bench(args = [1, 10, 50, 100, 200, 500, 1000])]
fn bench_snapshot_scaling_full(bencher: divan::Bencher, n: usize) {
    let field = make_field(n);
    bencher.bench_local(|| {
        divan::black_box(Snapshot::capture(&field, 1, DeviceState::Idle));
    });
}

// Fast (FNV-1a) path -- the hot-path clock tick.

#[divan::bench(args = [1, 10, 50, 100, 200, 500, 1000])]
fn bench_snapshot_scaling_fast(bencher: divan::Bencher, n: usize) {
    let field = make_field(n);
    bencher.bench_local(|| {
        divan::black_box(Snapshot::capture_fast(&field, 1, DeviceState::Idle));
    });
}

// ---------------------------------------------------------------------------
// [Taut] Full cycle: snapshot + dispatch + snapshot
// ---------------------------------------------------------------------------
// The real workload: snapshot before dispatch, dispatch, snapshot after.
// Three operations that define one tick of the superposition architecture.

#[divan::bench]
fn bench_full_tick_cycle(bencher: divan::Bencher) {
    let mut gpu = SpectralGpu::new();
    let field = make_full_field(200);
    bencher.bench_local(|| {
        // 1. Snapshot: measure the pre-dispatch state (fast path)
        let _pre = gpu.snapshot_fast(&field, DeviceState::Idle);
        // 2. Dispatch: collapse to render
        let _pixels = gpu.dispatch_render(&field);
        // 3. Snapshot: measure the post-dispatch state (fast path)
        let _post = gpu.snapshot_fast(&field, DeviceState::Render);
    });
}

fn main() {
    divan::main();
}
