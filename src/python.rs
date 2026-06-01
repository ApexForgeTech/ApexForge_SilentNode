/// PyO3 Python bindings for SilentNode.
///
/// Build: `maturin develop --features python`
/// Import: `import silentnode_core`
///
/// Only compiled when `--features python` is passed.
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::{EdgeType, FocusDepth};
use crate::materialize::MaterializationEngine;
use crate::workspace::SilentNodeWorkspace;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> PyResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| PyValueError::new_err(format!("invalid UUID: {e}")))
}

fn parse_edge_type(s: &str) -> EdgeType {
    match s {
        "resonance" => EdgeType::Resonance,
        "causal" => EdgeType::Causal,
        "temporal" => EdgeType::Temporal,
        _ => EdgeType::Connection,
    }
}

fn parse_focus_depth(s: &str) -> FocusDepth {
    match s {
        "read" => FocusDepth::Read,
        "edit" | "think" => FocusDepth::Edit,
        "deep_work" => FocusDepth::DeepWork,
        _ => FocusDepth::Glance,
    }
}

/// Serialize workspace to the JSON format expected by silentnode_py modules.
fn workspace_to_json(ws: &SilentNodeWorkspace) -> String {
    let nodes: Vec<Value> = ws
        .graph
        .nodes()
        .map(|n| {
            json!({
                "id": n.id.to_string(),
                "content": n.content,
                "entropy": n.entropy,
                "gravity": n.gravity,
                "velocity": n.velocity,
                "is_ghost": n.is_ghost,
                "is_fossil": n.is_fossil,
                "is_void": n.is_void,
                "created_at": n.created_at.to_rfc3339(),
                "accessed_at": n.accessed_at.to_rfc3339(),
                "civilization_id": n.civilization_id.map(|id| id.to_string()),
            })
        })
        .collect();

    let edges: Vec<Value> = ws
        .graph
        .edges()
        .map(|e| {
            json!({
                "source_id": e.source_id.to_string(),
                "target_id": e.target_id.to_string(),
                "weight": e.weight,
                "edge_type": format!("{:?}", e.edge_type).to_lowercase(),
            })
        })
        .collect();

    let focus_events: Vec<Value> = ws
        .focus
        .events()
        .iter()
        .map(|e| {
            json!({
                "node_id": e.node_id.to_string(),
                "timestamp": e.timestamp.to_rfc3339(),
                "duration_seconds": e.duration_seconds,
                "depth": match e.depth {
                    FocusDepth::Glance   => "glance",
                    FocusDepth::Read     => "read",
                    FocusDepth::Edit     => "edit",
                    FocusDepth::DeepWork => "deep_work",
                },
            })
        })
        .collect();

    let journal_entries: Vec<Value> = ws
        .journal
        .entries()
        .iter()
        .map(|j| {
            json!({
                "id": j.id.to_string(),
                "content": j.content,
                "timestamp": j.timestamp.to_rfc3339(),
                "linked_nodes": j.linked_nodes.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();

    serde_json::to_string(&json!({
        "nodes": nodes,
        "edges": edges,
        "focus_events": focus_events,
        "journal_entries": journal_entries,
    }))
    .unwrap_or_default()
}

/// Add silentnode_py package directory to Python sys.path.
/// Exposed as pub so main.rs can call it directly.
pub fn ensure_path(py: Python<'_>) -> PyResult<()> {
    ensure_package_path(py)
}

fn ensure_package_path(py: Python<'_>) -> PyResult<()> {
    let sys = py.import_bound("sys")?;
    let path = sys.getattr("path")?;
    let path_list = path.downcast::<PyList>()?;

    // Try paths relative to cwd and executable
    let candidates = [
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
            .unwrap_or_default(),
    ];

    for dir in &candidates {
        if !dir.is_empty() {
            // Check if silentnode_py exists in this dir
            let pkg = std::path::Path::new(dir).join("silentnode_py");
            if pkg.exists() {
                // Add the parent dir (not the package itself)
                let s = pyo3::types::PyString::new_bound(py, dir);
                if path_list.contains(s.as_any())? == false {
                    path_list.insert(0, s)?;
                }
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Call a Python function in a silentnode_py module, passing workspace JSON.
fn call_py_analysis(
    py: Python<'_>,
    module_path: &str,
    fn_name: &str,
    workspace_json: &str,
) -> PyResult<String> {
    ensure_package_path(py)?;
    let module = py.import_bound(module_path)?;
    let result = module.call_method1(fn_name, (workspace_json,))?;
    let json_str: String = result.extract()?;
    Ok(json_str)
}

/// Call a Python function with workspace JSON + one extra string argument.
fn call_py_analysis2(
    py: Python<'_>,
    module_path: &str,
    fn_name: &str,
    workspace_json: &str,
    arg2: &str,
) -> PyResult<String> {
    ensure_package_path(py)?;
    let module = py.import_bound(module_path)?;
    let result = module.call_method1(fn_name, (workspace_json, arg2))?;
    let json_str: String = result.extract()?;
    Ok(json_str)
}

// ── PySilentNode ──────────────────────────────────────────────────────────────

/// Python-facing wrapper around SilentNodeWorkspace.
///
/// Example
/// -------
/// ```python
/// import silentnode_core
/// ws = silentnode_core.SilentNode()
/// nid = ws.add_node("Attention is all you need")
/// ws.focus(nid, 120.0, "deep_work")
/// print(ws.cognitive_season())
/// ```
#[pyclass]
pub struct PySilentNode {
    inner: SilentNodeWorkspace,
}

#[pymethods]
impl PySilentNode {
    /// Create an empty in-memory workspace.
    #[new]
    fn new() -> Self {
        Self {
            inner: SilentNodeWorkspace::new(),
        }
    }

    /// Add a thought node. Returns the UUID string of the new node.
    fn add_node(&mut self, text: &str) -> PyResult<String> {
        let engine = MaterializationEngine::new();
        let result = self
            .inner
            .materialize_thought(&engine, text)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(result.node_id.to_string())
    }

    /// Connect two nodes. edge_type: "connection"|"resonance"|"conflict"|"causal"|"reference"
    fn connect(
        &mut self,
        source_id: &str,
        target_id: &str,
        weight: f32,
        edge_type: &str,
    ) -> PyResult<()> {
        let src = parse_uuid(source_id)?;
        let dst = parse_uuid(target_id)?;
        self.inner
            .connect_nodes(src, dst, parse_edge_type(edge_type), weight)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Record a focus session. depth: "glance"|"read"|"edit"|"deep_work"
    fn focus(&mut self, node_id: &str, duration_seconds: f32, depth: &str) -> PyResult<()> {
        let id = parse_uuid(node_id)?;
        self.inner
            .record_focus(id, duration_seconds, parse_focus_depth(depth))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Add a journal entry. Returns the UUID of the entry.
    #[pyo3(signature = (text, season=None))]
    fn journal(&mut self, text: &str, season: Option<&str>) -> String {
        let entry = self
            .inner
            .add_journal_entry(text, season.map(|s| s.to_string()));
        entry.id.to_string()
    }

    /// List all nodes as a list of dicts: {id, label, entropy, gravity, velocity}.
    fn list_nodes(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .graph
            .nodes()
            .map(|n| {
                let d = PyDict::new_bound(py);
                d.set_item("id", n.id.to_string())?;
                d.set_item("label", n.content.clone())?;
                d.set_item("entropy", n.entropy)?;
                d.set_item("gravity", n.gravity)?;
                d.set_item("velocity", n.velocity)?;
                Ok(d.into_any().unbind())
            })
            .collect()
    }

    /// Return a JSON string of the full workspace (nodes, edges, focus_events, journal_entries).
    fn to_json(&self) -> String {
        workspace_to_json(&self.inner)
    }

    /// Return total node count.
    fn node_count(&self) -> usize {
        self.inner.graph.node_count()
    }

    /// Return total edge count.
    fn edge_count(&self) -> usize {
        self.inner.graph.edge_count()
    }

    // ── Phase 6 Deep Analysis ─────────────────────────────────────────────────

    /// Detect behavioral rituals using DBSCAN session clustering.
    /// Returns JSON list of rituals.
    fn deep_rituals(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.patterns.ritual",
            "detect_rituals",
            &ws_json,
        )
    }

    /// Detect cognitive season with full statistical analysis.
    /// Returns JSON {season, confidence, signals, transition_prediction, aura_colors}.
    fn deep_season(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.patterns.seasons",
            "detect_season",
            &ws_json,
        )
    }

    /// Generate oracle signals — predictive behavioral hints.
    /// Returns JSON list of OracleSignal objects.
    fn deep_oracle(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.patterns.oracle",
            "generate_signals",
            &ws_json,
        )
    }

    /// Generate full cognitive self-portrait.
    /// Returns JSON {priority_gap, creative_patterns, blind_spots, obsessions, evolution}.
    fn deep_mirror(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.patterns.mirror",
            "generate_portrait",
            &ws_json,
        )
    }

    /// Compute thought heatmap with exponential decay.
    /// Returns JSON {energies, zones, top_hot, obsessive_loops, neglected_regions}.
    #[pyo3(signature = (window_days=30))]
    fn deep_heatmap(&self, py: Python<'_>, window_days: i64) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.patterns.heatmap")?;
        let result = module.call_method1("compute_heatmap", (ws_json.as_str(), window_days))?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    /// Detect silent contracts — invisible obligations from behavioral patterns.
    /// Returns JSON list of SilentContract objects.
    fn deep_contracts(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.patterns.contracts",
            "detect_contracts",
            &ws_json,
        )
    }

    /// Find semantic resonances using sklearn TF-IDF cosine similarity.
    /// Returns JSON {pairs, cluster_resonances, implied_absences}.
    #[pyo3(signature = (min_similarity=0.35))]
    fn deep_resonances(&self, py: Python<'_>, min_similarity: f64) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.patterns.resonance")?;
        let result = module.call_method1("find_resonances", (ws_json.as_str(), min_similarity))?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    // ── Phase 7 Deep Analysis ─────────────────────────────────────────────────

    /// Detect civilizations using Louvain community detection.
    /// Returns JSON {civilizations, events, modularity, algorithm}.
    #[pyo3(signature = (resolution=1.2))]
    fn louvain_civilizations(&self, py: Python<'_>, resolution: f64) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.clusters.civilization")?;
        let result = module.call_method1("detect_civilizations", (ws_json.as_str(), resolution))?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    /// Detect digital shadows — ideas in permanent becoming.
    /// Returns JSON list of DigitalShadow objects.
    fn deep_shadows(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.clusters.shadow",
            "detect_shadows",
            &ws_json,
        )
    }

    // ── Phase 9 Audio Analysis ────────────────────────────────────────────────

    /// Map current workspace state to audio parameters using the Python generator.
    /// Returns JSON {atmosphere, secondary, blend, params, derived_from, description}.
    fn deep_audio_params(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        call_py_analysis(
            py,
            "silentnode_py.audio.generator",
            "map_workspace_to_audio",
            &ws_json,
        )
    }

    // ── Phase 8 Python Analysis ───────────────────────────────────────────────

    /// Classify a portal activity and run ingestion proposals against the workspace.
    /// activity_json: serialised PortalActivity (at minimum: {"kind":"view","target":"..."})
    /// Returns JSON list of IngestionProposal objects.
    fn ingest_activity(&self, py: Python<'_>, activity_json: &str) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.ingestion.engine")?;
        let result = module.call_method1("ingest_activity", (activity_json, ws_json.as_str()))?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    /// Extract topics from arbitrary text (URL, title, content).
    /// Returns JSON {"topics": [...]}.
    fn extract_topics(&self, py: Python<'_>, text: &str) -> PyResult<String> {
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.ingestion.engine")?;
        let engine = module.getattr("IngestionEngine")?.call0()?;
        let result = engine.call_method1("extract_topics", (text,))?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    // ── Phase 10 Living Signature ─────────────────────────────────────────────

    /// Compute the Living Signature parameters from the current workspace.
    /// Returns JSON SignatureParams.
    fn living_signature(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.identity.signature")?;
        let result = module.call_method1("compute_signature", (ws_json.as_str(),))?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    /// Render the Living Signature as an SVG string.
    #[pyo3(signature = (size=200))]
    fn signature_svg(&self, py: Python<'_>, size: u32) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.identity.signature")?;
        let result = module.call_method1("render_signature_svg", (ws_json.as_str(), size))?;
        let svg: String = result.extract()?;
        Ok(svg)
    }

    /// Render the Living Signature as ASCII art (for TUI / terminal display).
    fn signature_ascii(&self, py: Python<'_>) -> PyResult<String> {
        let ws_json = workspace_to_json(&self.inner);
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.identity.signature")?;
        let result = module.call_method1("render_signature_ascii", (ws_json.as_str(),))?;
        let art: String = result.extract()?;
        Ok(art)
    }

    /// List all available atmosphere kinds with descriptions.
    fn audio_atmospheres(&self, py: Python<'_>) -> PyResult<String> {
        ensure_package_path(py)?;
        let module = py.import_bound("silentnode_py.audio.generator")?;
        let mapper = module.getattr("AudioStateMapper")?.call0()?;
        let result = mapper.call_method0("list_atmospheres")?;
        let json_str: String = result.extract()?;
        Ok(json_str)
    }

    // ── Legacy compatibility ──────────────────────────────────────────────────

    /// Return the current cognitive season (fast Rust version): "Spring"|"Summer"|"Autumn"|"Winter".
    fn cognitive_season(&self) -> String {
        use crate::systems::{CognitiveSeason, CognitiveSeasonDetector};
        let nodes: Vec<_> = self.inner.graph.nodes().collect();
        let events = self.inner.focus.events();
        let report =
            CognitiveSeasonDetector::new().detect_season(&nodes, events, &[], chrono::Utc::now());
        match report.season {
            CognitiveSeason::Spring => "Spring".to_string(),
            CognitiveSeason::Summer => "Summer".to_string(),
            CognitiveSeason::Autumn => "Autumn".to_string(),
            CognitiveSeason::Winter => "Winter".to_string(),
        }
    }

    /// Return civilization list (fast Rust version): [{id, member_count, density}].
    fn civilizations(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        self.inner
            .detect_civilizations()
            .into_iter()
            .map(|c| {
                let d = PyDict::new_bound(py);
                d.set_item("id", c.id.to_string())?;
                d.set_item("member_count", c.member_nodes.len())?;
                d.set_item("density", c.internal_density)?;
                d.set_item("age_days", c.age_days)?;
                Ok(d.into_any().unbind())
            })
            .collect()
    }

    /// Return resonance pairs (fast Rust TF-IDF version): [{node_a, node_b, similarity}].
    fn resonances(&self, py: Python<'_>, threshold: f32) -> PyResult<Vec<PyObject>> {
        use crate::systems::ResonanceChamberEngine;
        let nodes: Vec<_> = self.inner.graph.nodes().collect();
        let mut engine = ResonanceChamberEngine::new();
        engine.min_similarity = threshold;
        engine
            .find_resonances(&nodes)
            .into_iter()
            .map(|p| {
                let d = PyDict::new_bound(py);
                d.set_item("node_a", p.node_a.to_string())?;
                d.set_item("node_b", p.node_b.to_string())?;
                d.set_item("similarity", p.similarity)?;
                Ok(d.into_any().unbind())
            })
            .collect()
    }

    /// Get a single node by UUID as a dict, or None.
    fn get_node(&self, py: Python<'_>, node_id: &str) -> PyResult<Option<PyObject>> {
        let id = parse_uuid(node_id)?;
        match self.inner.get_node(id) {
            None => Ok(None),
            Some(n) => {
                let d = PyDict::new_bound(py);
                d.set_item("id", n.id.to_string())?;
                d.set_item("label", n.content.clone())?;
                d.set_item("entropy", n.entropy)?;
                d.set_item("gravity", n.gravity)?;
                d.set_item("velocity", n.velocity)?;
                d.set_item("is_void", n.is_void)?;
                Ok(Some(d.into_any().unbind()))
            }
        }
    }
}

// ── Module registration ───────────────────────────────────────────────────────

/// Python extension entry point — only compiled when building the cdylib
/// with `--features python-ext`. When embedding Python into the binary
/// (`--features python`), this export is not needed.
#[cfg(feature = "python-ext")]
#[pymodule]
pub fn silentnode_core(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<PySilentNode>()?;
    Ok(())
}
