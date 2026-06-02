"""
Feature extraction from SQLite for all SilentNode ML models.

Provides:
- Node loading with computed temporal fields
- Focus event aggregation per node
- Numeric feature matrix construction
- Graph-based features via NetworkX
- Temporal features: entropy velocity, focus recency decay
- Text tokenization for TF-IDF
"""

import sqlite3
import json
import numpy as np
from datetime import datetime, timezone
from typing import List, Dict, Optional
from pathlib import Path
import re

VAULTS_JSON = "data/vaults.json"
DB_PATH     = "data/silentnode.sqlite"   # fallback if no vaults.json


def _all_vault_paths() -> List[str]:
    """Return paths for every registered vault, deduplicated."""
    vf = Path(VAULTS_JSON)
    if not vf.exists():
        return [DB_PATH]
    try:
        data = json.loads(vf.read_text())
    except Exception:
        return [DB_PATH]
    seen, paths = set(), []
    for v in data.get("vaults", []):
        p = v.get("path", "")
        if p and p not in seen and Path(p).exists():
            seen.add(p)
            paths.append(p)
    return paths if paths else [DB_PATH]


def current_vault_path() -> str:
    """Return the currently selected vault path, falling back to DB_PATH."""
    vf = Path(VAULTS_JSON)
    if not vf.exists():
        return DB_PATH
    try:
        data = json.loads(vf.read_text())
        current = data.get("current")
        for vault in data.get("vaults", []):
            if vault.get("name") == current and vault.get("path"):
                return vault["path"]
    except Exception:
        pass
    return DB_PATH


def get_db(db_path: str = DB_PATH):
    """Open a SQLite connection with row_factory enabled."""
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn


def now_ts() -> float:
    """Return current UTC time as a Unix timestamp float."""
    return datetime.now(timezone.utc).timestamp()


def parse_ts(s: str) -> float:
    """Parse an ISO-8601 timestamp string (with optional nanoseconds) to Unix float.

    Falls back to now_ts() on any parse error.
    """
    try:
        s = s.replace("Z", "+00:00")
        if "." in s:
            base, frac_tz = s.split(".", 1)
            for sep in ["+", "-"]:
                if sep in frac_tz:
                    idx = frac_tz.rfind(sep)
                    frac = frac_tz[:idx][:6]   # truncate to microseconds
                    tz = frac_tz[idx:]
                    s = f"{base}.{frac}{tz}"
                    break
            else:
                frac = frac_tz[:6]
                s = f"{base}.{frac}"
        return datetime.fromisoformat(s).timestamp()
    except Exception:
        return now_ts()


# ---------------------------------------------------------------------------
# Node loading
# ---------------------------------------------------------------------------

def load_nodes(db_path: str = None) -> List[Dict]:
    """Load nodes from a specific DB or from ALL vaults combined.

    Each node gets a 'vault_path' field so models can track origin.
    Passing db_path explicitly reads only that database, which keeps tests
    and targeted training deterministic. The default None enables cross-vault ML.
    """
    all_nodes: List[Dict] = []
    seen_ids: set = set()
    current_path = current_vault_path()
    paths = [db_path] if db_path else _all_vault_paths()
    for path in paths:
        try:
            rows = _load_nodes_from(path)
            for r in rows:
                key = f"{path}::{r['id']}"
                if key not in seen_ids:
                    seen_ids.add(key)
                    r["vault_path"] = path
                    r["is_current_vault"] = path == current_path
                    all_nodes.append(r)
        except Exception:
            pass
    return all_nodes


def _load_nodes_from(db_path: str) -> List[Dict]:
    """Load all nodes with degree counts and computed temporal fields.

    Added computed fields per row:
        days_since_access  — float, days since last access
        days_since_created — float, days since creation
        connection_count   — int, total in + out degree
    """
    conn = get_db(db_path)
    cur = conn.cursor()
    cur.execute("PRAGMA table_info(node)")
    columns = {row["name"] for row in cur.fetchall()}
    metadata_select = "n.metadata_json" if "metadata_json" in columns else "'{}' AS metadata_json"
    cur.execute(f"""
        SELECT n.id, n.node_type, n.content, {metadata_select}, n.entropy, n.gravity,
               n.velocity, n.access_count, n.is_ghost, n.is_fossil,
               n.is_void, n.created_at, n.accessed_at,
               COUNT(DISTINCT e.target_id)  AS out_deg,
               COUNT(DISTINCT e2.source_id) AS in_deg
        FROM node n
        LEFT JOIN edge e  ON e.source_id = n.id
        LEFT JOIN edge e2 ON e2.target_id = n.id
        GROUP BY n.id
    """)
    rows = [dict(r) for r in cur.fetchall()]
    conn.close()

    now = now_ts()
    for r in rows:
        try:
            metadata = json.loads(r.get("metadata_json") or "{}")
        except Exception:
            metadata = {}
        r["metadata"] = metadata
        nickname = str(metadata.get("nickname") or "").strip()
        r["nickname"] = nickname
        # Type classification should describe the node's own text. Neighbor
        # context is still represented through numeric graph features; adding
        # neighbor words here made new suggestions drift toward the dominant
        # label in the current vault.
        text_parts = [nickname, r["content"]]
        r["model_text"] = "\n".join(part for part in text_parts if part)
        r["days_since_access"]  = (now - parse_ts(r["accessed_at"])) / 86400.0
        r["days_since_created"] = (now - parse_ts(r["created_at"])) / 86400.0
        r["connection_count"]   = (r["out_deg"] or 0) + (r["in_deg"] or 0)
    return rows


