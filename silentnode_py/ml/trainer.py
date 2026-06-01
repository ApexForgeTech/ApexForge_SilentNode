"""
ML Training Orchestrator for SilentNode.

Trains all four models in sequence and persists the training status to
data/ml_models/status.json.

Typical usage:
  from silentnode_py.ml.trainer import train_all, get_status
  result = train_all("data/silentnode.sqlite")

Incremental update:
  from silentnode_py.ml.trainer import incremental_update
  incremental_update("data/silentnode.sqlite")
"""

import json
import time
from pathlib import Path
from typing import Dict
from datetime import datetime, timezone

from .classifier      import NodeClassifier
from .ghost_predictor import GhostPredictor
from .sequence        import MarkovSequenceModel
from .cluster         import ClusterEngine
from .features        import DB_PATH

STATUS_PATH = Path("data/ml_models/status.json")


def train_all(db_path: str = None) -> Dict:
    """Train all four models and write status.json.

    Models trained in order:
      1. NodeClassifier    — predicts node_type
      2. GhostPredictor    — estimates ghost risk and days_to_ghost
      3. MarkovSequenceModel — focus sequence transitions
      4. ClusterEngine     — unsupervised node grouping

    Returns a result dict with per-model outcomes and total duration.
    """
    results: Dict = {
        "trained_at": datetime.now(timezone.utc).isoformat(),
        "db_path":    db_path,
        "models":     {},
        "status":     "ok",
    }

    t0 = time.time()

    print("[ML] Training node classifier ...")
    try:
        clf = NodeClassifier()
        results["models"]["classifier"] = clf.train(db_path)
    except Exception as exc:
        results["models"]["classifier"] = {"status": "error", "error": str(exc)}

    print("[ML] Training ghost predictor ...")
    try:
        gp = GhostPredictor()
        results["models"]["ghost_predictor"] = gp.train(db_path)
    except Exception as exc:
        results["models"]["ghost_predictor"] = {"status": "error", "error": str(exc)}

    print("[ML] Training sequence model ...")
    try:
        seq = MarkovSequenceModel()
        results["models"]["sequence"] = seq.train(db_path)
    except Exception as exc:
        results["models"]["sequence"] = {"status": "error", "error": str(exc)}

    print("[ML] Training cluster engine ...")
    try:
        ce = ClusterEngine()
        results["models"]["cluster"] = ce.train(db_path)
    except Exception as exc:
        results["models"]["cluster"] = {"status": "error", "error": str(exc)}

    results["duration_seconds"] = round(time.time() - t0, 2)

    # Mark overall status as partial if any model errored
    if any(v.get("status") == "error"
           for v in results["models"].values()):
        results["status"] = "partial"

    STATUS_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(STATUS_PATH, "w") as f:
        json.dump(results, f, indent=2)

    print(f"[ML] Training complete in {results['duration_seconds']}s "
          f"(status={results['status']})")
    return results


def incremental_update(db_path: str = DB_PATH) -> Dict:
    """Reload existing models and re-fit them without discarding version history.

    This is lighter than train_all() because it loads existing model objects
    (preserving their version counters and learned state) before fitting on
    fresh data.  Useful when only a small number of new nodes/events have
    been added since the last full train.

    Returns the same status dict as train_all().
    """
    results: Dict = {
        "trained_at": datetime.now(timezone.utc).isoformat(),
        "db_path":    db_path,
        "models":     {},
        "status":     "ok",
        "mode":       "incremental",
    }

    t0 = time.time()

    print("[ML] Incremental update: classifier ...")
    try:
        clf = NodeClassifier.load()
        results["models"]["classifier"] = clf.train(db_path)
    except Exception as exc:
        results["models"]["classifier"] = {"status": "error", "error": str(exc)}

    print("[ML] Incremental update: ghost predictor ...")
    try:
        gp = GhostPredictor.load()
        results["models"]["ghost_predictor"] = gp.train(db_path)
    except Exception as exc:
        results["models"]["ghost_predictor"] = {"status": "error", "error": str(exc)}

    print("[ML] Incremental update: sequence model ...")
    try:
        seq = MarkovSequenceModel.load()
        # Reset transition tables to avoid accumulating stale data
        seq.transitions    = {}
        seq.transitions2   = {}
        seq.hour_transitions = {b: {} for b in range(4)}
        seq.node_access_count = {}
        seq.n_transitions  = 0
        seq.n_transitions2 = 0
        results["models"]["sequence"] = seq.train(db_path)
    except Exception as exc:
        results["models"]["sequence"] = {"status": "error", "error": str(exc)}

    print("[ML] Incremental update: cluster engine ...")
    try:
        ce = ClusterEngine.load()
        results["models"]["cluster"] = ce.train(db_path)
    except Exception as exc:
        results["models"]["cluster"] = {"status": "error", "error": str(exc)}

    results["duration_seconds"] = round(time.time() - t0, 2)

    if any(v.get("status") == "error"
           for v in results["models"].values()):
        results["status"] = "partial"

    STATUS_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(STATUS_PATH, "w") as f:
        json.dump(results, f, indent=2)

    print(f"[ML] Incremental update complete in {results['duration_seconds']}s")
    return results


def get_status() -> Dict:
    """Read the last training status from status.json.

    Returns a dict with status, trained_at, model results, etc.
    Returns {'status': 'not_trained'} if no status file exists.
    """
    if STATUS_PATH.exists():
        with open(STATUS_PATH) as f:
            return json.load(f)
    return {
        "status":  "not_trained",
        "message": "No training run found. Execute train_all() or run: "
                   "python3 -m silentnode_py.ml.cli train",
    }


def should_retrain(db_path: str = DB_PATH) -> bool:
    """Return True if models are missing or older than 24 hours."""
    status = get_status()
    if status.get("status") not in ("ok", "partial"):
        return True

    try:
        trained_at = datetime.fromisoformat(status["trained_at"])
        # Make timezone-aware for comparison
        if trained_at.tzinfo is None:
            trained_at = trained_at.replace(tzinfo=timezone.utc)
        age_hours = ((datetime.now(timezone.utc) - trained_at).total_seconds()
                     / 3600.0)
        return age_hours > 24
    except Exception:
        return True
