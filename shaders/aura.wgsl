// Digital Aura — full-screen background, Phase 6/7 enhanced.
//
// Layers:
//   1. Deep nebula (domain-warped FBM, 6 octaves)
//   2. Cognitive Season color palette (Spring/Summer/Autumn/Winter)
//   3. Turbulence / chaos overlay
//   4. Dual vignette (softer than before)
//   5. Star field with cross-flares
//   6. Subtle digital scan-lines
//   7. Corner edge glow
//   8. Season accent filaments
//   9. Oracle shooting stars (oracle_pulse > 0)
//  10. Void gravity wells (void_density > 0)

struct AuraUniform {
    color_primary:   vec4<f32>,
    color_secondary: vec4<f32>,
    intensity:       f32,
    turbulence:      f32,
    pulse_rate:      f32,
    time:            f32,
    season:          f32,   // 0=spring 1=summer 2=autumn 3=winter
    oracle_pulse:    f32,
    void_density:    f32,
    _pad:            f32,
}

@group(0) @binding(0)
var<uniform> aura: AuraUniform;

struct AuraOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

fn fullscreen_uv(vid: u32) -> vec2<f32> {
    switch vid {
        case 0u: { return vec2<f32>(0.0, 0.0); }
        case 1u: { return vec2<f32>(2.0, 0.0); }
        default: { return vec2<f32>(0.0, 2.0); }
    }
}

@vertex
fn vs_aura(@builtin(vertex_index) vid: u32) -> AuraOut {
    let uv   = fullscreen_uv(vid);
    let clip = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    var out: AuraOut;
    out.clip_pos = vec4<f32>(clip, 0.999, 1.0);
    out.uv = uv * 0.5;
    return out;
}

// ─── Noise / FBM ─────────────────────────────────────────────────────────────

fn hash2(p: vec2<f32>) -> f32 {
    var q = p;
    q = fract(q * vec2<f32>(127.1, 311.7));
    q = q + dot(q, q + vec2<f32>(19.19, 7.31));
    return fract((q.x + q.y) * 43758.5453);
}

fn hash3(p: vec2<f32>) -> f32 {
    var q = p;
    q = fract(q * vec2<f32>(211.3, 113.7));
    q = q + dot(q, q + vec2<f32>(71.19, 59.31));
    return fract((q.x + q.y) * 91731.9453);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var value     = 0.0;
    var amplitude = 0.5;
    var freq      = 1.0;
    var pp        = p;
    for (var i = 0; i < octaves; i++) {
        value     += amplitude * value_noise(pp * freq);
        freq      *= 2.07;
        amplitude *= 0.47;
    }
    return value;
}

fn warp_fbm(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(
        fbm(p + vec2<f32>(0.0, 0.0), 4),
        fbm(p + vec2<f32>(5.2, 1.3), 4)
    );
    return fbm(p + q * 1.4, 5);
}

// ─── Star field ───────────────────────────────────────────────────────────────

fn star_field(uv: vec2<f32>, t: f32) -> f32 {
    let density: f32 = 68.0;
    let grid  = floor(uv * density);
    let local = fract(uv * density);

    let coin = hash2(grid + vec2<f32>(3.7, 8.1));
    if coin < 0.68 { return 0.0; }

    let cx = hash2(grid + vec2<f32>(1.0, 0.0));
    let cy = hash2(grid + vec2<f32>(0.0, 1.0));
    let d  = length(local - vec2<f32>(cx, cy));

    let freq    = 0.9 + hash2(grid + vec2<f32>(5.0, 11.0)) * 3.5;
    let twinkle = sin(t * freq + coin * 6.283) * 0.42 + 0.58;

    let brightness = (coin - 0.68) * 3.125;
    let radius     = 0.008 + brightness * 0.032;

    // Cross flare on bright stars
    let cross_h = smoothstep(0.008, 0.0, abs(local.y - cy));
    let cross_v = smoothstep(0.008, 0.0, abs(local.x - cx));
    let cross_mask = (cross_h + cross_v) * brightness * 0.45;

    return (smoothstep(radius, 0.0, d) + cross_mask) * twinkle * brightness * 1.2;
}

