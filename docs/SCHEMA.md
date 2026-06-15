# Database Schema

SnarkTerm uses SQLite for local sessions, stats, commentary, and achievements. Command output is not stored by default.

## `sessions`

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    shell TEXT NOT NULL,
    cwd TEXT,
    personality_level TEXT NOT NULL,
    roast_intensity INTEGER NOT NULL
);
```

## `commands`

```sql
CREATE TABLE commands (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command TEXT NOT NULL,
    normalized_command TEXT NOT NULL,
    cwd TEXT,
    exit_code INTEGER,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    duration_ms INTEGER,
    used_sudo INTEGER NOT NULL DEFAULT 0,
    was_dangerous INTEGER NOT NULL DEFAULT 0,
    was_force_push INTEGER NOT NULL DEFAULT 0,
    was_restart_fix INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);
```

## `commentary`

```sql
CREATE TABLE commentary (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    command_id TEXT,
    text TEXT NOT NULL,
    severity TEXT NOT NULL,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id),
    FOREIGN KEY(command_id) REFERENCES commands(id)
);
```

## `stats_daily`

```sql
CREATE TABLE stats_daily (
    date TEXT PRIMARY KEY,
    failed_commands INTEGER NOT NULL DEFAULT 0,
    sudo_count INTEGER NOT NULL DEFAULT 0,
    force_push_count INTEGER NOT NULL DEFAULT 0,
    restart_fix_count INTEGER NOT NULL DEFAULT 0,
    dangerous_command_count INTEGER NOT NULL DEFAULT 0,
    long_running_count INTEGER NOT NULL DEFAULT 0
);
```

## `achievements`

```sql
CREATE TABLE achievements (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    unlocked_at TEXT NOT NULL,
    session_id TEXT,
    command_id TEXT
);
```

## `settings`

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

## `command_patterns`

```sql
CREATE TABLE command_patterns (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    category TEXT NOT NULL,
    hit_count INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT
);
```

## Initial Statistics

Tracked values should include:

- Failed commands.
- Times `sudo` was used.
- Force pushes.
- Restart-based fixes.
- Dangerous command attempts.
- Long-running commands.
- Most failed command.
- Most used command.
- Most restarted service.
- Average build duration.
