"""
Node Type Classifier for SilentNode.

Learns from labeled nodes in the database to predict the node_type
of new content. Combines TF-IDF text features with weak numeric graph/temporal
signals via logistic regression, learned feedback, and text-first rules.

Behavior by data volume:
  < 3 samples  : returns error immediately
  3 – 9 samples: trains without cross-validation; accuracy = 0.0
  >= 10 samples: performs cross-validation with train/test split
  < 10 samples : uses rule-based cold-start fallback for predictions
"""

import pickle
import math
import numpy as np
from pathlib import Path
from typing import List, Dict, Optional
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import LabelEncoder
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.model_selection import train_test_split, cross_val_score
from sklearn.metrics import accuracy_score
from scipy.sparse import hstack, csr_matrix

from .features import (
    load_nodes, load_focus_events, node_focus_stats,
    build_numeric_features, build_text_tokens, load_ml_feedback,
)

MODEL_DIR = Path("data/ml_models")
CLASSIFIER_PATH = MODEL_DIR / "node_classifier.pkl"

_BASELINE_EXAMPLES = [
    ("idea", "Idea Concept Thought Question Brainstorm hypothesis proposal"),
    ("idea", "Fikir dusunce ideya sual teklif qeyd"),
    ("memory", "Memory Journal Remember personal note past reflection"),
    ("memory", "Xatire gunluk kecmis remember journal reflection"),
    ("project", "Project App System Sprint Roadmap Build Release"),
    ("project", "Layihe proyekt sprint roadmap magistr exam goal"),
    ("person", "Person Contact Friend Team Alice Bob Mentor"),
    ("person", "Elaqe sexs adam mentor muellim dost contact"),
    ("artifact", "Artifact File Spec Document PDF Dataset Asset"),
    ("artifact", "Kitab sened fayl pdf material chapter article"),
    ("media", "Media Book Video Article Podcast Lecture Course"),
    ("media", "Video kurs lecture listening dinleme media"),
    ("process", "Process Routine Daily Habit Workflow Practice Study Review"),
    ("process", "Gundelik rutin namaz quran teheccud study hazirliq prep"),
    ("world", "World Area Domain Environment Context Life System"),
    ("world", "Heyat sahesi domain environment context sistem"),
]

# ---------------------------------------------------------------------------
# Rule-based cold-start fallback
# ---------------------------------------------------------------------------

_MEDIA_KEYWORDS = {
    "book", "article", "video", "paper", "podcast", "tutorial", "course",
    "lecture", "guide", "manual", "documentation", "docs", "wiki", "blog",
    "post", "read", "watch", "listen", "chapter", "page", "pdf", "ebook",
    "kitab", "məqalə", "meqale", "kurs", "dərs", "ders", "sənəd", "sened",
    "fayl", "material",
}
_PROJECT_KEYWORDS = {
    "project", "app", "service", "system", "platform", "tool", "library",
    "framework", "module", "plugin", "extension", "package", "repo", "build",
    "deploy", "release", "milestone", "sprint", "roadmap", "exam",
    "magistr", "academy",
    "layihe", "layihə", "proyekt", "imtahan", "məqsəd", "meqsed",
}
_PERSON_KEYWORDS = {
    "person", "contact", "friend", "mentor", "teacher", "student",
    "manager", "colleague", "client", "customer", "team", "member",
    "profile", "phone", "email",
    "əlaqə", "elaqe", "dost", "müəllim", "muellim", "şəxs", "sexs", "adam",
}
_PROCESS_KEYWORDS = {
    "process", "workflow", "task", "job", "step", "procedure", "pipeline",
    "automation", "script", "batch", "cron", "schedule", "routine",
    "habit", "daily", "practice", "study", "listening", "review", "prep",
    "preparation", "prayer", "quran", "tahajjud", "namaz", "work",
    "responsibility", "test", "tests",
    "gundelik", "gündəlik", "rutin", "vərdiş", "verdis", "hazirliq",
    "hazırlıq", "hazirlasmaq", "hazırlaşmaq", "qebul", "qəbul",
    "informatika", "ingilis", "ingilisce", "ingiliscə", "dinleme",
    "dinləmə", "iş", "gomruk", "gömrük",
}
_IDEA_KEYWORDS = {
    "idea", "thought", "note", "hypothesis", "theory", "question", "brainstorm",
    "concept", "proposal", "draft",
    "fikir", "düşüncə", "dusunce", "ideya", "sual", "təklif", "teklif",
    "qeyd",
}


