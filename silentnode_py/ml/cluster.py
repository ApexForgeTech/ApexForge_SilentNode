"""
Cluster Engine for SilentNode.

Groups nodes into semantically coherent clusters using KMeans on combined
TF-IDF text features and normalized numeric features.

Cluster labels are generated from the most frequent meaningful words in
each cluster's node content — no digits, timestamps, or stop-words.

Provides:
  - Automatic k selection via silhouette score
  - Per-cluster dominant node_type summary
  - get_cluster_for_node() — which cluster a specific node belongs to
  - get_cluster_summary()  — type distribution + top words per cluster
"""

import pickle
import re
import numpy as np
from pathlib import Path
from typing import List, Dict, Optional
from sklearn.cluster import KMeans
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics import silhouette_score
from sklearn.preprocessing import normalize
from scipy.sparse import hstack, csr_matrix

from .features import (
    load_nodes, load_focus_events, node_focus_stats,
    build_numeric_features, build_text_tokens, DB_PATH,
)

MODEL_DIR = Path("data/ml_models")
CLUSTER_PATH = MODEL_DIR / "cluster_model.pkl"

_STOP_WORDS = {
    "the", "a", "an", "is", "in", "of", "and", "to", "for",
    "with", "on", "at", "by", "from", "or", "as", "it", "its",
    "thought", "idea", "node", "system", "this", "that", "are",
    "was", "has", "have", "be", "been", "will", "can", "new",
}


def _label_cluster(nodes_in_cluster: List[Dict], cluster_idx: int) -> str:
    """Generate a human-readable label for a cluster from node content.

    Extracts the top 3 most-frequent meaningful words (no digits, no
    timestamps, no stop-words, length > 2).  Falls back to the dominant
    node_type if no valid words are found.
    """
    if not nodes_in_cluster:
        return f"Cluster {cluster_idx + 1}"

    word_freq: Dict[str, int] = {}
    for n in nodes_in_cluster:
        for raw_word in n["content"].lower().split():
            word = re.sub(r"[^a-z]", "", raw_word)
            if (word
                    and len(word) > 2
                    and word not in _STOP_WORDS
                    and not word.isdigit()):
                word_freq[word] = word_freq.get(word, 0) + 1

    if not word_freq:
        types = [n["node_type"] for n in nodes_in_cluster
                 if n.get("node_type")]
        if types:
            dominant = max(set(types), key=types.count)
            return f"{dominant.capitalize()} Cluster {cluster_idx + 1}"
        return f"Cluster {cluster_idx + 1}"

    top = sorted(word_freq.items(), key=lambda x: x[1], reverse=True)[:3]
    return " · ".join(w.capitalize() for w, _ in top)


def _dominant_type(nodes_in_cluster: List[Dict]) -> str:
    """Return the most common node_type in the cluster."""
    types = [n["node_type"] for n in nodes_in_cluster if n.get("node_type")]
    if not types:
        return "unknown"
    return max(set(types), key=types.count)


