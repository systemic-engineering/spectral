// superposition.rs -- The GPU holds the superposition. The CPU collapses it.
//
// [Taut] The | operator is simultaneous, not alternative.
// metal | wgpu means both paths exist. The snapshot measures which one
// produced output this tick.
//
// SpectralGpu holds compute and render branches alive at the same time.
// The CPU's only job is `snapshot` -- content-address the current state,
// stamp an OID, tick the clock. That's the measurement.
//
// The cascade ratio (superposition_hold_time / snapshot_time) IS the
// system's aperture. Higher ratio = wider aperture = more can happen
// between anchors.
//
// Apache-2.0

use prism_core::oid::Oid;

use crate::context::Context;
use crate::field::Field;

// ---------------------------------------------------------------------------
// Device state: the branches of the superposition
// ---------------------------------------------------------------------------

/// Which dispatch branch is active this tick.
/// The `|` resolves to one path. This is the measurement result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceState {
    /// Metal compute path (fate inference on GPU).
    Compute,
    /// wgpu render path (spectral-ui eigenboard).
    Render,
    /// Neither branch dispatched. The system is between measurements.
    Idle,
}

// ---------------------------------------------------------------------------
// Snapshot: the CPU's clock tick
// ---------------------------------------------------------------------------

/// A content-addressed anchor in spacetime.
/// The CPU serializes the field state, hashes it, stamps the OID.
/// This is the clock tick. Sub-millisecond is the target.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The content address of the field state at this tick.
    pub oid: Oid,
    /// The tick number (monotonic).
    pub tick: u64,
    /// Which branch produced output this tick.
    pub state: DeviceState,
    /// Number of motes in the field at snapshot time.
    pub mote_count: usize,
}

impl Snapshot {
    /// Take a snapshot of the current field state.
    /// Serializes mote positions + energies, content-addresses the result.
    ///
    /// Uses the full CoincidenceHash content address (eigenvalue-based).
    /// For hot-path snapshots where sub-millisecond timing is critical,
    /// use `capture_fast` instead.
    pub fn capture(field: &Field, tick: u64, state: DeviceState) -> Self {
        let bytes = Self::serialize_field(field);
        let oid = Oid::hash(&bytes);
        Snapshot {
            oid,
            tick,
            state,
            mote_count: field.motes.len(),
        }
    }

    /// Fast snapshot: FNV-1a hash instead of CoincidenceHash.
    ///
    /// The full `capture` uses eigenvalue-based content addressing via
    /// CoincidenceHash<3> -- 16-dimensional projection, three observers.
    /// Beautiful, correct, but 8ms at 200 motes. The CPU clock tick must be
    /// sub-millisecond for the cascade ratio to work.
    ///
    /// FNV-1a is a non-cryptographic hash: 64-bit, no collisions for practical
    /// field sizes, and critically: O(n) with tiny constant factor. The OID
    /// is not content-addressed in the prism sense, but it's deterministic
    /// and unique enough for the hot-path superposition clock.
    ///
    /// For persistence / regression detection, use `capture` (full OID).
    /// For real-time aperture measurement, use `capture_fast`.
    pub fn capture_fast(field: &Field, tick: u64, state: DeviceState) -> Self {
        let bytes = Self::serialize_field(field);
        let hash = fnv1a_64(&bytes);
        let oid = Oid::new(format!("{:016x}", hash));
        Snapshot {
            oid,
            tick,
            state,
            mote_count: field.motes.len(),
        }
    }

    /// Serialize the field into a byte buffer for content addressing.
    /// Only position + energy -- the render-relevant state.
    /// 200 motes x 12 bytes (2 f32 pos + 1 f32 energy) = 2.4KB.
    fn serialize_field(field: &Field) -> Vec<u8> {
        let mut buf = Vec::with_capacity(field.motes.len() * 12 + 8);
        // Tick-independent: the content IS the identity.
        // Two fields with the same mote layout have the same OID.
        buf.extend_from_slice(&(field.motes.len() as u64).to_le_bytes());
        for mote in &field.motes {
            buf.extend_from_slice(&mote.position[0].to_le_bytes());
            buf.extend_from_slice(&mote.position[1].to_le_bytes());
            buf.extend_from_slice(&mote.energy.to_le_bytes());
        }
        buf
    }
}