// ─── Cognitive season palette ─────────────────────────────────────────────────
// Much brighter — these are the dominant nebula colors

fn season_primary(s: f32) -> vec3<f32> {
    let spring = vec3<f32>(0.04, 0.10, 0.28);  // rich deep blue
    let summer = vec3<f32>(0.20, 0.10, 0.02);  // warm dark amber
    let autumn = vec3<f32>(0.18, 0.05, 0.01);  // deep rust-red
    let winter = vec3<f32>(0.04, 0.07, 0.25);  // rich deep indigo

    if s < 1.0 {
        return mix(spring, summer, smoothstep(0.0, 1.0, s));
    } else if s < 2.0 {
        return mix(summer, autumn, smoothstep(1.0, 2.0, s));
    } else {
        return mix(autumn, winter, smoothstep(2.0, 3.0, s));
    }
}

fn season_secondary(s: f32) -> vec3<f32> {
    let spring = vec3<f32>(0.25, 0.95, 0.55);  // vivid emerald
    let summer = vec3<f32>(1.00, 0.82, 0.25);  // bright gold
    let autumn = vec3<f32>(1.00, 0.48, 0.12);  // vivid amber-orange
    let winter = vec3<f32>(0.52, 0.65, 1.00);  // bright icy indigo

    if s < 1.0 {
        return mix(spring, summer, smoothstep(0.0, 1.0, s));
    } else if s < 2.0 {
        return mix(summer, autumn, smoothstep(1.0, 2.0, s));
    } else {
        return mix(autumn, winter, smoothstep(2.0, 3.0, s));
    }
}

// ─── Oracle shooting star ─────────────────────────────────────────────────────

fn oracle_shooting_star(uv: vec2<f32>, t: f32, pulse: f32) -> f32 {
    if pulse < 0.01 { return 0.0; }
    var brightness = 0.0;
    for (var i = 0u; i < 3u; i++) {
        let seed  = f32(i) * 3.33;
        let speed = 0.55 + seed * 0.12;
        let angle = seed * 2.094;
        let dir   = vec2<f32>(cos(angle), sin(angle));
        let origin = vec2<f32>(
            hash2(vec2<f32>(seed, 1.0)) * 0.5 + 0.1,
            hash2(vec2<f32>(seed, 2.0)) * 0.5 + 0.1
        );
        let pos   = origin + dir * fract(t * speed + seed * 0.7);
        let offset = uv - pos;
        let along  = dot(offset, dir);
        let perp   = dot(offset, vec2<f32>(-dir.y, dir.x));
        let trail  = exp(-perp * perp * 4000.0) * exp(-max(along, 0.0) * 20.0);
        let head   = exp(-perp * perp * 4000.0 - along * along * 500.0);
        brightness += (trail * 0.7 + head * 3.0) * pulse
                    * smoothstep(0.5, 0.8, abs(fract(t * speed + seed * 0.7) - 0.5) * 2.0);
    }
    return brightness;
}

// ─── Void gravity well ────────────────────────────────────────────────────────

fn void_wells(uv: vec2<f32>, t: f32, density: f32) -> f32 {
    if density < 0.01 { return 0.0; }
    var darkening = 0.0;
    for (var i = 0u; i < 3u; i++) {
        let seed  = f32(i) * 1.61803;
        let pos   = vec2<f32>(
            hash2(vec2<f32>(seed, 0.5)) * 0.6 + 0.2,
            hash2(vec2<f32>(seed, 1.5)) * 0.6 + 0.2
        );
        let drift = pos + vec2<f32>(sin(t * 0.025 + seed), cos(t * 0.018 + seed)) * 0.05;
        let d     = length(uv - drift);
        darkening += exp(-d * d * 18.0) * density;
    }
    return clamp(darkening, 0.0, 0.85);
}

