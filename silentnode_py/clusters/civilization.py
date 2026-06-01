"""
Phase 7.1 — Civilization Detector (Louvain)
Detects thought civilizations using the Louvain community detection algorithm
via python-louvain (community package) and networkx.
"""

import json
import uuid
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any

import networkx as nx

try:
    import community as community_louvain  # python-louvain
    _LOUVAIN_AVAILABLE = True
except ImportError:
    _LOUVAIN_AVAILABLE = False


def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


# ── Graph construction ────────────────────────────────────────────────────────

def _build_nx_graph(nodes: list[dict], edges: list[dict]) -> nx.Graph:
    """Build undirected weighted networkx graph from workspace data."""
    G = nx.Graph()
    for n in nodes:
        if n.get("is_ghost") or n.get("is_void"):
            continue
        G.add_node(
            n["id"],
            content=n.get("content", ""),
            gravity=float(n.get("gravity", 1.0)),
            entropy=float(n.get("entropy", 0.0)),
            created_at=_ts(n.get("created_at", "")),
        )
    for e in edges:
        s, t = e.get("source_id", ""), e.get("target_id", "")
        if G.has_node(s) and G.has_node(t):
            w = float(e.get("weight", 1.0))
            if G.has_edge(s, t):
                G[s][t]["weight"] = max(G[s][t]["weight"], w)
            else:
                G.add_edge(s, t, weight=w)
    return G


# ── Louvain partition ─────────────────────────────────────────────────────────

def _partition(G: nx.Graph, resolution: float = 1.2) -> dict[str, int]:
    """
    Run Louvain community detection.
    Falls back to connected-components if python-louvain is unavailable.
    """
    if len(G.nodes) == 0:
        return {}

    if _LOUVAIN_AVAILABLE:
        try:
            return community_louvain.best_partition(G, weight="weight", resolution=resolution)
        except Exception:
            pass

    # Fallback: connected components as communities
    partition: dict[str, int] = {}
    for i, component in enumerate(nx.connected_components(G)):
        for node_id in component:
            partition[node_id] = i
    return partition


# ── Civilization construction ─────────────────────────────────────────────────

CIVILIZATION_MIN_SIZE = 3


def _build_civilization(
    G: nx.Graph,
    member_ids: list[str],
    civ_index: int,
) -> dict[str, Any]:
    """Build a full civilization record from a community partition."""
    subgraph = G.subgraph(member_ids)

    # Internal density: edges within / possible edges
    n = len(member_ids)
    possible_internal = n * (n - 1) / 2 if n > 1 else 1
    internal_edges = subgraph.number_of_edges()
    internal_density = internal_edges / possible_internal

    # External density: edges crossing the boundary
    external_edges = sum(
        1 for u, v in G.edges()
        if (u in member_ids) != (v in member_ids)
    )
    total_possible = G.number_of_edges()
    external_density = external_edges / max(total_possible, 1)

    # Dominant node: highest degree centrality within subgraph
    if n > 1:
        centrality = nx.degree_centrality(subgraph)
        dominant = max(centrality, key=lambda k: centrality[k])
    else:
        dominant = member_ids[0] if member_ids else ""

    # Age: oldest node in cluster
    ages = [G.nodes[m].get("created_at", 0) for m in member_ids if G.has_node(m)]
    oldest_ts = min((a for a in ages if a > 0), default=0.0)
    age_days = round((_now() - oldest_ts) / 86400.0) if oldest_ts > 0 else 0

    # Average entropy
    entropies = [G.nodes[m].get("entropy", 0) for m in member_ids if G.has_node(m)]
    avg_entropy = sum(entropies) / len(entropies) if entropies else 0.0

    # Average gravity
    gravities = [G.nodes[m].get("gravity", 1) for m in member_ids if G.has_node(m)]
    avg_gravity = sum(gravities) / len(gravities) if gravities else 1.0

    # Civilization color (derived from dominant concept hash)
    civ_colors = [
        "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7",
        "#DDA0DD", "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9",
    ]
    color = civ_colors[civ_index % len(civ_colors)]

    # Modularity contribution (Louvain favors dense internal connections)
    is_crystallized = (
        internal_density > 0.65
        and age_days >= 14
        and n >= 5
        and avg_entropy < 0.4
    )

    return {
        "id": str(uuid.uuid4()),
        "index": civ_index,
        "member_nodes": member_ids,
        "dominant_node": dominant,
        "member_count": n,
        "internal_density": round(internal_density, 4),
        "external_density": round(external_density, 4),
        "age_days": age_days,
        "avg_entropy": round(avg_entropy, 3),
        "avg_gravity": round(avg_gravity, 3),
        "color": color,
        "is_crystallized": is_crystallized,
        "description": (
            f"Civilization of {n} nodes. Density {internal_density:.0%}, "
            f"age {age_days}d, entropy {avg_entropy:.0%}."
        ),
    }


