// Phase 11: Self-contained HTML Dashboard
//
// Generates a single .html file with:
//   • Force-directed graph canvas (vanilla JS, no external deps)
//   • Live node list, journal feed, intelligence panel
//   • Dark cyberpunk theme matching GPU renderer color scheme
//   • Workspace data embedded as JSON — works offline
//   • If served via API (/dashboard), polls /nodes + /edges every 5s
//
// Usage:
//   cargo run -- dashboard [path]    → write standalone file
//   GET /dashboard                   → served from API (live polling)

use crate::intelligence::SuggestionEngine;
use crate::workspace::SilentNodeWorkspace;

pub fn export_html_dashboard(workspace: &SilentNodeWorkspace) -> String {
    // ── Serialize workspace data as JSON ──────────────────────────────────────
    let stats = workspace.graph.stats();
    let nodes_json: String = {
        let items: Vec<String> = workspace.graph.nodes().map(|n| {
            let type_str = format!("{:?}", n.node_type).to_lowercase();
            let flags = {
                let mut f = Vec::new();
                if n.is_ghost   { f.push("\"ghost\""); }
                if n.is_fossil  { f.push("\"fossil\""); }
                if n.is_void    { f.push("\"void\""); }
                f.join(",")
            };
            format!(
                r#"{{"id":"{id}","type":"{t}","content":{content},"entropy":{e:.4},"gravity":{g:.4},"velocity":{v:.4},"access_count":{ac},"x":{x:.3},"y":{y:.3},"z":{z:.3},"aura":"{aura}","flags":[{flags}]}}"#,
                id = n.id,
                t = type_str,
                content = json_str(&n.content),
                e = n.entropy,
                g = n.gravity,
                v = n.velocity,
                ac = n.access_count,
                x = n.position.x,
                y = n.position.y,
                z = n.position.z,
                aura = n.aura_color,
                flags = flags,
            )
        }).collect();
        format!("[{}]", items.join(","))
    };

    let edges_json: String = {
        let items: Vec<String> = workspace
            .graph
            .edges()
            .map(|e| {
                format!(
                    r#"{{"src":"{src}","dst":"{dst}","type":"{t}","weight":{w:.4}}}"#,
                    src = e.source_id,
                    dst = e.target_id,
                    t = format!("{:?}", e.edge_type).to_lowercase(),
                    w = e.weight,
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    };

    let journal_json: String = {
        let items: Vec<String> = workspace.journal.entries().iter().rev().take(20).map(|e| {
            let linked: Vec<String> = e.linked_nodes.iter()
                .filter_map(|id| workspace.graph.get_node(*id))
                .map(|n| json_str(&n.content))
                .collect();
            format!(
                r#"{{"id":"{id}","content":{content},"timestamp":"{ts}","season":{season},"linked":[{linked}]}}"#,
                id = e.id,
                content = json_str(&e.content),
                ts = e.timestamp.format("%Y-%m-%d %H:%M"),
                season = e.season.as_ref().map(|s| json_str(s)).unwrap_or("null".to_string()),
                linked = linked.join(","),
            )
        }).collect();
        format!("[{}]", items.join(","))
    };

    let suggestions_json: String =
        {
            let suggestions = SuggestionEngine::new().suggest_next_focus(workspace, 8);
            let items: Vec<String> = suggestions.iter().map(|s| {
            format!(
                r#"{{"id":"{id}","preview":{preview},"score":{score:.4},"reason":{reason}}}"#,
                id = s.node_id,
                preview = json_str(&s.content_preview),
                score = s.score,
                reason = json_str(&s.reason),
            )
        }).collect();
            format!("[{}]", items.join(","))
        };

    let season_json: String = {
        let r = workspace.cognitive_season();
        format!(
            r#"{{"season":"{s}","creation_rate":{cr:.3},"focus_density":{fd:.3},"exploration":{ex:.3},"avg_entropy":{ae:.3}}}"#,
            s = format!("{:?}", r.season),
            cr = r.creation_rate,
            fd = r.focus_density,
            ex = r.exploration_ratio,
            ae = r.avg_entropy,
        )
    };

    let stats_json = format!(
        r#"{{"nodes":{n},"edges":{e},"ghosts":{g},"fossils":{f},"void":{v},"focus_events":{fe},"journal_entries":{je}}}"#,
        n = stats.node_count,
        e = stats.edge_count,
        g = stats.ghost_count,
        f = stats.fossil_count,
        v = stats.void_count,
        fe = workspace.focus.events().len(),
        je = workspace.journal.entries().len(),
    );

    // ── HTML template ─────────────────────────────────────────────────────────
    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>SilentNode Dashboard</title>
<style>
:root {{
  --bg:        #080c18;
  --panel:     #0d1527;
  --panel2:    #111e38;
  --border:    #1a3060;
  --border-h:  #1e50a0;
  --cyan:      #40c8ff;
  --cyan2:     #20a0e0;
  --purple:    #8840ff;
  --green:     #40e090;
  --amber:     #f0c040;
  --red:       #e04040;
  --text:      #b0d0f0;
  --text-dim:  #4a6090;
  --ghost:     #3a4a6a;
  --fossil:    #6a5a30;
}}
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
  background: var(--bg);
  color: var(--text);
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 13px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}}

/* ── Header ── */
header {{
  display: flex;
  align-items: center;
  padding: 8px 16px;
  background: #060a12;
  border-bottom: 1px solid var(--border);
  gap: 24px;
  flex-shrink: 0;
}}
.logo {{
  font-size: 18px;
  font-weight: 700;
  color: var(--cyan);
  letter-spacing: 0.15em;
  text-shadow: 0 0 20px rgba(64,200,255,0.4);
}}
.logo-dot {{ color: var(--purple); }}
.stat-chip {{
  padding: 2px 10px;
  border: 1px solid var(--border-h);
  border-radius: 4px;
  font-size: 11px;
  color: var(--text-dim);
}}
.stat-chip span {{ color: var(--cyan); font-weight: 600; }}
.season-badge {{
  margin-left: auto;
  padding: 3px 12px;
  border-radius: 3px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
}}
.refresh-btn {{
  background: transparent;
  border: 1px solid var(--border-h);
  color: var(--text-dim);
  padding: 3px 10px;
  cursor: pointer;
  font-family: inherit;
  font-size: 11px;
  border-radius: 3px;
  transition: all 0.2s;
}}
.refresh-btn:hover {{ border-color: var(--cyan); color: var(--cyan); }}

/* ── Main layout ── */
main {{
  display: grid;
  grid-template-columns: 1fr 340px;
  grid-template-rows: 1fr 200px;
  gap: 1px;
  flex: 1;
  overflow: hidden;
  background: var(--border);
}}
.panel {{
  background: var(--panel);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}}
.panel-title {{
  padding: 6px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--cyan);
  letter-spacing: 0.1em;
  border-bottom: 1px solid var(--border);
  background: var(--panel2);
  flex-shrink: 0;
}}
.panel-body {{
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px;
}}
.panel-body::-webkit-scrollbar {{ width: 4px; }}
.panel-body::-webkit-scrollbar-track {{ background: transparent; }}
.panel-body::-webkit-scrollbar-thumb {{ background: var(--border-h); border-radius: 2px; }}

/* ── Graph canvas ── */
#graph-panel {{ grid-row: 1 / 2; grid-column: 1 / 2; }}
#graph-canvas {{
  width: 100%; height: 100%;
  display: block;
  cursor: grab;
}}
#graph-canvas:active {{ cursor: grabbing; }}

