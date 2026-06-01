"""
Phase 8.3 — Activity Ingestion Engine

Converts external portal activity into structured cognitive graph proposals.
No heavy ML dependencies — uses TF-IDF keyword extraction and pattern matching.
Fully local, no internet required.
"""

from silentnode_py.ingestion.engine import IngestionEngine, ingest_activity

__all__ = ["IngestionEngine", "ingest_activity"]
