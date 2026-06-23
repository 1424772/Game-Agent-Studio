use crate::models::{Document, DocumentChunk, RetrievalHit, RetrievalRun};
use crate::AppState;
use sha2::Digest;
use tauri::State;

const CHUNK_SIZE: usize = 2000;
const MIN_LIMIT: i32 = 1;
const MAX_LIMIT: i32 = 20;

#[derive(Debug)]
pub struct RetrievalResult {
    pub run: RetrievalRun,
    pub hits: Vec<RetrievalHit>,
    pub excerpts: Vec<(String, String, String)>, // (chunk_id, doc_title, chunk_excerpt)
}

/// Internal service: search + record retrieval trace + return context excerpts.
/// Called by both the UI command and the agent workflow engine.
pub fn retrieve_for_context(
    db: &mut rusqlite::Connection, project_id: &str, query: &str, limit: i32,
    usage_context: Option<(&str, &str, &str)>, // (run_id, step_key, agent_name)
) -> Result<RetrievalResult, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("Search query must not be empty".to_string());
    }
    if trimmed.len() > 500 {
        return Err(crate::models::sanitize_error("Query too long (max 500 chars)".into()));
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() > 20 {
        return Err(crate::models::sanitize_error(format!(
            "Too many search terms ({}). Maximum is 20.", words.len()
        )));
    }
    let like_words: Vec<String> = words.iter().map(|w| format!("%{}%", w)).collect();
    let lim = limit.clamp(MIN_LIMIT, MAX_LIMIT);

    let now = chrono::Utc::now().to_rfc3339();
    let start = std::time::Instant::now();

    let mut sql = String::from(
        "SELECT c.id, c.document_id, c.chunk_index, c.content, c.metadata, d.title, d.doc_type FROM document_chunks c JOIN documents d ON c.document_id=d.id WHERE d.project_id=?1 AND ("
    );
    for i in 0..like_words.len() {
        if i > 0 { sql.push_str(" OR "); }
        sql.push_str(&format!("c.content LIKE ?{}", i + 2));
    }
    sql.push_str(") LIMIT ?");

    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    params.push(Box::new(project_id.to_string()));
    for w in &like_words { params.push(Box::new(w.clone())); }
    params.push(Box::new((lim * 3) as i64)); // fetch extra for scoring

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let raw: Vec<(String, String, i32, String, Option<String>, String, String)> =
        db.prepare(&sql).map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .query_map(param_refs.as_slice(), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4).ok(), r.get(5)?, r.get(6)?))
        }).map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let score_words: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

    let mut scored: Vec<(f64, &(String, String, i32, String, Option<String>, String, String))> = raw.iter().map(|item| {
        let lower = item.3.to_lowercase();
        let score = score_words.iter().filter(|w| lower.contains(w.as_str())).count() as f64;
        (score, item)
    }).collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(lim as usize);

    let duration_ms = start.elapsed().as_millis() as i64;
    let hit_count = scored.len() as i32;
    let run_id = uuid::Uuid::new_v4().to_string();

    let usage_meta = usage_context.map(|(rid, sk, an)| {
        serde_json::json!({"run_id": rid, "step_key": sk, "agent_name": an}).to_string()
    });

    let tx = db.transaction().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    tx.execute(
        "INSERT INTO retrieval_runs (id, project_id, query_text, rewritten_queries, strategy, result_count, duration_ms, created_at) VALUES (?1,?2,?3,NULL,'keyword',?4,?5,?6)",
        rusqlite::params![run_id, project_id, trimmed, hit_count, duration_ms, now],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let mut hits = Vec::new();
    let mut excerpts = Vec::new();

    for (rank, (score, item)) in scored.iter().enumerate() {
        let hit_id = uuid::Uuid::new_v4().to_string();
        let excerpt: String = crate::models::sanitize_error(item.3.chars().take(300).collect());
        let used = usage_meta.clone();

        tx.execute(
            "INSERT INTO retrieval_hits (id, retrieval_run_id, chunk_id, score, rank, used_by_agent, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![hit_id, run_id, item.0, score, rank as i32, used, now],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        hits.push(RetrievalHit {
            id: hit_id.clone(), retrieval_run_id: run_id.clone(), chunk_id: item.0.clone(),
            score: *score, rank: rank as i32, used_by_agent: used.clone(), created_at: now.clone(),
        });
        excerpts.push((item.0.clone(), item.5.clone(), excerpt));
    }

    tx.commit().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let run = RetrievalRun {
        id: run_id, project_id: project_id.to_string(), query_text: trimmed.to_string(),
        rewritten_queries: None, strategy: Some("keyword".into()),
        result_count: hit_count, duration_ms, created_at: now,
    };

    Ok(RetrievalResult { run, hits, excerpts })
}

