DEFINE TABLE IF NOT EXISTS node SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON node TYPE record;
DEFINE FIELD IF NOT EXISTS type ON node TYPE string ASSERT $value IN ["idea","memory","project","person","artifact","media","process","world","ghost","fossil"];
DEFINE FIELD IF NOT EXISTS content ON node TYPE string;
DEFINE FIELD IF NOT EXISTS metadata ON node TYPE object;
DEFINE FIELD IF NOT EXISTS entropy ON node TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS gravity ON node TYPE float DEFAULT 1.0;
DEFINE FIELD IF NOT EXISTS velocity ON node TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS access_count ON node TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS created_at ON node TYPE datetime;
DEFINE FIELD IF NOT EXISTS accessed_at ON node TYPE datetime;
DEFINE FIELD IF NOT EXISTS is_ghost ON node TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS is_fossil ON node TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS is_void ON node TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS position ON node TYPE object;
DEFINE FIELD IF NOT EXISTS aura_color ON node TYPE string;
DEFINE FIELD IF NOT EXISTS soul_signature ON node TYPE object;
DEFINE FIELD IF NOT EXISTS civilization_id ON node TYPE option<uuid>;

DEFINE TABLE IF NOT EXISTS connects SCHEMAFULL TYPE RELATION;
DEFINE FIELD IF NOT EXISTS source_id ON connects TYPE string;
DEFINE FIELD IF NOT EXISTS target_id ON connects TYPE string;
DEFINE FIELD IF NOT EXISTS weight ON connects TYPE float DEFAULT 1.0;
DEFINE FIELD IF NOT EXISTS type ON connects TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON connects TYPE datetime;
DEFINE FIELD IF NOT EXISTS last_active ON connects TYPE datetime;

DEFINE TABLE IF NOT EXISTS focus_event SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS node_id ON focus_event TYPE record<node>;
DEFINE FIELD IF NOT EXISTS timestamp ON focus_event TYPE datetime;
DEFINE FIELD IF NOT EXISTS duration ON focus_event TYPE float;
DEFINE FIELD IF NOT EXISTS depth ON focus_event TYPE string;
DEFINE FIELD IF NOT EXISTS session_id ON focus_event TYPE uuid;

DEFINE TABLE IF NOT EXISTS temporal_snapshot SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS node_id ON temporal_snapshot TYPE record<node>;
DEFINE FIELD IF NOT EXISTS snapshot ON temporal_snapshot TYPE object;
DEFINE FIELD IF NOT EXISTS timestamp ON temporal_snapshot TYPE datetime;
DEFINE FIELD IF NOT EXISTS change_type ON temporal_snapshot TYPE string;

DEFINE TABLE IF NOT EXISTS journal_entry SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS content ON journal_entry TYPE string;
DEFINE FIELD IF NOT EXISTS timestamp ON journal_entry TYPE datetime;
DEFINE FIELD IF NOT EXISTS linked_nodes ON journal_entry TYPE array<record<node>>;
DEFINE FIELD IF NOT EXISTS mood_signature ON journal_entry TYPE object;
DEFINE FIELD IF NOT EXISTS season ON journal_entry TYPE option<string>;

DEFINE TABLE IF NOT EXISTS lore_entry SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS title ON lore_entry TYPE string;
DEFINE FIELD IF NOT EXISTS arc_type ON lore_entry TYPE string;
DEFINE FIELD IF NOT EXISTS timestamp ON lore_entry TYPE datetime;
DEFINE FIELD IF NOT EXISTS linked_nodes ON lore_entry TYPE array<record<node>>;
DEFINE FIELD IF NOT EXISTS narrative ON lore_entry TYPE string;
DEFINE FIELD IF NOT EXISTS significance ON lore_entry TYPE float;

DEFINE TABLE IF NOT EXISTS silent_contract SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS related_node ON silent_contract TYPE record<node>;
DEFINE FIELD IF NOT EXISTS detected_at ON silent_contract TYPE datetime;
DEFINE FIELD IF NOT EXISTS intensity ON silent_contract TYPE float;
DEFINE FIELD IF NOT EXISTS age_days ON silent_contract TYPE float;
DEFINE FIELD IF NOT EXISTS state ON silent_contract TYPE string;

DEFINE TABLE IF NOT EXISTS process_record SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS pid ON process_record TYPE int;
DEFINE FIELD IF NOT EXISTS name ON process_record TYPE string;
DEFINE FIELD IF NOT EXISTS linked_node ON process_record TYPE record<node>;
DEFINE FIELD IF NOT EXISTS started_at ON process_record TYPE datetime;
DEFINE FIELD IF NOT EXISTS ended_at ON process_record TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS cpu_usage ON process_record TYPE float;
DEFINE FIELD IF NOT EXISTS memory_mb ON process_record TYPE float;
