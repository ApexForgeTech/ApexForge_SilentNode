"""
Seed realistic training data into SilentNode SQLite.
Adds diverse nodes, focus events, edges, and journal entries
so the ML models have enough signal to train meaningfully.
"""

import sqlite3
import uuid
import json
import random
import math
from datetime import datetime, timezone, timedelta

DB = "data/silentnode.sqlite"
random.seed(42)

now = datetime.now(timezone.utc)

def ts(days_ago=0, hours_ago=0) -> str:
    t = now - timedelta(days=days_ago, hours=hours_ago)
    return t.isoformat()

def uid() -> str:
    return str(uuid.uuid4())

# ── Nodes ────────────────────────────────────────────────────────────────────

NODES = [
    # Projects
    ("project", "Rust Web Framework",          0.08, 9.2, 0.012, 12, 0, 0, 0),
    ("project", "Personal Finance Tracker",    0.45, 4.1, 0.003,  4, 0, 0, 0),
    ("project", "ML Pipeline Experiment",      0.22, 6.8, 0.008,  8, 0, 0, 0),
    ("project", "NightOS Kernel Module",       0.61, 3.2, 0.001,  2, 0, 0, 0),
    ("project", "CLI Todo Manager",            0.78, 1.8, 0.000,  1, 1, 0, 0),  # ghost

    # Ideas
    ("idea",    "Distributed consensus without leader election", 0.34, 5.5, 0.006, 6, 0, 0, 0),
    ("idea",    "Zero-copy serialization approach",              0.19, 4.8, 0.009, 9, 0, 0, 0),
    ("idea",    "Memory-mapped graph storage",                   0.51, 3.1, 0.002, 3, 0, 0, 0),
    ("idea",    "Adaptive entropy decay per node type",          0.28, 6.0, 0.007, 7, 0, 0, 0),
    ("idea",    "Focus trail as training signal",                0.15, 7.2, 0.011, 11,0, 0, 0),
    ("idea",    "Cognitive load estimation from typing speed",   0.42, 3.9, 0.004, 4, 0, 0, 0),
    ("idea",    "Graph partitioning for cache locality",         0.55, 2.7, 0.002, 2, 0, 0, 0),
    ("idea",    "Time-weighted PageRank variant",                0.33, 4.4, 0.005, 5, 0, 0, 0),
    ("idea",    "Semantic similarity without embeddings",        0.67, 2.1, 0.001, 1, 0, 0, 0),
    ("idea",    "Event sourcing for node history",               0.21, 5.8, 0.008, 8, 0, 0, 0),

    # Research / Media
    ("media",   "The Rust Programming Language (book)",         0.12, 8.1, 0.010, 15, 0, 0, 0),
    ("media",   "Programming Rust 2nd Ed",                      0.29, 5.3, 0.006,  6, 0, 0, 0),
    ("media",   "Database Internals by Alex Petrov",            0.44, 3.7, 0.003,  4, 0, 0, 0),
    ("media",   "Jon Gjengset async Rust YouTube series",       0.18, 6.4, 0.009, 10, 0, 0, 0),
    ("media",   "MIT 6.824 Distributed Systems lectures",       0.56, 3.0, 0.002,  3, 0, 0, 0),
    ("media",   "Designing Data-Intensive Applications",        0.38, 4.5, 0.005,  5, 0, 0, 0),

    # Artifacts (things created)
    ("artifact","axum middleware prototype",       0.14, 7.5, 0.011, 11, 0, 0, 0),
    ("artifact","SQLite WAL benchmark script",     0.31, 5.0, 0.005,  5, 0, 0, 0),
    ("artifact","Entropy decay formula v2",        0.25, 6.1, 0.007,  7, 0, 0, 0),
    ("artifact","Graph traversal BFS/DFS impl",   0.47, 3.4, 0.003,  3, 0, 0, 0),
    ("artifact","WGSL node shader v3",             0.09, 8.8, 0.013, 14, 0, 0, 0),

    # People
    ("person",  "Jon Gjengset",          0.22, 5.2, 0.005, 5, 0, 0, 0),
    ("person",  "Alex Petrov",           0.35, 3.8, 0.003, 3, 0, 0, 0),
    ("person",  "Arpad Borsos",          0.51, 2.5, 0.001, 1, 0, 0, 0),

    # Processes
    ("process", "Daily code review routine",    0.16, 6.9, 0.009, 9, 0, 0, 0),
    ("process", "Weekly architecture session",  0.29, 5.4, 0.006, 6, 0, 0, 0),
    ("process", "Benchmark → Profile → Fix",    0.42, 4.0, 0.004, 4, 0, 0, 0),

    # World nodes (external resources)
    ("world",   "tokio-rs/tokio GitHub",           0.11, 7.8, 0.010, 10, 0, 0, 0),
    ("world",   "seanmonstar/reqwest GitHub",       0.33, 4.6, 0.005,  5, 0, 0, 0),
    ("world",   "hyperium/hyper GitHub",            0.48, 3.3, 0.003,  3, 0, 0, 0),
    ("world",   "SQLite official docs",             0.20, 6.2, 0.008,  8, 0, 0, 0),

    # Memory
    ("memory",  "First time async clicked for me", 0.39, 4.2, 0.004, 4, 0, 0, 0),
    ("memory",  "Rewrote the graph engine in 3 days", 0.17, 7.0, 0.009, 9, 0, 0, 0),
    ("memory",  "Debugging the WAL corruption issue", 0.52, 3.0, 0.002, 2, 0, 0, 0),
]