// ════════════════════════════════════════════════════════════
// Tauri Commands
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn create_document(
    state: State<AppState>, project_id: String, title: String, content: String,
    doc_type: String, source_path: Option<String>,
) -> Result<Document, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO documents (id,project_id,title,content,doc_type,source_path,chunk_count,created_at) VALUES (?1,?2,?3,?4,?5,?6,0,?7)",
        rusqlite::params![id, project_id, title, content, doc_type, source_path, now],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(Document { id, project_id, title, content, doc_type, source_path, chunk_count: 0, created_at: now })
}

#[tauri::command]
pub fn chunk_document(
    state: State<AppState>, document_id: String,
) -> Result<Document, String> {
    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    let (title, doc_type, source_path, content): (String, String, Option<String>, String) = db.query_row(
        "SELECT title, doc_type, source_path, content FROM documents WHERE id=?1", rusqlite::params![document_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2).ok(), r.get(3)?)),
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let tx = db.transaction().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    tx.execute("DELETE FROM document_chunks WHERE document_id=?1", rusqlite::params![document_id])
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let content_hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
    let base_meta = serde_json::json!({
        "document_id": document_id,
        "document_title": title,
        "doc_type": doc_type,
        "source_path": source_path,
        "chunk_strategy": format!("paragraph_split_{}", CHUNK_SIZE),
        "content_hash": content_hash,
        "version": 1,
        "confidence": 1.0,
        "created_by": "chunk_document_command",
        "source": format!("document:{}", document_id),
        "provenance": serde_json::json!({
            "source_type": "document",
            "document_id": document_id,
            "ingestion_method": "chunk_document_command",
        }).to_string(),
    });

    let mut chunk_index: i32 = 0;
    let paragraphs: Vec<&str> = content.split("\n\n").collect();
    let mut current = String::new();

    for para in paragraphs {
        if current.len() + para.len() > CHUNK_SIZE && !current.is_empty() {
            chunk_index += 1;
            let cid = uuid::Uuid::new_v4().to_string();
            let mut chunk_meta = base_meta.clone();
            chunk_meta["chunk_index"] = serde_json::json!(chunk_index);
            tx.execute(
                "INSERT INTO document_chunks (id,document_id,chunk_index,content,metadata,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
                rusqlite::params![cid, document_id, chunk_index, current.trim(), chunk_meta.to_string(), now],
            ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            current.clear();
        }
        if !current.is_empty() { current.push_str("\n\n"); }
        current.push_str(para);
    }

    if !current.trim().is_empty() {
        chunk_index += 1;
        let cid = uuid::Uuid::new_v4().to_string();
        let mut chunk_meta = base_meta.clone();
        chunk_meta["chunk_index"] = serde_json::json!(chunk_index);
        tx.execute(
            "INSERT INTO document_chunks (id,document_id,chunk_index,content,metadata,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![cid, document_id, chunk_index, current.trim(), chunk_meta.to_string(), now],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    }

    tx.execute(
        "UPDATE documents SET chunk_count=?1 WHERE id=?2", rusqlite::params![chunk_index, document_id],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    tx.commit().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    db.query_row(
        "SELECT id, project_id, title, content, doc_type, source_path, chunk_count, created_at FROM documents WHERE id=?1",
        rusqlite::params![document_id], |r| {
            Ok(Document { id: r.get(0)?, project_id: r.get(1)?, title: r.get(2)?, content: r.get(3)?, doc_type: r.get(4)?, source_path: r.get(5).ok(), chunk_count: r.get(6)?, created_at: r.get(7)? })
        },
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))
}

#[tauri::command]
pub fn list_documents(state: State<AppState>, project_id: String) -> Result<Vec<Document>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id,project_id,title,content,doc_type,source_path,chunk_count,created_at FROM documents WHERE project_id=?1 ORDER BY created_at DESC").map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let docs = stmt.query_map(rusqlite::params![project_id], |r| Ok(Document{id:r.get(0)?,project_id:r.get(1)?,title:r.get(2)?,content:r.get(3)?,doc_type:r.get(4)?,source_path:r.get(5).ok(),chunk_count:r.get(6)?,created_at:r.get(7)?})).map_err(|e|crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    Ok(docs)
}

#[tauri::command]
pub fn get_document_chunks(state: State<AppState>, document_id: String) -> Result<Vec<DocumentChunk>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id,document_id,chunk_index,content,embedding_json,metadata,created_at FROM document_chunks WHERE document_id=?1 ORDER BY chunk_index ASC").map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    let chunks = stmt.query_map(rusqlite::params![document_id], |r| Ok(DocumentChunk{id:r.get(0)?,document_id:r.get(1)?,chunk_index:r.get(2)?,content:r.get(3)?,embedding_json:r.get(4).ok(),metadata:r.get(5).ok(),created_at:r.get(6)?})).map_err(|e|crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    Ok(chunks)
}