def load_ml_feedback(db_path: str = None) -> List[Dict]:
    """Load explicit user corrections for node-type classification.

    Feedback rows are treated as high-weight labeled examples by the classifier.
    Missing table/schema is allowed so old vaults and unit tests keep working.
    """
    rows: List[Dict] = []
    current_path = current_vault_path()
    paths = [db_path] if db_path else _all_vault_paths()
    for path in paths:
        try:
            conn = get_db(path)
            cur = conn.cursor()
            cur.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='ml_feedback'")
            if not cur.fetchone():
                conn.close()
                continue
            cur.execute("""
                SELECT node_id, content, nickname, predicted_type, selected_type,
                       confidence, source, created_at
                FROM ml_feedback
            """)
            for row in cur.fetchall():
                item = dict(row)
                item["vault_path"] = path
                item["is_current_vault"] = path == current_path
                rows.append(item)
            conn.close()
        except Exception:
            pass
    return rows


def load_focus_events(db_path: str = None) -> List[Dict]:
    """Load focus events from a specific DB or all vaults, sorted by timestamp."""
    all_events: List[Dict] = []
    paths = [db_path] if db_path else _all_vault_paths()
    for path in paths:
        try:
            conn = get_db(path)
            cur = conn.cursor()
            cur.execute("SELECT node_id, timestamp, duration_seconds, depth FROM focus_event")
            rows = [dict(r) for r in cur.fetchall()]
            conn.close()
            for r in rows:
                r["ts_float"]   = parse_ts(r["timestamp"])
                r["vault_path"] = path
            all_events.extend(rows)
        except Exception:
            pass
    all_events.sort(key=lambda e: e["ts_float"])
    return all_events


def load_edges(db_path: str = None) -> List[Dict]:
    """Load edges from a specific DB or all vaults combined."""
    all_edges: List[Dict] = []
    paths = [db_path] if db_path else _all_vault_paths()
    for path in paths:
        try:
            conn = get_db(path)
            cur = conn.cursor()
            cur.execute("SELECT source_id, target_id, edge_type, weight FROM edge")
            rows = [dict(r) for r in cur.fetchall()]
            conn.close()
            for r in rows:
                r["vault_path"] = path
            all_edges.extend(rows)
        except Exception:
            pass
    return all_edges


# ---------------------------------------------------------------------------
# Per-node focus statistics
# ---------------------------------------------------------------------------

_DEPTH_WEIGHTS = {
    "glance": 0.25, "Glance": 0.25,
    "read": 0.50,   "Read": 0.50,
    "edit": 0.75, "think": 0.75,
    "deep_work": 1.0, "DeepWork": 1.0,
}


def node_focus_stats(nodes: List[Dict], events: List[Dict]) -> Dict[str, Dict]:
    """Compute per-node focus statistics.

    Returns a mapping node_id -> dict with:
        focus_count      — number of focus events
        total_seconds    — total focused time in seconds
        avg_depth        — average depth weight [0, 1]
        last_focus_days  — days since most recent focus event
    """
    stats: Dict[str, Dict] = {}

    for n in nodes:
        nid = n["id"]
        fevts = [e for e in events if e["node_id"] == nid]
        stats[nid] = {
            "focus_count":    len(fevts),
            "total_seconds":  sum(e["duration_seconds"] for e in fevts),
            "avg_depth":      (float(np.mean([_DEPTH_WEIGHTS.get(e["depth"], 0.5)
                                              for e in fevts]))
                               if fevts else 0.0),
            "last_focus_days": ((now_ts() - max((e["ts_float"] for e in fevts),
                                                default=now_ts())) / 86400.0),
        }
    return stats