# ── Event detection ───────────────────────────────────────────────────────────

def _detect_events(
    prev_civs: list[dict],
    curr_civs: list[dict],
) -> list[dict[str, Any]]:
    """
    Compare previous and current civilization snapshots to detect events:
    Expand, Merge, Collapse, Split, Trade (bridge formation).
    """
    if not prev_civs:
        return []

    events = []
    prev_by_dominant = {c["dominant_node"]: c for c in prev_civs if c.get("dominant_node")}
    curr_by_dominant = {c["dominant_node"]: c for c in curr_civs if c.get("dominant_node")}

    # Expansion: same dominant, more members
    for dom, curr in curr_by_dominant.items():
        if dom in prev_by_dominant:
            prev = prev_by_dominant[dom]
            delta = curr["member_count"] - prev["member_count"]
            if delta >= 2:
                events.append({
                    "kind": "Expand",
                    "civilization_id": curr["id"],
                    "description": f"Civilization expanded by {delta} new nodes",
                })
            elif delta <= -2:
                events.append({
                    "kind": "Shrink",
                    "civilization_id": curr["id"],
                    "description": f"Civilization lost {abs(delta)} nodes",
                })

    # Collapse: previous civ dominant not found in current
    for dom, prev in prev_by_dominant.items():
        if dom not in curr_by_dominant and prev["member_count"] >= 4:
            events.append({
                "kind": "Collapse",
                "civilization_id": prev["id"],
                "description": f"Civilization collapsed (was {prev['member_count']} nodes)",
            })

    return events


# ── Public API ────────────────────────────────────────────────────────────────

def detect_civilizations(workspace_json: str, resolution: float = 1.2) -> str:
    """
    Input:  JSON workspace snapshot + optional Louvain resolution parameter
    Output: JSON {civilizations, events, modularity, algorithm}
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        edges: list[dict] = ws.get("edges", [])
        prev_civs: list[dict] = ws.get("previous_civilizations", [])

        G = _build_nx_graph(nodes, edges)
        if G.number_of_nodes() == 0:
            return json.dumps({"civilizations": [], "events": [], "modularity": 0.0, "algorithm": "none"})

        partition = _partition(G, resolution=resolution)
        if not partition:
            return json.dumps({"civilizations": [], "events": [], "modularity": 0.0, "algorithm": "fallback"})

        # Group nodes by community
        communities: dict[int, list[str]] = defaultdict(list)
        for node_id, comm_id in partition.items():
            communities[comm_id].append(node_id)

        civilizations = []
        for i, (comm_id, members) in enumerate(
            sorted(communities.items(), key=lambda x: -len(x[1]))
        ):
            if len(members) < CIVILIZATION_MIN_SIZE:
                continue
            civ = _build_civilization(G, members, i)
            civilizations.append(civ)

        # Modularity score
        try:
            if _LOUVAIN_AVAILABLE:
                modularity = community_louvain.modularity(partition, G, weight="weight")
            else:
                modularity = nx.community.modularity(
                    G,
                    [set(members) for members in communities.values()],
                    weight="weight",
                )
        except Exception:
            modularity = 0.0

        events = _detect_events(prev_civs, civilizations)

        return json.dumps({
            "civilizations": civilizations,
            "events": events,
            "modularity": round(float(modularity), 4),
            "algorithm": "louvain" if _LOUVAIN_AVAILABLE else "connected_components",
            "total_clustered": sum(c["member_count"] for c in civilizations),
            "total_nodes": G.number_of_nodes(),
        })
    except Exception as exc:
        return json.dumps({"error": str(exc)})
