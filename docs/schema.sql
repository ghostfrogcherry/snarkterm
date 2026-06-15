PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    shell TEXT NOT NULL,
    cwd TEXT,
    personality_level TEXT NOT NULL,
    roast_intensity INTEGER NOT NULL,
    work_mode INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS commands (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command TEXT,
    normalized_command TEXT,
    cwd TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    duration_ms INTEGER,
    used_sudo INTEGER NOT NULL DEFAULT 0,
    was_dangerous INTEGER NOT NULL DEFAULT 0,
    was_force_push INTEGER NOT NULL DEFAULT 0,
    was_chmod_777 INTEGER NOT NULL DEFAULT 0,
    was_restart_fix INTEGER NOT NULL DEFAULT 0,
    was_curl_pipe_shell INTEGER NOT NULL DEFAULT 0,
    looked_copied INTEGER NOT NULL DEFAULT 0,
    production_context INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS command_results (
    id TEXT PRIMARY KEY,
    command_id TEXT NOT NULL,
    exit_status INTEGER,
    signal INTEGER,
    succeeded INTEGER,
    output_sample_redacted TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(command_id) REFERENCES commands(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS commentary (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command_id TEXT,
    text TEXT NOT NULL,
    severity TEXT NOT NULL,
    personality TEXT NOT NULL,
    source TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(command_id) REFERENCES commands(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS rule_matches (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command_id TEXT,
    rule_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(command_id) REFERENCES commands(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY,
    achievement_key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    commentary TEXT NOT NULL,
    unlocked_at TEXT NOT NULL,
    session_id TEXT,
    command_id TEXT,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE SET NULL,
    FOREIGN KEY(command_id) REFERENCES commands(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS user_stats (
    key TEXT PRIMARY KEY,
    value INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS stats_daily (
    date TEXT PRIMARY KEY,
    failed_commands INTEGER NOT NULL DEFAULT 0,
    sudo_count INTEGER NOT NULL DEFAULT 0,
    force_push_count INTEGER NOT NULL DEFAULT 0,
    chmod_777_count INTEGER NOT NULL DEFAULT 0,
    restart_fix_count INTEGER NOT NULL DEFAULT 0,
    dangerous_command_count INTEGER NOT NULL DEFAULT 0,
    long_running_count INTEGER NOT NULL DEFAULT 0,
    total_build_wait_ms INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_events (
    id TEXT PRIMARY KEY,
    plugin_name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    status TEXT NOT NULL,
    message TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_commands_session ON commands(session_id);
CREATE INDEX IF NOT EXISTS idx_commands_normalized ON commands(normalized_command);
CREATE INDEX IF NOT EXISTS idx_commentary_session ON commentary(session_id);
CREATE INDEX IF NOT EXISTS idx_rule_matches_command ON rule_matches(command_id);