/// FNV-1a 64-bit hash. Non-cryptographic. Deterministic.
/// Zero dependencies. The hot-path clock tick hash.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ---------------------------------------------------------------------------
// SpectralGpu: the superposition holder
// ---------------------------------------------------------------------------

/// The GPU holds the superposition. The CPU collapses it.
///
/// Both paths (compute and render) exist simultaneously. The `|` is not
/// an enum -- it's simultaneous presence. The snapshot measures which
/// one produced output this tick.
///
/// The cascade: GPU holds everything, CPU ticks once, the tick narrows
/// the next hold. The ratio between hold and tick IS the system's
/// clock speed. Not Hz. Aperture.
pub struct SpectralGpu {
    /// The wgpu render context (spectral-ui's path).
    /// Always present -- this is the eigenboard renderer.
    pub wgpu: Context,

    /// Current tick counter (monotonic).
    pub tick: u64,

    /// Last snapshot OID -- the content-addressed anchor.
    pub last_snapshot: Oid,

    /// Current device state (which branch was last dispatched).
    pub state: DeviceState,

    /// Superposition width: how many ticks since the last state change.
    /// Measures how long the system held multiple branches alive.
    pub superposition_width: u64,

    /// History of superposition widths (for convergence analysis).
    pub width_history: Vec<u64>,
}

impl SpectralGpu {
    /// Create a new SpectralGpu context.
    /// Starts in Idle state with a dark OID (no measurement yet).
    pub fn new() -> Self {
        SpectralGpu {
            wgpu: Context::new(),
            tick: 0,
            last_snapshot: Oid::dark(),
            state: DeviceState::Idle,
            superposition_width: 0,
            width_history: Vec::new(),
        }
    }

    /// Take a snapshot: serialize the field, content-address it, advance the clock.
    /// This is the CPU's only job. The measurement.
    ///
    /// Uses the full CoincidenceHash (eigenvalue-based). For hot-path
    /// snapshots, use `snapshot_fast`.
    pub fn snapshot(&mut self, field: &Field, new_state: DeviceState) -> Snapshot {
        self.advance_state(new_state);
        let snap = Snapshot::capture(field, self.tick, new_state);
        self.last_snapshot = snap.oid.clone();
        snap
    }

    /// Fast snapshot: FNV-1a hash for the hot-path clock tick.
    /// Same state tracking, cheaper hash. Use this in the render loop.
    pub fn snapshot_fast(&mut self, field: &Field, new_state: DeviceState) -> Snapshot {
        self.advance_state(new_state);
        let snap = Snapshot::capture_fast(field, self.tick, new_state);
        self.last_snapshot = snap.oid.clone();
        snap
    }

    /// Advance the superposition state machine.
    fn advance_state(&mut self, new_state: DeviceState) {
        if new_state != self.state {
            if self.tick > 0 {
                self.width_history.push(self.superposition_width);
            }
            self.superposition_width = 0;
        } else {
            self.superposition_width += 1;
        }

        self.tick += 1;
        self.state = new_state;
    }

    /// Dispatch: collapse the superposition to a specific branch.
    /// Returns the field render output if dispatching to Render.
    pub fn dispatch_render(&mut self, field: &Field) -> Vec<u8> {
        self.state = DeviceState::Render;
        field.render(&mut self.wgpu)
    }

    /// Dispatch: collapse to the Compute branch.
    /// In production this would launch a Metal compute kernel.
    /// Here we simulate the decision point -- the latency is what matters.
    pub fn dispatch_compute(&mut self) {
        self.state = DeviceState::Compute;
        // Metal compute would happen here.
        // The benchmark measures the decision-to-dispatch latency,
        // not the kernel execution time.
    }