#[tauri::command]
pub fn search_documents(
    state: State<AppState>, project_id: String, query: String, limit: Option<i32>,
) -> Result<(RetrievalRun, Vec<RetrievalHit>), String> {
    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let result = retrieve_for_context(&mut db, &project_id, &query, limit.unwrap_or(10), None)?;
    Ok((result.run, result.hits))
}

#[tauri::command]
pub fn get_retrieval_runs(state: State<AppState>, project_id: String) -> Result<Vec<RetrievalRun>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id,project_id,query_text,rewritten_queries,strategy,result_count,duration_ms,created_at FROM retrieval_runs WHERE project_id=?1 ORDER BY created_at DESC LIMIT 50").map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    let runs = stmt.query_map(rusqlite::params![project_id], |r| Ok(RetrievalRun{id:r.get(0)?,project_id:r.get(1)?,query_text:r.get(2)?,rewritten_queries:r.get(3).ok(),strategy:r.get(4).ok(),result_count:r.get(5)?,duration_ms:r.get(6)?,created_at:r.get(7)?})).map_err(|e|crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    Ok(runs)
}

#[tauri::command]
pub fn get_retrieval_hits(state: State<AppState>, retrieval_run_id: String) -> Result<Vec<RetrievalHit>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id,retrieval_run_id,chunk_id,score,rank,used_by_agent,created_at FROM retrieval_hits WHERE retrieval_run_id=?1 ORDER BY rank ASC").map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    let hits = stmt.query_map(rusqlite::params![retrieval_run_id], |r| Ok(RetrievalHit{id:r.get(0)?,retrieval_run_id:r.get(1)?,chunk_id:r.get(2)?,score:r.get(3)?,rank:r.get(4)?,used_by_agent:r.get(5).ok(),created_at:r.get(6)?})).map_err(|e|crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    Ok(hits)
}

#[derive(serde::Serialize, Clone)]
pub struct HitExcerpt {
    pub id: String,
    pub retrieval_run_id: String,
    pub chunk_id: String,
    pub score: f64,
    pub rank: i32,
    pub used_by_agent: Option<String>,
    pub created_at: String,
    pub doc_title: String,
    pub doc_type: String,
    pub chunk_excerpt: String,
    pub source: Option<String>,
    pub provenance: Option<String>,
}