class ClusterEngine:
    """KMeans cluster engine over text + numeric node features.

    Attributes:
        trained        — whether train() has succeeded
        n_clusters     — number of clusters chosen
        cluster_labels — human-readable label per cluster
        silhouette     — silhouette score of the fitted clustering
        n_samples      — number of nodes used for training
        version        — integer version counter
    """

    def __init__(self):
        self.tfidf = TfidfVectorizer(
            analyzer=build_text_tokens,
            max_features=300,
            min_df=1,
            sublinear_tf=True,
        )
        self.kmeans: Optional[KMeans] = None
        self.trained = False
        self.n_clusters = 0
        self.cluster_labels: List[str] = []
        self.cluster_dominant_types: List[str] = []
        self.silhouette: float = 0.0
        self.n_samples = 0
        self.version: int = 0
        # Store active node IDs in training order for get_cluster_for_node
        self._train_node_ids: List[str] = []
        self._train_assignments: List[int] = []

    # ------------------------------------------------------------------
    # Feature construction
    # ------------------------------------------------------------------

    def _build_X(self, nodes: List[Dict],
                 focus_stats: Dict[str, Dict],
                 fit_tfidf: bool = False) -> np.ndarray:
        """Build normalized combined feature matrix."""
        contents = [n["content"] for n in nodes]
        if fit_tfidf:
            text_X = self.tfidf.fit_transform(contents)
        else:
            text_X = self.tfidf.transform(contents)

        num_X = build_numeric_features(nodes, focus_stats)
        # Weight text features slightly higher than numeric ones
        combined = hstack([text_X * 2.0, csr_matrix(num_X)]).toarray()
        return normalize(combined, norm="l2")

    def _choose_k(self, X: np.ndarray, max_k: int = 8) -> int:
        """Select optimal k via silhouette score grid search.

        Tries k from 2 up to min(max_k, n//2).  Returns k=2 if the
        data is too small or all silhouette scores are negative.
        """
        n = len(X)
        if n < 4:
            return 2

        best_k, best_score = 2, -1.0
        for k in range(2, min(max_k + 1, n // 2 + 1)):
            km = KMeans(n_clusters=k, random_state=42, n_init=10)
            labels = km.fit_predict(X)
            if len(set(labels)) < 2:
                continue
            try:
                score = silhouette_score(X, labels)
            except Exception:
                continue
            if score > best_score:
                best_score, best_k = score, k
        return best_k

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(self, db_path: str = DB_PATH) -> Dict:
        """Fit the cluster engine from the database.

        Returns a result dict with status, n_clusters, silhouette,
        and cluster labels/dominant types.
        """
        nodes  = load_nodes(db_path)
        events = load_focus_events(db_path)
        focus_stats = node_focus_stats(nodes, events)

        active_nodes = [n for n in nodes
                        if not n["is_ghost"] and not n["is_void"]]
        self.n_samples = len(active_nodes)

        if len(active_nodes) < 4:
            return {
                "status":    "insufficient_data",
                "n_samples": len(active_nodes),
                "message":   "Need at least 4 active nodes to cluster",
            }

        X = self._build_X(active_nodes, focus_stats, fit_tfidf=True)
        k = self._choose_k(X)
        self.n_clusters = k

        self.kmeans = KMeans(n_clusters=k, random_state=42, n_init=20)
        assignments = self.kmeans.fit_predict(X)

        if len(set(assignments)) > 1:
            try:
                self.silhouette = float(silhouette_score(X, assignments))
            except Exception:
                self.silhouette = 0.0

        # Store membership for fast node-lookup
        self._train_node_ids   = [n["id"] for n in active_nodes]
        self._train_assignments = list(assignments)

        # Build per-cluster metadata
        self.cluster_labels = []
        self.cluster_dominant_types = []
        for ci in range(k):
            members = [active_nodes[i]
                       for i, c in enumerate(assignments) if c == ci]
            self.cluster_labels.append(_label_cluster(members, ci))
            self.cluster_dominant_types.append(_dominant_type(members))

        self.trained = True
        self.version += 1
        self.save()

        return {
            "status":          "trained",
            "version":         self.version,
            "n_clusters":      k,
            "silhouette":      round(self.silhouette, 3),
            "n_samples":       self.n_samples,
            "labels":          self.cluster_labels,
            "dominant_types":  self.cluster_dominant_types,
        }

    # ------------------------------------------------------------------
    # Query API
    # ------------------------------------------------------------------

    def get_clusters(self, db_path: str = DB_PATH) -> List[Dict]:
        """Return all clusters with their member nodes.

        Each cluster dict has:
            cluster_id, label, dominant_type, size, avg_gravity, members
        """
        if not self.trained or self.kmeans is None:
            return []

        nodes  = load_nodes(db_path)
        events = load_focus_events(db_path)
        focus_stats = node_focus_stats(nodes, events)
        active_nodes = [n for n in nodes
                        if not n["is_ghost"] and not n["is_void"]]
        if not active_nodes:
            return []

        try:
            X = self._build_X(active_nodes, focus_stats, fit_tfidf=False)
            assignments = self.kmeans.predict(X)
        except Exception:
            return []

        clusters: Dict[int, List] = {i: [] for i in range(self.n_clusters)}
        for node, ci in zip(active_nodes, assignments):
            clusters[int(ci)].append({
                "node_id":   node["id"],
                "content":   node["content"][:60],
                "node_type": node["node_type"],
                "gravity":   round(float(node["gravity"]), 3),
                "entropy":   round(float(node["entropy"]), 3),
            })

        result = []
        for ci in range(self.n_clusters):
            members = clusters[ci]
            if not members:
                continue
            avg_gravity = float(np.mean([m["gravity"] for m in members]))
            label = (self.cluster_labels[ci]
                     if ci < len(self.cluster_labels)
                     else f"Cluster {ci + 1}")
            dom_type = (self.cluster_dominant_types[ci]
                        if ci < len(self.cluster_dominant_types)
                        else "unknown")
            result.append({
                "cluster_id":    ci,
                "label":         label,
                "dominant_type": dom_type,
                "size":          len(members),
                "avg_gravity":   round(avg_gravity, 3),
                "members":       sorted(members,
                                        key=lambda x: x["gravity"],
                                        reverse=True),
            })

        return sorted(result, key=lambda x: x["size"], reverse=True)

    def get_cluster_for_node(self, node_id: str,
                              db_path: str = DB_PATH) -> Optional[Dict]:
        """Return which cluster a specific node belongs to.

        First tries a cached lookup from training; if the node was not in
        the training set (e.g., newly added), runs predict() on it.

        Returns None if the model is not trained or node is not found.
        """
        if not self.trained or self.kmeans is None:
            return None

        # Fast path: check cached training assignments
        try:
            idx = self._train_node_ids.index(node_id)
            ci = self._train_assignments[idx]
        except (ValueError, IndexError):
            # Node not in training set — fetch and predict
            nodes = load_nodes(db_path)
            node = next((n for n in nodes if n["id"] == node_id), None)
            if node is None:
                return None
            if node.get("is_ghost") or node.get("is_void"):
                return None
            events = load_focus_events(db_path)
            focus_stats = node_focus_stats(nodes, events)
            try:
                X = self._build_X([node], focus_stats, fit_tfidf=False)
                ci = int(self.kmeans.predict(X)[0])
            except Exception:
                return None

        label = (self.cluster_labels[ci]
                 if ci < len(self.cluster_labels)
                 else f"Cluster {ci + 1}")
        dom_type = (self.cluster_dominant_types[ci]
                    if ci < len(self.cluster_dominant_types)
                    else "unknown")
        return {
            "node_id":       node_id,
            "cluster_id":    ci,
            "label":         label,
            "dominant_type": dom_type,
        }

    def get_cluster_summary(self, db_path: str = DB_PATH) -> List[Dict]:
        """Return a concise summary of each cluster.

        Each entry has cluster_id, label, size, dominant_type,
        and a type_distribution dict.
        """
        clusters = self.get_clusters(db_path)
        summaries = []
        for cl in clusters:
            type_dist: Dict[str, int] = {}
            for m in cl["members"]:
                nt = m["node_type"] or "unknown"
                type_dist[nt] = type_dist.get(nt, 0) + 1
            summaries.append({
                "cluster_id":       cl["cluster_id"],
                "label":            cl["label"],
                "dominant_type":    cl["dominant_type"],
                "size":             cl["size"],
                "avg_gravity":      cl["avg_gravity"],
                "type_distribution": type_dist,
            })
        return summaries

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------

    def save(self):
        """Serialize this instance to disk."""
        MODEL_DIR.mkdir(parents=True, exist_ok=True)
        with open(CLUSTER_PATH, "wb") as f:
            pickle.dump(self, f)

    @classmethod
    def load(cls) -> "ClusterEngine":
        """Load a persisted engine, or return a fresh instance."""
        if CLUSTER_PATH.exists():
            with open(CLUSTER_PATH, "rb") as f:
                return pickle.load(f)
        return cls()