    /// The cascade ratio: superposition_hold_time / snapshot_time.
    /// This IS the system's aperture.
    /// Higher = wider aperture = more uncertainty held between anchors.
    pub fn cascade_ratio(&self, hold_ns: u64, snapshot_ns: u64) -> f64 {
        if snapshot_ns == 0 {
            return f64::INFINITY;
        }
        hold_ns as f64 / snapshot_ns as f64
    }

    /// Convergence rate: does the superposition width decrease over time?
    /// Returns the slope of a linear fit to width_history.
    /// Negative slope = converging. Zero = stable. Positive = diverging.
    pub fn convergence_rate(&self) -> f64 {
        if self.width_history.len() < 2 {
            return 0.0;
        }

        let n = self.width_history.len() as f64;
        let sum_x: f64 = (0..self.width_history.len()).map(|i| i as f64).sum();
        let sum_y: f64 = self.width_history.iter().map(|&w| w as f64).sum();
        let sum_xy: f64 = self.width_history
            .iter()
            .enumerate()
            .map(|(i, &w)| i as f64 * w as f64)
            .sum();
        let sum_xx: f64 = (0..self.width_history.len()).map(|i| (i * i) as f64).sum();

        // Linear regression slope: (n*sum_xy - sum_x*sum_y) / (n*sum_xx - sum_x*sum_x)
        let denom = n * sum_xx - sum_x * sum_x;
        if denom.abs() < 1e-12 {
            return 0.0;
        }
        (n * sum_xy - sum_x * sum_y) / denom
    }
}

impl Default for SpectralGpu {
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
    use crate::mote::Mote;

    fn make_field(n: usize) -> Field {
        let motes: Vec<Mote> = (0..n)
            .map(|i| {
                let t = i as f32 / n.max(1) as f32;
                Mote {
                    position: [t, t],
                    radius: 0.02,
                    color: [1.0, 1.0, 1.0, 1.0],
                    glow_radius: 0.04,
                    energy: 0.5 + 0.5 * t,
                }
            })
            .collect();
        Field {
            motes,
            arcs: vec![],
            viewer_idx: 0,
        }
    }

    #[test]
    fn snapshot_is_deterministic() {
        let field = make_field(10);
        let a = Snapshot::capture(&field, 1, DeviceState::Render);
        let b = Snapshot::capture(&field, 1, DeviceState::Render);
        assert_eq!(a.oid, b.oid, "same field = same OID");
    }

    #[test]
    fn snapshot_differs_on_content_change() {
        let field_a = make_field(10);
        let field_b = make_field(20);
        let a = Snapshot::capture(&field_a, 1, DeviceState::Render);
        let b = Snapshot::capture(&field_b, 1, DeviceState::Render);
        assert_ne!(a.oid, b.oid, "different field = different OID");
    }

    #[test]
    fn snapshot_oid_is_not_dark() {
        let field = make_field(10);
        let snap = Snapshot::capture(&field, 1, DeviceState::Idle);
        assert!(!snap.oid.is_dark(), "snapshot of non-empty field must not be dark");
    }

    #[test]
    fn fast_snapshot_is_deterministic() {
        let field = make_field(10);
        let a = Snapshot::capture_fast(&field, 1, DeviceState::Render);
        let b = Snapshot::capture_fast(&field, 1, DeviceState::Render);
        assert_eq!(a.oid, b.oid, "same field = same fast OID");
    }

    #[test]
    fn fast_snapshot_differs_on_content_change() {
        let field_a = make_field(10);
        let field_b = make_field(20);
        let a = Snapshot::capture_fast(&field_a, 1, DeviceState::Render);
        let b = Snapshot::capture_fast(&field_b, 1, DeviceState::Render);
        assert_ne!(a.oid, b.oid, "different field = different fast OID");
    }