# ---------------------------------------------------------------------------
# Numeric feature matrix
# ---------------------------------------------------------------------------

# Feature names in the same order as build_numeric_features columns
NUMERIC_FEATURE_NAMES = [
    "entropy",
    "gravity_norm",
    "velocity_norm",
    "access_count_norm",
    "days_since_access_norm",
    "days_since_created_norm",
    "connection_count_norm",
    "focus_count_norm",
    "total_seconds_norm",
    "avg_depth",
    "last_focus_days_norm",
    "is_ghost",
    "is_fossil",
    "is_void",
]


def build_numeric_features(nodes: List[Dict],
                           focus_stats: Dict[str, Dict]) -> np.ndarray:
    """Build a float32 feature matrix of shape (N, 14) for the given nodes.

    All features are normalized to [0, 1] where applicable.
    Column order is defined by NUMERIC_FEATURE_NAMES.
    """
    rows = []
    for n in nodes:
        fs = focus_stats.get(n["id"], {})
        rows.append([
            float(n["entropy"]),
            min(float(n["gravity"])      / 10.0,   1.0),
            min(float(n["velocity"])     * 100.0,  1.0),
            min(float(n["access_count"]) / 20.0,   1.0),
            min(float(n["days_since_access"])  / 30.0,  1.0),
            min(float(n["days_since_created"]) / 180.0, 1.0),
            min(float(n["connection_count"])   / 15.0,  1.0),
            min(float(fs.get("focus_count",   0)) / 10.0,   1.0),
            min(float(fs.get("total_seconds", 0)) / 3600.0, 1.0),
            float(fs.get("avg_depth", 0.0)),
            min(float(fs.get("last_focus_days", 30.0)) / 30.0, 1.0),
            float(n.get("is_ghost",  0)),
            float(n.get("is_fossil", 0)),
            float(n.get("is_void",   0)),
        ])
    return np.array(rows, dtype=np.float32)


# ---------------------------------------------------------------------------
# Graph-based features via NetworkX
# ---------------------------------------------------------------------------

def build_graph_features(nodes: List[Dict],
                         db_path: str = None) -> Dict[str, Dict]:
    """Compute graph-centrality metrics for each node using NetworkX.

    Returns a mapping node_id -> dict with:
        degree_centrality      — fraction of nodes this node is connected to
        betweenness_centrality — fraction of shortest paths passing through
        clustering_coeff       — local clustering coefficient
        pagerank               — PageRank score

    For isolated nodes or very small graphs (< 2 edges) all values are 0.
    """
    try:
        import networkx as nx
    except ImportError:
        # Return zeros if networkx is unavailable
        return {n["id"]: {"degree_centrality": 0.0,
                          "betweenness_centrality": 0.0,
                          "clustering_coeff": 0.0,
                          "pagerank": 0.0} for n in nodes}

    edges = load_edges(db_path)
    node_ids = {n["id"] for n in nodes}

    G = nx.DiGraph()
    G.add_nodes_from(node_ids)
    for e in edges:
        if e["source_id"] in node_ids and e["target_id"] in node_ids:
            G.add_edge(e["source_id"], e["target_id"],
                       weight=float(e["weight"] or 1.0))

    # Compute metrics; fall back gracefully on errors
    try:
        deg_cent = nx.degree_centrality(G)
    except Exception:
        deg_cent = {nid: 0.0 for nid in node_ids}

    try:
        between_cent = nx.betweenness_centrality(G, normalized=True)
    except Exception:
        between_cent = {nid: 0.0 for nid in node_ids}

    try:
        undirected = G.to_undirected()
        clust = nx.clustering(undirected)
    except Exception:
        clust = {nid: 0.0 for nid in node_ids}

    try:
        pr = nx.pagerank(G, alpha=0.85, max_iter=200)
    except Exception:
        n_nodes = len(node_ids) or 1
        pr = {nid: 1.0 / n_nodes for nid in node_ids}

    result: Dict[str, Dict] = {}
    for nid in node_ids:
        result[nid] = {
            "degree_centrality":      round(deg_cent.get(nid, 0.0), 6),
            "betweenness_centrality": round(between_cent.get(nid, 0.0), 6),
            "clustering_coeff":       round(clust.get(nid, 0.0), 6),
            "pagerank":               round(pr.get(nid, 0.0), 6),
        }
    return result


