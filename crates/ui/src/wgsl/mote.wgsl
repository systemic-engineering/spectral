// mote.wgsl — Radial gradient circle shader with additive blending.
// One quad (2 triangles, 6 vertices) is emitted per mote via vertex_index.
// Data comes from a storage buffer — no vertex buffer needed.
//
// MoteData layout (std430):
//   offset  0: position    vec2<f32>  (8 bytes)
//   offset  8: radius      f32        (4 bytes)
//   offset 12: glow_radius f32        (4 bytes)
//   offset 16: color       vec4<f32>  (16 bytes)
//   offset 32: energy      f32        (4 bytes)
//   offset 36: _pad0..2    f32 × 3   (12 bytes)
//   total: 48 bytes. struct align = 16. 48 = 3 × 16.

struct MoteData {
    position:    vec2<f32>,
    radius:      f32,
    glow_radius: f32,
    color:       vec4<f32>,
    energy:      f32,
    _pad0:       f32,
    _pad1:       f32,
    _pad2:       f32,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) frag_pos:       vec2<f32>,
    @location(1) mote_center:    vec2<f32>,
    @location(2) radius:         f32,
    @location(3) glow_radius:    f32,
    @location(4) color:          vec4<f32>,
    @location(5) energy:         f32,
}

// Quad corners in local space (two triangles, CCW winding)
var<private> QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
);

@group(0) @binding(0) var<storage, read> motes: array<MoteData>;

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32) -> VertexOut {
    let mote_idx  = vert_idx / 6u;
    let corner    = QUAD[vert_idx % 6u];
    let m         = motes[mote_idx];
    let world_pos = m.position + corner * m.glow_radius;

    var out: VertexOut;
    out.clip_pos    = vec4<f32>(world_pos, 0.0, 1.0);
    out.frag_pos    = world_pos;
    out.mote_center = m.position;
    out.radius      = m.radius;
    out.glow_radius = m.glow_radius;
    out.color       = m.color;
    out.energy      = m.energy;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dist = length(in.frag_pos - in.mote_center);

    if dist > in.glow_radius {
        discard;
    }

    // Nucleus: solid circle inside radius
    let inner = smoothstep(in.radius, in.radius * 0.8, dist);
    // Glow halo: fades from radius out to glow_radius
    let glow  = smoothstep(in.glow_radius, in.radius, dist);

    let alpha = max(inner, glow * 0.4) * in.energy;

    // Pre-multiplied alpha for additive blending
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
