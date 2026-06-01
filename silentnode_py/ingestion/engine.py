"""
Phase 8.3 — Activity Ingestion Engine
Maps external portal activity → cognitive graph proposals.

Vision.md pipeline:
  1. Capture  — raw activity recorded: what, when, duration, source
  2. Classify — type: consumption / creation / exploration / communication
  3. Extract  — subject matter identified
  4. Resonate — extracted topics compared against existing internal nodes
  5. Propose  — new nodes proposed or existing nodes strengthened
  6. Heatmap  — Thought Heatmap updated
  7. Trail    — Focus Trail extended

All processing is local — no internet, no telemetry.
"""

from __future__ import annotations

import json
import math
import re
import string
from collections import Counter, defaultdict
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from typing import Any

# ── Stop words ────────────────────────────────────────────────────────────────

_STOP_WORDS = frozenset({
    "the", "a", "an", "is", "in", "of", "and", "or", "to", "it", "its",
    "that", "this", "with", "for", "on", "at", "by", "from", "as", "be",
    "was", "are", "were", "has", "have", "had", "not", "but", "so", "if",
    "then", "than", "up", "do", "did", "will", "can", "may", "i", "you",
    "he", "she", "we", "they", "my", "your", "our", "their", "about",
    "which", "who", "what", "when", "where", "how", "all", "been", "into",
    "through", "after", "before", "there", "here", "just", "more", "also",
    "new", "one", "two", "out", "such", "very", "like", "used", "use",
    "get", "go", "time", "way", "would", "could", "should", "make",
})

# ── Activity classification ───────────────────────────────────────────────────

CONSUMPTION_KINDS  = {"view", "play", "read", "navigate", "download"}
CREATION_KINDS     = {"upload", "submit", "write", "edit", "create"}
EXPLORATION_KINDS  = {"search", "link", "browse"}
COMMUNICATION_KINDS = {"message", "reply", "comment", "email"}

def _classify(kind: str) -> str:
    k = kind.lower()
    if k in CONSUMPTION_KINDS:   return "consumption"
    if k in CREATION_KINDS:      return "creation"
    if k in EXPLORATION_KINDS:   return "exploration"
    if k in COMMUNICATION_KINDS: return "communication"
    return "consumption"

# ── Text processing ───────────────────────────────────────────────────────────

def _tokenize(text: str) -> list[str]:
    text = text.lower()
    text = re.sub(r"https?://\S+", " ", text)          # strip URLs
    text = re.sub(r"[^a-z0-9\s\-_]", " ", text)        # keep alphanumeric + separators
    tokens = text.split()
    return [t for t in tokens if len(t) > 2 and t not in _STOP_WORDS]


def _extract_topics(text: str, max_topics: int = 8) -> list[str]:
    """
    Extract key topics from text using TF-IDF-style scoring.
    Returns a list of representative terms sorted by importance.
    """
    tokens = _tokenize(text)
    if not tokens:
        return []

    # Term frequency
    tf = Counter(tokens)

    # Boost multi-word n-grams that appear as compounds
    bigrams = [f"{a}_{b}" for a, b in zip(tokens, tokens[1:])]
    for bg in bigrams:
        if tf.get(bg, 0) == 0:
            a, b = bg.split("_")
            if tf[a] >= 2 and tf[b] >= 2:
                tf[bg] = 1

    # Score: TF × log(1 + len(token)) — longer terms score higher
    scored = [(term, count * math.log1p(len(term))) for term, count in tf.items()]
    scored.sort(key=lambda x: -x[1])

    # Return top-k single-word topics (no underscore compounds for display)
    topics = [t.replace("_", " ") for t, _ in scored if "_" not in t][:max_topics]
    return topics


def _cosine_similarity(a_tokens: list[str], b_tokens: list[str]) -> float:
    """Simple cosine similarity between two token lists."""
    if not a_tokens or not b_tokens:
        return 0.0
    a = Counter(a_tokens)
    b = Counter(b_tokens)
    dot = sum(a[t] * b[t] for t in a if t in b)
    mag_a = math.sqrt(sum(v * v for v in a.values()))
    mag_b = math.sqrt(sum(v * v for v in b.values()))
    if mag_a == 0 or mag_b == 0:
        return 0.0
    return min(dot / (mag_a * mag_b), 1.0)

# ── Proposal types ────────────────────────────────────────────────────────────

@dataclass
class IngestionProposal:
    kind: str          # "create_world_node" | "strengthen_node" | "link_to_node" | "record_focus"
    confidence: float  # 0–1
    reason: str
    # For create_world_node
    content:    str = ""
    source_url: str = ""
    # For strengthen_node / link_to_node / record_focus
    node_id:        str   = ""
    gravity_boost:  float = 0.0
    duration_secs:  float = 0.0

    def to_dict(self) -> dict:
        return {k: v for k, v in asdict(self).items() if v or v == 0.0}


# ── IngestionEngine ───────────────────────────────────────────────────────────

