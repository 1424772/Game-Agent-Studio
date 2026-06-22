import { create } from 'zustand';
import type { AgentRun, AgentMessage, ModelConfigPublic } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

interface AgentStore {
  runs: AgentRun[];
  messages: AgentMessage[];
  currentRun: AgentRun | null;
  running: boolean;
  runStatus: string;
  error: string | null;

  loadRuns: (projectId: string) => Promise<void>;
  loadMessages: (runId: string) => Promise<void>;
  runWorkflow: (projectId: string, taskDescription: string, workflowType: string, modelConfig: ModelConfigPublic) => Promise<void>;
  updateMessageStatus: (messageId: string, status: string) => Promise<void>;
}

export const useAgentStore = create<AgentStore>((set, get) => ({
  runs: [],
  messages: [],
  currentRun: null,
  running: false,
  runStatus: '',
  error: null,

  loadRuns: async (projectId) => {
    const runs = await tauri.getAgentRuns(projectId);
    set({ runs });
  },

  loadMessages: async (runId) => {
    const messages = await tauri.getAgentMessages(runId);
    set({ messages });
  },

  runWorkflow: async (projectId, taskDescription, workflowType, modelConfig) => {
    set({ running: true, runStatus: 'Creating agent run...', error: null });

    try {
      const run = await tauri.createAgentRun(projectId, taskDescription, workflowType);
      set({ currentRun: run });

      const { model, temperature, max_tokens } = modelConfig;

      // Step 1: ProducerAgent
      set({ runStatus: 'ProducerAgent planning...' });
      const producerSys = `You are a ProducerAgent. Analyze game creation tasks and break them down into structured components. Output JSON with: overview, components, agent_assignments, execution_plan.`;
      const producerUser = `Analyze this game creation task:\n\nTask: ${taskDescription}\nWorkflow Type: ${workflowType}\n\nOutput your analysis as JSON.`;
      const producerResult = await callLlmAndSave(run.id, model, temperature, max_tokens, 'ProducerAgent', producerSys, producerUser, 1, 'planning');

      // Step 2: GameDesignerAgent
      set({ runStatus: 'GameDesignerAgent generating...' });
      const isCardGame = workflowType.includes('card');
      const designerSys = `You are a GameDesignerAgent. Create detailed game design documents. ${isCardGame ? 'Focus on card mechanics, core loop, card types, resource systems, combat rules.' : 'Focus on story structure, branching narrative, character relationships, emotional pacing.'} Output with clear markdown sections.`;
      const designerUser = `Based on this producer plan, create a detailed game design:\n\n${producerResult}\n\nOutput comprehensive game design content.`;
      const designerResult = await callLlmAndSave(run.id, model, temperature, max_tokens, 'GameDesignerAgent', designerSys, designerUser, 2, 'design');

      await saveDesignToMemory(projectId, designerResult);

      // Step 3: QAAgent
      set({ runStatus: 'QAAgent reviewing...' });
      const qaSys = `You are a QAAgent. Review game design content and identify issues, inconsistencies, missing elements, balance problems, scope risks. Be constructive and specific.`;
      const qaUser = `Review this game design and identify issues:\n\n${designerResult}\n\nProvide structured review: 1) Issues Found 2) Missing Elements 3) Concerns 4) Scope Risks 5) Suggestions.`;
      await callLlmAndSave(run.id, model, temperature, max_tokens, 'QAAgent', qaSys, qaUser, 3, 'review');

      await tauri.logEvent(projectId, 'agent_workflow_completed', JSON.stringify({
        workflowType, task: taskDescription,
      }), run.id, 'system', 'info');

      set({ running: false, runStatus: 'Workflow completed' });
      await get().loadRuns(projectId);
      await get().loadMessages(run.id);
    } catch (e: any) {
      set({ running: false, runStatus: 'Failed', error: e.toString() });
      await tauri.logEvent(projectId, 'agent_workflow_failed', JSON.stringify({
        error: e.toString(), workflowType,
      }), get().currentRun?.id, 'system', 'error');
    }
  },

  updateMessageStatus: async (messageId, status) => {
    await tauri.updateMessageStatus(messageId, status);
    const currentRun = get().currentRun;
    if (currentRun) {
      await tauri.logEvent(currentRun.project_id,
        status === 'accepted' ? 'output_accepted' : status === 'rejected' ? 'output_rejected' : 'output_edited',
        JSON.stringify({ messageId }), currentRun.id, 'user', 'info');
    }
  },
}));

async function callLlmAndSave(
  runId: string, model: string, temperature: number, maxTokens: number,
  agentName: string, systemPrompt: string, userPrompt: string, stepOrder: number, stepType: string
): Promise<string> {
  const inputJson = JSON.stringify({ system: systemPrompt, user: userPrompt });

  await tauri.saveAgentStep(runId, agentName, stepOrder, stepType, inputJson, null, 'running', null, null, null);
  await tauri.saveAgentMessage(runId, agentName, 'system', systemPrompt, JSON.stringify({ stepOrder, stepType }));

  const response = await tauri.runLlmCompletion({
    model,
    temperature,
    max_tokens: maxTokens,
    messages: [
      { role: 'system', content: systemPrompt },
      { role: 'user', content: userPrompt },
    ],
  });

  await tauri.saveAgentMessage(runId, agentName, 'assistant', response.content, JSON.stringify({
    usage: response.usage,
    stepOrder,
  }));
  await tauri.saveAgentStep(runId, agentName, stepOrder, stepType, inputJson, response.content, 'completed', null,
    response.usage.prompt_tokens, response.usage.completion_tokens);

  return response.content;
}

async function saveDesignToMemory(projectId: string, content: string) {
  const sections = content.split(/\n## /);
  const title = sections[0].replace(/^# /, '').trim();
  await tauri.saveProjectMemory(projectId, 'world_setting', 'design_title', title, 'L2', 'project', 'GameDesignerAgent', 1.0, 1);

  for (let i = 1; i < sections.length; i++) {
    const lines = sections[i].split('\n');
    const sectionKey = lines[0].trim().toLowerCase().replace(/\s+/g, '_').slice(0, 60);
    const sectionContent = lines.slice(1).join('\n').trim();

    if (sectionContent && sectionContent.length > 10) {
      let memoryType = 'world_setting';
      if (sectionKey.includes('character')) memoryType = 'character';
      else if (sectionKey.includes('plot') || sectionKey.includes('story')) memoryType = 'plot';
      else if (sectionKey.includes('rule') || sectionKey.includes('mechanic') || sectionKey.includes('card')) memoryType = 'rule';
      else if (sectionKey.includes('art') || sectionKey.includes('style')) memoryType = 'art_style';
      else if (sectionKey.includes('balance') || sectionKey.includes('economy')) memoryType = 'rule';

      await tauri.saveProjectMemory(projectId, memoryType, sectionKey, sectionContent, 'L2', 'project', 'GameDesignerAgent', 0.8, 1);
    }
  }
}