/* ── Right column ── */
#right-col {{
  grid-row: 1 / 3;
  grid-column: 2 / 3;
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--border);
}}

/* ── Bottom row ── */
#bottom-row {{
  grid-row: 2 / 3;
  grid-column: 1 / 2;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1px;
  background: var(--border);
}}

/* ── Node list ── */
.node-item {{
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  border-radius: 3px;
  cursor: pointer;
  transition: background 0.15s;
  border-bottom: 1px solid rgba(26,48,96,0.4);
}}
.node-item:hover {{ background: var(--panel2); }}
.node-item.selected {{ background: rgba(64,200,255,0.08); border-left: 2px solid var(--cyan); }}
.node-icon {{ font-size: 14px; width: 18px; text-align: center; flex-shrink: 0; }}
.node-content {{ flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
.node-meta {{ font-size: 10px; color: var(--text-dim); flex-shrink: 0; }}
.entropy-pip {{
  width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0;
}}

/* ── Node detail popup ── */
#node-detail {{
  position: fixed;
  bottom: 24px; right: 360px;
  width: 280px;
  background: #0a1220;
  border: 1px solid var(--border-h);
  border-radius: 6px;
  padding: 12px;
  display: none;
  box-shadow: 0 4px 24px rgba(0,0,0,0.6);
  z-index: 100;
}}
#node-detail.visible {{ display: block; }}
#node-detail h3 {{ color: var(--cyan); font-size: 13px; margin-bottom: 8px; }}
.detail-row {{
  display: flex; justify-content: space-between;
  padding: 2px 0;
  font-size: 11px;
  color: var(--text-dim);
}}
.detail-row span:last-child {{ color: var(--text); }}
.gauge-row {{ margin: 4px 0; }}
.gauge-label {{ font-size: 10px; color: var(--text-dim); margin-bottom: 2px; }}
.gauge-bar {{
  height: 5px; background: var(--border);
  border-radius: 3px; overflow: hidden;
}}
.gauge-fill {{ height: 100%; border-radius: 3px; transition: width 0.3s; }}