#[tauri::command]
pub fn get_retrieval_hit_excerpts(
    state: State<AppState>, retrieval_run_id: String,
) -> Result<Vec<HitExcerpt>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare(
        "SELECT h.id, h.retrieval_run_id, h.chunk_id, h.score, h.rank, h.used_by_agent, h.created_at, c.content, c.metadata, d.title, d.doc_type FROM retrieval_hits h JOIN document_chunks c ON h.chunk_id=c.id JOIN documents d ON c.document_id=d.id WHERE h.retrieval_run_id=?1 ORDER BY h.rank ASC"
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let hits = stmt.query_map(rusqlite::params![retrieval_run_id], |r| {
        let content: String = r.get(7)?;
        let metadata_str: Option<String> = r.get(8).ok();
        let excerpt: String = crate::models::sanitize_error(content.chars().take(200).collect());
        let (source, provenance) = metadata_str.as_ref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .map(|v| {
                (v.get("source").and_then(|s| s.as_str()).map(|s| s.to_string()),
                 v.get("provenance").and_then(|p| p.as_str()).map(|s| s.to_string()))
            }).unwrap_or((None, None));
        Ok(HitExcerpt {
            id: r.get(0)?, retrieval_run_id: r.get(1)?, chunk_id: r.get(2)?,
            score: r.get(3)?, rank: r.get(4)?, used_by_agent: r.get(5).ok(),
            created_at: r.get(6)?, doc_title: r.get(9)?, doc_type: r.get(10)?,
            chunk_excerpt: excerpt, source, provenance,
        })
    }).map_err(|e| crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieve_empty_query_rejected() {
        let r = retrieve_for_context(
            &mut rusqlite::Connection::open_in_memory().unwrap(),
            "test", "", 5, None,
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not be empty"));
    }

    #[test]
    fn retrieve_long_query_rejected() {
        let long = "a".repeat(501);
        let r = retrieve_for_context(
            &mut rusqlite::Connection::open_in_memory().unwrap(),
            "test", &long, 5, None,
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("max 500"));
    }

    #[test]
    fn retrieve_too_many_words_rejected() {
        let q = (0..25).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
        let r = retrieve_for_context(
            &mut rusqlite::Connection::open_in_memory().unwrap(),
            "test", &q, 5, None,
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Maximum is 20"));
    }

    #[test]
    fn limit_clamped_to_range() {
        assert_eq!(0_i32.clamp(MIN_LIMIT, MAX_LIMIT), 1);
        assert_eq!((-5_i32).clamp(MIN_LIMIT, MAX_LIMIT), 1);
        assert_eq!(100_i32.clamp(MIN_LIMIT, MAX_LIMIT), 20);
    }

    // ── DB-level tests with in-memory SQLite ──

    fn setup_rag_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE documents (id TEXT PRIMARY KEY, project_id TEXT, title TEXT, content TEXT, doc_type TEXT, source_path TEXT, chunk_count INTEGER, created_at TEXT);
             CREATE TABLE document_chunks (id TEXT PRIMARY KEY, document_id TEXT, chunk_index INTEGER, content TEXT, embedding_json TEXT, metadata TEXT, created_at TEXT);
             CREATE TABLE retrieval_runs (id TEXT PRIMARY KEY, project_id TEXT, query_text TEXT, rewritten_queries TEXT, strategy TEXT, result_count INTEGER, duration_ms BIGINT, created_at TEXT);
             CREATE TABLE retrieval_hits (id TEXT PRIMARY KEY, retrieval_run_id TEXT, chunk_id TEXT, score REAL, rank INTEGER, used_by_agent TEXT, created_at TEXT);"
        ).unwrap();
        conn
    }

    #[test]
    fn retrieve_writes_used_by_agent() {
        let mut db = setup_rag_db();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute("INSERT INTO documents (id,project_id,title,content,doc_type,chunk_count,created_at) VALUES ('d1','proj1','Test Doc','hello world','game_design',0,?1)", rusqlite::params![now]).unwrap();
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,created_at) VALUES ('c1','d1',1,'hello world example content for search testing',?1)", rusqlite::params![now]).unwrap();

        let result = retrieve_for_context(
            &mut db, "proj1", "hello search", 5,
            Some(("run-1", "qa.review", "QAAgent")),
        ).unwrap();

        assert_eq!(result.run.query_text, "hello search");
        assert!(!result.hits.is_empty());
        for hit in &result.hits {
            let used = hit.used_by_agent.as_ref().unwrap();
            assert!(used.contains("run-1"));
            assert!(used.contains("qa.review"));
            assert!(used.contains("QAAgent"));
        }

        let db_count: i32 = db.query_row("SELECT COUNT(*) FROM retrieval_runs", [], |r| r.get(0)).unwrap();
        assert!(db_count >= 1);
        let hit_count: i32 = db.query_row("SELECT COUNT(*) FROM retrieval_hits WHERE used_by_agent IS NOT NULL", [], |r| r.get(0)).unwrap();
        assert!(hit_count >= 1);
    }
}
