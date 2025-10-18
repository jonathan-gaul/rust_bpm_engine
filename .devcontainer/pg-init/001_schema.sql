-- simple schema for persistence

CREATE TABLE IF NOT EXISTS process_definitions (
  id TEXT PRIMARY KEY,
  definition JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS process_instances (
  id TEXT PRIMARY KEY,
  process_def_id TEXT NOT NULL REFERENCES process_definitions(id) ON DELETE CASCADE,
  state TEXT NOT NULL,
  variables JSONB NOT NULL,
  current_node_ids JSONB NOT NULL,
  active_tokens INTEGER NOT NULL,
  join_counters JSONB NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);
