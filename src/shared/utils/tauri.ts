import { invoke } from '@tauri-apps/api/core';
import type {
  Project, ModelConfigPublic, AgentRun, AgentStep, AgentMessage,
  Event, ProjectMemory, UserPreference, ExportRecord,
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
  baseUrl: string, apiKey: string, model: string, temperature: number, maxTokens: number
): Promise<ModelConfigPublic> {
  return invoke('save_model_config', { baseUrl, apiKey, model, temperature, maxTokens });
}
export async function getModelConfig(): Promise<ModelConfigPublic | null> {
  return invoke('get_model_config');
}
export async function testModelConnection(baseUrl: string, apiKey: string, model: string): Promise<boolean> {
  return invoke('test_model_connection', { baseUrl, apiKey, model });
}

// --- Agent Runs ---
export async function createAgentRun(projectId: string, taskDescription: string, workflowType: string): Promise<AgentRun> {
  return invoke('create_agent_run', { projectId, taskDescription, workflowType });
}
export async function getAgentRuns(projectId: string): Promise<AgentRun[]> {
  return invoke('get_agent_runs', { projectId });
}

// --- Agent Steps ---
export async function saveAgentStep(
  runId: string, agentName: string, stepOrder: number, stepType: string,
  inputJson: string | null, outputJson: string | null, status: string,
  errorMessage: string | null, promptTokens: number | null, completionTokens: number | null
): Promise<AgentStep> {
  return invoke('save_agent_step', {
    runId, agentName, stepOrder, stepType, inputJson, outputJson,
    status, errorMessage, promptTokens, completionTokens,
  });
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
  });
}
export async function getEvents(projectId: string | null, limit: number): Promise<Event[]> {
  return invoke('get_events', { projectId, limit });
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
