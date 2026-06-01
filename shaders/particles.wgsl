// Particle system — compute physics + render sparks, Phase 6/7 enhanced.
//
// Render effects:
//   • Multi-layer glow: tight core + soft wide halo
//   • Flare ring at mid-distance
//   • Color-driven particle style:
//       Blue/Cyan → knowledge spark (standard)
//       Red/Orange → shadow particle (phantom trailing)
//       Purple/Violet → void particle (dark inward)
//       Gold/Yellow → crystallization debris (sharp angular)
//   • Lifetime fade with smooth trail effect

// ─── Compute pass ─────────────────────────────────────────────────────────────

struct Particle {
    position: vec3<f32>,
    lifetime: f32,
    velocity: vec3<f32>,
    size: f32,
    color: vec4<f32>,
    dest: vec3<f32>,
    node_affinity: f32,
}

struct SimParams {
    delta_time:       f32,
    gravity_strength: f32,
    drag:             f32,
    _pad:             f32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

@compute @workgroup_size(64)
fn cs_update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if i >= arrayLength(&particles) { return; }

    var p = particles[i];
    if p.lifetime <= 0.0 { return; }

    let to_dest = p.dest - p.position;
    let dist = length(to_dest);
    if dist > 0.01 {
        let pull = normalize(to_dest) * p.node_affinity * params.gravity_strength;
        p.velocity = p.velocity + pull * params.delta_time;
    }

    p.velocity = p.velocity * (1.0 - params.drag * params.delta_time);
    p.position = p.position + p.velocity * params.delta_time;
    p.lifetime = p.lifetime - params.delta_time;

    // Smooth alpha fade with cubic easing
    let t = clamp(p.lifetime / 2.0, 0.0, 1.0);
    p.color.a = t * t * (3.0 - 2.0 * t);

    particles[i] = p;
}

// ─── Render pass ──────────────────────────────────────────────────────────────

struct CameraUniform {
    view_proj:  mat4x4<f32>,
    camera_pos: vec3<f32>,
    time:       f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct ParticleIn {
    @location(0) position: vec3<f32>,
    @location(1) lifetime: f32,
    @location(2) size: f32,
    @location(3) color: vec4<f32>,
}

struct ParticleOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) lifetime: f32,
}

fn particle_corner(vid: u32) -> vec2<f32> {
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
fn vs_particle(inst: ParticleIn, @builtin(vertex_index) vid: u32) -> ParticleOut {
    if inst.lifetime <= 0.0 {
        var dead: ParticleOut;
        dead.clip_pos = vec4<f32>(0.0, 0.0, -2.0, 1.0);
        dead.color    = vec4<f32>(0.0);
        dead.uv       = vec2<f32>(0.0);
        dead.lifetime = 0.0;
        return dead;
    }

    let corner = particle_corner(vid);
    let view_right = vec3<f32>(camera.view_proj[0][0], camera.view_proj[1][0], camera.view_proj[2][0]);
    let view_up    = vec3<f32>(camera.view_proj[0][1], camera.view_proj[1][1], camera.view_proj[2][1]);

    let world_pos = inst.position
        + view_right * (corner.x * inst.size)
        + view_up    * (corner.y * inst.size);

    var out: ParticleOut;
    out.clip_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.color    = inst.color;
    out.uv       = corner;
    out.lifetime = inst.lifetime;
    return out;
}

// ── Classify particle style from dominant color channel ──────────────────────
// 0 = standard spark (blue)
// 1 = shadow (red/orange dominant)
// 2 = void (purple/dark)
// 3 = crystal debris (gold/yellow)

fn particle_style(c: vec4<f32>) -> u32 {
    let max_r = c.r;
    let max_b = c.b;
    let max_g = c.g;
    // Shadow: red-dominant and warm
    if c.r > 0.5 && c.r > c.b * 1.5 && c.r > c.g * 1.2 { return 1u; }
    // Crystal: green+red dominant (gold = high r+g)
    if c.r > 0.5 && c.g > 0.45 && c.b < 0.35 { return 3u; }
    // Void: blue-purple without much cyan
    if c.b > 0.4 && c.r > 0.2 && c.g < 0.35 { return 2u; }
    // Standard
    return 0u;
}

@fragment
fn fs_particle(in: ParticleOut) -> @location(0) vec4<f32> {
    let dist = length(in.uv);
    if dist > 1.0 { discard; }

    let t  = camera.time;
    let lt = in.lifetime;
    let style = particle_style(in.color);

    // ── Base glow layers ──────────────────────────────────────────────────────
    let core = exp(-dist * dist * 5.5);
    let halo = exp(-dist * dist * 1.7) * 0.38;
    var brightness = core + halo;

    // ── Style-specific rendering ──────────────────────────────────────────────
    var rgb   = in.color.rgb;
    var alpha = in.color.a;

    if style == 1u {
        // Shadow particle: flickering red-orange ring + trailing fade
        let flicker  = sin(t * 7.3 + dist * 12.0) * 0.25 + 0.75;
        let s_ring   = smoothstep(0.65, 0.5, dist) * smoothstep(0.30, 0.5, dist);
        brightness   = core * 0.7 + halo * 0.4 + s_ring * 0.5;
        rgb          = mix(vec3<f32>(1.0, 0.18, 0.04), in.color.rgb, dist) * flicker;

    } else if style == 2u {
        // Void particle: dark inward spiral effect
        let angle   = atan2(in.uv.y, in.uv.x);
        let swirl   = sin(angle * 5.0 - t * 3.0 - dist * 6.0) * 0.4 + 0.6;
        let inv_g   = 1.0 - exp(-dist * dist * 3.0);  // brighter at edge
        brightness  = inv_g * swirl * 0.7;
        rgb         = mix(vec3<f32>(0.06, 0.0, 0.14), vec3<f32>(0.22, 0.08, 0.42), dist);

    } else if style == 3u {
        // Crystal debris: sharp angular sparkle facets
        let angle   = atan2(in.uv.y, in.uv.x);
        let facets  = abs(sin(angle * 6.0 + t * 0.5)) * 0.45 + 0.55;
        let sharp_c = exp(-dist * dist * 9.0);
        brightness  = sharp_c * facets + halo * 0.3;
        rgb         = mix(vec3<f32>(1.0, 0.92, 0.55), vec3<f32>(0.95, 0.85, 1.0), dist);

    } else {
        // Standard: flare ring at mid-distance for larger particles
        let ring = smoothstep(0.65, 0.5, dist) * smoothstep(0.32, 0.5, dist) * 0.28;
        brightness = core + halo + ring;
    }

    alpha = alpha * brightness;
    return vec4<f32>(rgb * brightness, clamp(alpha, 0.0, 1.0));
}