# node_id lookup by content
NODE_IDS: dict[str, str] = {}

def insert_nodes(cur):
    for i, (ntype, content, entropy, gravity, velocity, access_count,
            is_ghost, is_fossil, is_void) in enumerate(NODES):
        nid = uid()
        NODE_IDS[content] = nid
        days_old = random.randint(5, 90)
        days_acc = random.randint(0, min(days_old, 30))
        created = ts(days_ago=days_old)
        accessed = ts(days_ago=days_acc)
        cur.execute("""
            INSERT OR IGNORE INTO node
              (id, node_type, content, entropy, gravity, velocity,
               access_count, is_ghost, is_fossil, is_void,
               created_at, accessed_at,
               position_x, position_y, position_z,
               aura_color, soul_signature_json, metadata_json)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
        """, (
            nid, ntype, content, entropy, gravity, velocity,
            access_count, is_ghost, is_fossil, is_void,
            created, accessed,
            random.uniform(-20, 20), random.uniform(-10, 10), random.uniform(-15, 15),
            "#40c8ff", "{}", "{}",
        ))
    print(f"  Inserted {len(NODES)} nodes")

# ── Edges ────────────────────────────────────────────────────────────────────

EDGES = [
    # Rust Web Framework project
    ("Rust Web Framework",          "axum middleware prototype",        "causal",     0.90),
    ("Rust Web Framework",          "tokio-rs/tokio GitHub",            "connection", 0.85),
    ("Rust Web Framework",          "The Rust Programming Language (book)", "connection", 0.80),
    ("Rust Web Framework",          "Zero-copy serialization approach", "causal",     0.75),
    ("Rust Web Framework",          "Jon Gjengset",                     "connection", 0.70),
    ("Rust Web Framework",          "Jon Gjengset async Rust YouTube series", "connection", 0.80),

    # ML Pipeline
    ("ML Pipeline Experiment",      "Focus trail as training signal",   "causal",     0.85),
    ("ML Pipeline Experiment",      "Adaptive entropy decay per node type", "causal", 0.78),
    ("ML Pipeline Experiment",      "Time-weighted PageRank variant",   "connection", 0.65),

    # SilentNode Core (existing)
    ("Cognitive Graph Engine",      "Barnes-Hut Gravity",               "causal",     0.90),
    ("Cognitive Graph Engine",      "Graph traversal BFS/DFS impl",     "causal",     0.80),
    ("Cognitive Graph Engine",      "Memory-mapped graph storage",      "connection", 0.70),
    ("Cognitive Graph Engine",      "Time-weighted PageRank variant",   "connection", 0.68),

    # Entropy Engine
    ("Entropy Engine",              "Adaptive entropy decay per node type", "causal", 0.88),
    ("Entropy Engine",              "Entropy decay formula v2",         "causal",     0.92),

    # WGSL shader
    ("WGSL node shader v3",         "SilentNode Core",                  "causal",     0.85),
    ("WGSL node shader v3",         "Particle System",                  "connection", 0.75),

    # Database chain
    ("Database Internals by Alex Petrov", "SQLite + Surreal Storage",   "connection", 0.80),
    ("Database Internals by Alex Petrov", "SQLite WAL benchmark script","causal",     0.75),
    ("Database Internals by Alex Petrov", "Alex Petrov",               "connection", 0.95),
    ("SQLite official docs",        "SQLite + Surreal Storage",         "connection", 0.85),
    ("SQLite WAL benchmark script", "Debugging the WAL corruption issue","temporal",  0.88),

    # Distributed systems thread
    ("MIT 6.824 Distributed Systems lectures", "Distributed consensus without leader election", "causal", 0.82),
    ("Designing Data-Intensive Applications",  "Event sourcing for node history", "connection", 0.78),
    ("Designing Data-Intensive Applications",  "Graph partitioning for cache locality","connection",0.65),

    # Focus-related
    ("Focus trail as training signal", "Adaptive entropy decay per node type", "resonance", 0.72),
    ("Focus trail as training signal", "Cognitive load estimation from typing speed","connection",0.68),

    # People connections
    ("Jon Gjengset", "Jon Gjengset async Rust YouTube series", "connection", 0.95),
    ("Jon Gjengset", "Programming Rust 2nd Ed",               "connection", 0.70),

    # Process connections
    ("Daily code review routine",   "Benchmark → Profile → Fix",        "temporal",   0.70),
    ("Weekly architecture session", "Distributed consensus without leader election","connection",0.65),
    ("Weekly architecture session", "Memory-mapped graph storage",       "connection", 0.60),
]

