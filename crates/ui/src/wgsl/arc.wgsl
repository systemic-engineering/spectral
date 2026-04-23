// arc.wgsl — Coupling arc shader.
// Each arc is a line segment drawn as a thin quad (2 triangles, 6 vertices).
// Width is proportional to strength; alpha fades with distance from segment midpoint.

struct ArcData {
    from_pos:  vec2<f32>,  // world-space start
    to_pos:    vec2<f32>,  // world-space end
    strength:  f32,        // coupling strength → opacity and width
    _pad:      vec3<f32>,  // alignment
}

struct VertexOut {
    @builtin(position) clip_pos:    vec4<f32>,
    @location(0)       t:           f32,    // parametric position along arc [0,1]
    @location(1)       perp_dist:   f32,    // signed distance from arc centre-line
    @location(2)       half_width:  f32,    // half-width in NDC
    @location(3)       strength:    f32,
}

// Each arc expands to 6 vertices (one quad, two triangles).
// vertices 0,1 = start-left, start-right
// vertices 2,3 = end-left, end-right
// Triangle 1: 0,1,2  Triangle 2: 1,3,2
var<private> QUAD_T:    array<f32, 6>    = array<f32, 6>(0.0, 0.0, 1.0, 0.0, 1.0, 1.0);
var<private> QUAD_SIDE: array<f32, 6>    = array<f32, 6>(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0);

@group(0) @binding(0) var<storage, read> arcs: array<ArcData>;

@vertex
fn vs_main(@builtin(vertex_index) vert_idx: u32) -> VertexOut {
    let arc_idx    = vert_idx / 6u;
    let corner_idx = vert_idx % 6u;
    let a = arcs[arc_idx];

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
    // Alpha falls off toward the ends and edges
    let edge_fade = smoothstep(in.half_width, in.half_width * 0.5, abs(in.perp_dist));
    let end_fade  = smoothstep(0.0, 0.05, in.t) * smoothstep(1.0, 0.95, in.t);
    let alpha     = in.strength * edge_fade * end_fade * 0.6;

    // Arcs are a cool blue-white
    let color = vec3<f32>(0.6, 0.8, 1.0);
    return vec4<f32>(color * alpha, alpha);
}