    #[test]
    fn fast_snapshot_differs_from_full_snapshot() {
        let field = make_field(10);
        let fast = Snapshot::capture_fast(&field, 1, DeviceState::Render);
        let full = Snapshot::capture(&field, 1, DeviceState::Render);
        // Different hash algorithms, different OIDs
        assert_ne!(fast.oid, full.oid, "fast and full use different hashes");
    }

    #[test]
    fn fnv1a_known_values() {
        // FNV-1a("") = offset basis
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325);
        // Deterministic
        assert_eq!(fnv1a_64(b"hello"), fnv1a_64(b"hello"));
        // Different input = different hash
        assert_ne!(fnv1a_64(b"hello"), fnv1a_64(b"world"));
    }

    #[test]
    fn spectral_gpu_starts_idle() {
        let gpu = SpectralGpu::new();
        assert_eq!(gpu.state, DeviceState::Idle);
        assert_eq!(gpu.tick, 0);
        assert!(gpu.last_snapshot.is_dark());
    }

    #[test]
    fn spectral_gpu_snapshot_advances_tick() {
        let mut gpu = SpectralGpu::new();
        let field = make_field(10);
        let snap = gpu.snapshot(&field, DeviceState::Render);
        assert_eq!(snap.tick, 1);
        assert_eq!(gpu.tick, 1);
        assert!(!gpu.last_snapshot.is_dark());
    }

    #[test]
    fn superposition_width_tracks_state_hold() {
        let mut gpu = SpectralGpu::new();
        let field = make_field(10);

        // Three consecutive Render ticks
        gpu.snapshot(&field, DeviceState::Render);
        gpu.snapshot(&field, DeviceState::Render);
        gpu.snapshot(&field, DeviceState::Render);
        assert_eq!(gpu.superposition_width, 2, "held Render for 2 extra ticks");

        // Switch to Compute -- records old width, resets
        gpu.snapshot(&field, DeviceState::Compute);
        assert_eq!(gpu.superposition_width, 0, "reset after state change");
        assert_eq!(gpu.width_history.len(), 1);
        assert_eq!(gpu.width_history[0], 2);
    }

    #[test]
    fn convergence_rate_flat_for_constant_width() {
        let mut gpu = SpectralGpu::new();
        gpu.width_history = vec![5, 5, 5, 5, 5];
        let rate = gpu.convergence_rate();
        assert!(rate.abs() < 1e-10, "constant width = zero slope, got {}", rate);
    }

    #[test]
    fn convergence_rate_negative_for_narrowing() {
        let mut gpu = SpectralGpu::new();
        gpu.width_history = vec![10, 8, 6, 4, 2];
        let rate = gpu.convergence_rate();
        assert!(rate < 0.0, "narrowing width = negative slope, got {}", rate);
    }

    #[test]
    fn convergence_rate_positive_for_widening() {
        let mut gpu = SpectralGpu::new();
        gpu.width_history = vec![2, 4, 6, 8, 10];
        let rate = gpu.convergence_rate();
        assert!(rate > 0.0, "widening width = positive slope, got {}", rate);
    }

    #[test]
    fn cascade_ratio_computes_correctly() {
        let gpu = SpectralGpu::new();
        let ratio = gpu.cascade_ratio(10_000, 500);
        assert!((ratio - 20.0).abs() < 1e-10);
    }

    #[test]
    fn cascade_ratio_infinity_on_zero_snapshot() {
        let gpu = SpectralGpu::new();
        let ratio = gpu.cascade_ratio(10_000, 0);
        assert!(ratio.is_infinite());
    }

    #[test]
    fn device_state_enum_complete() {
        let states = [DeviceState::Compute, DeviceState::Render, DeviceState::Idle];
        assert_eq!(states.len(), 3);
        assert_ne!(states[0], states[1]);
        assert_ne!(states[1], states[2]);
        assert_ne!(states[0], states[2]);
    }
}
