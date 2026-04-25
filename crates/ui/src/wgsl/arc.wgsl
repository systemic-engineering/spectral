// arc.wgsl — Coupling arc shader.
// Each arc is a line segment drawn as a thin quad (2 triangles, 6 vertices).
// Width is proportional to strength; alpha fades toward endpoints and edges.
//
// ArcGpu layout (std430):
//   offset  0: from_pos   vec2<f32>  (8 bytes)
//   offset  8: to_pos     vec2<f32>  (8 bytes)
//   offset 16: strength   f32        (4 bytes)
//   offset 20: _pad0..2   f32 × 3   (12 bytes)
//   total: 32 bytes.

struct ArcData {
    from_pos:  vec2<f32>,
    to_pos:    vec2<f32>,
    strength:  f32,
    _pad0:     f32,
    _pad1:     f32,
    _pad2:     f32,
}

struct VertexOut {
    @builtin(position) clip_pos:    vec4<f32>,
    @location(0)       t:           f32,
    @location(1)       perp_dist:   f32,
    @location(2)       half_width:  f32,
    @location(3)       strength:    f32,
}

// t (parametric, 0..1) and side (-1 or +1) for each of 6 corners
var<private> QUAD_T:    array<f32, 6> = array<f32, 6>(0.0, 0.0, 1.0, 0.0, 1.0, 1.0);
var<private> QUAD_SIDE: array<f32, 6> = array<f32, 6>(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0);

@group(0) @binding(0) var<storage, read> arcs: array<ArcData>;

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32) -> VertexOut {
    let arc_idx    = vert_idx / 6u;
    let corner_idx = vert_idx % 6u;
    let a          = arcs[arc_idx];

    let t    = QUAD_T[corner_idx];
    let side = QUAD_SIDE[corner_idx];

    let dir  = normalize(a.to_pos - a.from_pos);
    let perp = vec2<f32>(-dir.y, dir.x);

    let half_w = max(0.002, a.strength * 0.02);
    let center = mix(a.from_pos, a.to_pos, t);
    let world  = center + perp * (half_w * side);

    var out: VertexOut;
    out.clip_pos   = vec4<f32>(world, 0.0, 1.0);
    out.t          = t;
    out.perp_dist  = side * half_w;
    out.half_width = half_w;
    out.strength   = a.strength;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let edge_fade = smoothstep(in.half_width, in.half_width * 0.5, abs(in.perp_dist));
    let end_fade  = smoothstep(0.0, 0.05, in.t) * smoothstep(1.0, 0.95, in.t);
    let alpha     = in.strength * edge_fade * end_fade * 0.6;

    let color = vec3<f32>(0.6, 0.8, 1.0);
    return vec4<f32>(color * alpha, alpha);
}
