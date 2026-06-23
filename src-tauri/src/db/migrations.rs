use crate::crypto::SecretStore;
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
            step_key TEXT NOT NULL DEFAULT '',
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
            target_area TEXT,
            proposed_change TEXT,
            created_at TEXT NOT NULL,
            reviewed_at TEXT
        );

        CREATE TABLE IF NOT EXISTS message_revisions (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            revision INTEGER NOT NULL DEFAULT 1,
            original_content TEXT NOT NULL DEFAULT '',
            edited_content TEXT NOT NULL DEFAULT '',
            editor TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS memory_versions (
            id TEXT PRIMARY KEY,
            memory_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            memory_type TEXT NOT NULL,
            key TEXT NOT NULL,
            old_value TEXT NOT NULL DEFAULT '',
            new_value TEXT NOT NULL DEFAULT '',
            source TEXT,
            provenance TEXT,
            created_at TEXT NOT NULL
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

    conn.execute(
        "ALTER TABLE agent_steps ADD COLUMN step_key TEXT NOT NULL DEFAULT ''",
        [],
    ).ok();

    migrate_step_key_column(conn)?;

    conn.execute_batch(
        "DROP INDEX IF EXISTS idx_agent_steps_run_step;
         CREATE INDEX IF NOT EXISTS idx_events_project_created ON events(project_id, created_at);
         CREATE INDEX IF NOT EXISTS idx_events_run_id ON events(run_id);
         CREATE INDEX IF NOT EXISTS idx_events_correlation_id ON events(correlation_id);
         CREATE INDEX IF NOT EXISTS idx_events_type_created ON events(event_type, created_at);
         CREATE INDEX IF NOT EXISTS idx_events_run_created ON events(run_id, created_at);
         CREATE INDEX IF NOT EXISTS idx_agent_messages_run_id ON agent_messages(run_id);
         CREATE INDEX IF NOT EXISTS idx_agent_steps_run_id ON agent_steps(run_id);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_steps_run_step_key ON agent_steps(run_id, step_key);
         CREATE INDEX IF NOT EXISTS idx_project_memory_project_type ON project_memory(project_id, memory_type);
         CREATE INDEX IF NOT EXISTS idx_memory_versions_memory_id ON memory_versions(memory_id);",
    )?;

    migrate_step_key_column(conn)?;
    migrate_old_api_key_column(conn)?;

    conn.execute("ALTER TABLE improvement_proposals ADD COLUMN target_area TEXT", []).ok();
    conn.execute("ALTER TABLE improvement_proposals ADD COLUMN proposed_change TEXT", []).ok();

    migrate_keychain_api_key(conn);

    Ok(())
}

fn migrate_step_key_column(conn: &Connection) -> Result<()> {
    let rows: Vec<(String, String, i32)> = {
        let mut stmt = conn
            .prepare("SELECT id, step_type, step_order FROM agent_steps WHERE step_key = '' OR step_key IS NULL")
            .ok();
        match stmt {
            Some(ref mut s) => s
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
                })
                .ok()
                .into_iter()
                .flat_map(|r| r.filter_map(|r| r.ok()))
                .collect(),
            None => return Ok(()),
        }
    };

    for (id, step_type, step_order) in rows {
        let step_key = if step_type.is_empty() {
            format!("step.{}", step_order)
        } else {
            format!("{}.{}", step_type, step_order)
        };
        conn.execute(
            "UPDATE agent_steps SET step_key = ?1 WHERE id = ?2",
            rusqlite::params![step_key, id],
        )
        .ok();
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
enum MigrationOutcome {
    NothingToMigrate,
    DecryptFailed,
    KeychainWriteFailed,
    MarkerUpdateFailed,
    Success,
}

/// Attempt to migrate the API key from SQLite to the OS keychain.
fn migrate_keychain_api_key(conn: &Connection) {
    use crate::crypto::{KeychainSecretStore, LocalEncryptedSecretStore, SecretStore};
    let keychain = match KeychainSecretStore::try_new() {
        Ok(k) => k,
        Err(_) => return,
    };
    let local = LocalEncryptedSecretStore::new();
    let _outcome = migrate_keychain_api_key_inner(conn, &local, &keychain);
}

/// Testable inner — returns outcome for each path.
fn migrate_keychain_api_key_inner(
    conn: &Connection,
    local_decrypt: &dyn crate::crypto::SecretStore,
    keychain_store: &dyn crate::crypto::SecretStore,
) -> MigrationOutcome {
    let encrypted: Option<String> = conn
        .query_row("SELECT encrypted_api_key FROM model_configs WHERE encrypted_api_key != '' AND encrypted_api_key != 'keychain_stored' LIMIT 1", [], |r| r.get(0))
        .ok();

    let encrypted = match encrypted {
        Some(e) => e,
        None => return MigrationOutcome::NothingToMigrate,
    };

    let plaintext = match local_decrypt.decrypt(&encrypted) {
        Ok(p) => p,
        Err(_) => return MigrationOutcome::DecryptFailed,
    };

    match keychain_store.encrypt(&plaintext) {
        Ok(_) => {
            match conn.execute(
                "UPDATE model_configs SET encrypted_api_key = 'keychain_stored' WHERE encrypted_api_key = ?1",
                rusqlite::params![encrypted],
            ) {
                Ok(rows) if rows > 0 => MigrationOutcome::Success,
                _ => MigrationOutcome::MarkerUpdateFailed,
            }
        }
        Err(_) => MigrationOutcome::KeychainWriteFailed,
    }
}

fn migrate_old_api_key_column(conn: &Connection) -> Result<()> {
    let has_encrypted = conn
        .prepare("SELECT encrypted_api_key FROM model_configs LIMIT 0")
        .is_ok();

    if !has_encrypted {
        conn.execute(
            "ALTER TABLE model_configs ADD COLUMN encrypted_api_key TEXT NOT NULL DEFAULT ''",
            [],
        )
        .ok();
    }

    let has_old_api_key: bool = conn
        .prepare("SELECT api_key FROM model_configs LIMIT 0")
        .is_ok();

    if !has_old_api_key {
        return Ok(());
    }

    let store = crate::crypto::LocalEncryptedSecretStore::new();

    let mut stmt = conn
        .prepare("SELECT id, api_key, encrypted_api_key FROM model_configs")
        .ok();

    if let Some(ref mut stmt) = stmt {
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                ))
            })
            .ok()
            .into_iter()
            .flat_map(|r| r.filter_map(|r| r.ok()))
            .collect();

        for (id, plaintext_key, encrypted_key) in rows {
            if plaintext_key.is_empty() || !encrypted_key.is_empty() {
                continue;
            }
            if let Ok(enc) = store.encrypt(&plaintext_key) {
                conn.execute(
                    "UPDATE model_configs SET encrypted_api_key = ?1 WHERE id = ?2",
                    rusqlite::params![enc, id],
                )
                .ok();
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::LocalEncryptedSecretStore;
    use crate::crypto::SecretStore;

    fn setup_model_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE model_configs (id TEXT PRIMARY KEY, base_url TEXT, encrypted_api_key TEXT, model TEXT, temperature REAL, max_tokens INTEGER, created_at TEXT, updated_at TEXT)").unwrap();
        conn
    }

    struct FakeKeychainStore { stored: std::sync::Mutex<Option<String>>, fail_write: bool }

    impl FakeKeychainStore {
        fn new() -> Self { Self { stored: std::sync::Mutex::new(None), fail_write: false } }
        fn with_fail_write() -> Self { Self { stored: std::sync::Mutex::new(None), fail_write: true } }
    }

    impl SecretStore for FakeKeychainStore {
        fn encrypt(&self, plaintext: &str) -> Result<String, String> {
            if self.fail_write { return Err("keychain write failed".into()); }
            *self.stored.lock().unwrap() = Some(plaintext.to_string());
            Ok("keychain_stored".into())
        }
        fn decrypt(&self, _: &str) -> Result<String, String> {
            self.stored.lock().unwrap().clone().ok_or_else(|| "no key".into())
        }
    }

    #[test]
    fn migration_success_writes_keychain_marker() {
        let db = setup_model_db();
        let local = LocalEncryptedSecretStore::new();
        let ct = local.encrypt("sk-legacy-key").unwrap();
        db.execute("INSERT INTO model_configs (id,encrypted_api_key,base_url,model,temperature,max_tokens,created_at,updated_at) VALUES ('c1',?1,'https://t.com','gpt',0.7,4096,datetime('now'),datetime('now'))", rusqlite::params![ct]).unwrap();
        let kc = FakeKeychainStore::new();
        let outcome = migrate_keychain_api_key_inner(&db, &local, &kc);
        assert_eq!(outcome, MigrationOutcome::Success);
        let s: String = db.query_row("SELECT encrypted_api_key FROM model_configs WHERE id='c1'", [], |r| r.get(0)).unwrap();
        assert_eq!(s, "keychain_stored");
        assert_eq!(kc.decrypt("").unwrap(), "sk-legacy-key");
    }

    #[test]
    fn migration_write_failure_preserves_ciphertext() {
        let db = setup_model_db();
        let local = LocalEncryptedSecretStore::new();
        let ct = local.encrypt("sk-legacy-key").unwrap();
        db.execute("INSERT INTO model_configs (id,encrypted_api_key,base_url,model,temperature,max_tokens,created_at,updated_at) VALUES ('c1',?1,'https://t.com','gpt',0.7,4096,datetime('now'),datetime('now'))", rusqlite::params![ct]).unwrap();
        let kc = FakeKeychainStore::with_fail_write();
        let outcome = migrate_keychain_api_key_inner(&db, &local, &kc);
        assert_eq!(outcome, MigrationOutcome::KeychainWriteFailed);
        let s: String = db.query_row("SELECT encrypted_api_key FROM model_configs WHERE id='c1'", [], |r| r.get(0)).unwrap();
        assert_eq!(s, ct, "old ciphertext must survive");
    }

    #[test]
    fn migration_failure_decrypt_still_works() {
        let db = setup_model_db();
        let local = LocalEncryptedSecretStore::new();
        let ct = local.encrypt("sk-legacy-key").unwrap();
        db.execute("INSERT INTO model_configs (id,encrypted_api_key,base_url,model,temperature,max_tokens,created_at,updated_at) VALUES ('c1',?1,'https://t.com','gpt',0.7,4096,datetime('now'),datetime('now'))", rusqlite::params![ct]).unwrap();
        let kc = FakeKeychainStore::with_fail_write();
        migrate_keychain_api_key_inner(&db, &local, &kc);
        let s: String = db.query_row("SELECT encrypted_api_key FROM model_configs WHERE id='c1'", [], |r| r.get(0)).unwrap();
        assert_eq!(crate::crypto::decrypt_saved_api_key(&s).unwrap(), "sk-legacy-key");
    }

    #[test]
    fn marker_update_failure_keeps_old_ciphertext() {
        let db = setup_model_db();
        let local = LocalEncryptedSecretStore::new();
        let ct = local.encrypt("sk-legacy-key").unwrap();
        db.execute("INSERT INTO model_configs (id,encrypted_api_key,base_url,model,temperature,max_tokens,created_at,updated_at) VALUES ('c1',?1,'https://t.com','gpt',0.7,4096,datetime('now'),datetime('now'))", rusqlite::params![ct]).unwrap();

        // Block UPDATE via trigger
        db.execute_batch(
            "CREATE TRIGGER block_model_config_update BEFORE UPDATE ON model_configs
             BEGIN SELECT RAISE(FAIL, 'marker update blocked'); END;"
        ).unwrap();

        let kc = FakeKeychainStore::new(); // keychain write succeeds
        let outcome = migrate_keychain_api_key_inner(&db, &local, &kc);
        assert_eq!(outcome, MigrationOutcome::MarkerUpdateFailed);

        let s: String = db.query_row("SELECT encrypted_api_key FROM model_configs WHERE id='c1'", [], |r| r.get(0)).unwrap();
        assert_eq!(s, ct, "old ciphertext must survive marker update failure");
        assert_eq!(crate::crypto::decrypt_saved_api_key(&s).unwrap(), "sk-legacy-key");
    }
}
