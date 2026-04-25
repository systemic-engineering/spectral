// mote.rs — Mote: the eigenboard primitive (position, radius, color, glow, energy).

use bytemuck::{Pod, Zeroable};

/// A single glowing node in the eigenboard field.
/// Rendered as a radial-gradient circle with an additive glow halo.
#[derive(Clone, Debug, PartialEq)]
pub struct Mote {
    /// Position in normalized device coords, [-1, 1] on both axes.
    pub position: [f32; 2],
    /// Geometric radius of the solid nucleus, in NDC units.
    pub radius: f32,
    /// RGBA color (linear, 0.0–1.0). Alpha drives peak opacity.
    pub color: [f32; 4],
    /// Halo radius — the glow extends from `radius` to `glow_radius`.
    /// Must be >= radius. The quad covers this extent.
    pub glow_radius: f32,
    /// Spectral energy — scales total brightness of the mote.
    pub energy: f32,
}

/// GPU-layout mirror of `Mote`.
/// Must match the `MoteData` struct in `mote.wgsl` exactly.
/// std430 layout: vec2=align8, f32=align4, vec4=align16.
/// Total: 8+4+4+16+4+4+4+4 = 48 bytes. Struct align=16. 48 = 3×16.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct MoteGpu {
    pub position:    [f32; 2],  // offset  0
    pub radius:      f32,        // offset  8
    pub glow_radius: f32,        // offset 12
    pub color:       [f32; 4],  // offset 16
    pub energy:      f32,        // offset 32
    pub _pad0:       f32,        // offset 36
    pub _pad1:       f32,        // offset 40
    pub _pad2:       f32,        // offset 44 → total 48
}

impl From<&Mote> for MoteGpu {
    fn from(m: &Mote) -> Self {
        Self {
            position:    m.position,
            radius:      m.radius,
            glow_radius: m.glow_radius,
            color:       m.color,
            energy:      m.energy,
            _pad0:       0.0,
            _pad1:       0.0,
            _pad2:       0.0,
        }
    }
}