/* ── Journal ── */
.journal-entry {{
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}}
.journal-ts {{
  font-size: 10px;
  color: var(--text-dim);
  margin-bottom: 3px;
}}
.journal-season {{
  display: inline-block;
  padding: 0 6px;
  border-radius: 3px;
  font-size: 10px;
  margin-left: 6px;
}}
.journal-text {{ color: var(--text); line-height: 1.4; }}
.journal-linked {{ font-size: 10px; color: var(--text-dim); margin-top: 3px; }}

/* ── Suggestions ── */
.suggestion-item {{
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  border-radius: 3px;
  border-bottom: 1px solid rgba(26,48,96,0.4);
}}
.suggestion-rank {{
  font-size: 10px;
  color: var(--text-dim);
  width: 16px;
  flex-shrink: 0;
}}
.suggestion-content {{ flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
.suggestion-score {{
  width: 60px; height: 4px;
  background: var(--border); border-radius: 2px; overflow: hidden; flex-shrink: 0;
}}
.suggestion-bar {{ height: 100%; background: var(--cyan2); border-radius: 2px; }}
.suggestion-reason {{ font-size: 10px; color: var(--text-dim); }}

/* ── Misc ── */
.empty {{ color: var(--text-dim); font-size: 11px; padding: 8px; }}
</style>
</head>
<body>

<header>
  <div class="logo">◈ SILENT<span class="logo-dot">NODE</span></div>
  <div class="stat-chip">nodes <span id="s-nodes">…</span></div>
  <div class="stat-chip">edges <span id="s-edges">…</span></div>
  <div class="stat-chip">ghosts <span id="s-ghosts">…</span></div>
  <div class="stat-chip">focus <span id="s-focus">…</span></div>
  <div id="season-badge" class="season-badge"></div>
  <button class="refresh-btn" onclick="refresh()">⟳ refresh</button>
</header>

<main>
  <div id="graph-panel" class="panel">
    <div class="panel-title">Graph View — click node for details · scroll to zoom · drag to pan</div>
    <canvas id="graph-canvas"></canvas>
  </div>

  <div id="right-col">
    <div class="panel" style="flex:1">
      <div class="panel-title">Nodes</div>
      <input id="search-box" placeholder="filter nodes…"
        style="margin:6px 8px; padding:4px 8px; background:#0a1220; border:1px solid var(--border-h);
               color:var(--text); font-family:inherit; font-size:11px; border-radius:3px; outline:none;"
        oninput="filterNodes(this.value)">
      <div class="panel-body" id="node-list"></div>
    </div>
    <div class="panel" style="height:180px; flex-shrink:0">
      <div class="panel-title">Focus Suggestions</div>
      <div class="panel-body" id="suggestion-list"></div>
    </div>
  </div>

  <div id="bottom-row">
    <div class="panel">
      <div class="panel-title">Journal</div>
      <div class="panel-body" id="journal-list"></div>
    </div>
    <div class="panel">
      <div class="panel-title">Intelligence</div>
      <div class="panel-body" id="intel-panel"></div>
    </div>
  </div>
</main>

<div id="node-detail">
  <div style="display:flex; justify-content:space-between; align-items:center">
    <h3 id="detail-name"></h3>
    <span style="cursor:pointer;color:var(--text-dim)" onclick="closeDetail()">✕</span>
  </div>
  <div id="detail-body"></div>
</div>

<script>
// ── Embedded data (baked in at export time) ───────────────────────────────────
const INIT_NODES   = {nodes_json};
const INIT_EDGES   = {edges_json};
const INIT_JOURNAL = {journal_json};
const INIT_SUGGEST = {suggestions_json};
const INIT_SEASON  = {season_json};
const INIT_STATS   = {stats_json};

// ── State ─────────────────────────────────────────────────────────────────────
let nodes = [], edges = [], journal = [], suggestions = [], season = {{}}, stats = {{}};
let simNodes = [];      // physics simulation nodes
let selectedId = null;
let filterQuery = '';
let camera = {{ x: 0, y: 0, scale: 1 }};
let dragging = false, dragStart = null;
let animFrame = null;

// ── Node type colors + icons ──────────────────────────────────────────────────
const TYPE_COLOR = {{
  idea:     '#40c8ff', memory: '#c860ff', project: '#40e090',
  person:   '#f0c040', artifact:'#6090ff', media:   '#40b0c0',
  process:  '#80ffa0', world:   '#ffffff', ghost:   '#3a4a6a',
  fossil:   '#9a8050', default: '#8090b0',
}};
const TYPE_ICON = {{
  idea:'◆', memory:'◉', project:'▣', person:'◎',
  artifact:'◧', media:'◐', process:'◑', world:'◯', ghost:'◌', fossil:'◫',
}};
function nodeColor(n) {{
  if (n.flags.includes('void'))   return '#6a1080';
  if (n.flags.includes('fossil')) return TYPE_COLOR.fossil;
  if (n.flags.includes('ghost'))  return TYPE_COLOR.ghost;
  return TYPE_COLOR[n.type] || TYPE_COLOR.default;
}}
function nodeIcon(n) {{ return TYPE_ICON[n.type] || '●'; }}

// ── Simulation (force-directed) ───────────────────────────────────────────────
function initSim() {{
  const nodeMap = {{}};
  simNodes = nodes.map(n => {{
    const sn = {{
      id: n.id, n,
      x: (n.x || (Math.random()-0.5)*200),
      y: (n.z || (Math.random()-0.5)*200),
      vx: 0, vy: 0,
    }};
    nodeMap[n.id] = sn;
    return sn;
  }});
  // fix existing positions if non-zero
  simNodes.forEach(sn => {{
    if (Math.abs(sn.n.x) > 0.01 || Math.abs(sn.n.z) > 0.01) {{
      sn.x = sn.n.x * 8;
      sn.y = sn.n.z * 8;
    }}
  }});
  return nodeMap;
}}

let nodeMap = {{}};

function stepSim() {{
  const k = 80, grav = 0.01, damp = 0.85;
  // repulsion
  for (let i = 0; i < simNodes.length; i++) {{
    for (let j = i+1; j < simNodes.length; j++) {{
      const a = simNodes[i], b = simNodes[j];
      const dx = b.x - a.x, dy = b.y - a.y;
      const d = Math.sqrt(dx*dx + dy*dy) || 0.1;
      const f = (k*k) / (d*d);
      const fx = (dx/d)*f, fy = (dy/d)*f;
      a.vx -= fx; a.vy -= fy;
      b.vx += fx; b.vy += fy;
    }}
  }}
  // attraction along edges
  edges.forEach(e => {{
    const a = nodeMap[e.src], b = nodeMap[e.dst];
    if (!a || !b) return;
    const dx = b.x - a.x, dy = b.y - a.y;
    const d = Math.sqrt(dx*dx + dy*dy) || 0.1;
    const f = (d - k) * e.weight * 0.6;
    const fx = (dx/d)*f, fy = (dy/d)*f;
    a.vx += fx; a.vy += fy;
    b.vx -= fx; b.vy -= fy;
  }});
  // center gravity
  simNodes.forEach(sn => {{
    sn.vx -= sn.x * grav;
    sn.vy -= sn.y * grav;
    sn.x += sn.vx; sn.y += sn.vy;
    sn.vx *= damp;  sn.vy *= damp;
  }});
}}

// ── Canvas rendering ──────────────────────────────────────────────────────────
const canvas = document.getElementById('graph-canvas');
const ctx2 = canvas.getContext('2d');

function resizeCanvas() {{
  const rect = canvas.parentElement.getBoundingClientRect();
  canvas.width  = rect.width;
  canvas.height = rect.height;
}}

function drawGraph() {{
  resizeCanvas();
  const w = canvas.width, h = canvas.height;
  ctx2.clearRect(0, 0, w, h);
  ctx2.save();
  ctx2.translate(w/2 + camera.x, h/2 + camera.y);
  ctx2.scale(camera.scale, camera.scale);

  // edges
  edges.forEach(e => {{
    const a = nodeMap[e.src], b = nodeMap[e.dst];
    if (!a || !b) return;
    ctx2.beginPath();
    ctx2.moveTo(a.x, a.y);
    ctx2.lineTo(b.x, b.y);
    ctx2.strokeStyle = `rgba(30,80,180,${{0.3 + e.weight * 0.5}})`;
    ctx2.lineWidth = 0.5 + e.weight * 1.5;
    ctx2.stroke();
  }});

  // nodes
  simNodes.forEach(sn => {{
    const n = sn.n;
    const r = 4 + Math.min(n.gravity, 5) * 2;
    const color = nodeColor(n);
    const isSelected = n.id === selectedId;

    if (isSelected) {{
      ctx2.beginPath();
      ctx2.arc(sn.x, sn.y, r + 5, 0, Math.PI*2);
      ctx2.strokeStyle = '#40c8ff';
      ctx2.lineWidth = 1.5;
      ctx2.stroke();
    }}

    // glow
    const grd = ctx2.createRadialGradient(sn.x, sn.y, 0, sn.x, sn.y, r*2.5);
    grd.addColorStop(0, color + '44');
    grd.addColorStop(1, 'transparent');
    ctx2.beginPath();
    ctx2.arc(sn.x, sn.y, r*2.5, 0, Math.PI*2);
    ctx2.fillStyle = grd;
    ctx2.fill();

    // node body
    ctx2.beginPath();
    ctx2.arc(sn.x, sn.y, r, 0, Math.PI*2);
    ctx2.fillStyle = color;
    ctx2.fill();

    // entropy overlay (red tint if high entropy)
    if (n.entropy > 0.5) {{
      ctx2.beginPath();
      ctx2.arc(sn.x, sn.y, r, 0, Math.PI*2);
      ctx2.fillStyle = `rgba(220,60,60,${{(n.entropy - 0.5) * 0.6}})`;
      ctx2.fill();
    }}

    // label (only when zoomed in)
    if (camera.scale > 0.8) {{
      ctx2.font = `${{Math.max(9, 10 * camera.scale)}}px monospace`;
      ctx2.fillStyle = 'rgba(180,210,255,0.85)';
      ctx2.textAlign = 'center';
      const label = n.content.length > 16 ? n.content.slice(0,15)+'…' : n.content;
      ctx2.fillText(label, sn.x, sn.y + r + 10);
    }}
  }});

  ctx2.restore();
  stepSim();
  animFrame = requestAnimationFrame(drawGraph);
}}

// ── Pan + zoom ────────────────────────────────────────────────────────────────
canvas.addEventListener('mousedown', e => {{
  dragging = true;
  dragStart = {{ x: e.clientX - camera.x, y: e.clientY - camera.y }};
}});
canvas.addEventListener('mousemove', e => {{
  if (dragging) {{
    camera.x = e.clientX - dragStart.x;
    camera.y = e.clientY - dragStart.y;
  }}
}});
canvas.addEventListener('mouseup', e => {{ dragging = false; }});
canvas.addEventListener('wheel', e => {{
  e.preventDefault();
  const delta = e.deltaY > 0 ? 0.9 : 1.1;
  camera.scale = Math.max(0.2, Math.min(5, camera.scale * delta));
}}, {{ passive: false }});
canvas.addEventListener('click', e => {{
  if (Math.abs(e.movementX) + Math.abs(e.movementY) > 4) return;
  const rect = canvas.getBoundingClientRect();
  const mx = (e.clientX - rect.left - canvas.width/2 - camera.x) / camera.scale;
  const my = (e.clientY - rect.top  - canvas.height/2 - camera.y) / camera.scale;
  let hit = null, bestD = 20;
  simNodes.forEach(sn => {{
    const d = Math.sqrt((sn.x-mx)**2 + (sn.y-my)**2);
    if (d < bestD) {{ bestD = d; hit = sn.n; }}
  }});
  if (hit) selectNode(hit.id);
  else closeDetail();
}});

// ── Node selection + detail ───────────────────────────────────────────────────
function selectNode(id) {{
  selectedId = id;
  const n = nodes.find(n => n.id === id);
  if (!n) return;
  document.getElementById('detail-name').textContent = n.content;
  const body = document.getElementById('detail-body');
  const eColor = n.entropy > 0.7 ? '#e04040' : n.entropy > 0.4 ? '#f0c040' : '#40e090';
  body.innerHTML = `
    <div class="detail-row"><span>Type</span><span style="color:${{nodeColor(n)}}">${{n.type}}</span></div>
    <div class="detail-row"><span>Accesses</span><span>${{n.access_count}}</span></div>
    <div class="gauge-row">
      <div class="gauge-label">Entropy ${{n.entropy.toFixed(3)}}</div>
      <div class="gauge-bar"><div class="gauge-fill" style="width:${{n.entropy*100}}%;background:${{eColor}}"></div></div>
    </div>
    <div class="gauge-row">
      <div class="gauge-label">Gravity ${{n.gravity.toFixed(3)}}</div>
      <div class="gauge-bar"><div class="gauge-fill" style="width:${{Math.min(n.gravity/5,1)*100}}%;background:#4090ff"></div></div>
    </div>
    <div class="detail-row"><span>Pos</span><span>(${{n.x.toFixed(1)}}, ${{n.y.toFixed(1)}}, ${{n.z.toFixed(1)}})</span></div>
    ${{n.flags.length ? `<div class="detail-row"><span>Flags</span><span style="color:#f0c040">${{n.flags.join(', ')}}</span></div>` : ''}}
  `;
  document.getElementById('node-detail').classList.add('visible');
  // highlight in node list
  document.querySelectorAll('.node-item').forEach(el => {{
    el.classList.toggle('selected', el.dataset.id === id);
  }});
}}
function closeDetail() {{
  selectedId = null;
  document.getElementById('node-detail').classList.remove('visible');
  document.querySelectorAll('.node-item').forEach(el => el.classList.remove('selected'));
}}

// ── Render panels ─────────────────────────────────────────────────────────────
function renderNodeList(query) {{
  const q = (query||'').toLowerCase();
  const filtered = nodes.filter(n => !q || n.content.toLowerCase().includes(q));
  const sorted = [...filtered].sort((a,b) => b.gravity - a.gravity);
  const el = document.getElementById('node-list');
  if (sorted.length === 0) {{
    el.innerHTML = '<div class="empty">no nodes match</div>';
    return;
  }}
  el.innerHTML = sorted.map(n => {{
    const eColor = n.entropy > 0.7 ? '#e04040' : n.entropy > 0.4 ? '#f0c040' : '#40e090';
    return `<div class="node-item" data-id="${{n.id}}" onclick="selectNode('${{n.id}}')">
      <span class="node-icon" style="color:${{nodeColor(n)}}">${{nodeIcon(n)}}</span>
      <span class="node-content">${{esc(n.content)}}</span>
      <span class="entropy-pip" style="background:${{eColor}}"></span>
      <span class="node-meta">${{n.gravity.toFixed(1)}}</span>
    </div>`;
  }}).join('');
}}

function renderJournal() {{
  const el = document.getElementById('journal-list');
  if (journal.length === 0) {{ el.innerHTML = '<div class="empty">no journal entries</div>'; return; }}
  const SEASON_COLOR = {{ spring:'#40e090', summer:'#f0c040', autumn:'#e07020', winter:'#6090ff' }};
  el.innerHTML = journal.map(e => {{
    const sc = SEASON_COLOR[e.season] || '#4a6090';
    const linked = e.linked.length ? `<div class="journal-linked">⟳ ${{e.linked.map(esc).join(' • ')}}</div>` : '';
    return `<div class="journal-entry">
      <div class="journal-ts">${{e.timestamp}}
        ${{e.season ? `<span class="journal-season" style="background:${{sc}}22;color:${{sc}};border:1px solid ${{sc}}44">${{e.season}}</span>` : ''}}
      </div>
      <div class="journal-text">${{esc(e.content)}}</div>
      ${{linked}}
    </div>`;
  }}).join('');
}}

function renderSuggestions() {{
  const el = document.getElementById('suggestion-list');
  if (suggestions.length === 0) {{ el.innerHTML = '<div class="empty">no suggestions</div>'; return; }}
  const maxScore = suggestions[0]?.score || 1;
  el.innerHTML = suggestions.map((s, i) => {{
    const pct = Math.min((s.score / maxScore) * 100, 100);
    return `<div class="suggestion-item" onclick="selectNode('${{s.id}}')" style="cursor:pointer">
      <span class="suggestion-rank">${{i+1}}.</span>
      <div style="flex:1;min-width:0">
        <div class="suggestion-content">${{esc(s.preview)}}</div>
        <div class="suggestion-reason">${{esc(s.reason)}}</div>
      </div>
      <div class="suggestion-score"><div class="suggestion-bar" style="width:${{pct}}%"></div></div>
    </div>`;
  }}).join('');
}}

function renderIntel() {{
  const el = document.getElementById('intel-panel');
  const SEASON_COLOR = {{ Spring:'#40e090', Summer:'#f0c040', Autumn:'#e07020', Winter:'#6090ff' }};
  const sc = SEASON_COLOR[season.season] || '#40c8ff';
  el.innerHTML = `
    <div style="margin-bottom:8px">
      <div style="font-size:11px;color:var(--text-dim);margin-bottom:4px">COGNITIVE SEASON</div>
      <div style="color:${{sc}};font-weight:700;font-size:14px;margin-bottom:6px">${{season.season || '—'}}</div>
      ${{gaugeHtml('Creation', season.creation_rate, '#40c8ff')}}
      ${{gaugeHtml('Focus', season.focus_density, '#8840ff')}}
      ${{gaugeHtml('Explore', season.exploration, '#40e090')}}
      ${{gaugeHtml('Entropy', season.avg_entropy, '#e04040')}}
    </div>`;
}}

function gaugeHtml(label, val, color) {{
  const pct = Math.min((val||0)*100, 100).toFixed(1);
  return `<div style="margin:3px 0">
    <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text-dim);margin-bottom:1px">
      <span>${{label}}</span><span>${{(val||0).toFixed(3)}}</span>
    </div>
    <div style="height:3px;background:var(--border);border-radius:2px">
      <div style="height:100%;width:${{pct}}%;background:${{color}};border-radius:2px;transition:width 0.3s"></div>
    </div>
  </div>`;
}}

function renderStats() {{
  document.getElementById('s-nodes').textContent  = stats.nodes  || '—';
  document.getElementById('s-edges').textContent  = stats.edges  || '—';
  document.getElementById('s-ghosts').textContent = stats.ghosts || '—';
  document.getElementById('s-focus').textContent  = stats.focus_events || '—';
  const badge = document.getElementById('season-badge');
  const SEASON_COLOR = {{ Spring:'#40e090', Summer:'#f0c040', Autumn:'#e07020', Winter:'#6090ff' }};
  const sc = SEASON_COLOR[season.season] || '#40c8ff';
  badge.textContent = season.season ? season.season.toUpperCase() : '';
  badge.style.background = sc + '22';
  badge.style.color = sc;
  badge.style.border = `1px solid ${{sc}}66`;
}}

// ── Load data + optional API poll ─────────────────────────────────────────────
function loadData(n, e, j, s, sg, st) {{
  nodes = n; edges = e; journal = j; season = sg; suggestions = s; stats = st;
  nodeMap = initSim();
  renderNodeList(filterQuery);
  renderJournal();
  renderSuggestions();
  renderIntel();
  renderStats();
}}

async function tryFetchApi() {{
  try {{
    const [nRes, eRes] = await Promise.all([
      fetch('/nodes', {{signal: AbortSignal.timeout(2000)}}),
      fetch('/edges', {{signal: AbortSignal.timeout(2000)}}),
    ]);
    if (nRes.ok && eRes.ok) {{
      const newNodes = await nRes.json();
      const newEdges = await eRes.json();
      // transform API shape to dashboard shape
      nodes = newNodes.map(n => ({{
        id: n.id, type: n.node_type.toLowerCase(), content: n.content,
        entropy: n.entropy, gravity: n.gravity, velocity: n.velocity,
        access_count: n.access_count, x: n.position.x, y: n.position.y, z: n.position.z,
        aura: n.aura_color, flags: [
          ...(n.is_ghost ? ['ghost'] : []),
          ...(n.is_fossil ? ['fossil'] : []),
          ...(n.is_void ? ['void'] : []),
        ]
      }}));
      edges = newEdges.map(e => ({{ src: e.source_id, dst: e.target_id, weight: e.weight, type: e.edge_type.toLowerCase() }}));
      nodeMap = {{}};
      // keep existing sim positions
      const oldMap = {{}};
      simNodes.forEach(sn => {{ oldMap[sn.id] = sn; }});
      simNodes = nodes.map(n => {{
        const old = oldMap[n.id];
        const sn = old || {{ id: n.id, n, x: (Math.random()-0.5)*200, y: (Math.random()-0.5)*200, vx:0, vy:0 }};
        sn.n = n;
        nodeMap[n.id] = sn;
        return sn;
      }});
      renderNodeList(filterQuery);
      stats.nodes = nodes.length;
      stats.edges = edges.length;
      renderStats();
    }}
  }} catch(_) {{ /* offline — use embedded data */ }}
}}

function refresh() {{ tryFetchApi(); }}
function filterNodes(q) {{ filterQuery = q; renderNodeList(q); }}
function esc(s) {{
  if (!s) return '';
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}}

// ── Init ──────────────────────────────────────────────────────────────────────
window.addEventListener('load', () => {{
  loadData(INIT_NODES, INIT_EDGES, INIT_JOURNAL, INIT_SUGGEST, INIT_SEASON, INIT_STATS);
  drawGraph();
  tryFetchApi();
  setInterval(tryFetchApi, 8000);
}});
window.addEventListener('resize', resizeCanvas);
</script>
</body>
</html>
"###,
        nodes_json = nodes_json,
        edges_json = edges_json,
        journal_json = journal_json,
        suggestions_json = suggestions_json,
        season_json = season_json,
        stats_json = stats_json,
    )
}

fn json_str(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
        .replace('\t', " ");
    format!("\"{}\"", escaped)
}