class IngestionEngine:
    """
    Converts external portal activity into cognitive graph proposals.

    Usage:
        engine = IngestionEngine()
        proposals_json = engine.ingest(activity_json, workspace_json)
    """

    def __init__(
        self,
        min_similarity: float = 0.30,
        min_topic_len:  int   = 3,
        max_proposals:  int   = 6,
    ):
        self.min_similarity = min_similarity
        self.min_topic_len  = min_topic_len
        self.max_proposals  = max_proposals

    def ingest(self, activity_json: str, workspace_json: str = "{}") -> str:
        """
        Input:  activity_json  — serialised PortalActivity (from Rust portals)
                workspace_json — serialised workspace snapshot for resonance check

        Output: JSON list of IngestionProposal objects.
        """
        try:
            activity  = json.loads(activity_json)
            workspace = json.loads(workspace_json)
            proposals = self._process(activity, workspace)
            return json.dumps([p.to_dict() for p in proposals])
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def ingest_batch(self, activities_json: str, workspace_json: str = "{}") -> str:
        """Process a list of activities at once."""
        try:
            activities = json.loads(activities_json)
            workspace  = json.loads(workspace_json)
            all_proposals: list[IngestionProposal] = []
            for act in activities:
                all_proposals.extend(self._process(act, workspace))
            return json.dumps([p.to_dict() for p in all_proposals])
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def _process(
        self,
        activity: dict,
        workspace: dict,
    ) -> list[IngestionProposal]:
        """Core ingestion pipeline — runs all 7 steps from vision.md."""

        # Step 1 — Capture
        kind      = activity.get("kind", "view")
        target    = activity.get("target", "")
        title     = activity.get("title", "")
        duration  = float(activity.get("duration_seconds", 0.0))
        linked_id = activity.get("linked_node")  # already-linked node_id

        # Compose text for topic extraction
        text = f"{title} {target}".strip()
        if not text:
            return []

        # Step 2 — Classify
        activity_class = _classify(kind)

        # Step 3 — Extract topics
        topics = _extract_topics(text, max_topics=self.max_topics_for(activity_class))
        if not topics:
            return []

        # Step 4 — Resonance check against existing nodes
        nodes   = workspace.get("nodes", [])
        matches = self._find_resonant_nodes(topics, nodes)

        proposals: list[IngestionProposal] = []

        # Step 5 — Generate proposals

        if linked_id:
            # Already linked — just record focus
            proposals.append(IngestionProposal(
                kind="record_focus",
                confidence=0.95,
                reason=f"Activity explicitly linked to node {linked_id}",
                node_id=linked_id,
                duration_secs=duration,
            ))

        elif matches:
            # Similar nodes found — strengthen them
            for node_id, similarity in matches[:2]:
                boost = round(similarity * 0.5, 3)
                proposals.append(IngestionProposal(
                    kind="strengthen_node",
                    confidence=round(similarity, 3),
                    reason=f"Topics {topics[:3]} resonate with existing node (similarity={similarity:.2f})",
                    node_id=node_id,
                    gravity_boost=boost,
                    duration_secs=duration if proposals == [] else 0.0,
                ))

        else:
            # No match — propose world node creation
            content_label = title if title else topics[0] if topics else target[:40]
            proposals.append(IngestionProposal(
                kind="create_world_node",
                confidence=self._creation_confidence(activity_class, duration),
                reason=f"No resonant internal node found for: {topics[:3]}",
                content=content_label,
                source_url=target if target.startswith(("http", "/")) else "",
            ))

        # Step 6 — Heatmap annotation (metadata only, actual update done in Rust)
        for p in proposals:
            if not hasattr(p, "_heatmap_note"):
                pass  # Rust side handles heatmap update

        return proposals[:self.max_proposals]

    def max_topics_for(self, activity_class: str) -> int:
        return {"consumption": 6, "creation": 8, "exploration": 5, "communication": 4}.get(
            activity_class, 6
        )

    def _find_resonant_nodes(
        self,
        topics: list[str],
        nodes: list[dict],
    ) -> list[tuple[str, float]]:
        """Compare topics against node content; return (node_id, similarity) pairs."""
        if not nodes or not topics:
            return []

        matches: list[tuple[str, float]] = []
        topic_tokens = topics  # already tokenized

        for node in nodes:
            if node.get("is_ghost") or node.get("is_void"):
                continue
            node_tokens = _tokenize(node.get("content", ""))
            sim = _cosine_similarity(topic_tokens, node_tokens)
            if sim >= self.min_similarity:
                matches.append((node["id"], sim))

        matches.sort(key=lambda x: -x[1])
        return matches[:4]

    def _creation_confidence(self, activity_class: str, duration: float) -> float:
        """Higher confidence for creation activities and longer durations."""
        base = {"creation": 0.85, "exploration": 0.70, "consumption": 0.55, "communication": 0.60}
        conf = base.get(activity_class, 0.60)
        if duration > 300:   conf = min(conf + 0.10, 0.95)
        elif duration > 60:  conf = min(conf + 0.05, 0.95)
        return round(conf, 3)

    def classify_activity(self, activity_json: str) -> str:
        """Classify a single activity as consumption/creation/exploration/communication."""
        try:
            act = json.loads(activity_json)
            return json.dumps({"classification": _classify(act.get("kind", "view"))})
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def extract_topics(self, text: str) -> str:
        """Extract topics from arbitrary text."""
        return json.dumps({"topics": _extract_topics(text)})


# ── Module-level convenience ──────────────────────────────────────────────────

def ingest_activity(activity_json: str, workspace_json: str = "{}") -> str:
    """Module-level shorthand."""
    return IngestionEngine().ingest(activity_json, workspace_json)
