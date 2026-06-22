import { create } from 'zustand';
import type { AgentRun, AgentMessage, AgentStep } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

interface AgentStore {
  runs: AgentRun[];
  messages: AgentMessage[];
  steps: AgentStep[];
  currentRun: AgentRun | null;
  running: boolean;
  error: string | null;

  loadRuns: (projectId: string) => Promise<void>;
  loadRunDetail: (runId: string) => Promise<void>;
  runWorkflow: (projectId: string, taskDescription: string, workflowType: string) => Promise<void>;
  updateMessageStatus: (messageId: string, status: string) => Promise<void>;
  updateMessageContent: (messageId: string, editedContent: string) => Promise<void>;
  clearError: () => void;
}

export const useAgentStore = create<AgentStore>((set, get) => ({
  runs: [],
  messages: [],
  steps: [],
  currentRun: null,
  running: false,
  error: null,

  loadRuns: async (projectId) => {
    const runs = await tauri.getAgentRuns(projectId);
    set({ runs });
  },

  loadRunDetail: async (runId) => {
    const [messages, steps, run] = await Promise.all([
      tauri.getAgentMessages(runId),
      tauri.getAgentSteps(runId),
      tauri.getAgentRun(runId),
    ]);
    set({ messages, steps, currentRun: run });
  },

  runWorkflow: async (projectId, taskDescription, workflowType) => {
    set({ running: true, error: null, currentRun: null });
    try {
      const run = await tauri.runWorkflow(projectId, taskDescription, workflowType);
      set({ currentRun: run, running: false });
      await get().loadRuns(projectId);
      await get().loadRunDetail(run.id);
    } catch (e: any) {
      set({ running: false, error: e.toString() });
      await get().loadRuns(projectId);
    }
  },

  updateMessageStatus: async (messageId, status) => {
    await tauri.updateMessageStatus(messageId, status);
    const currentRun = get().currentRun;
    if (currentRun) {
      const cid = await getCorrelationId(currentRun.id, currentRun.project_id);
      await tauri.logEvent(currentRun.project_id,
        status === 'accepted' ? 'output_accepted' : 'output_rejected',
        JSON.stringify({ messageId }),
        currentRun.id, 'user', 'info', cid);
      await get().loadRunDetail(currentRun.id);
    }
  },

  updateMessageContent: async (messageId, editedContent) => {
    await tauri.updateAgentMessageContent(messageId, editedContent);
    const currentRun = get().currentRun;
    if (currentRun) {
      const cid = await getCorrelationId(currentRun.id, currentRun.project_id);
      await tauri.logEvent(currentRun.project_id, 'output_edited',
        JSON.stringify({ messageId }),
        currentRun.id, 'user', 'info', cid);
      await get().loadRunDetail(currentRun.id);
    }
  },

  clearError: () => set({ error: null }),
}));

async function getCorrelationId(runId: string, projectId: string): Promise<string | null> {
  try {
    const events = await tauri.getEvents(projectId, runId, null, 'workflow_start', 1);
    if (events.length > 0 && events[0].correlation_id) {
      return events[0].correlation_id;
    }
  } catch { /* ignore */ }
  return null;
}