def insert_edges(cur):
    count = 0
    for src_content, dst_content, etype, weight in EDGES:
        src = NODE_IDS.get(src_content)
        dst = NODE_IDS.get(dst_content)
        if not src:
            # Try existing nodes from DB
            cur.execute("SELECT id FROM node WHERE content=?", (src_content,))
            r = cur.fetchone()
            src = r[0] if r else None
        if not dst:
            cur.execute("SELECT id FROM node WHERE content=?", (dst_content,))
            r = cur.fetchone()
            dst = r[0] if r else None
        if src and dst:
            cur.execute("""
                INSERT OR IGNORE INTO edge (source_id, target_id, edge_type, weight, created_at, last_active)
                VALUES (?,?,?,?,?,?)
            """, (src, dst, etype, weight, ts(days_ago=random.randint(1,30)), ts(days_ago=random.randint(0,5))))
            count += 1
    print(f"  Inserted {count} edges")

# ── Focus Events ─────────────────────────────────────────────────────────────

def nid(content: str) -> str | None:
    return NODE_IDS.get(content)

# Realistic focus sessions — ordered sequences (most recent first)
SESSIONS = [
    # Session 1: deep Rust work (today)
    [("Rust Web Framework", 0.1,  120*60, "deep_work"),
     ("axum middleware prototype", 0.1, 90*60, "deep_work"),
     ("tokio-rs/tokio GitHub", 0.15, 30*60, "read"),
     ("Jon Gjengset async Rust YouTube series", 0.2, 45*60, "read"),
     ("Zero-copy serialization approach", 0.25, 20*60, "edit")],

    # Session 2: ML research (1 day ago)
    [("ML Pipeline Experiment", 1.0, 90*60, "deep_work"),
     ("Focus trail as training signal", 1.1, 60*60, "deep_work"),
     ("Adaptive entropy decay per node type", 1.2, 45*60, "edit"),
     ("Time-weighted PageRank variant", 1.3, 20*60, "read")],

    # Session 3: graph engine work (2 days ago)
    [("Cognitive Graph Engine", 2.0, 180*60, "deep_work"),
     ("Graph traversal BFS/DFS impl", 2.1, 120*60, "deep_work"),
     ("Memory-mapped graph storage", 2.2, 30*60, "edit"),
     ("Graph partitioning for cache locality", 2.3, 20*60, "read")],

    # Session 4: reading (3 days ago)
    [("The Rust Programming Language (book)", 3.0, 60*60, "read"),
     ("Programming Rust 2nd Ed", 3.1, 45*60, "read"),
     ("Zero-copy serialization approach", 3.2, 20*60, "edit")],

    # Session 5: database work (4 days ago)
    [("SQLite + Surreal Storage", 4.0, 120*60, "deep_work"),
     ("SQLite WAL benchmark script", 4.1, 90*60, "deep_work"),
     ("Database Internals by Alex Petrov", 4.2, 30*60, "read"),
     ("SQLite official docs", 4.3, 20*60, "read"),
     ("Debugging the WAL corruption issue", 4.4, 15*60, "glance")],

    # Session 6: distributed systems (5 days ago)
    [("MIT 6.824 Distributed Systems lectures", 5.0, 60*60, "read"),
     ("Distributed consensus without leader election", 5.1, 45*60, "edit"),
     ("Event sourcing for node history", 5.2, 20*60, "edit"),
     ("Designing Data-Intensive Applications", 5.3, 30*60, "read")],

    # Session 7: rendering work (6 days ago)
    [("WGSL node shader v3", 6.0, 180*60, "deep_work"),
     ("Particle System", 6.1, 90*60, "deep_work"),
     ("SilentNode Core", 6.2, 30*60, "glance")],

    # Session 8: people / review (7 days ago)
    [("Daily code review routine", 7.0, 45*60, "deep_work"),
     ("Weekly architecture session", 7.1, 60*60, "deep_work"),
     ("Jon Gjengset", 7.2, 15*60, "glance"),
     ("Jon Gjengset async Rust YouTube series", 7.3, 30*60, "read")],

    # Session 9: finance project (9 days ago)
    [("Personal Finance Tracker", 9.0, 90*60, "deep_work"),
     ("Benchmark → Profile → Fix", 9.1, 30*60, "edit"),
     ("SQLite WAL benchmark script", 9.2, 20*60, "glance")],

    # Session 10: memory / reflection (12 days ago)
    [("Rewrote the graph engine in 3 days", 12.0, 30*60, "read"),
     ("First time async clicked for me", 12.1, 15*60, "read"),
     ("Cognitive Graph Engine", 12.2, 60*60, "read")],

    # Older sessions
    [("Entropy Engine", 15.0, 120*60, "deep_work"),
     ("Entropy decay formula v2", 15.1, 90*60, "deep_work"),
     ("Adaptive entropy decay per node type", 15.2, 30*60, "edit")],

    [("SilentNode Core", 18.0, 120*60, "deep_work"),
     ("Cognitive Graph Engine", 18.1, 90*60, "deep_work"),
     ("Barnes-Hut Gravity", 18.2, 60*60, "deep_work")],

    [("The Rust Programming Language (book)", 20.0, 90*60, "read"),
     ("Rust Web Framework", 20.2, 60*60, "edit"),
     ("axum middleware prototype", 20.3, 45*60, "edit")],

    [("Silence Analyzer TF-IDF", 25.0, 60*60, "deep_work"),
     ("ML Pipeline Experiment", 25.1, 45*60, "edit"),
     ("Semantic similarity without embeddings", 25.2, 20*60, "edit")],
]

