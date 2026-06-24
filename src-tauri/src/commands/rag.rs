use crate::models::{Document, DocumentChunk, RetrievalHit, RetrievalRun};
use crate::AppState;
use sha2::Digest;
use tauri::State;

const CHUNK_SIZE: usize = 2000;
const MIN_LIMIT: i32 = 1;
const MAX_LIMIT: i32 = 20;

pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let set_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if set_a.is_empty() && set_b.is_empty() { return 1.0; }
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

pub fn deduplicate_excerpts(excerpts: &[(String, String, String)], threshold: f64) -> Vec<(String, String, String)> {
    let mut result: Vec<(String, String, String)> = Vec::new();
    for excerpt in excerpts {
        let is_dup = result.iter().any(|existing| {
            jaccard_similarity(&existing.2, &excerpt.2) >= threshold
        });
        if !is_dup {
            result.push(excerpt.clone());
        }
    }
    result
}

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
    usage_context: Option<(&str, &str, &str)>,
    query_embedding: Option<&[f64]>,
    force_strategy: Option<&str>,
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

    let mut strategy = force_strategy.unwrap_or("keyword").to_string();
    let has_vector = query_embedding.map_or(false, |v| !v.is_empty());
    if !has_vector && (strategy == "hybrid" || strategy == "vector") { strategy = "keyword_fallback".to_string(); }
    if !has_vector && strategy != "vector" && strategy != "hybrid" && strategy != "keyword_fallback" { strategy = "keyword".to_string(); }

    let mut _owned_scored: Vec<(String, String, i32, String, Option<String>, String, String)> = Vec::new();
    if has_vector && (strategy == "hybrid" || strategy == "vector") {
        let qv = query_embedding.unwrap();
        // For hybrid: merge keyword + vector; for vector: only vector candidates
        let mut kw_scored: Vec<(f64, (String, String, i32, String, Option<String>, String, String))> = if strategy == "hybrid" {
            scored.iter().map(|(ks, item)| {
                let vs = db.query_row("SELECT embedding_json FROM document_chunks WHERE id=?1", rusqlite::params![item.0], |r| r.get::<_,Option<String>>(0))
                    .ok().flatten().and_then(|s| serde_json::from_str::<Vec<f64>>(&s).ok())
                    .map(|cv| crate::commands::embedding::cosine_similarity(qv, &cv)).unwrap_or(0.0);
                ((*ks).min(1.0) * 0.3 + vs * 0.7, (item.0.clone(), item.1.clone(), item.2, item.3.clone(), item.4.clone(), item.5.clone(), item.6.clone()))
            }).collect()
        } else { Vec::new() };
        // Vector candidates
        let kw_ids: std::collections::HashSet<String> = scored.iter().map(|(_, item)| item.0.clone()).collect();
        let vec_sql = "SELECT c.id, c.document_id, c.chunk_index, c.content, c.metadata, d.title, d.doc_type, c.embedding_json FROM document_chunks c JOIN documents d ON c.document_id=d.id WHERE d.project_id=?1 AND c.embedding_json IS NOT NULL AND c.embedding_json != ''";
        let vec_rows: Vec<(String, String, i32, String, Option<String>, String, String, String)> = db.prepare(vec_sql)
            .map_err(|e| crate::models::sanitize_error(e.to_string()))?
            .query_map(rusqlite::params![project_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4).ok(), r.get(5)?, r.get(6)?, r.get(7)?)))
            .map_err(|e| crate::models::sanitize_error(e.to_string()))?
            .filter_map(|r| r.ok()).collect();
        for (cid, did, idx, content, meta, title, dtype, emb_json) in &vec_rows {
            if strategy == "vector" || !kw_ids.contains(cid) {
                if let Ok(vec) = serde_json::from_str::<Vec<f64>>(emb_json) {
                    let vs = crate::commands::embedding::cosine_similarity(qv, &vec);
                    if vs > (if strategy == "vector" { 0.3 } else { 0.5 }) {
                        kw_scored.push((vs, (cid.clone(), did.clone(), *idx, content.clone(), meta.clone(), title.clone(), dtype.clone())));
                    }
                }
            }
        }
        kw_scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        kw_scored.truncate(lim as usize);
        _owned_scored = kw_scored.iter().map(|(_, item)| item.clone()).collect();
        scored = kw_scored.iter().zip(_owned_scored.iter()).map(|((s, _), item)| (*s, item)).collect();
    }

    let duration_ms = start.elapsed().as_millis() as i64;
    let hit_count = scored.len() as i32;
    let run_id = uuid::Uuid::new_v4().to_string();

    let usage_meta = usage_context.map(|(rid, sk, an)| {
        serde_json::json!({"run_id": rid, "step_key": sk, "agent_name": an}).to_string()
    });

    // Pre-fetch embeddings for hybrid scoring (before transaction borrows db)
    let chunk_embeddings: std::collections::HashMap<String, Option<Vec<f64>>> = if query_embedding.is_some() {
        let mut map = std::collections::HashMap::new();
        for (_score, item) in &scored {
            let emb = db.query_row("SELECT embedding_json FROM document_chunks WHERE id=?1", rusqlite::params![item.0], |r| r.get::<_,Option<String>>(0)).ok().flatten().and_then(|s| serde_json::from_str(&s).ok());
            map.insert(item.0.clone(), emb);
        }
        map
    } else { std::collections::HashMap::new() };

    let tx = db.transaction().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    tx.execute(
        "INSERT INTO retrieval_runs (id, project_id, query_text, rewritten_queries, strategy, result_count, duration_ms, created_at) VALUES (?1,?2,?3,NULL,?4,?5,?6,?7)",
        rusqlite::params![run_id, project_id, trimmed, strategy, hit_count, duration_ms, now],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let mut hits = Vec::new();
    let mut excerpts = Vec::new();

    for (rank, (score, item)) in scored.iter().enumerate() {
        let hit_id = uuid::Uuid::new_v4().to_string();
        let excerpt: String = crate::models::sanitize_error(item.3.chars().take(300).collect());
        let used = usage_meta.clone();
        let breakthrough = if query_embedding.is_some() {
            let vec_s = chunk_embeddings.get(&item.0).and_then(|e| e.as_ref())
                .map(|cv| crate::commands::embedding::cosine_similarity(query_embedding.unwrap(), cv)).unwrap_or(0.0);
            let kw_s = score_words.iter().filter(|w| item.3.to_lowercase().contains(w.as_str())).count() as f64;
            Some(serde_json::json!({"keyword": kw_s, "vector": vec_s, "final": *score}).to_string())
        } else { None };

        tx.execute(
            "INSERT INTO retrieval_hits (id, retrieval_run_id, chunk_id, score, rank, used_by_agent, score_breakdown, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![hit_id, run_id, item.0, score, rank as i32, used, breakthrough, now],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        hits.push(RetrievalHit {
            id: hit_id.clone(), retrieval_run_id: run_id.clone(), chunk_id: item.0.clone(),
            score: *score, rank: rank as i32, used_by_agent: used.clone(),
            score_breakdown: breakthrough, created_at: now.clone(),
        });
        excerpts.push((item.0.clone(), item.5.clone(), excerpt));
    }

    tx.commit().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let run = RetrievalRun {
        id: run_id, project_id: project_id.to_string(), query_text: trimmed.to_string(),
        rewritten_queries: None, strategy: Some(strategy.clone()),
        result_count: hit_count, duration_ms, created_at: now,
    };

    Ok(RetrievalResult { run, hits, excerpts })
}

// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲
// Tauri Commands
// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲

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
    let mut stmt = db.prepare("SELECT id,document_id,chunk_index,content,embedding_json,embedding_model,embedding_provider,embedding_dim,embedding_version,embedded_at,embedding_status,embedding_error,metadata,created_at FROM document_chunks WHERE document_id=?1 ORDER BY chunk_index ASC").map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    let chunks = stmt.query_map(rusqlite::params![document_id], |r| Ok(DocumentChunk{id:r.get(0)?,document_id:r.get(1)?,chunk_index:r.get(2)?,content:r.get(3)?,embedding_json:r.get(4).ok(),embedding_model:r.get(5).ok(),embedding_provider:r.get(6).ok(),embedding_dim:r.get(7).ok(),embedding_version:r.get(8).ok(),embedded_at:r.get(9).ok(),embedding_status:r.get(10).ok(),embedding_error:r.get(11).ok(),metadata:r.get(12).ok(),created_at:r.get(13)?})).map_err(|e|crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    Ok(chunks)
}

#[tauri::command]
pub fn search_documents(
    state: State<AppState>, project_id: String, query: String, limit: Option<i32>,
) -> Result<(RetrievalRun, Vec<RetrievalHit>), String> {
    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let result = retrieve_for_context(&mut db, &project_id, &query, limit.unwrap_or(10), None, None, None)?;
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
    let mut stmt = db.prepare("SELECT id,retrieval_run_id,chunk_id,score,rank,used_by_agent,score_breakdown,created_at FROM retrieval_hits WHERE retrieval_run_id=?1 ORDER BY rank ASC").map_err(|e|crate::models::sanitize_error(e.to_string()))?;
    let hits = stmt.query_map(rusqlite::params![retrieval_run_id], |r| Ok(RetrievalHit{id:r.get(0)?,retrieval_run_id:r.get(1)?,chunk_id:r.get(2)?,score:r.get(3)?,rank:r.get(4)?,used_by_agent:r.get(5).ok(),score_breakdown:r.get(6).ok(),created_at:r.get(7)?})).map_err(|e|crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e|crate::models::sanitize_error(e.to_string()))?;
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
    pub score_breakdown: Option<String>,
    pub source: Option<String>,
    pub provenance: Option<String>,
}

