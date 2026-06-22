import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { useModelStore } from '../stores/useModelStore';
import { useAgentStore } from '../stores/useAgentStore';
import { WORKFLOW_TYPES } from '../../shared/constants';
import { useT } from '../../shared/i18n';

export default function AgentWorkspace() {
  const t = useT();
  const currentProject = useProjectStore((s) => s.currentProject);
  const modelConfig = useModelStore((s) => s.config);
  const { runs, messages, steps, currentRun, running, error, loadRuns, loadRunDetail, runWorkflow, updateMessageStatus, updateMessageContent, clearError } = useAgentStore();
  const [workflowType, setWorkflowType] = useState<string>(WORKFLOW_TYPES[0].value);
  const [taskDescription, setTaskDescription] = useState('Generate a card game concept');
  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState('');

  useEffect(() => { if (currentProject) loadRuns(currentProject.id); }, [currentProject]);

  if (!currentProject) return <div className="page"><div className="empty-state"><h2 className="page-title">{t.workspace.title}</h2><p className="text-secondary">{t.workspace.noProject}</p></div></div>;

  return (
    <div className="page">
      <h2 className="page-title">{t.workspace.title}: {currentProject.name}</h2>
      {!modelConfig && <div className="alert alert-warning">{t.workspace.noModel}</div>}
      {error && <div className="alert alert-error">{error}</div>}
      <div className="card form-card">
        <h3 className="form-title">{t.workspace.createTask}</h3>
        <div className="form-group"><label className="form-label">{t.workspace.workflowType}</label><select className="form-input" value={workflowType} onChange={e => setWorkflowType(e.target.value)}>{WORKFLOW_TYPES.map(w => <option key={w.value} value={w.value}>{w.label}</option>)}</select></div>
        <div className="form-group"><label className="form-label">{t.workspace.taskDesc}</label><textarea className="form-input form-textarea" value={taskDescription} onChange={e => setTaskDescription(e.target.value)} rows={4} /></div>
        <button className="btn btn-primary" onClick={() => { if (currentProject && modelConfig) { clearError(); runWorkflow(currentProject.id, taskDescription, workflowType); } }} disabled={running || !modelConfig}>{running ? t.workspace.running : t.workspace.run}</button>
      </div>
      <div className="card"><h3 className="form-title">{t.workspace.agentRuns}</h3>{runs.length === 0 ? <p className="text-secondary">{t.workspace.noRuns}</p> : runs.map(run => <div key={run.id} className={`run-item ${currentRun?.id===run.id?'run-item-active':''}`} onClick={()=>loadRunDetail(run.id)}><div className="run-item-header"><span className={`status-badge status-${run.status}`}>{run.status}</span><span className="text-secondary">{run.workflow_type}</span></div><p className="run-item-desc">{run.task_description}</p><p className="text-sm text-secondary">{new Date(run.created_at).toLocaleString()}</p></div>)}</div>
      <div className="card"><h3 className="form-title">{t.workspace.currentOutput}</h3>{steps.length>0 && <div style={{marginBottom:'1rem',display:'flex',gap:'0.5rem'}}>{steps.map(s=><span key={s.id} className={`status-badge status-${s.status}`}>{s.agent_name} ({s.status})</span>)}</div>}{messages.length===0 ? <p className="text-secondary">{t.workspace.noMessages}</p> : messages.map(msg=><div key={msg.id} className={`message-item message-${msg.status}`}><div className="message-header"><span className="message-agent">{msg.agent_name}</span><span className="message-role">{msg.role}</span><span className={`status-badge status-${msg.status}`}>{msg.status}</span></div>{editingMessageId===msg.id?<div className="message-edit"><textarea className="form-input form-textarea" value={editContent} onChange={e=>setEditContent(e.target.value)} rows={8}/><div className="form-actions"><button className="btn btn-primary btn-sm" onClick={()=>{updateMessageContent(msg.id,editContent);setEditingMessageId(null);setEditContent('');}}>{t.workspace.save}</button><button className="btn btn-secondary btn-sm" onClick={()=>setEditingMessageId(null)}>{t.workspace.cancel}</button></div></div>:<pre className="message-content">{msg.content}</pre>}<div className="message-actions"><button className="btn btn-success btn-sm" onClick={()=>updateMessageStatus(msg.id,'accepted')}>{t.workspace.accept}</button><button className="btn btn-danger btn-sm" onClick={()=>updateMessageStatus(msg.id,'rejected')}>{t.workspace.reject}</button><button className="btn btn-secondary btn-sm" onClick={()=>{setEditingMessageId(msg.id);setEditContent(msg.content);}}>{t.workspace.edit}</button></div></div>)}</div>
    </div>
  );
}