def insert_focus_events(cur):
    count = 0
    for session in SESSIONS:
        session_id = uid()
        for content, days_ago, duration, depth in session:
            node_id = NODE_IDS.get(content)
            if not node_id:
                cur.execute("SELECT id FROM node WHERE content=?", (content,))
                r = cur.fetchone()
                node_id = r[0] if r else None
            if not node_id:
                continue
            event_ts = ts(days_ago=days_ago)
            cur.execute("""
                INSERT INTO focus_event (node_id, timestamp, duration_seconds, depth, session_id)
                VALUES (?,?,?,?,?)
            """, (node_id, event_ts, float(duration), depth, session_id))
            count += 1
    print(f"  Inserted {count} focus events")

# ── Journal Entries ──────────────────────────────────────────────────────────

JOURNALS = [
    (0.5, "Spent the morning on the axum middleware. The async context propagation is tricky — lifetimes make the borrow checker unhappy with async closures. Need to revisit zero-copy approach."),
    (1.5, "Made good progress on the ML training pipeline. The focus trail gives surprisingly good signal for predicting next node. Markov chain with second-order transitions is clearly better."),
    (3.5, "Read two chapters of Programming Rust. The ownership model finally makes sense when I think of it as move semantics from C++. Need to connect this to the serialization idea."),
    (5.5, "Distributed consensus is harder than I thought. The MIT lectures are dense. Bookmarking the Raft paper section on log compaction — might relate to temporal snapshot design."),
    (8.0, "Long session on the graph engine. Rewrote the BFS to be iterative — stack-based. Stack overflow on deep graphs was embarrassing. Connected it to the partitioning idea."),
    (14.0,"Week of heavy work. Entropy system is mostly stable. The decay formula needs tuning — nodes that are connected to many others should decay slower. Added connection bonus."),
    (21.0, "Slow week. Finance tracker is stuck. The SQL schema for recurring expenses is wrong. Deprioritizing for now — putting in void zone mentally."),
]

