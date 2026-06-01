"""
Phase 10.1 — Living Signature Generator
Computes and renders the user's unique, continuously evolving visual symbol.

Vision.md:
  "The Living Signature is a continuously evolving visual symbol — unique to
   each user — that represents the complete accumulated pattern of their
   cognitive existence within SilentNode."
  "No two signatures are alike. No signature is ever finished."

No external dependencies — pure Python + stdlib.
SVG output is fully compatible with browsers, TUI export, and HTML dashboard.
"""

from __future__ import annotations

import json
import math
import hashlib
from datetime import datetime, timezone
from typing import Any


# ── Helpers ───────────────────────────────────────────────────────────────────

def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _rgba_css(c: list[float]) -> str:
    r = int(c[0] * 255)
    g = int(c[1] * 255)
    b = int(c[2] * 255)
    a = c[3] if len(c) > 3 else 1.0
    return f"rgba({r},{g},{b},{a:.2f})"


def _hex(c: list[float]) -> str:
    r = int(c[0] * 255)
    g = int(c[1] * 255)
    b = int(c[2] * 255)
    return f"#{r:02x}{g:02x}{b:02x}"


# ── Signature parameter extraction ────────────────────────────────────────────

def _extract_params(ws: dict) -> dict:
    """
    Full derivation of SignatureParams from workspace snapshot.
    More sophisticated than the Rust side — uses Python to do heavier analysis.
    """
    nodes       = ws.get("nodes", [])
    civs        = ws.get("civilizations", [])
    lore        = ws.get("lore_entries", [])
    rituals     = ws.get("rituals", [])
    crystals    = ws.get("crystals", [])
    focus_events = ws.get("focus_events", [])
    season      = ws.get("season", "Summer")

    n_total = max(len(nodes), 1)

    # ── Colors from civilizations ─────────────────────────────────────────────
    civ_colors = [c.get("color", [0.2, 0.5, 0.9, 1.0]) for c in civs[:5]]
    if civ_colors:
        # Primary: average of top-3 civilization colors weighted by member count
        weights = [c.get("member_count", 1) for c in civs[:3]]
        total_w = max(sum(weights), 1)
        primary = [0.0, 0.0, 0.0, 1.0]
        for i, cc in enumerate(civ_colors[:3]):
            w = weights[i] / total_w
            primary[0] += cc[0] * w
            primary[1] += cc[1] * w
            primary[2] += cc[2] * w
    else:
        primary = [0.25, 0.55, 0.92, 1.0]

    if len(civ_colors) >= 2:
        secondary = [float(v) for v in civ_colors[1]]
    else:
        # Complementary rotation
        secondary = [
            1.0 - primary[0] * 0.7,
            1.0 - primary[1] * 0.7,
            primary[2] * 0.8 + 0.2,
            1.0,
        ]

    if crystals:
        accent = [0.95, 0.82, 0.28, 1.0]  # Gold — crystallized knowledge
    else:
        ghost_r = sum(1 for n in nodes if n.get("is_ghost")) / n_total
        accent = [0.65, 0.40, 0.90, 1.0] if ghost_r > 0.2 else [0.28, 0.95, 0.68, 1.0]

    # ── Geometry from lore arc types ──────────────────────────────────────────
    arc_counts: dict[str, int] = {}
    for entry in lore:
        t = entry.get("arc_type", "origin")
        arc_counts[t] = arc_counts.get(t, 0) + 1

    origin_legacy      = arc_counts.get("origin", 0) + arc_counts.get("legacy", 0)
    conflict_res       = arc_counts.get("conflict", 0) + arc_counts.get("resolution", 0)
    transform_rev      = arc_counts.get("transformation", 0) + arc_counts.get("revelation", 0)
    tectonic           = arc_counts.get("tectonic", 0)
    total_arcs         = origin_legacy + conflict_res + transform_rev + tectonic

    if total_arcs == 0:
        geometry = "line"
    else:
        best = max(
            [("circle", origin_legacy), ("wave", conflict_res),
             ("spiral", transform_rev), ("fractal", tectonic)],
            key=lambda x: x[1]
        )[0]
        geometry = best

    # ── Symmetry from ritual structure ────────────────────────────────────────
    if rituals:
        seq_lens = [len(r.get("sequence", [])) for r in rituals if r.get("sequence")]
        if seq_lens:
            from collections import Counter
            dominant_len = Counter(seq_lens).most_common(1)[0][0]
            symmetry = {2: "bilateral", 3: "radial-3", 4: "radial-4"}.get(
                dominant_len, "rotational" if dominant_len >= 5 else "none"
            )
        else:
            symmetry = "none"
    else:
        symmetry = "none"

    fold_count = {"none": 1, "bilateral": 2, "radial-3": 3, "radial-4": 4, "rotational": 6}
    folds = fold_count.get(symmetry, 1)

    # ── Motion from season ────────────────────────────────────────────────────
    motion = {
        "Spring": "flow",
        "Summer": "pulse",
        "Autumn": "breathe",
        "Winter": "crystallize",
    }.get(season, "drift")

    anim_ms = {"flow": 3000, "pulse": 1500, "breathe": 6000,
               "crystallize": 12000, "drift": 8000}.get(motion, 4000)

    # ── Complexity, vitality, depth ───────────────────────────────────────────
    complexity = min(len(civs) / 10.0, 1.0)
    active     = sum(1 for n in nodes if not n.get("is_ghost") and not n.get("is_void") and not n.get("is_fossil"))
    vitality   = active / n_total
    fossils    = sum(1 for n in nodes if n.get("is_fossil"))
    depth      = min((fossils + len(lore)) / 20.0, 1.0)

    # ── Deterministic seed for session-invariant variation ───────────────────
    # Hash of all node IDs for a stable but unique seed
    node_ids = "".join(sorted(n.get("id", "") for n in nodes))
    seed_hash = int(hashlib.md5(node_ids.encode()).hexdigest()[:8], 16) if node_ids else 42

    return {
        "primary_color":   primary,
        "secondary_color": secondary,
        "accent_color":    accent,
        "geometry":        geometry,
        "symmetry":        symmetry,
        "motion":          motion,
        "fold_count":      folds,
        "complexity":      round(complexity, 3),
        "vitality":        round(vitality, 3),
        "depth":           round(depth, 3),
        "animation_ms":    anim_ms,
        "seed":            seed_hash,
        "season":          season,
        "civ_count":       len(civs),
        "lore_count":      len(lore),
        "crystal_count":   len(crystals),
        "ritual_count":    len(rituals),
    }


