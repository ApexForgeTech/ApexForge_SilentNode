"""
Diagnostics for SilentNode ML.

The goal is not only to predict, but to make the local model auditable: what
data it learned from, where labels are weak, and how example phrases behave.
"""

from __future__ import annotations

from typing import Dict, List

from .classifier import NodeClassifier
from .features import build_text_tokens, load_ml_feedback, load_nodes


_PROBE_TEXTS = [
    "Alice Johnson contact",
    "yeni fikir qeyd et",
    "namaz quran teheccud gundelik rutin",
    "magistr imtahan hazirliq informatika test",
    "ingilisce listening 25 deqiqe",
    "Rust programming book chapter",
    "gomruk komitesine getmek is rutini",
    "launch personal finance app project",
]


def classifier_diagnostics(limit_tokens: int = 20) -> Dict:
    """Return a compact health report for the node classifier."""
    clf = NodeClassifier.load()
    if not clf.trained:
        train_result = clf.train()
    else:
        train_result = None

    class_counts = getattr(clf, "class_counts", {}) or {}
    weak_classes = [
        {"type": label, "samples": count}
        for label, count in sorted(class_counts.items())
        if count < 3
    ]

    token_profile = getattr(clf, "token_type_scores", {}) or {}
    top_profile_tokens: List[Dict] = []
    for token, per_type in token_profile.items():
        strength = max(per_type.values()) if per_type else 0.0
        top_profile_tokens.append({
            "token": token,
            "types": per_type,
            "strength": round(float(strength), 4),
        })
    top_profile_tokens.sort(key=lambda item: item["strength"], reverse=True)

    probes = []
    for text in _PROBE_TEXTS:
        pred = clf.predict(text)
        probes.append({
            "text": text,
            "tokens": build_text_tokens(text)[:limit_tokens],
            "prediction": pred,
        })

    return {
        "status": "ok",
        "trained": bool(clf.trained),
        "trained_now": train_result is not None,
        "n_samples": int(getattr(clf, "n_samples", 0)),
        "feedback_samples": int(getattr(clf, "feedback_samples", 0)),
        "class_counts": class_counts,
        "weak_classes": weak_classes,
        "feedback_rows": len(load_ml_feedback()),
        "top_features": getattr(clf, "feature_importances", [])[:15],
        "top_profile_tokens": top_profile_tokens[:limit_tokens],
        "probe_results": probes,
        "vault_node_count": len(load_nodes()),
        "recommendations": _recommendations(weak_classes, len(load_ml_feedback())),
    }


def _recommendations(weak_classes: List[Dict], feedback_rows: int) -> List[str]:
    recs = []
    if weak_classes:
        weak = ", ".join(f"{item['type']}({item['samples']})" for item in weak_classes)
        recs.append(f"Add or correct more examples for weak classes: {weak}.")
    if feedback_rows < 10:
        recs.append("Use manual type corrections in Add Node; feedback has the highest training weight.")
    recs.append("Retrain after meaningful new nodes or corrections.")
    recs.append("Prefer stable process nodes for repeated routines so sequence/focus models can learn habits.")
    return recs
