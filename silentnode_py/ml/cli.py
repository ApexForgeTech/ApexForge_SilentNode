"""
ML CLI — invoked by the Rust layer or directly for testing.

Usage:
    python3 -m silentnode_py.ml.cli <command> [args]

Commands:
    train                            Train all models from the database
    status                           Show last training status (JSON)
    classify <content>               Predict node_type for a content string
    classify-batch <c1> <c2> ...     Predict node_type for multiple contents
    ghost-risk                       List all live nodes with ghost risk scores
    entropy-trajectory <node_id> <days>
                                     Project entropy N days ahead for a node
    next-focus <node_id>             Predict next focus targets (JSON)
    daily-plan                       Recommend today's focus plan (JSON)
    diagnostics                      Inspect classifier health and probe texts
    clusters                         Show node clusters (JSON)
    cluster-for <node_id>            Show which cluster a node belongs to
    full-predict [node_id]           Combined prediction payload for the API
"""

import sys
import json
import os

# Change to the SilentNode project root so relative DB paths resolve
os.chdir(os.path.dirname(os.path.dirname(os.path.dirname(
    os.path.abspath(__file__)))))

from silentnode_py.ml.trainer         import train_all, get_status
from silentnode_py.ml.classifier      import NodeClassifier
from silentnode_py.ml.ghost_predictor import GhostPredictor
from silentnode_py.ml.sequence        import MarkovSequenceModel
from silentnode_py.ml.cluster         import ClusterEngine
from silentnode_py.ml.advisor         import daily_plan
from silentnode_py.ml.diagnostics     import classifier_diagnostics
from silentnode_py.ml.features        import current_vault_path


def out(data):
    """Print data as indented JSON to stdout."""
    print(json.dumps(data, ensure_ascii=True, indent=2))


def _ensure_trained(model, train_fn):
    """Train a model in-place if it has not yet been fitted."""
    if not model.trained:
        train_fn()
    return model


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return

    cmd = args[0]

    # ------------------------------------------------------------------ train
    if cmd == "train":
        result = train_all()
        out(result)

    # ------------------------------------------------------------------ status
    elif cmd == "status":
        out(get_status())

    # --------------------------------------------------------------- classify
    elif cmd == "classify":
        content = " ".join(args[1:]) if len(args) > 1 else ""
        if not content:
            out({"error": "content required: classify <content>"})
            return
        clf = NodeClassifier.load()
        if not clf.trained:
            clf.train()
        out(clf.predict(content))

    # --------------------------------------------------------- classify-batch
    elif cmd == "classify-batch":
        contents = args[1:]
        if not contents:
            out({"error": "at least one content string required"})
            return
        clf = NodeClassifier.load()
        if not clf.trained:
            clf.train()
        out(clf.predict_batch(contents))

    # ------------------------------------------------------------- ghost-risk
    elif cmd == "ghost-risk":
        gp = GhostPredictor.load()
        if not gp.trained:
            gp.train()
        out(gp.predict_all())

    # ------------------------------------------------------- entropy-trajectory
    elif cmd == "entropy-trajectory":
        if len(args) < 3:
            out({"error": "usage: entropy-trajectory <node_id> <days>"})
            return
        node_id = args[1]
        try:
            days = int(args[2])
        except ValueError:
            out({"error": "days must be an integer"})
            return
        gp = GhostPredictor.load()
        if not gp.trained:
            gp.train()
        out(gp.predict_entropy_trajectory(node_id, days))

    # --------------------------------------------------------------- next-focus
    elif cmd == "next-focus":
        node_id = args[1] if len(args) > 1 else ""
        seq = MarkovSequenceModel()
        seq.train(current_vault_path())
        if node_id:
            out(seq.predict_next(node_id))
        else:
            out(seq._fallback_popular(5))

    # -------------------------------------------------------------- daily-plan
    elif cmd == "daily-plan":
        limit = 8
        if len(args) > 1:
            try:
                limit = max(1, min(int(args[1]), 25))
            except ValueError:
                out({"error": "limit must be an integer"})
                return
        out(daily_plan(limit=limit))

    # ------------------------------------------------------------- diagnostics
    elif cmd == "diagnostics":
        out(classifier_diagnostics())

    # ---------------------------------------------------------------- clusters
    elif cmd == "clusters":
        ce = ClusterEngine.load()
        if not ce.trained:
            ce.train()
        out(ce.get_clusters())

    # ----------------------------------------------------------- cluster-for
    elif cmd == "cluster-for":
        node_id = args[1] if len(args) > 1 else ""
        if not node_id:
            out({"error": "usage: cluster-for <node_id>"})
            return
        ce = ClusterEngine.load()
        if not ce.trained:
            ce.train()
        result = ce.get_cluster_for_node(node_id)
        out(result if result else {"error": f"node '{node_id}' not found or not clusterable"})

    # ------------------------------------------------------------- full-predict
    elif cmd == "full-predict":
        node_id = args[1] if len(args) > 1 else ""

        gp  = GhostPredictor.load()
        seq = MarkovSequenceModel()
        seq.train(current_vault_path())
        ce  = ClusterEngine.load()

        ghost_risks = gp.predict_all()  if gp.trained  else []
        next_focus  = (seq.predict_next(node_id, 5)
                       if (seq.trained and node_id) else [])
        clusters    = ce.get_clusters() if ce.trained  else []

        out({
            "ghost_risks":  ghost_risks[:10],
            "next_focus":   next_focus,
            "clusters":     clusters[:5],
            "status":       get_status().get("status", "unknown"),
        })

    else:
        print(f"Unknown command: {cmd!r}")
        print(__doc__)
        sys.exit(1)


if __name__ == "__main__":
    main()