// ─── Fragment ─────────────────────────────────────────────────────────────────

@fragment
fn fs_aura(in: AuraOut) -> @location(0) vec4<f32> {
    let t  = aura.time;
    let uv = in.uv;

    // ── Season palette (blend weather + season) ──────────────────────────────
    let s_primary   = mix(season_primary(aura.season),   aura.color_primary.rgb,   0.30);
    let s_secondary = mix(season_secondary(aura.season), aura.color_secondary.rgb, 0.30);

    // ── Nebula (domain-warped FBM) ──────────────────────────────────────────
    let drift   = vec2<f32>(t * 0.026, t * 0.016);
    let n_base  = warp_fbm(uv * 2.6 + drift);

    let turb_scale = 7.0 + aura.turbulence * 14.0;
    let turb_drift = vec2<f32>(-t * 0.060, t * 0.044);
    let n_turb     = fbm(uv * turb_scale + turb_drift, 3);
    let n_combined = mix(n_base, n_turb, aura.turbulence * 0.50);

    let pulse      = sin(t * aura.pulse_rate * 6.283) * 0.5 + 0.5;
    let pulse_mask = n_combined * pulse * aura.intensity;

    // ── Vignette (softer — less darkening) ──────────────────────────────────
    let centre         = uv - 0.5;
    let vignette       = clamp(1.0 - dot(centre, centre) * 1.45, 0.0, 1.0);
    let inner_vignette = clamp(1.0 - dot(centre, centre) * 0.55, 0.65, 1.0);

    // ── Nebula color — richer mix ────────────────────────────────────────────
    let mix_factor  = clamp(n_combined * 1.8 + pulse_mask * 0.50, 0.0, 1.0);
    let nebula_rgb  = mix(s_primary, s_secondary, mix_factor * 0.62)
                    * (1.0 + n_combined * 0.6); // brighten by noise structure

    // ── Edge glow (stronger corner bleeding) ────────────────────────────────
    let glow_dist = length(centre) * 1.42;
    let edge_glow = s_secondary
                  * max(0.0, 1.0 - glow_dist)
                  * aura.intensity * 0.48 * pulse;

    // ── Season filaments (veins from center) ─────────────────────────────────
    let vein_angle = atan2(uv.y - 0.5, uv.x - 0.5);
    let vein_dist  = length(centre);
    let vein_mask  = sin(vein_angle * 12.0 + t * 0.3) * 0.5 + 0.5;
    let vein_fade  = smoothstep(0.5, 0.0, vein_dist);
    let vein_rgb   = s_secondary * vein_mask * vein_fade * 0.12 * aura.intensity;

    // ── Scan-lines ────────────────────────────────────────────────────────────
    let scanline = sin(uv.y * 460.0) * 0.008 + 1.0;

    // ── Stars ─────────────────────────────────────────────────────────────────
    let star_gap   = clamp(1.0 - n_combined * 1.0, 0.0, 1.0);
    let stars      = star_field(uv, t) * star_gap;
    let star_color = vec3<f32>(stars * 0.90, stars * 0.95, stars * 1.05);

    // ── Oracle shooting stars ─────────────────────────────────────────────────
    let oracle       = oracle_shooting_star(uv, t, aura.oracle_pulse);
    let oracle_color = vec3<f32>(1.0, 0.92, 0.65) * oracle;

    // ── Void wells ────────────────────────────────────────────────────────────
    let void_dark = void_wells(uv, t, aura.void_density);

    // ── Final composite ───────────────────────────────────────────────────────
    var final_rgb = (nebula_rgb + edge_glow + vein_rgb) * vignette * inner_vignette * scanline
                  + star_color + oracle_color;

    final_rgb = final_rgb * (1.0 - void_dark * 0.70);

    // Ensure a minimum visible brightness so it's never completely black
    final_rgb = max(final_rgb, s_primary * 0.4);

    let alpha = 0.88 + n_combined * 0.10;

    return vec4<f32>(final_rgb, clamp(alpha, 0.0, 1.0));
}
