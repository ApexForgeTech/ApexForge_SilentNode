use crate::workspace::SilentNodeWorkspace;
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct VisualizationEngine;

impl VisualizationEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn render_to_file(
        &self,
        workspace: &SilentNodeWorkspace,
        output_path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        fs::write(output_path, self.render_html(workspace))
    }

    pub fn render_html(&self, workspace: &SilentNodeWorkspace) -> String {
        let export = workspace.graph.export();
        let nodes = export.nodes;
        let edges = export.edges;
        let heatmap = workspace.heatmap_report(7);
        let recent_trail = workspace.recent_trail(24);
        let recent_journal = workspace.recent_journal(14);
        let animation_frames = simulate_frames(workspace, 18, 0.85);

        let node_lookup = nodes
            .iter()
            .map(|node| (node.id, node))
            .collect::<std::collections::HashMap<_, _>>();
        let heatmap_lookup = heatmap
            .iter()
            .copied()
            .collect::<std::collections::HashMap<_, _>>();

        let mut edge_lines = String::new();
        for edge in &edges {
            let source = node_lookup.get(&edge.source_id);
            let target = node_lookup.get(&edge.target_id);
            if let (Some(source), Some(target)) = (source, target) {
                edge_lines.push_str(&format!(
                    r#"<line class="graph-edge" data-source-id="{source_id}" data-target-id="{target_id}" x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="rgba(148,163,184,0.35)" stroke-width="{:.2}" />"#,
                    source.position.x + 450.0,
                    source.position.y + 320.0,
                    target.position.x + 450.0,
                    target.position.y + 320.0,
                    1.0 + edge.weight * 2.0,
                    source_id = edge.source_id,
                    target_id = edge.target_id,
                ));
            }
        }

        let mut trail_lines = String::new();
        for pair in recent_trail.windows(2) {
            let source = node_lookup.get(&pair[0].node_id);
            let target = node_lookup.get(&pair[1].node_id);
            if let (Some(source), Some(target)) = (source, target) {
                trail_lines.push_str(&format!(
                    r#"<line class="trail-edge" data-source-id="{source_id}" data-target-id="{target_id}" x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="rgba(34,211,238,0.55)" stroke-dasharray="6 4" stroke-width="2.2" />"#,
                    source.position.x + 450.0,
                    source.position.y + 320.0,
                    target.position.x + 450.0,
                    target.position.y + 320.0,
                    source_id = pair[0].node_id,
                    target_id = pair[1].node_id,
                ));
            }
        }

        let mut node_circles = String::new();
        for node in &nodes {
            let radius = 10.0 + node.gravity * 1.8;
            let color = entropy_color(node.entropy, &node.aura_color);
            let glow = 0.25 + heatmap_lookup.get(&node.id).copied().unwrap_or(0.0) * 0.75;
            node_circles.push_str(&format!(
                r##"<g class="node" data-node-id="{id}"><circle cx="{cx:.2}" cy="{cy:.2}" r="{r:.2}" fill="{fill}" fill-opacity="0.88" stroke="#e2e8f0" stroke-width="1.5" style="filter: drop-shadow(0 0 14px rgba(125,211,252,{glow:.2}));"/><text x="{cx:.2}" y="{ty:.2}" fill="#e2e8f0" font-size="11" text-anchor="middle">{label}</text></g>"##,
                id = node.id,
                cx = node.position.x + 450.0,
                cy = node.position.y + 320.0,
                r = radius,
                fill = color,
                ty = node.position.y + 324.0,
                glow = glow,
                label = html_escape(&short_label(&node.content)),
            ));
        }

        let graph_payload = json!({
            "nodes": nodes.iter().map(|node| {
                json!({
                    "id": node.id.to_string(),
                    "type": format!("{:?}", node.node_type),
                    "content": node.content,
                    "entropy": node.entropy,
                    "gravity": node.gravity,
                    "velocity": node.velocity,
                    "access_count": node.access_count,
                    "position": { "x": node.position.x, "y": node.position.y, "z": node.position.z },
                    "is_ghost": node.is_ghost,
                    "is_fossil": node.is_fossil,
                    "is_void": node.is_void,
                })
            }).collect::<Vec<_>>(),
            "edges": edges.iter().map(|edge| {
                json!({
                    "source_id": edge.source_id.to_string(),
                    "target_id": edge.target_id.to_string(),
                    "edge_type": format!("{:?}", edge.edge_type),
                    "weight": edge.weight,
                })
            }).collect::<Vec<_>>(),
            "heatmap": heatmap.iter().map(|(node_id, score)| {
                json!({ "node_id": node_id.to_string(), "score": score })
            }).collect::<Vec<_>>(),
            "trail": recent_trail.iter().map(|event| {
                json!({
                    "node_id": event.node_id.to_string(),
                    "timestamp": event.timestamp.to_rfc3339(),
                    "duration_seconds": event.duration_seconds,
                    "depth": format!("{:?}", event.depth),
                })
            }).collect::<Vec<_>>(),
            "journal": recent_journal.iter().map(|entry| {
                json!({
                    "id": entry.id.to_string(),
                    "content": entry.content,
                    "timestamp": entry.timestamp.to_rfc3339(),
                    "linked_nodes": entry.linked_nodes.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "season": entry.season,
                })
            }).collect::<Vec<_>>(),
            "frames": animation_frames,
        });

        let initial_node_id = nodes
            .first()
            .map(|node| node.id.to_string())
            .unwrap_or_default();

        format!(
            r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>SilentNode Universe</title>
  <style>
    :root {{
      --bg: #07111f;
      --panel: #0f1b31;
      --panel-soft: rgba(30, 41, 59, 0.55);
      --text: #e2e8f0;
      --muted: #94a3b8;
      --accent: #22d3ee;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      background: radial-gradient(circle at top, #142847 0%, var(--bg) 55%);
      color: var(--text);
      font-family: "Iosevka", "JetBrains Mono", monospace;
      display: grid;
      grid-template-columns: minmax(0, 2fr) minmax(360px, 1fr);
      min-height: 100vh;
    }}
    .stage {{
      position: relative;
      overflow: hidden;
    }}
    svg {{
      width: 100%;
      height: 100vh;
      display: block;
      background:
        radial-gradient(circle at 20% 10%, rgba(34,211,238,0.10), transparent 30%),
        linear-gradient(180deg, rgba(125,211,252,0.05), transparent 40%);
    }}
    .legend {{
      position: absolute;
      left: 20px;
      top: 20px;
      padding: 12px 14px;
      border-radius: 14px;
      background: rgba(15, 27, 49, 0.82);
      border: 1px solid rgba(148,163,184,0.15);
      color: var(--muted);
      font-size: 12px;
      line-height: 1.5;
    }}
    aside {{
      padding: 20px;
      background: rgba(15, 27, 49, 0.94);
      border-left: 1px solid rgba(148,163,184,0.18);
      overflow-y: auto;
      display: grid;
      gap: 16px;
      align-content: start;
    }}
    h1, h2, h3 {{
      margin: 0;
      font-weight: 700;
    }}
    h1 {{ font-size: 26px; }}
    h2 {{ font-size: 16px; }}
    h3 {{ font-size: 14px; color: var(--muted); }}
    p, li, span, label {{ color: var(--muted); }}
    .card {{
      padding: 16px;
      border-radius: 16px;
      background: var(--panel-soft);
      border: 1px solid rgba(148,163,184,0.10);
    }}
    .search {{
      width: 100%;
      margin-top: 10px;
      padding: 10px 12px;
      border-radius: 12px;
      border: 1px solid rgba(148,163,184,0.18);
      background: rgba(2, 6, 23, 0.55);
      color: var(--text);
      outline: none;
    }}
    .grid {{
      display: grid;
      gap: 10px;
    }}
    .node-list button {{
      width: 100%;
      text-align: left;
      padding: 10px 12px;
      border: 0;
      border-radius: 12px;
      background: rgba(2, 6, 23, 0.45);
      color: var(--text);
      cursor: pointer;
      font: inherit;
    }}
    .node-list button:hover,
    .node-list button.active {{
      background: rgba(34, 211, 238, 0.14);
    }}
    ul {{
      list-style: none;
      padding: 0;
      margin: 0;
      display: grid;
      gap: 10px;
    }}
    li {{
      padding: 10px 12px;
      border-radius: 12px;
      background: rgba(2, 6, 23, 0.38);
    }}
    .metric {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      margin-top: 12px;
    }}
    .metric div {{
      padding: 10px;
      border-radius: 12px;
      background: rgba(2, 6, 23, 0.35);
    }}
    .node {{
      cursor: pointer;
      transition: transform 120ms ease;
    }}
    .node:hover {{
      transform: scale(1.04);
    }}
    .node.selected circle {{
      stroke: var(--accent);
      stroke-width: 2.8;
    }}
    .controls {{
      display: grid;
      gap: 10px;
      margin-top: 12px;
    }}
    .controls-row {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
    }}
    button.control {{
      padding: 10px 12px;
      border-radius: 12px;
      border: 1px solid rgba(148,163,184,0.18);
      background: rgba(2, 6, 23, 0.55);
      color: var(--text);
      cursor: pointer;
      font: inherit;
    }}
    button.control:hover {{
      background: rgba(34, 211, 238, 0.14);
    }}
    .inline-label {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      font-size: 13px;
    }}
    .inline-label input[type="range"] {{
      width: 100%;
    }}
    .checkbox-row {{
      display: flex;
      align-items: center;
      gap: 10px;
      color: var(--text);
    }}
    code {{
      color: var(--text);
      word-break: break-all;
    }}
  </style>
