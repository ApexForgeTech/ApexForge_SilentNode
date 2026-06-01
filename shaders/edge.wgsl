// Edge rendering — Phase 8 enhanced.
//
// Visual features:
//   • Activity-driven color: dormant deep-blue → active bright cyan-white
//   • Triple flowing energy pulses (primary + echo + faint tertiary)
//   • Weight-driven thickness and alpha
//   • Ghost edges: scrolling animated dashes with soft shimmer
//   • Faint ambient glow that breathes slowly
//   • Resonance edges: rainbow shimmer on high-similarity pairs
//   • Phase 8: highlight system — adjacent-to-selected bright | others dimmed

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct EdgeVertex {
    @location(0) position:  vec3<f32>,
    @location(1) normal:    vec3<f32>,
    @location(2) weight:    f32,
    @location(3) activity:  f32,
    @location(4) is_ghost:  u32,
    @location(5) uv_along:  f32,
    @location(6) highlight: u32,   // 0=normal 1=adjacent-to-selected 2=dimmed
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) weight:    f32,
    @location(1) activity:  f32,
    @location(2) is_ghost:  f32,
    @location(3) uv_along:  f32,
    @location(4) @interpolate(flat) highlight: u32,
}

@vertex
fn vs_main(v: EdgeVertex) -> VertexOut {
    let half_width = 0.018 + v.weight * 0.110;
    let world_pos  = v.position + v.normal * half_width;

    var out: VertexOut;
    out.clip_pos  = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.weight    = v.weight;
    out.activity  = v.activity;
    out.is_ghost  = f32(v.is_ghost);
    out.uv_along  = v.uv_along;
    out.highlight = v.highlight;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let t = camera.time;

    // ── Ghost edges: scrolling dashed shimmer ─────────────────────────────────
    if in.is_ghost > 0.5 {
        let scroll  = fract(in.uv_along * 8.0 - t * 0.50);
        if scroll > 0.42 { discard; }
        let shimmer = sin(t * 2.9 + in.uv_along * 24.0) * 0.12 + 0.22;
        let ghost_r = sin(t * 0.6 + in.uv_along * 3.0) * 0.15 + 0.50;
        let ghost_g = sin(t * 0.4 + in.uv_along * 2.0 + 1.0) * 0.10 + 0.32;
        let ghost_b = sin(t * 0.5 + in.uv_along * 4.0 + 2.0) * 0.12 + 0.72;
        let dim = select(1.0, 0.18, in.highlight == 2u);
        return vec4<f32>(ghost_r, ghost_g, ghost_b, shimmer * dim);
    }

    // ── Dimmed edges: not adjacent to selected ────────────────────────────────
    if in.highlight == 2u {
        let dormant = vec3<f32>(0.18, 0.30, 0.85);
        let base_a  = mix(0.028, 0.092, in.weight);
        return vec4<f32>(dormant * 0.35, clamp(base_a, 0.0, 1.0));
    }

    // ── Base edge color: dormant cool-blue → active bright cyan ──────────────
    let dormant  = vec3<f32>(0.18, 0.30, 0.85);
    let act_col  = vec3<f32>(0.40, 0.92, 1.0);
    var base_rgb = mix(dormant, act_col, clamp(in.activity, 0.0, 1.0));

    // Adjacent-to-selected: extra electric boost
    var sel_boost = 1.0;
    if in.highlight == 1u {
        sel_boost = 1.80;
        let sel_phase = in.uv_along * 6.0 - t * 3.5;
        let sel_flare = sin(sel_phase) * 0.5 + 0.5;
        base_rgb = mix(base_rgb, vec3<f32>(0.85, 1.0, 1.0), sel_flare * 0.45);
    }

    // ── Triple flowing energy pulses ──────────────────────────────────────────
    let flow_speed = 0.32 + in.activity * 2.2;
    let cursor     = fract(t * flow_speed);

    let d1  = abs(fract(in.uv_along - cursor + 0.5) - 0.5) * 2.0;
    let pw  = 0.07 + in.weight * 0.07;
    let p1  = exp(-d1 * d1 / (pw * pw)) * in.activity;

    let d2  = abs(fract(in.uv_along - cursor + 0.70) - 0.5) * 2.0;
    let p2  = exp(-d2 * d2 / (pw * pw)) * in.activity * 0.42;

    let d3  = abs(fract(in.uv_along - cursor + 0.85) - 0.5) * 2.0;
    let p3  = exp(-d3 * d3 / (pw * pw)) * in.activity * 0.18;

    let pulse_strength = p1 + p2 + p3;
    let pulse_rgb = mix(base_rgb, vec3<f32>(0.92, 0.97, 1.0), pulse_strength * 0.88);

    // ── Resonance shimmer ─────────────────────────────────────────────────────
    var resonance_rgb = vec3<f32>(0.0);
    if in.activity > 0.68 {
        let r_strength = (in.activity - 0.68) / 0.32;
        let r_phase    = in.uv_along * 8.0 - t * 1.8;
        let r_r = sin(r_phase + 0.0) * 0.5 + 0.5;
        let r_g = sin(r_phase + 2.094) * 0.5 + 0.5;
        let r_b = sin(r_phase + 4.189) * 0.5 + 0.5;
        resonance_rgb = vec3<f32>(r_r, r_g, r_b) * r_strength * 0.28;
    }

    // ── Ambient slow glow breathe ─────────────────────────────────────────────
    let breathe    = sin(t * 0.65 + in.uv_along * 3.14) * 0.09 + 0.09;
    let ambient_rgb = mix(pulse_rgb + resonance_rgb, act_col, breathe * in.activity);

    // ── Final alpha ───────────────────────────────────────────────────────────
    let base_a  = mix(0.28, 0.92, in.weight);
    let pulse_a = pulse_strength * 0.50;
    var final_a = clamp(base_a + pulse_a, 0.0, 1.0);

    // Adjacent-to-selected: boost alpha too
    if in.highlight == 1u {
        final_a = clamp(final_a * 1.35, 0.0, 1.0);
    }

    return vec4<f32>(ambient_rgb * sel_boost, final_a);
}
