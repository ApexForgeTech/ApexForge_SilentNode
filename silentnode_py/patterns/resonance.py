"""
Phase 6.7 — Resonance Chamber Engine
Finds semantically similar nodes from different clusters using sklearn TF-IDF
and cosine similarity. Fully local — no internet, no model downloads.
"""

import json
from collections import defaultdict
from typing import Any

import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity


# ── TF-IDF resonance ──────────────────────────────────────────────────────────

def _build_tfidf_matrix(nodes: list[dict]):
    """Build TF-IDF matrix from node content. Returns (matrix, vectorizer)."""
    contents = [n.get("content", "") for n in nodes]
    # Use character n-grams (1-3) in addition to word n-grams for better matching
    vec = TfidfVectorizer(
        analyzer="word",
        ngram_range=(1, 2),
        min_df=1,
        max_features=4096,
        stop_words="english",
        sublinear_tf=True,
    )
    try:
        matrix = vec.fit_transform(contents)
        return matrix, vec
    except Exception:
        return None, None


def _find_resonant_pairs(
    nodes: list[dict],
    min_similarity: float = 0.35,
    max_pairs: int = 30,
) -> list[dict[str, Any]]:
    """
    Find pairs of nodes with high TF-IDF cosine similarity but from
    different civilization clusters (or unconnected regions).
    """
    if len(nodes) < 2:
        return []

    matrix, _ = _build_tfidf_matrix(nodes)
    if matrix is None:
        return []

    # Cosine similarity (dense array — manageable for < 10k nodes)
    sim = cosine_similarity(matrix)
    n = len(nodes)
    civ_ids = [nd.get("civilization_id", None) for nd in nodes]

    pairs = []
    for i in range(n):
        for j in range(i + 1, n):
            s = float(sim[i, j])
            if s < min_similarity:
                continue
            # Prefer cross-civilization pairs (different civ or one/both unclustered)
            civ_i = civ_ids[i]
            civ_j = civ_ids[j]
            cross_civ = (civ_i is None or civ_j is None or civ_i != civ_j)
            pairs.append({
                "node_a": nodes[i]["id"],
                "node_b": nodes[j]["id"],
                "label_a": nodes[i].get("content", "")[:35],
                "label_b": nodes[j].get("content", "")[:35],
                "similarity": round(s, 4),
                "cross_civilization": cross_civ,
                "resonance_type": "deep" if s >= 0.70 else ("moderate" if s >= 0.50 else "weak"),
            })

    # Sort: cross-civ first, then by similarity
    pairs.sort(key=lambda p: (-int(p["cross_civilization"]), -p["similarity"]))
    return pairs[:max_pairs]


# ── Cluster-level resonance ───────────────────────────────────────────────────

def _find_cluster_resonances(
    nodes: list[dict],
    civilization_members: dict[str, list[str]],
    min_similarity: float = 0.30,
) -> list[dict[str, Any]]:
    """
    Find resonance between entire civilizations (average centroid similarity).
    """
    if len(civilization_members) < 2:
        return []

    node_map = {n["id"]: n for n in nodes}
    civ_ids = list(civilization_members.keys())
    centroids: dict[str, list[str]] = {}  # civ_id → contents

    for cid, members in civilization_members.items():
        contents = [node_map[m].get("content", "") for m in members if m in node_map]
        centroids[cid] = contents

    # Represent each civ as combined TF-IDF vector
    all_docs = [" ".join(centroids[cid]) for cid in civ_ids]
    if not any(all_docs):
        return []

    try:
        vec = TfidfVectorizer(ngram_range=(1, 2), min_df=1, sublinear_tf=True)
        mat = vec.fit_transform(all_docs)
        sim = cosine_similarity(mat)
    except Exception:
        return []

    cluster_pairs = []
    for i in range(len(civ_ids)):
        for j in range(i + 1, len(civ_ids)):
            s = float(sim[i, j])
            if s >= min_similarity:
                cluster_pairs.append({
                    "civilization_a": civ_ids[i],
                    "civilization_b": civ_ids[j],
                    "similarity": round(s, 4),
                    "member_count_a": len(civilization_members[civ_ids[i]]),
                    "member_count_b": len(civilization_members[civ_ids[j]]),
                })
    cluster_pairs.sort(key=lambda p: -p["similarity"])
    return cluster_pairs[:10]


# ── Implied absent nodes ──────────────────────────────────────────────────────

def _find_implied_absences(
    nodes: list[dict],
    min_similarity: float = 0.45,
) -> list[dict[str, Any]]:
    """
    Finds groups of nodes that form a structural 'triangle' where two nodes
    are similar to a third, but those two nodes aren't similar to each other —
    suggesting a conceptual bridge concept is missing.
    """
    pairs = _find_resonant_pairs(nodes, min_similarity=min_similarity, max_pairs=100)
    # Build adjacency from resonant pairs
    adj: dict[str, set[str]] = defaultdict(set)
    for p in pairs:
        adj[p["node_a"]].add(p["node_b"])
        adj[p["node_b"]].add(p["node_a"])

    absences = []
    node_ids = [n["id"] for n in nodes]
    node_map = {n["id"]: n for n in nodes}

    seen_triples: set[frozenset] = set()
    for nid in node_ids:
        neighbors = list(adj.get(nid, set()))
        for i in range(len(neighbors)):
            for j in range(i + 1, len(neighbors)):
                a, b = neighbors[i], neighbors[j]
                triple = frozenset([nid, a, b])
                if triple in seen_triples:
                    continue
                seen_triples.add(triple)
                # a and b both resonate with nid but check if they resonate with each other
                if b not in adj.get(a, set()):
                    absences.append({
                        "hub_node": nid,
                        "hub_label": node_map.get(nid, {}).get("content", "")[:35],
                        "spoke_a": a,
                        "spoke_b": b,
                        "description": (
                            f"Both '{node_map.get(a,{}).get('content','')[:25]}' and "
                            f"'{node_map.get(b,{}).get('content','')[:25]}' resonate with "
                            f"'{node_map.get(nid,{}).get('content','')[:25]}' "
                            f"but not with each other — a bridge concept may be missing."
                        ),
                    })

    return absences[:6]


# ── Public API ────────────────────────────────────────────────────────────────

def find_resonances(workspace_json: str, min_similarity: float = 0.35) -> str:
    """
    Input:  JSON workspace snapshot + optional min_similarity threshold
    Output: JSON {pairs, cluster_resonances, implied_absences}
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        if len(nodes) < 2:
            return json.dumps({"pairs": [], "cluster_resonances": [], "implied_absences": []})

        # Exclude ghost/void nodes from resonance
        active_nodes = [n for n in nodes if not n.get("is_ghost") and not n.get("is_void")]

        pairs = _find_resonant_pairs(active_nodes, min_similarity=min_similarity)

        # Build civ membership map from node data
        civ_map: dict[str, list[str]] = defaultdict(list)
        for n in active_nodes:
            cid = n.get("civilization_id")
            if cid:
                civ_map[cid].append(n["id"])
        cluster_res = _find_cluster_resonances(active_nodes, dict(civ_map), min_similarity=min_similarity * 0.8)

        absences = _find_implied_absences(active_nodes, min_similarity=min_similarity + 0.1)

        return json.dumps({
            "pairs": pairs,
            "cluster_resonances": cluster_res,
            "implied_absences": absences,
            "stats": {
                "nodes_analyzed": len(active_nodes),
                "pairs_found": len(pairs),
                "threshold": min_similarity,
            },
        })
    except Exception as exc:
        return json.dumps({"error": str(exc)})