# ── SVG renderer ──────────────────────────────────────────────────────────────

def _svg_spiral(cx: float, cy: float, r: float, folds: int,
                color1: str, color2: str, accent: str, anim_ms: int, complexity: float) -> str:
    """Generate a spiral Living Signature SVG path."""
    paths: list[str] = []
    stroke_w = 1.5 + complexity * 2.0
    turns = 2.0 + complexity * 2.0

    for fold in range(folds):
        angle_offset = fold * (2 * math.pi / folds)
        points = []
        for t in range(200):
            frac = t / 199.0
            radius = r * 0.1 + r * 0.9 * frac
            angle  = angle_offset + turns * 2 * math.pi * frac
            x = cx + radius * math.cos(angle)
            y = cy + radius * math.sin(angle)
            points.append(f"{x:.2f},{y:.2f}")
        d = "M " + " L ".join(points)
        col = color1 if fold % 2 == 0 else color2
        paths.append(f'<path d="{d}" stroke="{col}" stroke-width="{stroke_w:.1f}" fill="none" opacity="0.8"/>')

    # Accent dot at center
    paths.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r*0.06:.1f}" fill="{accent}" opacity="0.9"/>')
    return "\n".join(paths)


def _svg_wave(cx: float, cy: float, r: float, folds: int,
              color1: str, color2: str, accent: str, anim_ms: int, complexity: float) -> str:
    """Generate a wave Living Signature SVG path."""
    paths: list[str] = []
    amplitude = r * 0.35 * (0.5 + complexity * 0.5)
    stroke_w = 1.5 + complexity * 2.0
    freq = 3 + int(complexity * 4)

    for fold in range(folds):
        angle_offset = fold * (math.pi / folds)
        points = []
        for i in range(200):
            t = i / 199.0
            base_angle = angle_offset + t * 2 * math.pi
            wave = amplitude * math.sin(freq * t * 2 * math.pi + fold)
            radius = r * 0.6 + wave
            x = cx + radius * math.cos(base_angle)
            y = cy + radius * math.sin(base_angle)
            points.append(f"{x:.2f},{y:.2f}")
        d = "M " + " L ".join(points) + " Z"
        col = color1 if fold % 2 == 0 else color2
        paths.append(f'<path d="{d}" stroke="{col}" stroke-width="{stroke_w:.1f}" fill="none" opacity="0.75"/>')

    # Center ring
    paths.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r*0.08:.1f}" fill="{accent}" opacity="0.85"/>')
    return "\n".join(paths)