def build_graph_feature_matrix(nodes: List[Dict],
                               graph_features: Dict[str, Dict]) -> np.ndarray:
    """Convert graph_features dict to a float32 matrix aligned with nodes list.

    Shape: (N, 4) — [degree_cent, betweenness_cent, clustering_coeff, pagerank]
    """
    rows = []
    for n in nodes:
        gf = graph_features.get(n["id"], {})
        rows.append([
            gf.get("degree_centrality",      0.0),
            gf.get("betweenness_centrality",  0.0),
            gf.get("clustering_coeff",        0.0),
            gf.get("pagerank",               0.0),
        ])
    return np.array(rows, dtype=np.float32)


# ---------------------------------------------------------------------------
# Temporal features
# ---------------------------------------------------------------------------

def build_temporal_features(nodes: List[Dict],
                             focus_stats: Dict[str, Dict]) -> np.ndarray:
    """Build temporal feature matrix of shape (N, 4).

    Columns:
        entropy_velocity    — estimated entropy rate of change (per day)
        focus_recency_decay — exponential decay of last focus event
        age_log_norm        — log-normalized node age in days
        access_rate         — accesses per day since creation

    These capture dynamic behavior beyond the static numeric features.
    """
    rows = []
    for n in nodes:
        fs = focus_stats.get(n["id"], {})
        entropy = float(n["entropy"])
        days_access = max(float(n["days_since_access"]), 0.1)
        days_created = max(float(n["days_since_created"]), 0.1)
        access_count = float(n["access_count"])

        # Entropy velocity: nodes with high entropy and long inactivity
        # are decaying quickly; model as entropy / days_since_access
        entropy_velocity = min(entropy / days_access, 1.0)

        # Focus recency decay: exponential decay with half-life = 7 days
        last_focus_days = float(fs.get("last_focus_days", 30.0))
        focus_recency_decay = float(np.exp(-0.693 * last_focus_days / 7.0))

        # Log-normalized age
        age_log_norm = min(float(np.log1p(days_created)) / float(np.log1p(365.0)), 1.0)

        # Access rate per day (bounded)
        access_rate = min(access_count / days_created, 1.0)

        rows.append([
            entropy_velocity,
            focus_recency_decay,
            age_log_norm,
            access_rate,
        ])
    return np.array(rows, dtype=np.float32)


# ---------------------------------------------------------------------------
# Text tokenizer
# ---------------------------------------------------------------------------

def build_text_tokens(content: str) -> List[str]:
    """Unicode-aware tokenizer producing unigrams, aliases, and bigrams.

    SilentNode is often used with mixed English/Azerbaijani/Turkish shorthand.
    Keeping Unicode letters and adding semantic aliases lets the local model
    learn from the user's real notes instead of losing meaning at tokenization.
    """
    aliases = {
        "fikir": "idea",
        "dusunce": "idea",
        "düşüncə": "idea",
        "düşünce": "idea",
        "ideya": "idea",
        "qeyd": "note",
        "not": "note",
        "layihe": "project",
        "layihə": "project",
        "proyekt": "project",
        "imtahan": "exam",
        "imtahani": "exam",
        "imtahanı": "exam",
        "hazirliq": "prep",
        "hazırlıq": "prep",
        "hazirlasmaq": "prep",
        "hazırlaşmaq": "prep",
        "magister": "magistr",
        "magistratura": "magistr",
        "qebul": "admission",
        "qəbul": "admission",
        "informatika": "informatics",
        "testler": "tests",
        "testlər": "tests",
        "namaz": "prayer",
        "dua": "prayer",
        "quran": "quran",
        "qurani": "quran",
        "quranı": "quran",
        "teheccud": "tahajjud",
        "təhəccüd": "tahajjud",
        "ingilis": "english",
        "ingilisce": "english",
        "ingiliscə": "english",
        "listening": "listening",
        "dinleme": "listening",
        "dinləmə": "listening",
        "gomruk": "customs",
        "gömrük": "customs",
        "is": "work",
        "iş": "work",
        "gundelik": "daily",
        "gündəlik": "daily",
        "her": "daily",
        "hər": "daily",
        "gun": "daily",
        "gün": "daily",
        "rutina": "routine",
        "rutin": "routine",
        "kitab": "book",
        "video": "video",
        "meqale": "article",
        "məqalə": "article",
        "elaqe": "contact",
        "əlaqə": "contact",
        "adam": "person",
        "sexs": "person",
        "şəxs": "person",
    }

    content = content.lower().replace("ə", "ə")
    raw_tokens = re.findall(r"[^\W_]+", content, flags=re.UNICODE)
    tokens = [t for t in raw_tokens if t]

    expanded = []
    for token in tokens:
        expanded.append(token)
        alias = aliases.get(token)
        if alias and alias != token:
            expanded.append(alias)

    bigrams = [f"{expanded[i]}_{expanded[i + 1]}"
               for i in range(len(expanded) - 1)]
    return expanded + bigrams
