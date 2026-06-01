// Node billboard — crisp cognitive orb with Phase 6/7 visual systems.
//
// Visual layers (inside → out):
//   1. Hard anti-aliased disc (clear boundary)
//   2. White-hot spotlight core
//   3. Bright emissive rim at disc edge
//   4. Inner pulse ring (animated)
//   5. Narrow outer bloom
//   6. Entropy chromatic aberration
//   7. Velocity glow ring
//   8. Civilization membership band (outer ring)
//   9.  Void singularity  (bit 0 flags)
//  10. Crystal shimmer    (bit 1 flags)
//  11. Digital shadow     (bit 2 flags)
//  12. Oracle corona      (bit 3 flags)
//  13. Selected ring      (bit 4 flags)

struct CameraUniform {
    view_proj:  mat4x4<f32>,
    camera_pos: vec3<f32>,
    time:       f32,
}

struct NodeInstance {
    @location(0) position:    vec3<f32>,
    @location(1) radius:      f32,
    @location(2) color:       vec4<f32>,
    @location(3) entropy:     f32,
    @location(4) gravity_mass: f32,
    @location(5) velocity_mag: f32,
    @location(6) node_type:   u32,
    @location(7) civ_color:   vec4<f32>,
    @location(8) flags:       u32,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:           vec2<f32>,
    @location(1) color:        vec4<f32>,
    @location(2) entropy:      f32,
    @location(3) velocity_mag: f32,
    @location(4) civ_color:    vec4<f32>,
    @location(5) flags:        u32,
    @location(6) @interpolate(flat) node_type: u32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

fn quad_corner(vid: u32) -> vec2<f32> {
    switch vid {
        case 0u: { return vec2<f32>(-1.0, -1.0); }
        case 1u: { return vec2<f32>( 1.0, -1.0); }
        case 2u: { return vec2<f32>(-1.0,  1.0); }
        case 3u: { return vec2<f32>(-1.0,  1.0); }
        case 4u: { return vec2<f32>( 1.0, -1.0); }
        default: { return vec2<f32>( 1.0,  1.0); }
    }
}

@vertex
fn vs_main(inst: NodeInstance, @builtin(vertex_index) vid: u32) -> VertexOut {
    let corner = quad_corner(vid);
    let view_right = vec3<f32>(camera.view_proj[0][0], camera.view_proj[1][0], camera.view_proj[2][0]);
    let view_up    = vec3<f32>(camera.view_proj[0][1], camera.view_proj[1][1], camera.view_proj[2][1]);
    // 1.9× scale — room for oracle corona + civ ring
    let world_pos = inst.position
        + view_right * (corner.x * inst.radius * 1.9)
        + view_up    * (corner.y * inst.radius * 1.9);

    var out: VertexOut;
    out.clip_pos   = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv         = corner;
    out.color      = inst.color;
    out.entropy    = inst.entropy;
    out.velocity_mag = inst.velocity_mag;
    out.civ_color  = inst.civ_color;
    out.flags      = inst.flags;
    out.node_type  = inst.node_type;
    return out;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hash_f(n: f32) -> f32 {
    return fract(sin(n * 127.1 + 311.7) * 43758.5453);
}

fn hex_dist(uv: vec2<f32>) -> f32 {
    let p = abs(uv);
    return max(p.x * 0.866025 + p.y * 0.5, p.y);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    if dist > 1.0 { discard; }

    let t = camera.time;

    // Core disc occupies inner 1/1.9 ≈ 0.526 of the quad
    let core_edge: f32 = 0.526;

    // ── Animation ───────────────────────────────────────────────────────────
    let phase  = f32(in.node_type) * 0.739;
    let pulse  = sin(t * 1.6 + phase) * 0.5 + 0.5;
    let breath = sin(t * 0.55 + phase + 1.0) * 0.5 + 0.5;

    // ── Boosted base color (richer, more saturated) ──────────────────────────
    let heat_tint = vec3<f32>(1.0, 0.40, 0.10);
    let cool_tint = vec3<f32>(0.85, 0.92, 1.0);
    let tinted_rgb = mix(
        in.color.rgb * cool_tint,
        in.color.rgb * heat_tint,
        in.entropy * 0.5
    ) * 1.35; // boost saturation/brightness

    // ── 1. Hard disc ─────────────────────────────────────────────────────────
    // Very sharp anti-aliased boundary — gives nodes a clear defined shape
    let disc = smoothstep(core_edge + 0.008, core_edge - 0.008, dist);

    // ── 2. White-hot spotlight core ──────────────────────────────────────────
    let inner_g    = exp(-dist * dist * 22.0);   // tight bright center
    let mid_g      = exp(-dist * dist * 7.5);    // medium fill glow
    // Blend from white-hot center → node color toward edge
    let core_rgb   = mix(tinted_rgb, vec3<f32>(1.4, 1.4, 1.6), inner_g * 0.65)
                   * (0.85 + mid_g * 0.50);

    // ── 3. Emissive rim at disc edge ─────────────────────────────────────────
    let rim_d = abs(dist - core_edge);
    let rim   = exp(-rim_d * rim_d * 280.0);
    let rim_rgb = mix(tinted_rgb * 1.8, vec3<f32>(1.0, 1.0, 1.0), 0.35);
    let rim_a   = rim * 0.80;

    // ── 4. Inner pulse ring ──────────────────────────────────────────────────
    let ring_r   = 0.20 + pulse * 0.05;
    let ring_d   = abs(dist - ring_r);
    let ring_m   = exp(-ring_d * ring_d * 480.0);
    let ring_rgb = mix(vec3<f32>(0.20, 0.88, 1.0), vec3<f32>(1.0, 0.80, 0.15), clamp(in.velocity_mag / 5.0, 0.0, 1.0));
    let ring_a   = ring_m * (0.40 + pulse * 0.45);

    // ── 5. Narrow outer bloom (just outside disc) ────────────────────────────
    let bloom = exp(-dist * dist * 3.2) * (1.0 - disc) * 0.55;
    let bloom_rgb = tinted_rgb;
    let bloom_a   = bloom;

    // ── 6. Entropy chromatic fringe ──────────────────────────────────────────
    let chroma_r = exp(-(dist + in.entropy * 0.07) * (dist + in.entropy * 0.07) * 12.0);
    let chroma_b = exp(-(dist - in.entropy * 0.05) * (dist - in.entropy * 0.05) * 12.0);
    let chroma_add = vec3<f32>(chroma_r * 0.30, 0.0, chroma_b * 0.50) * in.entropy;

    // ── 7. Velocity glow ring ────────────────────────────────────────────────
    let vel    = clamp(in.velocity_mag / 5.0, 0.0, 1.0);
    let vring_r = core_edge * 0.75 - pulse * 0.03;
    let vring_d = abs(dist - vring_r);
    let vring_m = smoothstep(0.06, 0.0, vring_d);
    let vring_a = vring_m * vel * (0.50 + pulse * 0.50);

    // ── 8. Civilization membership ring ──────────────────────────────────────
    let has_civ    = f32(in.civ_color.a > 0.01);
    let civ_ring_r = 0.82;
    let civ_ring_d = abs(dist - civ_ring_r);
    let civ_ring_w = 0.048 + pulse * 0.016;
    let civ_ring_m = smoothstep(civ_ring_w, 0.0, civ_ring_d);
    let civ_angle  = atan2(in.uv.y, in.uv.x);
    let civ_rot    = sin(civ_angle * 6.0 - t * 0.9) * 0.20 + 0.80;
    let civ_a      = civ_ring_m * civ_rot * has_civ * 0.92;

    // ── Per-type special effects ─────────────────────────────────────────────

    var ghost_mult = 1.0;
    if in.node_type == 8u {
        let f1 = sin(t * 5.7 + dist * 9.0) * 0.5 + 0.5;
        let f2 = sin(t * 13.3 + dist * 17.0) * 0.3 + 0.7;
        ghost_mult = 0.15 + f1 * 0.40 * f2;
    }

    var fossil_mult = 1.0;
    if in.node_type == 9u {
        let angle   = atan2(in.uv.y, in.uv.x);
        let facet   = abs(sin(angle * 6.0)) * 0.35 + 0.65;
        let shimmer = sin(t * 0.4 + dist * 8.0) * 0.08 + 0.92;
        fossil_mult = facet * shimmer * 0.75;
    }

    var world_extra = 0.0;
    if in.node_type == 7u {
        let ripple  = sin((dist - t * 0.18) * 18.0) * 0.5 + 0.5;
        let ripple2 = sin((dist - t * 0.09) * 10.0) * 0.5 + 0.5;
        world_extra = (ripple * 0.6 + ripple2 * 0.4) * (1.0 - dist) * 0.18;
    }

    // ── Void singularity (flag bit 0) ─────────────────────────────────────────
    var void_mult  = 1.0;
    var void_rgb   = vec3<f32>(0.0);
    var void_a_add = 0.0;
    if (in.flags & 1u) != 0u {
        let inv_g    = 1.0 - exp(-dist * dist * 6.0);
        void_mult    = inv_g * 0.35;
        let vring_d2 = abs(dist - 0.52);
        let vring_m2 = smoothstep(0.12, 0.0, vring_d2);
        let vring_r2 = sin(atan2(in.uv.y, in.uv.x) * 4.0 - t * 1.8) * 0.3 + 0.7;
        void_rgb     = vec3<f32>(0.04, 0.0, 0.14) * vring_m2 * vring_r2;
        void_a_add   = vring_m2 * 0.55;
        let swirl    = sin(atan2(in.uv.y, in.uv.x) * 7.0 - t * 2.4 + dist * 5.0) * 0.5 + 0.5;
        void_rgb    += vec3<f32>(0.0, 0.0, 0.10) * swirl * (1.0 - dist) * 0.3;
    }

    // ── Crystal shimmer (flag bit 1) ───────────────────────────────────────────
    var crystal_rgb = vec3<f32>(0.0);
    var crystal_a   = 0.0;
    if (in.flags & 2u) != 0u {
        let hex_uv  = in.uv * 4.0;
        let hd      = hex_dist(fract(hex_uv) * 2.0 - 1.0);
        let edge    = smoothstep(0.85, 1.0, hd);
        let sparkle = sin(t * 1.2 + hd * 8.0 + dist * 12.0) * 0.5 + 0.5;
        crystal_rgb = vec3<f32>(0.85, 0.95, 1.0) * edge * sparkle;
        crystal_a   = edge * sparkle * 0.70;
    }

    // ── Digital shadow (flag bit 2) ────────────────────────────────────────────
    var shadow_rgb = vec3<f32>(0.0);
    var shadow_a   = 0.0;
    if (in.flags & 4u) != 0u {
        let s_ring_r = core_edge * 1.1 + pulse * 0.06;
        let s_ring_d = abs(dist - s_ring_r);
        let s_ring_m = smoothstep(0.09, 0.0, s_ring_d);
        let s_flick  = sin(t * 3.2 + dist * 11.0 + phase) * 0.4 + 0.6;
        shadow_rgb   = vec3<f32>(1.0, 0.28, 0.08) * s_ring_m * s_flick;
        shadow_a     = s_ring_m * s_flick * 0.65;
    }

    // ── Oracle corona (flag bit 3) ─────────────────────────────────────────────
    var oracle_rgb = vec3<f32>(0.0);
    var oracle_a   = 0.0;
    if (in.flags & 8u) != 0u {
        let oc_p    = sin(t * 2.8 + phase) * 0.5 + 0.5;
        let oc_r    = 0.78 + oc_p * 0.12;
        let oc_d    = abs(dist - oc_r);
        let oc_m    = smoothstep(0.12, 0.0, oc_d);
        let oc_ang  = atan2(in.uv.y, in.uv.x);
        let oc_segs = sin(oc_ang * 8.0 - t * 2.0) * 0.5 + 0.5;
        oracle_rgb  = vec3<f32>(1.0, 0.95, 0.70) * oc_m * (0.6 + oc_segs * 0.4);
        oracle_a    = oc_m * (0.55 + oc_segs * 0.35) * oc_p;
    }

    // ── Selected ring (flag bit 4) ─────────────────────────────────────────────
    var sel_rgb = vec3<f32>(0.0);
    var sel_a   = 0.0;
    if (in.flags & 16u) != 0u {
        // Bright white pulsing ring just outside the civ ring
        let sel_pulse = sin(t * 4.0 + phase) * 0.5 + 0.5;
        let sel_r     = 0.92 + sel_pulse * 0.04;
        let sel_d     = abs(dist - sel_r);
        let sel_m     = smoothstep(0.042, 0.0, sel_d);
        let sel_rot   = sin(atan2(in.uv.y, in.uv.x) * 8.0 - t * 3.5) * 0.25 + 0.75;
        sel_rgb       = vec3<f32>(1.0, 1.0, 1.0) * sel_m * sel_rot;
        sel_a         = sel_m * sel_rot * (0.75 + sel_pulse * 0.25);
    }

    // ── Entropy alpha suppression ──────────────────────────────────────────────
    let entropy_fade = 1.0 - in.entropy * 0.60;

    // ── Final composite ────────────────────────────────────────────────────────
    var final_rgb =
        disc   * (core_rgb + chroma_add) +
        rim    * rim_rgb +
        ring_m * ring_rgb * ring_a +
        bloom  * bloom_rgb +
        vring_m * ring_rgb * vring_a +
        in.color.rgb * world_extra +
        void_rgb  +
        crystal_rgb +
        shadow_rgb +
        oracle_rgb +
        in.civ_color.rgb * civ_a +
        sel_rgb;

    final_rgb = final_rgb * void_mult;

    var final_a = (
        disc * 0.95 +
        rim_a +
        ring_a * 0.75 +
        bloom_a * 0.60 +
        vring_a * 0.65 +
        world_extra * 0.55 +
        void_a_add +
        crystal_a +
        shadow_a +
        oracle_a +
        civ_a * 0.65 +
        sel_a
    ) * entropy_fade * ghost_mult * fossil_mult;

    return vec4<f32>(final_rgb, clamp(final_a, 0.0, 1.0));
}
