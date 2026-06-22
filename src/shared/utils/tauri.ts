import { invoke } from '@tauri-apps/api/core';
import type {
  Project, ModelConfigPublic, AgentRun, AgentStep, AgentMessage,
  Event, ProjectMemory, MemoryVersion, UserPreference, ExportRecord,
  Document as DocDocument, DocumentChunk, RetrievalRun, RetrievalHit, HitExcerpt,
  LlmRequest, LlmResponse, ImprovementProposal,
} from '../types';

// --- Projects ---
export async function createProject(name: string, gameType: string, description: string): Promise<Project> {
  return invoke('create_project', { name, gameType, description });
}
export async function listProjects(): Promise<Project[]> {
  return invoke('list_projects');
}
export async function getProject(id: string): Promise<Project> {
  return invoke('get_project', { id });
}
export async function deleteProject(id: string): Promise<boolean> {
  return invoke('delete_project', { id });
}

// --- Model Config ---
export async function saveModelConfig(
  baseUrl: string, apiKey: string | null, model: string, temperature: number, maxTokens: number
): Promise<ModelConfigPublic> {
  return invoke('save_model_config', { baseUrl, apiKey, model, temperature, maxTokens });
}
export async function getModelConfig(): Promise<ModelConfigPublic | null> {
  return invoke('get_model_config');
}
export async function testModelConnection(baseUrl: string, apiKey: string | null, model: string): Promise<boolean> {
  return invoke('test_model_connection', { baseUrl, apiKey, model });
}

// --- Agent Runs ---
export async function createAgentRun(projectId: string, taskDescription: string, workflowType: string): Promise<AgentRun> {
  return invoke('create_agent_run', { projectId, taskDescription, workflowType });
}
export async function updateAgentRun(runId: string, status: string): Promise<AgentRun> {
  return invoke('update_agent_run', { runId, status });
}
export async function getAgentRun(runId: string): Promise<AgentRun> {
  return invoke('get_agent_run', { runId });
}
export async function getAgentRuns(projectId: string): Promise<AgentRun[]> {
  return invoke('get_agent_runs', { projectId });
}

// --- Workflow Orchestration ---
export async function runWorkflow(projectId: string, taskDescription: string, workflowType: string): Promise<AgentRun> {
  return invoke('run_workflow', { projectId, taskDescription, workflowType });
}

// --- Agent Steps ---
export async function saveAgentStep(
  runId: string, agentName: string, stepKey: string, stepOrder: number, stepType: string,
  inputJson: string | null, outputJson: string | null, status: string,
  errorMessage: string | null, promptTokens: number | null, completionTokens: number | null
): Promise<AgentStep> {
  return invoke('save_agent_step', {
    runId, agentName, stepKey, stepOrder, stepType, inputJson, outputJson,
    status, errorMessage, promptTokens, completionTokens,
  });
}
export async function getAgentSteps(runId: string): Promise<AgentStep[]> {
  return invoke('get_agent_steps', { runId });
}

// --- Agent Messages ---
export async function saveAgentMessage(
  runId: string, agentName: string, role: string, content: string, metadata: string | null
): Promise<AgentMessage> {
  return invoke('save_agent_message', { runId, agentName, role, content, metadata: metadata || '{}' });
}
export async function getAgentMessages(runId: string): Promise<AgentMessage[]> {
  return invoke('get_agent_messages', { runId });
}
export async function updateMessageStatus(messageId: string, status: string): Promise<void> {
  return invoke('update_message_status', { messageId, status });
}
export async function updateAgentMessageContent(messageId: string, editedContent: string): Promise<void> {
  return invoke('update_agent_message_content', { messageId, editedContent });
}

// --- LLM ---
export async function runLlmCompletion(request: LlmRequest): Promise<LlmResponse> {
  return invoke('run_llm_completion', { request });
}