def _rule_based_classify(content: str) -> str:
    """Classify content into a node_type using keyword heuristics.

    Checks in priority order:
    1. Artifact/media keywords
    2. Project keywords
    3. Process keywords
    4. Idea keywords
    5. Person (single capitalized word or known name patterns)
    6. Default: "idea"
    """
    tokens = set(build_text_tokens(content))

    if tokens & _MEDIA_KEYWORDS:
        return "artifact"
    if tokens & _PERSON_KEYWORDS:
        return "person"
    if tokens & _PROJECT_KEYWORDS and not (tokens & _PROCESS_KEYWORDS):
        return "project"
    if tokens & _PROCESS_KEYWORDS:
        return "process"
    if tokens & _PROJECT_KEYWORDS:
        return "project"
    if tokens & _IDEA_KEYWORDS:
        return "idea"

    # Single capitalized word with no lowercase → likely a person/entity name
    stripped = content.strip()
    words = stripped.split()
    if (len(words) == 1 and stripped[0].isupper() and stripped[1:].islower()
            and len(stripped) > 2):
        return "person"
    # Multi-word where each word starts with capital (proper noun phrase)
    if (len(words) >= 2
            and all(w[0].isupper() for w in words if w)
            and not any(w.lower() in _PROJECT_KEYWORDS for w in words)):
        return "person"

    return "idea"


def _has_explicit_idea_signal(content: str) -> bool:
    return bool(set(build_text_tokens(content)) & _IDEA_KEYWORDS)


def _rule_evidence(content: str, rule_type: str) -> Dict:
    tokens = set(build_text_tokens(content))
    groups = {
        "artifact": _MEDIA_KEYWORDS,
        "person": _PERSON_KEYWORDS,
        "project": _PROJECT_KEYWORDS,
        "process": _PROCESS_KEYWORDS,
        "idea": _IDEA_KEYWORDS,
    }
    matched = sorted(tokens & groups.get(rule_type, set()))
    return {
        "rule_type": rule_type,
        "matched_tokens": matched[:8],
        "text_signal": "explicit" if matched else "weak",
    }


def _probability_margin(probs: np.ndarray) -> float:
    if len(probs) < 2:
        return 1.0
    ranked = np.sort(probs)[::-1]
    return float(ranked[0] - ranked[1])


def _confidence_band(confidence: float, margin: float) -> str:
    if confidence >= 0.72 and margin >= 0.24:
        return "strong"
    if confidence >= 0.58 and margin >= 0.12:
        return "usable"
    return "uncertain"


def _should_boost_rule(content: str, rule_type: str, ml_type: str, ml_conf: float) -> bool:
    if rule_type == "idea" and not _has_explicit_idea_signal(content):
        return False
    return ml_type == "idea" or ml_conf < 0.72 or rule_type == "idea"


def _boost_rule_probability(probs: np.ndarray, classes: np.ndarray, rule_type: str) -> np.ndarray:
    """Return a probability vector where the rule-picked class is prominent.

    Random forests trained on mixed vaults can be timid for new private-vault
    habits. When a clear rule matches, keep the model's distribution shape but
    lift the rule class enough for the UI to present a useful suggestion.
    """
    boosted = probs.astype(float).copy()
    rule_idx = int(np.where(classes == rule_type)[0][0])
    target = max(float(boosted[rule_idx]), 0.68)
    if len(boosted) == 1:
        boosted[rule_idx] = 1.0
        return boosted

    other_total = float(boosted.sum() - boosted[rule_idx])
    boosted[rule_idx] = target
    remainder = 1.0 - target
    for idx in range(len(boosted)):
        if idx == rule_idx:
            continue
        boosted[idx] = (boosted[idx] / other_total * remainder) if other_total > 0 else remainder / (len(boosted) - 1)
    return boosted