def _svg_circle(cx: float, cy: float, r: float, folds: int,
                color1: str, color2: str, accent: str, anim_ms: int, complexity: float) -> str:
    """Generate a mandala/circle Living Signature SVG path."""
    paths: list[str] = []
    n_rings = 2 + int(complexity * 4)
    stroke_w = 1.2 + complexity * 1.5

    for ring in range(n_rings):
        frac = (ring + 1) / n_rings
        radius = r * frac * 0.92
        col = color1 if ring % 2 == 0 else color2
        alpha = 0.4 + frac * 0.5
        paths.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{radius:.1f}" '
                     f'stroke="{col}" stroke-width="{stroke_w:.1f}" fill="none" opacity="{alpha:.2f}"/>')

    # Radial spokes
    for fold in range(folds):
        angle = fold * 2 * math.pi / folds
        x1 = cx + r * 0.15 * math.cos(angle)
        y1 = cy + r * 0.15 * math.sin(angle)
        x2 = cx + r * 0.90 * math.cos(angle)
        y2 = cy + r * 0.90 * math.sin(angle)
        paths.append(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
                     f'stroke="{accent}" stroke-width="1.0" opacity="0.6"/>')

    # Center dot
    paths.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r*0.05:.1f}" fill="{accent}" opacity="1.0"/>')
    return "\n".join(paths)


def _svg_fractal(cx: float, cy: float, r: float, folds: int,
                 color1: str, color2: str, accent: str, anim_ms: int, complexity: float) -> str:
    """Generate a fractal tree Living Signature SVG path."""
    paths: list[str] = []
    depth = 3 + int(complexity * 3)
    stroke_w = 2.0 + complexity * 1.5

    def branch(x: float, y: float, angle: float, length: float, d: int, col: str) -> None:
        if d == 0 or length < 2:
            return
        x2 = x + length * math.cos(angle)
        y2 = y + length * math.sin(angle)
        sw = stroke_w * (d / depth)
        paths.append(f'<line x1="{x:.1f}" y1="{y:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
                     f'stroke="{col}" stroke-width="{sw:.1f}" opacity="{0.4 + d/depth*0.5:.2f}"/>')
        spread = math.pi / 4 + complexity * math.pi / 8
        branch(x2, y2, angle - spread, length * 0.65, d - 1, color2)
        branch(x2, y2, angle + spread, length * 0.65, d - 1, color1)

    for fold in range(folds):
        start_angle = fold * 2 * math.pi / folds - math.pi / 2
        sx = cx + r * 0.3 * math.cos(start_angle + math.pi)
        sy = cy + r * 0.3 * math.sin(start_angle + math.pi)
        branch(sx, sy, start_angle, r * 0.55, depth, color1 if fold % 2 == 0 else color2)

    paths.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r*0.05:.1f}" fill="{accent}" opacity="0.9"/>')
    return "\n".join(paths)