def insert_journals(cur):
    count = 0
    for days_ago, content in JOURNALS:
        jid = uid()
        cur.execute("""
            INSERT INTO journal_entry (id, content, timestamp, season, mood_signature_json)
            VALUES (?,?,?,?,?)
        """, (jid, content, ts(days_ago=days_ago), "Spring", "{}"))
        # Link to related nodes
        words = content.lower().split()
        cur.execute("SELECT id, content FROM node")
        for node_id, node_content in cur.fetchall():
            if any(w in node_content.lower() for w in words if len(w) > 5):
                cur.execute("""
                    INSERT OR IGNORE INTO journal_link (journal_id, node_id) VALUES (?,?)
                """, (jid, node_id))
        count += 1
    print(f"  Inserted {count} journal entries")

# ── Main ─────────────────────────────────────────────────────────────────────

def main():
    conn = sqlite3.connect(DB)
    cur = conn.cursor()

    print("Seeding training data...")
    insert_nodes(cur)
    insert_edges(cur)
    insert_focus_events(cur)
    insert_journals(cur)

    conn.commit()
    conn.close()

    # Summary
    conn2 = sqlite3.connect(DB)
    cur2 = conn2.cursor()
    cur2.execute("SELECT COUNT(*) FROM node")
    nc = cur2.fetchone()[0]
    cur2.execute("SELECT COUNT(*) FROM focus_event")
    fc = cur2.fetchone()[0]
    cur2.execute("SELECT COUNT(*) FROM edge")
    ec = cur2.fetchone()[0]
    cur2.execute("SELECT COUNT(*) FROM journal_entry")
    jc = cur2.fetchone()[0]
    conn2.close()

    print(f"\nDatabase now has:")
    print(f"  {nc} nodes")
    print(f"  {ec} edges")
    print(f"  {fc} focus events")
    print(f"  {jc} journal entries")
    print("\nNow run: silentnode ml-train")

if __name__ == "__main__":
    main()