# ---------------------------------------------------------------------------
# Classifier
# ---------------------------------------------------------------------------

class NodeClassifier:
    """Text-first classifier combining TF-IDF and weak numeric features.

    Attributes:
        trained        — whether the model has been fitted
        accuracy       — cross-val or held-out accuracy (0 if insufficient data)
        class_counts   — dict mapping class label -> count in training set
        n_samples      — number of training samples
        feature_importances — ranked feature names with importance scores
        version        — integer version counter, incremented on each train
    """

    def __init__(self):
        self.tfidf = TfidfVectorizer(
            analyzer=build_text_tokens,
            max_features=1000,
            min_df=1,
            sublinear_tf=True,
            ngram_range=(1, 1),  # bigrams already handled in tokenizer
        )
        self.clf = LogisticRegression(
            class_weight="balanced",
            C=3.0,
            max_iter=2000,
            solver="lbfgs",
            random_state=42,
        )
        self.label_enc = LabelEncoder()
        self.trained = False
        self.accuracy = 0.0
        self.class_counts: Dict[str, int] = {}
        self.n_samples = 0
        self.feature_importances: List[Dict] = []
        self.token_type_scores: Dict[str, Dict[str, float]] = {}
        self.feedback_samples: int = 0
        self.version: int = 0

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _build_X(self, nodes: List[Dict],
                 focus_stats: Dict[str, Dict],
                 fit_tfidf: bool = False) -> np.ndarray:
        """Assemble the combined feature matrix (text + numeric)."""
        contents = [n.get("model_text") or n["content"] for n in nodes]
        if fit_tfidf:
            text_X = self.tfidf.fit_transform(contents)
        else:
            text_X = self.tfidf.transform(contents)

        # Text decides the semantic type. Numeric graph/time/vault signals are
        # deliberately weak helpers so a process-heavy private vault does not
        # make unrelated text look like another process.
        num_X = build_numeric_features(nodes, focus_stats) * 0.05
        return hstack([text_X, csr_matrix(num_X)]).toarray()

    def _compute_feature_importances(self, n_text_features: int) -> None:
        """Populate self.feature_importances with name/importance pairs."""
        from .features import NUMERIC_FEATURE_NAMES
        if hasattr(self.clf, "coef_"):
            importances = np.mean(np.abs(self.clf.coef_), axis=0)
        else:
            importances = getattr(self.clf, "feature_importances_", np.zeros(n_text_features))
        vocab = self.tfidf.get_feature_names_out()

        all_names = list(vocab) + NUMERIC_FEATURE_NAMES
        paired = [(all_names[i], float(importances[i]))
                  for i in range(min(len(all_names), len(importances)))]
        paired.sort(key=lambda x: x[1], reverse=True)
        self.feature_importances = [
            {"feature": name, "importance": round(imp, 5)}
            for name, imp in paired[:30]  # top-30 only
        ]

    def _fit_token_type_scores(self, nodes: List[Dict], labels: List[str], sample_weight: np.ndarray) -> None:
        """Learn a lightweight personal token profile from weighted examples."""
        token_scores: Dict[str, Dict[str, float]] = {}
        for node, label, weight in zip(nodes, labels, sample_weight):
            text = node.get("model_text") or node.get("content", "")
            for token in set(build_text_tokens(text)):
                if len(token) < 3:
                    continue
                per_type = token_scores.setdefault(token, {})
                per_type[label] = per_type.get(label, 0.0) + float(weight)

        compact: Dict[str, Dict[str, float]] = {}
        for token, per_type in token_scores.items():
            total = sum(per_type.values())
            if total < 2.0:
                continue
            compact[token] = {
                label: round(score / total, 4)
                for label, score in per_type.items()
                if score / total >= 0.15
            }
        self.token_type_scores = compact

    def _profile_probs(self, content: str, classes: np.ndarray) -> Optional[np.ndarray]:
        """Infer class probabilities from the learned personal token profile."""
        scores = np.zeros(len(classes), dtype=float)
        hits = 0
        class_to_idx = {str(label): idx for idx, label in enumerate(classes)}
        for token in set(build_text_tokens(content)):
            per_type = getattr(self, "token_type_scores", {}).get(token)
            if not per_type:
                continue
            hits += 1
            for label, score in per_type.items():
                idx = class_to_idx.get(label)
                if idx is not None:
                    scores[idx] += float(score)
        if hits == 0 or scores.sum() <= 0:
            return None
        scores = scores + 0.02
        return scores / scores.sum()

    def _blend_profile_probs(self, probs: np.ndarray, content: str, classes: np.ndarray) -> np.ndarray:
        profile = self._profile_probs(content, classes)
        if profile is None:
            return probs
        return (0.82 * probs) + (0.18 * profile)

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(self, db_path: str = None) -> Dict:
        """Train the classifier from the database.

        Returns a result dict with status, accuracy, class distribution,
        and top feature importances.
        """
        nodes = load_nodes(db_path)
        events = load_focus_events(db_path)
        feedback = load_ml_feedback(db_path)
        focus_stats = node_focus_stats(nodes, events)

        # Exclude ghost/fossil nodes — their types are not predictive for
        # general classification purposes
        train_nodes = [
            n for n in nodes
            if not n["is_ghost"] and not n["is_fossil"]
            and n["node_type"] not in ("", None)
        ]

        if len(train_nodes) < 3:
            return {
                "status": "insufficient_data",
                "n_samples": len(train_nodes),
                "message": "Need at least 3 labeled nodes to train",
            }

        baseline_nodes = []
        if db_path is None:
            for idx, (node_type, content) in enumerate(_BASELINE_EXAMPLES):
                baseline_nodes.append({
                    "id": f"__baseline_{idx}__",
                    "node_type": node_type,
                    "content": content,
                    "nickname": node_type,
                    "model_text": content,
                    "entropy": 0.0,
                    "gravity": 1.0,
                    "velocity": 0.0,
                    "access_count": 0,
                    "days_since_access": 0.0,
                    "days_since_created": 365.0,
                    "connection_count": 0,
                    "is_ghost": 0,
                    "is_fossil": 0,
                    "is_void": 0,
                    "is_current_vault": False,
                    "is_baseline": True,
                })

        feedback_nodes = []
        for idx, fb in enumerate(feedback):
            selected_type = str(fb.get("selected_type") or "").lower().strip()
            content = str(fb.get("content") or "").strip()
            if not selected_type or not content:
                continue
            nickname = str(fb.get("nickname") or "").strip()
            created_at = fb.get("created_at") or ""
            feedback_nodes.append({
                "id": f"__feedback_{idx}__",
                "node_type": selected_type,
                "content": content,
                "nickname": nickname,
                "model_text": "\n".join(part for part in [nickname, content] if part),
                "entropy": 0.0,
                "gravity": 1.0,
                "velocity": 0.0,
                "access_count": 0,
                "days_since_access": 0.0,
                "days_since_created": 0.0,
                "connection_count": 0,
                "is_ghost": 0,
                "is_fossil": 0,
                "is_void": 0,
                "is_current_vault": bool(fb.get("is_current_vault")),
                "is_feedback": True,
                "feedback_source": fb.get("source", ""),
                "feedback_created_at": created_at,
            })

        self.feedback_samples = len(feedback_nodes)
        train_nodes = train_nodes + feedback_nodes + baseline_nodes

        labels = [n["node_type"].lower() for n in train_nodes]
        self.class_counts = {t: labels.count(t) for t in set(labels)}
        self.n_samples = len(train_nodes)

        X = self._build_X(train_nodes, focus_stats, fit_tfidf=True)
        y = self.label_enc.fit_transform(labels)
        sample_weight = np.array([self._sample_weight(n) for n in train_nodes], dtype=np.float32)
        self._fit_token_type_scores(train_nodes, labels, sample_weight)

        # Cross-validation or held-out accuracy when enough data
        if self.n_samples >= 20:
            n_classes = len(self.class_counts)
            test_count = max(math.ceil(self.n_samples * 0.2), n_classes)
            test_size = min(0.4, test_count / self.n_samples)
            stratify = y if min(self.class_counts.values()) > 1 and test_count >= n_classes else None
            X_tr, X_te, y_tr, y_te = train_test_split(
                X, y, test_size=test_size, random_state=42, stratify=stratify
            )
            self.clf.fit(X_tr, y_tr)
            self.accuracy = float(accuracy_score(y_te, self.clf.predict(X_te)))
            # Re-fit on full data for deployment
            self.clf.fit(X, y, sample_weight=sample_weight)
        elif self.n_samples >= 10:
            n_folds = min(3, min(self.class_counts.values(), default=1))
            n_folds = max(n_folds, 2)
            cv_scores = cross_val_score(self.clf, X, y, cv=n_folds,
                                        scoring="accuracy")
            self.accuracy = float(np.mean(cv_scores))
            self.clf.fit(X, y, sample_weight=sample_weight)
        else:
            # Not enough data for reliable cross-validation
            self.accuracy = 0.0
            self.clf.fit(X, y, sample_weight=sample_weight)

        self.trained = True
        self.version += 1
        self._compute_feature_importances(len(self.tfidf.vocabulary_))
        self.save()

        return {
            "status": "trained",
            "version": self.version,
            "n_samples": self.n_samples,
            "feedback_samples": self.feedback_samples,
            "accuracy": round(self.accuracy, 3),
            "classes": self.class_counts,
            "top_features": self.feature_importances[:10],
        }

    def _sample_weight(self, node: Dict) -> float:
        weight = 1.0
        if node.get("is_current_vault"):
            weight *= 2.0
        if node.get("is_feedback"):
            weight *= 12.0
        if node.get("is_baseline"):
            weight *= 1.0
        if float(node.get("days_since_created", 999.0)) <= 30.0:
            weight *= 1.2
        return weight

    # ------------------------------------------------------------------
    # Inference
    # ------------------------------------------------------------------

    def predict(self, content: str,
                extra_features: Optional[Dict] = None) -> Dict:
        """Predict node_type for a single content string.

        Falls back to rule-based heuristic when the model has not been
        trained or has fewer than 10 training samples.
        """
        if not self.trained or self.n_samples < 10:
            rule_type = _rule_based_classify(content)
            return {
                "type": rule_type,
                "confidence": 0.6,
                "all_probs": {rule_type: 0.6},
                "uncertain": False,
                "confidence_band": "usable",
                "evidence": _rule_evidence(content, rule_type),
                "method": "rule_based",
            }

        node = {
            "id": "__predict__",
            "content": content,
            "entropy": 0.1, "gravity": 1.0, "velocity": 0.0,
            "access_count": 0, "days_since_access": 0.0,
            "days_since_created": 0.0, "connection_count": 0,
            "is_ghost": 0, "is_fossil": 0, "is_void": 0,
        }
        if extra_features:
            node.update(extra_features)

        X = self._build_X([node], {"__predict__": {}}, fit_tfidf=False)
        probs = self.clf.predict_proba(X)[0]
        classes = self.label_enc.classes_
        probs = self._blend_profile_probs(probs, content, classes)

        best_idx = int(np.argmax(probs))
        ml_type = str(classes[best_idx])
        ml_conf = float(probs[best_idx])
        rule_type = _rule_based_classify(content)
        if rule_type in classes and _should_boost_rule(content, rule_type, ml_type, ml_conf):
            probs = _boost_rule_probability(probs, classes, rule_type)
            best_idx = int(np.argmax(probs))
            ml_type = str(classes[best_idx])
            ml_conf = float(probs[best_idx])

        return {
            "type": ml_type,
            "confidence": round(ml_conf, 3),
            "all_probs": {str(c): round(float(p), 3)
                          for c, p in zip(classes, probs)},
            "alternatives": self._alternatives(classes, probs),
            "uncertain": bool(ml_conf < 0.58 or _probability_margin(probs) < 0.10),
            "confidence_band": _confidence_band(ml_conf, _probability_margin(probs)),
            "evidence": _rule_evidence(content, rule_type),
            "method": "ml_rule_blend" if rule_type == ml_type else "ml",
        }

    def _alternatives(self, classes: np.ndarray, probs: np.ndarray, limit: int = 3) -> List[Dict]:
        order = np.argsort(probs)[::-1][:limit]
        return [
            {"type": str(classes[idx]), "confidence": round(float(probs[idx]), 3)}
            for idx in order
        ]

    def predict_batch(self, contents: List[str],
                      extra_features_list: Optional[List[Optional[Dict]]] = None
                      ) -> List[Dict]:
        """Predict node_type for a list of content strings.

        More efficient than calling predict() in a loop because TF-IDF
        transform is done once for all inputs.

        Args:
            contents: list of content strings
            extra_features_list: optional parallel list of extra_feature dicts
                                  (None entries are treated as empty dicts)

        Returns:
            List of prediction dicts in the same order as contents.
        """
        if not contents:
            return []

        if not self.trained or self.n_samples < 10:
            return [self.predict(c) for c in contents]

        if extra_features_list is None:
            extra_features_list = [None] * len(contents)

        nodes = []
        for i, content in enumerate(contents):
            node = {
                "id": f"__batch_{i}__",
                "content": content,
                "entropy": 0.1, "gravity": 1.0, "velocity": 0.0,
                "access_count": 0, "days_since_access": 0.0,
                "days_since_created": 0.0, "connection_count": 0,
                "is_ghost": 0, "is_fossil": 0, "is_void": 0,
            }
            ef = extra_features_list[i]
            if ef:
                node.update(ef)
            nodes.append(node)

        focus_stub = {f"__batch_{i}__": {} for i in range(len(contents))}
        X = self._build_X(nodes, focus_stub, fit_tfidf=False)
        all_probs = self.clf.predict_proba(X)
        classes = self.label_enc.classes_

        results = []
        for content, probs in zip(contents, all_probs):
            probs = self._blend_profile_probs(probs.copy(), content, classes)
            best_idx = int(np.argmax(probs))
            ml_type = str(classes[best_idx])
            ml_conf = float(probs[best_idx])
            rule_type = _rule_based_classify(content)
            if rule_type in classes and _should_boost_rule(content, rule_type, ml_type, ml_conf):
                probs = _boost_rule_probability(probs, classes, rule_type)
                best_idx = int(np.argmax(probs))
                ml_type = str(classes[best_idx])
                ml_conf = float(probs[best_idx])
            results.append({
                "type": ml_type,
                "confidence": round(ml_conf, 3),
                "all_probs": {str(c): round(float(p), 3)
                              for c, p in zip(classes, probs)},
                "alternatives": self._alternatives(classes, probs),
                "uncertain": bool(ml_conf < 0.58 or _probability_margin(probs) < 0.10),
                "confidence_band": _confidence_band(ml_conf, _probability_margin(probs)),
                "evidence": _rule_evidence(content, rule_type),
                "method": "ml_rule_blend" if rule_type == ml_type else "ml",
            })
        return results

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------

    def save(self):
        """Serialize this instance to disk."""
        MODEL_DIR.mkdir(parents=True, exist_ok=True)
        with open(CLASSIFIER_PATH, "wb") as f:
            pickle.dump(self, f)

    @classmethod
    def load(cls) -> "NodeClassifier":
        """Load a persisted classifier, or return a fresh instance."""
        if CLASSIFIER_PATH.exists():
            with open(CLASSIFIER_PATH, "rb") as f:
                return pickle.load(f)
        return cls()