// --- Events ---
export async function logEvent(
  projectId: string | null, eventType: string, eventData: string,
  runId?: string | null, actor?: string | null, severity?: string,
  correlationId?: string | null
): Promise<Event> {
  return invoke('log_event', {
    projectId, eventType, eventData,
    runId: runId || null, actor: actor || null,
    severity: severity || 'info', correlationId: correlationId || null,
    redactionLevel: null,
  });
}
export async function getEvents(
  projectId: string | null, runId?: string | null, correlationId?: string | null,
  eventType?: string | null, limit?: number
): Promise<Event[]> {
  return invoke('get_events', {
    projectId, runId: runId || null, correlationId: correlationId || null,
    eventType: eventType || null, limit: limit || 100,
  });
}

// --- Improvement Proposals ---
export async function createImprovementProposal(
  proposalType: string, summary: string, evidence?: string | null,
  riskLevel?: string | null, targetArea?: string | null, proposedChange?: string | null,
): Promise<ImprovementProposal> {
  return invoke('create_improvement_proposal', {
    proposalType, summary, evidence: evidence || null,
    riskLevel: riskLevel || null, targetArea: targetArea || null,
    proposedChange: proposedChange || null,
  });
}
export async function listImprovementProposals(status?: string | null): Promise<ImprovementProposal[]> {
  return invoke('list_improvement_proposals', { status: status || null });
}
export async function reviewImprovementProposal(proposalId: string, newStatus: string): Promise<ImprovementProposal> {
  return invoke('review_improvement_proposal', { proposalId, newStatus });
}

// --- RAG / Knowledge Base ---
export async function createDocument(projectId: string, title: string, content: string, docType: string, sourcePath: string | null): Promise<DocDocument> {
  return invoke('create_document', { projectId, title, content, docType, sourcePath });
}
export async function chunkDocument(documentId: string): Promise<DocDocument> {
  return invoke('chunk_document', { documentId });
}
export async function listDocuments(projectId: string): Promise<DocDocument[]> {
  return invoke('list_documents', { projectId });
}
export async function getDocumentChunks(documentId: string): Promise<DocumentChunk[]> {
  return invoke('get_document_chunks', { documentId });
}
export async function searchDocuments(projectId: string, query: string, limit?: number): Promise<{run: RetrievalRun; hits: RetrievalHit[]}> {
  return invoke('search_documents', { projectId, query, limit: limit || 10 });
}
export async function getRetrievalRuns(projectId: string): Promise<RetrievalRun[]> {
  return invoke('get_retrieval_runs', { projectId });
}
export async function getRetrievalHits(retrievalRunId: string): Promise<RetrievalHit[]> {
  return invoke('get_retrieval_hits', { retrievalRunId });
}
export async function getRetrievalHitExcerpts(retrievalRunId: string): Promise<HitExcerpt[]> {
  return invoke('get_retrieval_hit_excerpts', { retrievalRunId });
}

// --- Project Memory ---
export async function getProjectMemory(projectId: string, memoryType?: string): Promise<ProjectMemory[]> {
  return invoke('get_project_memory', { projectId, memoryType: memoryType || null });
}
export async function saveProjectMemory(
  projectId: string, memoryType: string, key: string, value: string,
  layer?: string, scope?: string, source?: string | null,
  confidence?: number, version?: number, provenance?: string | null
): Promise<ProjectMemory> {
  return invoke('save_project_memory', {
    projectId, memoryType, key, value,
    layer: layer || 'L1', scope: scope || 'project', source: source || null,
    confidence: confidence ?? 1.0, version: version ?? 1, provenance: provenance || null,
  });
}
export async function getMemoryVersions(memoryId: string): Promise<MemoryVersion[]> {
  return invoke('get_memory_versions', { memoryId });
}

// --- User Preferences ---
export async function getUserPreferences(): Promise<UserPreference[]> {
  return invoke('get_user_preferences');
}
export async function updateUserPreferences(
  preferenceKey: string, preferenceValue: string, confidence: number, evidence: string | null
): Promise<UserPreference> {
  return invoke('update_user_preferences', {
    preferenceKey, preferenceValue, confidence, evidence: evidence || '',
  });
}

// --- Exports ---
export async function exportMarkdown(projectId: string): Promise<ExportRecord> {
  return invoke('export_markdown', { projectId });
}
export async function exportJson(projectId: string): Promise<ExportRecord> {
  return invoke('export_json', { projectId });
}
export async function getExports(projectId: string): Promise<ExportRecord[]> {
  return invoke('get_exports', { projectId });
}
