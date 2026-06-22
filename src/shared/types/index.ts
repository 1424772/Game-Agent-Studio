export type GameType = 'card_game' | 'visual_novel' | 'rpg' | 'puzzle' | 'strategy' | 'simulation' | string;

export type WorkflowType = 'card_game_concept' | 'visual_novel_concept' | 'game_design_doc' | string;

export type AgentRunStatus = 'pending' | 'running' | 'waiting_for_input' | 'completed' | 'failed' | 'cancelled';

export type MessageStatus = 'pending' | 'streaming' | 'completed' | 'failed' | 'cancelled' | 'accepted' | 'rejected' | 'edited';

export type MemoryLayer = 'L1' | 'L2' | 'L3' | 'L4';

export type MemoryScope = 'project' | 'session' | 'global';

export type EventSeverity = 'debug' | 'info' | 'warning' | 'error' | 'critical';

export type ProposalType = 'workflow_improvement' | 'prompt_improvement' | 'code_improvement' | 'export_template_fix' | 'safety_enhancement' | 'ui_ux_improvement' | 'data_model_refinement' | string;

export type ProposalStatus = 'draft' | 'proposed' | 'accepted' | 'rejected' | 'implemented' | 'superseded';

export interface Project {
  id: string;
  name: string;
  game_type: GameType;
  description: string;
  created_at: string;
  updated_at: string;
}

export interface ModelConfigPublic {
  id: string;
  base_url: string;
  has_api_key: boolean;
  masked_api_key: string;
  model: string;
  temperature: number;
  max_tokens: number;
  created_at: string;
  updated_at: string;
}

export interface AgentRun {
  id: string;
  project_id: string;
  task_description: string;
  workflow_type: WorkflowType;
  status: AgentRunStatus;
  created_at: string;
  updated_at: string;
}

export interface AgentStep {
  id: string;
  run_id: string;
  agent_name: string;
  step_order: number;
  step_type: string;
  input_json: string | null;
  output_json: string | null;
  status: AgentRunStatus;
  error_message: string | null;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

export interface AgentMessage {
  id: string;
  run_id: string;
  agent_name: string;
  role: string;
  content: string;
  metadata: string | null;
  status: MessageStatus;
  created_at: string;
}

export interface Event {
  id: string;
  project_id: string | null;
  run_id: string | null;
  actor: string | null;
  event_type: string;
  event_data: string;
  severity: EventSeverity;
  correlation_id: string | null;
  redaction_level: string | null;
  created_at: string;
}

export interface ProjectMemory {
  id: string;
  project_id: string;
  memory_type: string;
  key: string;
  value: string;
  layer: MemoryLayer;
  scope: MemoryScope;
  source: string | null;
  confidence: number;
  version: number;
  provenance: string | null;
  created_at: string;
  updated_at: string;
}

export interface UserPreference {
  id: string;
  preference_key: string;
  preference_value: string;
  confidence: number;
  evidence: string | null;
  updated_at: string;
}

export interface Document {
  id: string;
  project_id: string;
  title: string;
  content: string;
  doc_type: string;
  source_path: string | null;
  chunk_count: number;
  created_at: string;
}

export interface DocumentChunk {
  id: string;
  document_id: string;
  chunk_index: number;
  content: string;
  embedding_json: string | null;
  metadata: string | null;
  created_at: string;
}

export interface RetrievalRun {
  id: string;
  project_id: string;
  query_text: string;
  rewritten_queries: string | null;
  strategy: string | null;
  result_count: number;
  duration_ms: number;
  created_at: string;
}

export interface RetrievalHit {
  id: string;
  retrieval_run_id: string;
  chunk_id: string;
  score: number;
  rank: number;
  used_by_agent: string | null;
  created_at: string;
}

export interface ImprovementProposal {
  id: string;
  proposal_type: ProposalType;
  summary: string;
  evidence: string | null;
  risk_level: string | null;
  status: ProposalStatus;
  requires_human_approval: boolean;
  created_at: string;
  reviewed_at: string | null;
}

export interface ExportRecord {
  id: string;
  project_id: string;
  export_type: string;
  file_path: string;
  created_at: string;
}

export interface LlmMessage {
  role: string;
  content: string;
}

export interface LlmRequest {
  model: string;
  temperature: number;
  max_tokens: number;
  messages: LlmMessage[];
}

export interface LlmResponse {
  content: string;
  model: string;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}
