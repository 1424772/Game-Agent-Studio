use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            game_type TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS model_configs (
            id TEXT PRIMARY KEY,
            base_url TEXT NOT NULL,
            encrypted_api_key TEXT NOT NULL,
            model TEXT NOT NULL,
            temperature REAL NOT NULL DEFAULT 0.7,
            max_tokens INTEGER NOT NULL DEFAULT 4096,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_runs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            task_description TEXT NOT NULL,
            workflow_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_messages (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            metadata TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_steps (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            agent_name TEXT NOT NULL,
            step_order INTEGER NOT NULL,
            step_type TEXT NOT NULL,
            input_json TEXT,
            output_json TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            error_message TEXT,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            project_id TEXT,
            run_id TEXT,
            actor TEXT,
            event_type TEXT NOT NULL,
            event_data TEXT NOT NULL DEFAULT '{}',
            severity TEXT NOT NULL DEFAULT 'info',
            correlation_id TEXT,
            redaction_level TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS project_memory (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            memory_type TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL DEFAULT '',
            layer TEXT NOT NULL DEFAULT 'L1',
            scope TEXT NOT NULL DEFAULT 'project',
            source TEXT,
            confidence REAL NOT NULL DEFAULT 1.0,
            version INTEGER NOT NULL DEFAULT 1,
            provenance TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS user_preferences (
            id TEXT PRIMARY KEY,
            preference_key TEXT NOT NULL UNIQUE,
            preference_value TEXT NOT NULL DEFAULT '',
            confidence REAL NOT NULL DEFAULT 0.5,
            evidence TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS exports (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            export_type TEXT NOT NULL,
            file_path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            doc_type TEXT NOT NULL,
            source_path TEXT,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS document_chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            embedding_json TEXT,
            metadata TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS retrieval_runs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            query_text TEXT NOT NULL,
            rewritten_queries TEXT,
            strategy TEXT,
            result_count INTEGER NOT NULL DEFAULT 0,
            duration_ms BIGINT NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS retrieval_hits (
            id TEXT PRIMARY KEY,
            retrieval_run_id TEXT NOT NULL,
            chunk_id TEXT NOT NULL,
            score REAL NOT NULL DEFAULT 0.0,
            rank INTEGER NOT NULL DEFAULT 0,
            used_by_agent TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS improvement_proposals (
            id TEXT PRIMARY KEY,
            proposal_type TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            evidence TEXT,
            risk_level TEXT,
            status TEXT NOT NULL DEFAULT 'draft',
            requires_human_approval INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            reviewed_at TEXT
        );",
    )?;

    conn.execute(
        "ALTER TABLE events ADD COLUMN run_id TEXT",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE events ADD COLUMN actor TEXT",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE events ADD COLUMN severity TEXT NOT NULL DEFAULT 'info'",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE events ADD COLUMN correlation_id TEXT",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE events ADD COLUMN redaction_level TEXT",
        [],
    ).ok();

    conn.execute(
        "ALTER TABLE project_memory ADD COLUMN layer TEXT NOT NULL DEFAULT 'L1'",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE project_memory ADD COLUMN scope TEXT NOT NULL DEFAULT 'project'",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE project_memory ADD COLUMN source TEXT",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE project_memory ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE project_memory ADD COLUMN version INTEGER NOT NULL DEFAULT 1",
        [],
    ).ok();
    conn.execute(
        "ALTER TABLE project_memory ADD COLUMN provenance TEXT",
        [],
    ).ok();

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_events_project_created ON events(project_id, created_at);
         CREATE INDEX IF NOT EXISTS idx_events_run_id ON events(run_id);
         CREATE INDEX IF NOT EXISTS idx_agent_messages_run_id ON agent_messages(run_id);
         CREATE INDEX IF NOT EXISTS idx_agent_steps_run_id ON agent_steps(run_id);
         CREATE INDEX IF NOT EXISTS idx_project_memory_project_type ON project_memory(project_id, memory_type);",
    )?;

    Ok(())
}
