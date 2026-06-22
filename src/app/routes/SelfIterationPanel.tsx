import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { ImprovementProposal } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function SelfIterationPanel() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const [proposals, setProposals] = useState<ImprovementProposal[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => { loadProposals(); }, [currentProject]);

  async function loadProposals() {
    setLoading(true);
    try {
      const p = await tauri.listImprovementProposals(null);
      setProposals(p);
    } catch (e) { console.error(e); }
    setLoading(false);
  }

  async function handleGenerateProposals() {
    if (!currentProject) return;
    setLoading(true);
    try {
      const events = await tauri.getEvents(currentProject.id, null, null, null, 200);
      const accepted = events.filter(e => e.event_type === 'output_accepted').length;
      const rejected = events.filter(e => e.event_type === 'output_rejected').length;
      const edited = events.filter(e => e.event_type === 'output_edited').length;
      const total = accepted + rejected + edited;
      const workflowsFailed = events.filter(e => e.event_type === 'agent_workflow_failed' || e.event_type === 'workflow_failed').length;
      const workflowsCompleted = events.filter(e => e.event_type === 'agent_workflow_completed' || e.event_type === 'workflow_complete').length;

      if (total > 0) {
        const acceptRate = Math.round((accepted / total) * 100);
        if (acceptRate < 50) {
          await tauri.createImprovementProposal(
            'prompt_improvement',
            'Low output acceptance rate detected',
            `Acceptance rate: ${acceptRate}% (${accepted}/${total}). Consider reviewing prompt templates for relevance and clarity.`,
            'high', 'agent_prompts',
            'Review and refine system prompts for ProducerAgent, GameDesignerAgent, and QAAgent to improve output quality and relevance.'
          );
        }
        if (edited > accepted) {
          await tauri.createImprovementProposal(
            'prompt_improvement',
            'High edit rate exceeds acceptance rate',
            `Edits: ${edited}, Accepted: ${accepted}. Outputs require frequent manual correction.`,
            'medium', 'agent_prompts',
            'Adjust prompt templates to reduce need for manual edits. Consider adding output format constraints.'
          );
        }
      }

      if (workflowsFailed > 2) {
        await tauri.createImprovementProposal(
          'workflow_improvement',
          'Multiple workflow failures detected',
          `${workflowsFailed} workflow failures. Check model connection, API configuration, and prompt validity.`,
          'high', 'workflow_execution',
          'Add retry logic for failed steps and improve error recovery in the workflow engine.'
        );
      }

      if (workflowsCompleted >= 3 && total === 0) {
        await tauri.createImprovementProposal(
          'ui_ux_improvement',
          'Workflows completed but no user feedback recorded',
          `${workflowsCompleted} workflows completed with no user accept/reject actions. Consider making output review more prominent.`,
          'low', 'agent_workspace_ui',
          'Add a review notification or highlight un-reviewed outputs in the Agent Workspace.'
        );
      }

      await loadProposals();
    } catch (e) { console.error(e); }
    setLoading(false);
  }

  async function handleReview(proposalId: string, newStatus: string) {
    try {
      await tauri.reviewImprovementProposal(proposalId, newStatus);
      await loadProposals();
    } catch (e) { console.error(e); }
  }

  const pendingProposals = proposals.filter(p => p.status === 'proposed');
  const reviewedProposals = proposals.filter(p => p.status !== 'proposed' && p.status !== 'draft');

  return (
    <div className="page">
      <h2 className="page-title">Self-Iteration Panel</h2>

      <div className="card">
        <h3 className="form-title">Proposal Generation</h3>
        <p className="text-secondary" style={{marginBottom: '1rem'}}>
          Analyze recent events and generate improvement proposals. All proposals require human review before any action is taken. No automatic code or config changes are ever applied.
        </p>
        <button className="btn btn-primary" onClick={handleGenerateProposals} disabled={loading || !currentProject}>
          {loading ? 'Analyzing...' : 'Generate Proposals from Events'}
        </button>
        {!currentProject && <p className="text-secondary" style={{marginTop: '0.5rem'}}>Open a project first to analyze project-specific events.</p>}
      </div>

      <div className="card">
        <h3 className="form-title">Pending Proposals ({pendingProposals.length})</h3>
        {pendingProposals.length === 0 ? (
          <p className="text-secondary">No pending proposals. Generate proposals or review existing ones.</p>
        ) : (
          pendingProposals.map(p => (
            <div key={p.id} className="proposal-item" style={{marginBottom: '1rem', padding: '0.75rem', border: '1px solid var(--border)', borderRadius: '4px'}}>
              <div style={{display:'flex', justifyContent:'space-between', marginBottom: '0.5rem'}}>
                <span className="badge">{p.proposal_type}</span>
                <span className={`badge ${p.risk_level === 'high' ? 'status-failed' : p.risk_level === 'medium' ? 'status-running' : ''}`}>
                  risk: {p.risk_level || 'low'}
                </span>
                {p.requires_human_approval && <span className="badge" style={{background:'var(--primary)'}}>requires approval</span>}
              </div>
              <p><strong>{p.summary}</strong></p>
              {p.evidence && <p className="text-sm text-secondary">Evidence: {p.evidence}</p>}
              {p.target_area && <p className="text-sm text-secondary">Target: {p.target_area}</p>}
              {p.proposed_change && <p className="text-sm" style={{marginTop:'0.25rem'}}>{p.proposed_change}</p>}
              <div style={{marginTop:'0.75rem', display:'flex', gap:'0.5rem'}}>
                <button className="btn btn-success btn-sm" onClick={() => handleReview(p.id, 'accepted')}>Accept</button>
                <button className="btn btn-danger btn-sm" onClick={() => handleReview(p.id, 'rejected')}>Reject</button>
              </div>
            </div>
          ))
        )}
      </div>

      {reviewedProposals.length > 0 && (
        <div className="card">
          <h3 className="form-title">Reviewed Proposals ({reviewedProposals.length})</h3>
          {reviewedProposals.map(p => (
            <div key={p.id} className="proposal-item" style={{marginBottom:'0.5rem', padding:'0.5rem', opacity:0.7}}>
              <div style={{display:'flex', justifyContent:'space-between'}}>
                <span className="badge">{p.proposal_type}</span>
                <span className={`badge status-${p.status}`}>{p.status}</span>
              </div>
              <p className="text-sm">{p.summary}</p>
              {p.reviewed_at && <p className="text-sm text-secondary">Reviewed: {new Date(p.reviewed_at).toLocaleString()}</p>}
            </div>
          ))}
        </div>
      )}

      <div className="card">
        <h3 className="form-title">Safety Notice</h3>
        <p className="text-secondary">
          This panel generates improvement <strong>suggestions only</strong>. No code, configuration, or file changes are ever applied automatically. All proposals marked "requires approval" must be explicitly accepted by a human. Accepted proposals are logged for future reference but do not trigger any automated actions.
        </p>
      </div>
    </div>
  );
}
