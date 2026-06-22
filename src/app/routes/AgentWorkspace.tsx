import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { useModelStore } from '../stores/useModelStore';
import { useAgentStore } from '../stores/useAgentStore';
import { WORKFLOW_TYPES } from '../../shared/constants';

export default function AgentWorkspace() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const modelConfig = useModelStore((s) => s.config);
  const {
    runs, messages, currentRun, running, runStatus, error,
    loadRuns, loadMessages, runWorkflow, updateMessageStatus,
  } = useAgentStore();

  const [workflowType, setWorkflowType] = useState<string>(WORKFLOW_TYPES[0].value);
  const [taskDescription, setTaskDescription] = useState('Generate a card game concept');
  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState('');

  useEffect(() => {
    if (currentProject) {
      loadRuns(currentProject.id);
    }
  }, [currentProject, loadRuns]);

  useEffect(() => {
    setTaskDescription(
      workflowType === 'visual_novel_concept'
        ? 'Generate a visual novel concept'
        : workflowType === 'game_design_doc'
        ? 'Create a comprehensive game design document'
        : 'Generate a card game concept'
    );
  }, [workflowType]);

  function handleRunWorkflow() {
    if (!currentProject || !modelConfig) return;
    runWorkflow(currentProject.id, taskDescription, workflowType, modelConfig);
  }

  function handleSelectRun(runId: string) {
    loadMessages(runId);
  }

  function handleStartEdit(msgId: string, content: string) {
    setEditingMessageId(msgId);
    setEditContent(content);
  }

  function handleSaveEdit(msgId: string) {
    updateMessageStatus(msgId, 'edited');
    setEditingMessageId(null);
    setEditContent('');
  }

  if (!currentProject) {
    return (
      <div className="page">
        <div className="empty-state">
          <h2 className="page-title">Agent Workspace</h2>
          <p className="text-secondary">Please open a project first from the Project Dashboard.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="page">
      <h2 className="page-title">Agent Workspace: {currentProject.name}</h2>

      {!modelConfig && (
        <div className="alert alert-warning">
          No model configured. Please go to Model Settings to configure your LLM connection.
        </div>
      )}

      {error && <div className="alert alert-error">{error}</div>}

      <div className="card form-card">
        <h3 className="form-title">Create Task</h3>
        <div className="form-group">
          <label className="form-label">Workflow Type</label>
          <select
            className="form-input"
            value={workflowType}
            onChange={(e) => setWorkflowType(e.target.value)}
          >
            {WORKFLOW_TYPES.map((wt) => (
              <option key={wt.value} value={wt.value}>
                {wt.label}
              </option>
            ))}
          </select>
        </div>
        <div className="form-group">
          <label className="form-label">Task Description</label>
          <textarea
            className="form-input form-textarea"
            value={taskDescription}
            onChange={(e) => setTaskDescription(e.target.value)}
            rows={4}
          />
        </div>
        <button
          className="btn btn-primary"
          onClick={handleRunWorkflow}
          disabled={running || !modelConfig}
        >
          {running ? `Running: ${runStatus}` : 'Run Workflow'}
        </button>
      </div>

      <div className="card">
        <h3 className="form-title">Agent Runs</h3>
        {runs.length === 0 ? (
          <p className="text-secondary">No runs yet. Create a task to start.</p>
        ) : (
          <div className="runs-list">
            {runs.map((run) => (
              <div
                key={run.id}
                className={`run-item ${currentRun?.id === run.id ? 'run-item-active' : ''}`}
                onClick={() => handleSelectRun(run.id)}
              >
                <div className="run-item-header">
                  <span className={`status-badge status-${run.status}`}>{run.status}</span>
                  <span className="text-secondary">{run.workflow_type}</span>
                </div>
                <p className="run-item-desc">{run.task_description}</p>
                <p className="text-sm text-secondary">
                  {new Date(run.created_at).toLocaleString()}
                </p>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <h3 className="form-title">Current Run Output</h3>
        {messages.length === 0 ? (
          <p className="text-secondary">No messages. Run a workflow to generate output.</p>
        ) : (
          <div className="messages-list">
            {messages.map((msg) => (
              <div key={msg.id} className={`message-item message-${msg.status}`}>
                <div className="message-header">
                  <span className="message-agent">{msg.agent_name}</span>
                  <span className="message-role">{msg.role}</span>
                  <span className={`status-badge status-${msg.status}`}>{msg.status}</span>
                </div>
                {editingMessageId === msg.id ? (
                  <div className="message-edit">
                    <textarea
                      className="form-input form-textarea"
                      value={editContent}
                      onChange={(e) => setEditContent(e.target.value)}
                      rows={8}
                    />
                    <div className="form-actions">
                      <button className="btn btn-primary btn-sm" onClick={() => handleSaveEdit(msg.id)}>
                        Save
                      </button>
                      <button className="btn btn-secondary btn-sm" onClick={() => setEditingMessageId(null)}>
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : (
                  <pre className="message-content">{msg.content}</pre>
                )}
                <div className="message-actions">
                  <button className="btn btn-success btn-sm" onClick={() => updateMessageStatus(msg.id, 'accepted')}>
                    Accept
                  </button>
                  <button className="btn btn-danger btn-sm" onClick={() => updateMessageStatus(msg.id, 'rejected')}>
                    Reject
                  </button>
                  <button className="btn btn-secondary btn-sm" onClick={() => handleStartEdit(msg.id, msg.content)}>
                    Edit
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
