use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    CardGame,
    VisualNovel,
    Rpg,
    Puzzle,
    Strategy,
    Simulation,
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    CardGameConcept,
    VisualNovelConcept,
    GameDesignDoc,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Pending,
    Running,
    WaitingForInput,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Streaming,
    Completed,
    Failed,
    Cancelled,
    Accepted,
    Rejected,
    Edited,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLayer {
    L1,
    L2,
    L3,
    L4,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Project,
    Session,
    Global,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalType {
    WorkflowImprovement,
    PromptImprovement,
    CodeImprovement,
    ExportTemplateFix,
    SafetyEnhancement,
    UiUxImprovement,
    DataModelRefinement,
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Draft,
    Proposed,
    Accepted,
    Rejected,
    Implemented,
    Superseded,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub game_type: GameType,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub id: String,
    pub base_url: String,
    pub encrypted_api_key: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfigPublic {
    pub id: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub masked_api_key: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl ModelConfigPublic {
    pub fn from_config(config: &ModelConfig) -> Self {
        let has_api_key = !config.encrypted_api_key.is_empty();
        let masked_api_key = if has_api_key {
            mask_key(&config.encrypted_api_key)
        } else {
            String::new()
        };
        ModelConfigPublic {
            id: config.id.clone(),
            base_url: config.base_url.clone(),
            has_api_key,
            masked_api_key,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            created_at: config.created_at.clone(),
            updated_at: config.updated_at.clone(),
        }
    }
}

fn mask_key(encrypted: &str) -> String {
    if encrypted.len() <= 6 {
        return "****".to_string();
    }
    let prefix = &encrypted[..2];
    let suffix = &encrypted[encrypted.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentRun {
    pub id: String,
    pub project_id: String,
    pub task_description: String,
    pub workflow_type: WorkflowType,
    pub status: AgentRunStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentStep {
    pub id: String,
    pub run_id: String,
    pub agent_name: String,
    pub step_key: String,
    pub step_order: i32,
    pub step_type: String,
    pub input_json: Option<String>,
    pub output_json: Option<String>,
    pub status: AgentRunStatus,
    pub error_message: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentMessage {
    pub id: String,
    pub run_id: String,
    pub agent_name: String,
    pub role: String,
    pub content: String,
    pub metadata: Option<String>,
    pub status: MessageStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: String,
    pub project_id: Option<String>,
    pub run_id: Option<String>,
    pub actor: Option<String>,
    pub event_type: String,
    pub event_data: String,
    pub severity: EventSeverity,
    pub correlation_id: Option<String>,
    pub redaction_level: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectMemory {
    pub id: String,
    pub project_id: String,
    pub memory_type: String,
    pub key: String,
    pub value: String,
    pub layer: MemoryLayer,
    pub scope: MemoryScope,
    pub source: Option<String>,
    pub confidence: f64,
    pub version: i32,
    pub provenance: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPreference {
    pub id: String,
    pub preference_key: String,
    pub preference_value: String,
    pub confidence: f64,
    pub evidence: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Document {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub content: String,
    pub doc_type: String,
    pub source_path: Option<String>,
    pub chunk_count: i32,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub embedding_json: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_dim: Option<i32>,
    pub embedding_version: Option<i32>,
    pub embedded_at: Option<String>,
    pub embedding_status: Option<String>,
    pub embedding_error: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetrievalRun {
    pub id: String,
    pub project_id: String,
    pub query_text: String,
    pub rewritten_queries: Option<String>,
    pub strategy: Option<String>,
    pub result_count: i32,
    pub duration_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetrievalHit {
    pub id: String,
    pub retrieval_run_id: String,
    pub chunk_id: String,
    pub score: f64,
    pub rank: i32,
    pub used_by_agent: Option<String>,
    pub score_breakdown: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImprovementProposal {
    pub id: String,
    pub proposal_type: ProposalType,
    pub summary: String,
    pub evidence: Option<String>,
    pub risk_level: Option<String>,
    pub status: ProposalStatus,
    pub requires_human_approval: bool,
    pub target_area: Option<String>,
    pub proposed_change: Option<String>,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowDefinition {
    pub workflow_type: WorkflowType,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowStep {
    pub agent_name: String,
    pub role: String,
    pub system_prompt_template: String,
    pub user_prompt_template: String,
    pub output_schema: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportRecord {
    pub id: String,
    pub project_id: String,
    pub export_type: String,
    pub file_path: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryVersion {
    pub id: String,
    pub memory_id: String,
    pub project_id: String,
    pub memory_type: String,
    pub key: String,
    pub old_value: String,
    pub new_value: String,
    pub source: Option<String>,
    pub provenance: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub messages: Vec<LlmMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: LlmUsage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub fn sanitize_error(mut message: String) -> String {
    let bearer_re = regex_lite::Regex::new(r"(?i)(bearer\s+)[\w\-\.]+");
    if let Ok(re) = bearer_re {
        message = re.replace_all(&message, "${1}[REDACTED]").to_string();
    }

    let auth_re = regex_lite::Regex::new(r#"(?i)(Authorization[=:]\s*['"]?)[^\s,'"]+"#);
    if let Ok(re) = auth_re {
        message = re.replace_all(&message, "${1}[REDACTED]").to_string();
    }

    let sk_re = regex_lite::Regex::new(r"(?i)(sk-[a-zA-Z0-9]{16,})");
    if let Ok(re) = sk_re {
        message = re.replace_all(&message, "sk-...[REDACTED]").to_string();
    }

    let apikey_re = regex_lite::Regex::new(r#"(?i)(api[_-]?key[=:]\s*['"]?)([^&\s'"]+)"#);
    if let Ok(re) = apikey_re {
        message = re.replace_all(&message, "${1}[REDACTED]").to_string();
    }

    message
}