def _svg_line(cx: float, cy: float, r: float, folds: int,
              color1: str, color2: str, accent: str, anim_ms: int, complexity: float) -> str:
    """Generate a flowing line Living Signature SVG path."""
    paths: list[str] = []
    n_lines = 3 + int(complexity * 5)
    stroke_w = 1.5 + complexity * 1.5

    for i in range(n_lines):
        frac = i / max(n_lines - 1, 1)
        y0 = cy - r + 2 * r * frac
        # Sinusoidal flow
        points = []
        for j in range(100):
            t = j / 99.0
            x = cx - r * 0.9 + 2 * r * 0.9 * t
            wave = r * 0.12 * math.sin(math.pi * 3 * t + i * math.pi / n_lines)
            y = y0 + wave
            points.append(f"{x:.2f},{y:.2f}")
        d = "M " + " L ".join(points)
        col = color1 if i % 2 == 0 else color2
        alpha = 0.4 + 0.5 * abs(frac - 0.5) * 2
        paths.append(f'<path d="{d}" stroke="{col}" stroke-width="{stroke_w:.1f}" '
                     f'fill="none" opacity="{alpha:.2f}"/>')

    paths.append(f'<circle cx="{cx:.1f}" cy="{cy:.1f}" r="{r*0.05:.1f}" fill="{accent}" opacity="0.9"/>')
    return "\n".join(paths)


_GEOMETRY_RENDERERS = {
    "spiral":  _svg_spiral,
    "wave":    _svg_wave,
    "circle":  _svg_circle,
    "fractal": _svg_fractal,
    "line":    _svg_line,
}


def _build_svg(params: dict, size: int = 200) -> str:
    """Build the complete SVG string for a Living Signature."""
    p1  = _hex(params["primary_color"])
    p2  = _hex(params["secondary_color"])
    acc = _hex(params["accent_color"])
    geo = params["geometry"]
    cx = cy = size / 2.0
    r = size * 0.43

    renderer = _GEOMETRY_RENDERERS.get(geo, _svg_line)
    inner = renderer(cx, cy, r, params["fold_count"], p1, p2, acc,
                     params["animation_ms"], params["complexity"])

    # Animation style based on motion kind
    motion = params["motion"]
    anim_ms = params["animation_ms"]
    anim_style = ""
    if motion == "pulse":
        anim_style = (
            f'@keyframes sig_pulse {{0%{{opacity:0.7}} 50%{{opacity:1.0}} 100%{{opacity:0.7}}}}'
            f'.sig_inner{{animation:sig_pulse {anim_ms}ms ease-in-out infinite;}}'
        )
    elif motion == "breathe":
        anim_style = (
            f'@keyframes sig_breathe {{0%{{transform:scale(0.95)}} 50%{{transform:scale(1.05)}} 100%{{transform:scale(0.95)}}}}'
            f'.sig_inner{{animation:sig_breathe {anim_ms}ms ease-in-out infinite;transform-origin:{cx:.0f}px {cy:.0f}px;}}'
        )
    elif motion == "flow":
        anim_style = (
            f'@keyframes sig_flow {{0%{{transform:rotate(0deg)}} 100%{{transform:rotate(360deg)}}}}'
            f'.sig_inner{{animation:sig_flow {anim_ms * 20}ms linear infinite;transform-origin:{cx:.0f}px {cy:.0f}px;}}'
        )
    elif motion == "crystallize":
        anim_style = (
            f'@keyframes sig_crystal {{0%{{opacity:0.8}} 50%{{opacity:1.0;filter:brightness(1.2)}} 100%{{opacity:0.8}}}}'
            f'.sig_inner{{animation:sig_crystal {anim_ms}ms ease-in-out infinite;}}'
        )

    bg_color = f"#0a0f1a"
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">
  <style>
    {anim_style}
  </style>
  <rect width="{size}" height="{size}" fill="{bg_color}" rx="8"/>
  <g class="sig_inner">
    {inner}
  </g>
