// mote.wgsl — Radial gradient circle shader with additive blending.
// One quad (2 triangles, 6 vertices) is emitted per mote.
// The vertex shader expands each mote to cover its glow_radius extent.
// The fragment shader computes the radial gradient.

// Layout: std430 rules for storage buffers.
// vec2<f32>=align8/size8, f32=align4/size4, vec4<f32>=align16/size16
// Total: 8+4+4+16+4+4+4+4 = 48 bytes. Struct align=16. 48 = 3×16. OK.
struct MoteData {
    position:    vec2<f32>,  // offset  0, size  8
    radius:      f32,        // offset  8, size  4
    glow_radius: f32,        // offset 12, size  4
    color:       vec4<f32>,  // offset 16, size 16
    energy:      f32,        // offset 32, size  4
    _pad0:       f32,        // offset 36, size  4
    _pad1:       f32,        // offset 40, size  4
    _pad2:       f32,        // offset 44, size  4 → total 48
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) frag_pos:       vec2<f32>, // position within the quad in [-1,1] NDC
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
fn vs_main(
    @builtin(vertex_index) vert_idx: u32,
) -> VertexOut {
    let mote_idx = vert_idx / 6u;
    let corner_idx = vert_idx % 6u;
    let m = motes[mote_idx];

    let corner = QUAD[corner_idx];
    // Scale the quad to cover the glow radius
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

    // Discard fragments entirely outside the glow radius
    if dist > in.glow_radius {
        discard;
    }

    // Nucleus: solid circle inside radius
    let inner = smoothstep(in.radius, in.radius * 0.8, dist);
    // Glow halo: fades from radius out to glow_radius
    let glow  = smoothstep(in.glow_radius, in.radius, dist);

    let alpha = max(inner, glow * 0.4) * in.energy;

    // Premultiplied alpha for additive blending
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