</head>
<body>
  <div class="stage">
    <div class="legend">
      <div>Solid links: graph edges</div>
      <div>Dashed links: recent focus trail</div>
      <div>Node glow: recent heatmap intensity</div>
    </div>
    <svg viewBox="0 0 900 640" role="img" aria-label="SilentNode cognitive graph">
      {edge_lines}
      {trail_lines}
      {node_circles}
    </svg>
  </div>
  <aside>
    <section class="card">
      <h1>SilentNode Universe</h1>
      <p>Node count: {node_count} | Edge count: {edge_count}</p>
      <div class="metric">
        <div><strong>{trail_count}</strong><br/>recent trail events</div>
        <div><strong>{journal_count}</strong><br/>recent journal entries</div>
      </div>
    </section>

    <section class="card">
      <h2>Node Search</h2>
      <input id="node-search" class="search" type="search" placeholder="Search nodes, ideas, projects..." />
      <div class="controls">
        <label class="inline-label">Heat threshold
          <input id="heat-filter" type="range" min="0" max="100" value="0" />
        </label>
        <label class="checkbox-row">
          <input id="ghost-toggle" type="checkbox" checked />
          <span>Show ghost and fossil nodes</span>
        </label>
      </div>
      <div id="node-list" class="node-list grid"></div>
    </section>

    <section class="card">
      <h2>Motion Controls</h2>
      <div class="controls-row">
        <button id="play-toggle" class="control" type="button">Pause</button>
        <button id="step-frame" class="control" type="button">Step</button>
        <button id="reset-frame" class="control" type="button">Reset</button>
      </div>
      <p>Gravity frames replay the recent layout drift of the graph.</p>
    </section>

    <section class="card">
      <h2>Selected Node</h2>
      <div id="selected-node">
        <p>Select a node to inspect its state.</p>
      </div>
    </section>

    <section class="card">
      <h2>Recent Focus Trail</h2>
      <ul id="trail-list"></ul>
    </section>

    <section class="card">
      <h2>Recent Journal</h2>
      <ul id="journal-list"></ul>
    </section>

    <section class="card">
      <h2>Heatmap Leaders</h2>
      <ul id="heatmap-list"></ul>
    </section>
  </aside>

  <script id="graph-payload" type="application/json">{payload}</script>
  <script>
    const payload = JSON.parse(document.getElementById('graph-payload').textContent);
    const nodeMap = new Map(payload.nodes.map(node => [node.id, node]));
    const edgeMap = new Map();
    for (const edge of payload.edges) {{
      const list = edgeMap.get(edge.source_id) || [];
      list.push(edge);
      edgeMap.set(edge.source_id, list);
    }}

    const nodeList = document.getElementById('node-list');
    const selectedNode = document.getElementById('selected-node');
    const trailList = document.getElementById('trail-list');
    const journalList = document.getElementById('journal-list');
    const heatmapList = document.getElementById('heatmap-list');
    const searchInput = document.getElementById('node-search');
    const heatFilter = document.getElementById('heat-filter');
    const ghostToggle = document.getElementById('ghost-toggle');
    const playToggle = document.getElementById('play-toggle');
    const stepFrameButton = document.getElementById('step-frame');
    const resetFrameButton = document.getElementById('reset-frame');
    const svgNodes = Array.from(document.querySelectorAll('.node'));
    const graphEdges = Array.from(document.querySelectorAll('.graph-edge'));
    const trailEdges = Array.from(document.querySelectorAll('.trail-edge'));
    const frames = payload.frames || [];
    let currentFrameIndex = 0;
    let isPlaying = true;
    let animationTimer = null;

    function renderNodeButtons(filter = '') {{
      const normalized = filter.trim().toLowerCase();
      const minimumHeat = Number(heatFilter.value) / 100;
      const showGhosts = ghostToggle.checked;
      const filtered = payload.nodes.filter(node =>
        (!normalized ||
          node.content.toLowerCase().includes(normalized) ||
          node.type.toLowerCase().includes(normalized)) &&
        (showGhosts || (!node.is_ghost && !node.is_fossil)) &&
        heatForNode(node.id) >= minimumHeat
      );

      nodeList.innerHTML = filtered.map(node => `
        <button data-node-button="${{node.id}}">
          <strong>${{escapeHtml(node.content)}}</strong><br/>
          <span>${{node.type}} | entropy ${{node.entropy.toFixed(2)}} | gravity ${{node.gravity.toFixed(2)}}</span>
        </button>
      `).join('');

      for (const button of nodeList.querySelectorAll('button')) {{
        button.addEventListener('click', () => selectNode(button.dataset.nodeButton));
      }}
    }}

    function renderTrail() {{
      trailList.innerHTML = payload.trail.map(entry => {{
        const node = nodeMap.get(entry.node_id);
        return `
          <li>
            <strong>${{escapeHtml(node ? node.content : entry.node_id)}}</strong><br/>
            <span>${{entry.depth}} | ${{Math.round(entry.duration_seconds)}}s | ${{entry.timestamp}}</span>
          </li>
        `;
      }}).join('') || '<li>No recent trail data yet.</li>';
    }}

    function renderJournal() {{
      journalList.innerHTML = payload.journal.map(entry => `
        <li>
          <strong>${{entry.season || 'unspecified season'}}</strong><br/>
          <span>${{entry.timestamp}}</span><br/>
          <span>${{linkedNodeNames(entry.linked_nodes)}}</span><br/>
          ${{escapeHtml(entry.content)}}
        </li>
      `).join('') || '<li>No journal entries yet.</li>';
    }}

    function renderHeatmap() {{
      const rows = [...payload.heatmap]
        .sort((left, right) => right.score - left.score)
        .slice(0, 8)
        .map(item => {{
          const node = nodeMap.get(item.node_id);
          return `
            <li>
              <strong>${{escapeHtml(node ? node.content : item.node_id)}}</strong><br/>
              <span>heat ${{item.score.toFixed(2)}} | ${{node ? node.type : 'unknown'}}</span>
            </li>
          `;
        }});
      heatmapList.innerHTML = rows.join('') || '<li>No heatmap activity yet.</li>';
    }}

    function selectNode(nodeId) {{
      const node = nodeMap.get(nodeId);
      if (!node) return;

      for (const group of svgNodes) {{
        group.classList.toggle('selected', group.dataset.nodeId === nodeId);
      }}
      for (const button of nodeList.querySelectorAll('button')) {{
        button.classList.toggle('active', button.dataset.nodeButton === nodeId);
      }}

      const outgoing = edgeMap.get(nodeId) || [];
      const incoming = payload.edges.filter(edge => edge.target_id === nodeId);
      const heat = payload.heatmap.find(item => item.node_id === nodeId);
      const linkedJournal = payload.journal.filter(entry => entry.linked_nodes.includes(nodeId));
      const focusEvents = payload.trail.filter(entry => entry.node_id === nodeId);
      const outgoingHtml = outgoing.length
        ? outgoing.map(edge => {{
            const target = nodeMap.get(edge.target_id);
            return `<li><strong>${{escapeHtml(target ? target.content : edge.target_id)}}</strong><br/><span>${{edge.edge_type}} | weight ${{edge.weight.toFixed(2)}}</span></li>`;
          }}).join('')
        : '<li>No outgoing links.</li>';
      const incomingHtml = incoming.length
        ? incoming.map(edge => {{
            const source = nodeMap.get(edge.source_id);
            return `<li><strong>${{escapeHtml(source ? source.content : edge.source_id)}}</strong><br/><span>${{edge.edge_type}} | weight ${{edge.weight.toFixed(2)}}</span></li>`;
          }}).join('')
        : '<li>No incoming links.</li>';
      const journalHtml = linkedJournal.length
        ? linkedJournal.map(entry => `<li><strong>${{entry.timestamp}}</strong><br/>${{escapeHtml(entry.content)}}</li>`).join('')
        : '<li>No linked journal entries.</li>';
      const focusHtml = focusEvents.length
        ? focusEvents.map(entry => `<li><strong>${{entry.depth}}</strong><br/><span>${{Math.round(entry.duration_seconds)}}s | ${{entry.timestamp}}</span></li>`).join('')
        : '<li>No recent focus events.</li>';
      selectedNode.innerHTML = `
        <h3>${{escapeHtml(node.content)}}</h3>
        <p><code>${{node.id}}</code></p>
        <div class="metric">
          <div><strong>${{node.type}}</strong><br/>node type</div>
          <div><strong>${{node.access_count}}</strong><br/>access count</div>
          <div><strong>${{node.entropy.toFixed(2)}}</strong><br/>entropy</div>
          <div><strong>${{node.gravity.toFixed(2)}}</strong><br/>gravity</div>
          <div><strong>${{node.velocity.toFixed(2)}}</strong><br/>velocity</div>
          <div><strong>${{heat ? heat.score.toFixed(2) : '0.00'}}</strong><br/>heatmap score</div>
        </div>
        <p>Ghost: ${{node.is_ghost}} | Fossil: ${{node.is_fossil}} | Void: ${{node.is_void}}</p>
        <h3>Outgoing Links</h3>
        <ul>${{outgoingHtml}}</ul>
        <h3>Incoming Links</h3>
        <ul>${{incomingHtml}}</ul>
        <h3>Linked Journal</h3>
        <ul>${{journalHtml}}</ul>
        <h3>Recent Focus</h3>
        <ul>${{focusHtml}}</ul>
      `;
    }}

    function heatForNode(nodeId) {{
      const item = payload.heatmap.find(entry => entry.node_id === nodeId);
      return item ? item.score : 0;
    }}

    function linkedNodeNames(nodeIds) {{
      if (!nodeIds || !nodeIds.length) return 'linked nodes: none';
      const labels = nodeIds
        .map(nodeId => nodeMap.get(nodeId))
        .filter(Boolean)
        .map(node => node.content)
        .slice(0, 3);
      return `linked nodes: ${{labels.join(', ') || 'none'}}`;
    }}

    function escapeHtml(value) {{
      return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;');
    }}

    function applyVisibilityFilters() {{
      const normalized = searchInput.value.trim().toLowerCase();
      const minimumHeat = Number(heatFilter.value) / 100;
      const showGhosts = ghostToggle.checked;
      const visible = new Set();

      for (const node of payload.nodes) {{
        const isVisible =
          (!normalized ||
            node.content.toLowerCase().includes(normalized) ||
            node.type.toLowerCase().includes(normalized)) &&
          (showGhosts || (!node.is_ghost && !node.is_fossil)) &&
          heatForNode(node.id) >= minimumHeat;
        if (isVisible) {{
          visible.add(node.id);
        }}
      }}

      for (const group of svgNodes) {{
        const isVisible = visible.has(group.dataset.nodeId);
        group.style.opacity = isVisible ? '1' : '0.12';
      }}
      for (const line of graphEdges) {{
        const isVisible = visible.has(line.dataset.sourceId) && visible.has(line.dataset.targetId);
        line.style.opacity = isVisible ? '1' : '0.08';
      }}
      for (const line of trailEdges) {{
        const isVisible = visible.has(line.dataset.sourceId) && visible.has(line.dataset.targetId);
        line.style.opacity = isVisible ? '1' : '0.08';
      }}

      renderNodeButtons(searchInput.value);
    }}

    searchInput.addEventListener('input', applyVisibilityFilters);
    heatFilter.addEventListener('input', applyVisibilityFilters);
    ghostToggle.addEventListener('change', applyVisibilityFilters);
    for (const group of svgNodes) {{
      group.addEventListener('click', () => selectNode(group.dataset.nodeId));
    }}

    function applyFrame(frame) {{
      if (!frame) return;
      for (const group of svgNodes) {{
        const nodeId = group.dataset.nodeId;
        const circle = group.querySelector('circle');
        const label = group.querySelector('text');
        const target = frame.find(item => item.id === nodeId);
        if (!target || !circle || !label) continue;
        circle.setAttribute('cx', (target.x + 450).toFixed(2));
        circle.setAttribute('cy', (target.y + 320).toFixed(2));
        label.setAttribute('x', (target.x + 450).toFixed(2));
        label.setAttribute('y', (target.y + 324).toFixed(2));
      }}
    }}

    function startAnimation() {{
      if (frames.length < 2) return;
      if (animationTimer) {{
        window.clearInterval(animationTimer);
      }}
      animationTimer = window.setInterval(() => {{
        if (!isPlaying) return;
        currentFrameIndex = (currentFrameIndex + 1) % frames.length;
        applyFrame(frames[currentFrameIndex]);
      }}, 850);
    }}

    playToggle.addEventListener('click', () => {{
      isPlaying = !isPlaying;
      playToggle.textContent = isPlaying ? 'Pause' : 'Play';
    }});
    stepFrameButton.addEventListener('click', () => {{
      isPlaying = false;
      playToggle.textContent = 'Play';
      currentFrameIndex = (currentFrameIndex + 1) % Math.max(frames.length, 1);
      applyFrame(frames[currentFrameIndex]);
    }});
    resetFrameButton.addEventListener('click', () => {{
      currentFrameIndex = 0;
      applyFrame(frames[0]);
    }});

    renderNodeButtons();
    renderTrail();
    renderJournal();
    renderHeatmap();
    applyFrame(frames[0]);
    applyVisibilityFilters();
    startAnimation();
    if ('{initial_node_id}') {{
      selectNode('{initial_node_id}');
    }}
  </script>