</svg>'''
    return svg


# ── ASCII renderer ────────────────────────────────────────────────────────────

def _build_ascii(params: dict, width: int = 40, height: int = 20) -> str:
    """
    Approximate the Living Signature as ASCII art for TUI display.
    Uses block characters for a simple but recognizable representation.
    """
    geo      = params["geometry"]
    folds    = params["fold_count"]
    complex_ = params["complexity"]
    vitality = params["vitality"]

    cx = width / 2.0
    cy = height / 2.0
    r  = min(width, height) * 0.4

    grid = [[" "] * width for _ in range(height)]

    def plot(x: float, y: float, ch: str = "·") -> None:
        xi, yi = int(round(x)), int(round(y))
        if 0 <= xi < width and 0 <= yi < height:
            grid[yi][xi] = ch

    # Draw based on geometry
    if geo == "circle":
        for ring in range(1 + int(complex_ * 3)):
            rr = r * (ring + 1) / (1 + int(complex_ * 3) + 1)
            for fold in range(folds):
                for i in range(60):
                    a = fold * 2 * math.pi / folds + i * 2 * math.pi / 60
                    plot(cx + rr * math.cos(a) * 2.1, cy + rr * math.sin(a), "○" if ring == 0 else "·")
        for fold in range(folds):
            a = fold * 2 * math.pi / folds
            plot(cx + r * 0.9 * math.cos(a) * 2.1, cy + r * 0.9 * math.sin(a), "◆")

    elif geo == "spiral":
        turns = 2 + complex_ * 2
        for fold in range(folds):
            ao = fold * 2 * math.pi / folds
            for i in range(120):
                frac = i / 119.0
                rr = r * 0.1 + r * 0.85 * frac
                a  = ao + turns * 2 * math.pi * frac
                ch = "·" if frac < 0.7 else "○" if frac < 0.9 else "◆"
                plot(cx + rr * math.cos(a) * 2.1, cy + rr * math.sin(a), ch)

    elif geo == "wave":
        for fold in range(folds):
            ao = fold * math.pi / folds
            for i in range(100):
                t = i / 99.0
                ba = ao + t * 2 * math.pi
                wave = r * 0.3 * math.sin(3 * t * 2 * math.pi + fold)
                rr = r * 0.55 + wave
                plot(cx + rr * math.cos(ba) * 2.1, cy + rr * math.sin(ba), "~" if abs(wave) < r*0.1 else "·")

    elif geo == "fractal":
        def abr(x: float, y: float, a: float, ln: float, d: int) -> None:
            if d == 0 or ln < 1: return
            x2 = x + ln * math.cos(a) * 2.1
            y2 = y + ln * math.sin(a)
            steps = max(int(ln * 1.5), 2)
            for s in range(steps):
                t = s / max(steps - 1, 1)
                plot(x + (x2-x)*t, y + (y2-y)*t, "│" if abs(math.cos(a)) < 0.3 else "─" if abs(math.sin(a)) < 0.3 else "·")
            sp = math.pi/4 + complex_*math.pi/8
            abr(x2, y2, a - sp, ln * 0.6, d - 1)
            abr(x2, y2, a + sp, ln * 0.6, d - 1)
        for fold in range(folds):
            sa = fold * 2 * math.pi / folds - math.pi / 2
            abr(cx, cy + r * 0.1, sa, r * 0.5, 2 + int(complex_ * 2))

    else:  # line
        for i in range(3 + int(complex_ * 3)):
            frac = i / max(3 + int(complex_ * 3) - 1, 1)
            y0 = cy - r * 0.7 + r * 1.4 * frac
            for j in range(width - 4):
                t = j / (width - 5)
                x = 2 + j
                wave = r * 0.15 * math.sin(math.pi * 3 * t + i * math.pi / 4)
                y = y0 + wave
                plot(x, y, "─" if abs(wave) < 0.3 else "~")

    # Center marker
    plot(cx, cy, "✦" if vitality > 0.6 else "◇")

    return "\n".join("".join(row) for row in grid)


# ── Public API ────────────────────────────────────────────────────────────────

class LivingSignatureGenerator:
    """
    Generates and renders the Living Signature from workspace state.

    Usage:
        gen = LivingSignatureGenerator()
        params_json = gen.compute_signature_params(workspace_json)
        svg_str = gen.render_svg(params_json)
        ascii_art = gen.render_ascii(params_json)
    """

    def compute_signature_params(self, workspace_json: str) -> str:
        """
        Input:  JSON workspace snapshot
        Output: JSON SignatureParams
        """
        try:
            ws = json.loads(workspace_json)
            params = _extract_params(ws)
            return json.dumps(params)
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def render_svg(self, params_json: str, size: int = 200) -> str:
        """
        Input:  JSON SignatureParams (from compute_signature_params)
        Output: SVG string
        """
        try:
            params = json.loads(params_json)
            if "error" in params:
                return f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}"><text fill="red" x="10" y="20">Error: {params["error"]}</text></svg>'
            return _build_svg(params, size)
        except Exception as exc:
            return f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}"><text fill="red" x="10" y="20">Error: {exc}</text></svg>'

    def render_ascii(self, params_json: str, width: int = 40, height: int = 20) -> str:
        """
        Input:  JSON SignatureParams
        Output: ASCII art string (for TUI display)
        """
        try:
            params = json.loads(params_json)
            if "error" in params:
                return f"[Signature Error: {params['error']}]"
            return _build_ascii(params, width, height)
        except Exception as exc:
            return f"[Render Error: {exc}]"

    def compute_and_render_svg(self, workspace_json: str, size: int = 200) -> str:
        """Convenience: compute params then render SVG in one call."""
        params_json = self.compute_signature_params(workspace_json)
        return self.render_svg(params_json, size)

    def full_report(self, workspace_json: str) -> str:
        """
        Returns a full JSON report: params + svg + ascii + description.
        """
        try:
            ws = json.loads(workspace_json)
            params = _extract_params(ws)
            svg = _build_svg(params, 200)
            ascii_art = _build_ascii(params, 38, 18)

            geo_descriptions = {
                "circle":  "Cyclic and foundational — you return to and build on core truths",
                "wave":    "Oscillatory — you move through cycles of tension and resolution",
                "spiral":  "Expanding outward — each revolution is a higher turn of the same path",
                "fractal": "Self-similar complexity — major shifts echo at every scale",
                "line":    "Sequential — you move forward, leaving what came before behind",
            }
            motion_descriptions = {
                "flow":        "Spring energy — expanding outward",
                "pulse":       "Summer strength — regular, powerful heartbeat",
                "breathe":     "Autumn reflection — slow and contemplative",
                "crystallize": "Winter stillness — precise and incubating",
                "drift":       "Balanced — no dominant season",
            }

            return json.dumps({
                "params":      params,
                "svg":         svg,
                "ascii":       ascii_art,
                "description": {
                    "geometry": geo_descriptions.get(params["geometry"], ""),
                    "motion":   motion_descriptions.get(params["motion"], ""),
                    "summary":  (
                        f"{params['geometry'].title()} {params['symmetry']} {params['motion']} — "
                        f"complexity={params['complexity']:.2f} vitality={params['vitality']:.2f} "
                        f"depth={params['depth']:.2f}"
                    ),
                },
            })
        except Exception as exc:
            return json.dumps({"error": str(exc)})


# ── Module-level shortcuts ────────────────────────────────────────────────────

def compute_signature(workspace_json: str) -> str:
    """Compute signature params from workspace JSON."""
    return LivingSignatureGenerator().compute_signature_params(workspace_json)


def render_signature_svg(workspace_json: str, size: int = 200) -> str:
    """Compute and render the Living Signature as SVG."""
    return LivingSignatureGenerator().compute_and_render_svg(workspace_json, size)


def render_signature_ascii(workspace_json: str, width: int = 38, height: int = 18) -> str:
    """Render the Living Signature as ASCII art."""
    gen = LivingSignatureGenerator()
    params_json = gen.compute_signature_params(workspace_json)
    return gen.render_ascii(params_json, width, height)