#[tauri::command]
pub fn get_retrieval_hit_excerpts(
    state: State<AppState>, retrieval_run_id: String,
) -> Result<Vec<HitExcerpt>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare(
        "SELECT h.id, h.retrieval_run_id, h.chunk_id, h.score, h.rank, h.used_by_agent, h.score_breakdown, h.created_at, c.content, c.metadata, d.title, d.doc_type FROM retrieval_hits h JOIN document_chunks c ON h.chunk_id=c.id JOIN documents d ON c.document_id=d.id WHERE h.retrieval_run_id=?1 ORDER BY h.rank ASC"
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let hits = stmt.query_map(rusqlite::params![retrieval_run_id], |r| {
        let content: String = r.get(8)?;
        let metadata_str: Option<String> = r.get(9).ok();
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
            score_breakdown: r.get(6).ok(), created_at: r.get(7)?,
            doc_title: r.get(10)?, doc_type: r.get(11)?,
            chunk_excerpt: excerpt, source, provenance,
        })
    }).map_err(|e| crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(hits)
}

// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲
// Embedding commands
// 鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲鈺愨晲

#[tauri::command]
pub async fn embed_pending_chunks(
    state: State<'_, AppState>, project_id: String, limit: Option<i32>,
) -> Result<i32, String> {
    const MAX_CHARS: usize = 8000;
    let lim = limit.unwrap_or(10).clamp(1, 20);
    let (model, chunk_ids, encrypted_key, base_url) = {
        let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        let model: String = db.query_row("SELECT model FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or_else(|_| "text-embedding-3-small".into());
        let encrypted: String = db.query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        let url: String = db.query_row("SELECT base_url FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        crate::commands::security::validate_base_url(&url)?;
        let chunks: Vec<(String, String)> = db.prepare(
            "SELECT c.id, c.content FROM document_chunks c JOIN documents d ON c.document_id=d.id WHERE d.project_id=?1 AND (c.embedding_status='pending' OR c.embedding_status IS NULL) LIMIT ?2"
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .query_map(rusqlite::params![project_id, lim], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .filter_map(|r| r.ok()).collect();
        (model, chunks, encrypted, url)
    };

    if chunk_ids.is_empty() { return Ok(0); }

    let inputs: Vec<String> = chunk_ids.iter().map(|(_, c)| c.chars().take(MAX_CHARS).collect()).collect();
    let api_key = crate::crypto::decrypt_saved_api_key(&encrypted_key)?;
    let result = crate::commands::embedding::embed_batch(&base_url, &api_key, &model, &inputs).await;

    let (vectors, emb_model, _tokens) = match result {
        Ok(v) => v,
        Err(e) => {
            let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            let err = crate::models::sanitize_error(e);
            for (cid, _) in &chunk_ids { db.execute("UPDATE document_chunks SET embedding_status='failed', embedding_error=?1 WHERE id=?2", rusqlite::params![err, cid]).map_err(|e| crate::models::sanitize_error(e.to_string()))?; }
            return Err(err);
        }
    };

    let dim = vectors.first().map(|v| v.len() as i32).unwrap_or(0);
    let now = chrono::Utc::now().to_rfc3339();
    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut count = 0;
    for (i, (chunk_id, _)) in chunk_ids.iter().enumerate() {
        if i >= vectors.len() { break; }
        if let Err(e) = crate::commands::embedding::validate_embedding(&vectors[i], Some(dim as usize)) {
            db.execute("UPDATE document_chunks SET embedding_status='failed', embedding_error=?1 WHERE id=?2",
                rusqlite::params![crate::models::sanitize_error(e), chunk_id],
            ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            continue;
        }
        let vec_json = serde_json::to_string(&vectors[i]).unwrap_or_default();
        db.execute(
            "UPDATE document_chunks SET embedding_json=?1, embedding_model=?2, embedding_dim=?3, embedding_status='embedded', embedded_at=?4 WHERE id=?5",
            rusqlite::params![vec_json, emb_model, dim, now, chunk_id],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieve_empty_query_rejected() {
        let r = retrieve_for_context(
            &mut rusqlite::Connection::open_in_memory().unwrap(),
            "test", "", 5, None, None, None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not be empty"));
    }

    #[test]
    fn retrieve_long_query_rejected() {
        let long = "a".repeat(501);
        let r = retrieve_for_context(
            &mut rusqlite::Connection::open_in_memory().unwrap(),
            "test", &long, 5, None, None, None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("max 500"));
    }

    #[test]
    fn retrieve_too_many_words_rejected() {
        let q = (0..25).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
        let r = retrieve_for_context(
            &mut rusqlite::Connection::open_in_memory().unwrap(),
            "test", &q, 5, None, None, None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Maximum is 20"));
    }

    #[test]
    fn limit_clamped_to_range() {
        assert_eq!(0_i32.clamp(MIN_LIMIT, MAX_LIMIT), 1);
        assert_eq!((-5_i32).clamp(MIN_LIMIT, MAX_LIMIT), 1);
        assert_eq!(100_i32.clamp(MIN_LIMIT, MAX_LIMIT), 20);
    }

    // 鈹€鈹€ DB-level tests with in-memory SQLite 鈹€鈹€

    fn setup_rag_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE documents (id TEXT PRIMARY KEY, project_id TEXT, title TEXT, content TEXT, doc_type TEXT, source_path TEXT, chunk_count INTEGER, created_at TEXT);
             CREATE TABLE document_chunks (id TEXT PRIMARY KEY, document_id TEXT, chunk_index INTEGER, content TEXT, embedding_json TEXT, metadata TEXT, created_at TEXT);
             CREATE TABLE retrieval_runs (id TEXT PRIMARY KEY, project_id TEXT, query_text TEXT, rewritten_queries TEXT, strategy TEXT, result_count INTEGER, duration_ms BIGINT, created_at TEXT);
             CREATE TABLE retrieval_hits (id TEXT PRIMARY KEY, retrieval_run_id TEXT, chunk_id TEXT, score REAL, rank INTEGER, used_by_agent TEXT, score_breakdown TEXT, created_at TEXT);"
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
            Some(("run-1", "qa.review", "QAAgent")), None, None).unwrap();

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

    #[test]
    fn keyword_fallback_writes_strategy() {
        let mut db = setup_rag_db();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute("INSERT INTO documents (id,project_id,title,content,doc_type,chunk_count,created_at) VALUES ('d1','proj1','Test','hello','game_design',0,?1)", rusqlite::params![now]).unwrap();
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,created_at) VALUES ('c1','d1',1,'hello world',?1)", rusqlite::params![now]).unwrap();
        let r = retrieve_for_context(&mut db, "proj1", "hello", 5, None, Some(&[]), Some("keyword_fallback")).unwrap();
        assert_eq!(r.run.strategy.as_deref(), Some("keyword_fallback"));
    }

    #[test]
    fn hybrid_writes_strategy() {
        let mut db = setup_rag_db();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute("INSERT INTO documents (id,project_id,title,content,doc_type,chunk_count,created_at) VALUES ('d1','proj1','Test','hello','game_design',0,?1)", rusqlite::params![now]).unwrap();
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,embedding_json,created_at) VALUES ('c1','d1',1,'hello world','[0.1,0.2,0.3]',?1)", rusqlite::params![now]).unwrap();
        let qv = vec![0.1, 0.2, 0.3];
        let r = retrieve_for_context(&mut db, "proj1", "hello", 5, None, Some(&qv), Some("hybrid")).unwrap();
        assert_eq!(r.run.strategy.as_deref(), Some("hybrid"));
    }

    #[test]
    fn vector_only_strategy() {
        let mut db = setup_rag_db();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute("INSERT INTO documents (id,project_id,title,content,doc_type,chunk_count,created_at) VALUES ('d1','proj1','Test','hello','game_design',0,?1)", rusqlite::params![now]).unwrap();
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,embedding_json,created_at) VALUES ('c1','d1',1,'hello world','[0.1,0.2,0.3]',?1)", rusqlite::params![now]).unwrap();
        let qv = vec![0.1, 0.2, 0.3];
        let r = retrieve_for_context(&mut db, "proj1", "hello", 5, None, Some(&qv), Some("vector")).unwrap();
        assert_eq!(r.run.strategy.as_deref(), Some("vector"));
        assert!(!r.hits.is_empty());
    }

    #[test]
    fn hybrid_merges_vector_only_candidate() {
        let mut db = setup_rag_db();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute("INSERT INTO documents (id,project_id,title,content,doc_type,chunk_count,created_at) VALUES ('d1','proj1','Test','hello','game_design',0,?1)", rusqlite::params![now]).unwrap();
        // Chunk with no keyword match, only vector match
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,embedding_json,created_at) VALUES ('c1','d1',1,'completely unrelated text here','[0.5,0.5,0.5]',?1)", rusqlite::params![now]).unwrap();
        // Chunk with keyword match
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,embedding_json,created_at) VALUES ('c2','d1',2,'hello world','[0.6,0.6,0.6]',?1)", rusqlite::params![now]).unwrap();
        let qv = vec![0.5, 0.5, 0.5]; // matches c1 better
        let r = retrieve_for_context(&mut db, "proj1", "hello", 5, None, Some(&qv), Some("hybrid")).unwrap();
        // Both chunks should appear (c1 via vector, c2 via keyword+vector)
        assert!(r.hits.len() >= 1);
        let chunk_ids: Vec<&str> = r.hits.iter().map(|h| h.chunk_id.as_str()).collect();
        assert!(chunk_ids.contains(&"c2"), "keyword-matched chunk should appear");
        // c1 should also appear from vector-only merge
        assert!(chunk_ids.contains(&"c1"), "vector-only candidate should be merged");
    }

    #[test]
    fn hybrid_score_breakdown_present() {
        let mut db = setup_rag_db();
        let now = chrono::Utc::now().to_rfc3339();
        db.execute("INSERT INTO documents (id,project_id,title,content,doc_type,chunk_count,created_at) VALUES ('d1','proj1','Test','hello','game_design',0,?1)", rusqlite::params![now]).unwrap();
        db.execute("INSERT INTO document_chunks (id,document_id,chunk_index,content,embedding_json,created_at) VALUES ('c1','d1',1,'hello world test','[0.1,0.2,0.3]',?1)", rusqlite::params![now]).unwrap();
        let qv = vec![0.1, 0.2, 0.3];
        let r = retrieve_for_context(&mut db, "proj1", "hello", 5, None, Some(&qv), Some("hybrid")).unwrap();
        for hit in &r.hits {
            let bd = hit.score_breakdown.as_ref().unwrap();
            assert!(bd.contains("keyword") || bd.contains("vector"));
        }
    }

    #[test]
    fn single_chunk_validation_failure_isolated() {
        use crate::commands::embedding;
        // Per-vector validation: a bad vector fails individually; good vector passes.
        // In embed_pending_chunks: single chunk with NaN/finite/dimension error -> marked failed,
        // other chunks in same batch still get embedded (via 'continue' in the loop).
        // Provider-level batch failure (HTTP error) -> all chunks in batch marked failed.
        assert!(embedding::validate_embedding(&[1.0, f64::NAN], None).is_err());
        assert!(embedding::validate_embedding(&[0.1, 0.2], Some(2)).is_ok());
        assert!(embedding::validate_embedding(&[0.5, 0.5, 0.5], Some(3)).is_ok());
        assert!(embedding::validate_embedding(&[0.5, 0.5], Some(3)).is_err());
    }

    #[test]
    fn jaccard_identical_strings_is_one() {
        let s = "hello world test";
        assert!((jaccard_similarity(s, s) - 1.0).abs() < 0.001);
    }

    #[test]
    fn jaccard_disjoint_strings_is_zero() {
        let a = "hello world";
        let b = "foo bar baz";
        assert_eq!(jaccard_similarity(a, b), 0.0);
    }

    #[test]
    fn jaccard_partial_overlap() {
        let a = "hello world test";
        let b = "hello world exam";
        let sim = jaccard_similarity(a, b);
        assert!(sim > 0.4 && sim < 0.9, "partial overlap: {}", sim);
    }

    #[test]
    fn jaccard_both_empty_is_one() {
        assert_eq!(jaccard_similarity("", ""), 1.0);
    }

    #[test]
    fn deduplicate_removes_near_duplicates() {
        let excerpts = vec![
            ("c1".into(), "Doc1".into(), "hello world test content here".into()),
            ("c2".into(), "Doc2".into(), "hello world test content there".into()),
            ("c3".into(), "Doc3".into(), "completely different topic stuff".into()),
        ];
        let result = deduplicate_excerpts(&excerpts, 0.8);
        assert_eq!(result.len(), 2, "similar excerpts should be deduped");
        let titles: Vec<&str> = result.iter().map(|(_, t, _)| t.as_str()).collect();
        assert!(titles.contains(&"Doc3"), "dissimilar excerpt should be kept");
    }

    #[test]
    fn deduplicate_all_unique_preserves_all() {
        let excerpts = vec![
            ("c1".into(), "Doc1".into(), "alpha beta gamma".into()),
            ("c2".into(), "Doc2".into(), "delta epsilon zeta".into()),
            ("c3".into(), "Doc3".into(), "eta theta iota".into()),
        ];
        let result = deduplicate_excerpts(&excerpts, 0.5);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn deduplicate_empty_list() {
        let excerpts: Vec<(String, String, String)> = vec![];
        let result = deduplicate_excerpts(&excerpts, 0.5);
        assert!(result.is_empty());
    }
}