</body>
</html>"#,
            edge_lines = edge_lines,
            trail_lines = trail_lines,
            node_circles = node_circles,
            payload = graph_payload,
            node_count = nodes.len(),
            edge_count = edges.len(),
            trail_count = recent_trail.len(),
            journal_count = recent_journal.len(),
            initial_node_id = initial_node_id,
        )
    }
}

fn entropy_color(entropy: f32, fallback: &str) -> String {
    if entropy >= 0.92 {
        "#64748b".to_string()
    } else if entropy >= 0.6 {
        "#f59e0b".to_string()
    } else if entropy >= 0.3 {
        "#22d3ee".to_string()
    } else {
        fallback.to_string()
    }
}

fn short_label(content: &str) -> String {
    content.chars().take(8).collect()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn simulate_frames(
    workspace: &SilentNodeWorkspace,
    steps: usize,
    delta_time: f32,
) -> Vec<serde_json::Value> {
    let mut clone = workspace.clone();
    let gravity = crate::gravity::GravityEngine::new();
    let mut frames = Vec::with_capacity(steps.max(1));

    frames.push(frame_snapshot(&clone));
    for _ in 1..steps {
        clone.step_gravity(&gravity, delta_time);
        frames.push(frame_snapshot(&clone));
    }

    frames
}

fn frame_snapshot(workspace: &SilentNodeWorkspace) -> serde_json::Value {
    let export = workspace.graph.export();
    json!(export
        .nodes
        .iter()
        .map(|node| json!({
            "id": node.id.to_string(),
            "x": node.position.x,
            "y": node.position.y,
        }))
        .collect::<Vec<_>>())
}
