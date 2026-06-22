import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { ImprovementProposal } from '../../shared/types';
import { useT } from '../../shared/i18n';
import * as tauri from '../../shared/utils/tauri';

export default function SelfIterationPanel() {
  const t = useT();
  const currentProject = useProjectStore((s) => s.currentProject);
  const [proposals, setProposals] = useState<ImprovementProposal[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => { loadProposals(); }, [currentProject]);

  async function loadProposals() { setLoading(true); try { setProposals(await tauri.listImprovementProposals(null)); } catch(e){} setLoading(false); }

  async function handleGenerateProposals() {
    if (!currentProject) return; setLoading(true);
    try {
      const events = await tauri.getEvents(currentProject.id, null, null, null, 200);
      const accepted = events.filter(e=>e.event_type==='output_accepted').length;
      const rejected = events.filter(e=>e.event_type==='output_rejected').length;
      const edited = events.filter(e=>e.event_type==='output_edited').length;
      const total = accepted+rejected+edited;
      const wfFailed = events.filter(e=>e.event_type==='agent_workflow_failed'||e.event_type==='workflow_failed').length;
      const wfCompleted = events.filter(e=>e.event_type==='agent_workflow_completed'||e.event_type==='workflow_complete').length;

      if (total>0) {
        const rate = Math.round((accepted/total)*100);
        if (rate<50) await tauri.createImprovementProposal('prompt_improvement','Low output acceptance rate',`Acceptance rate: ${rate}% (${accepted}/${total})`,'high','agent_prompts','Review and refine system prompts for better output quality.');
        if (edited>accepted) await tauri.createImprovementProposal('prompt_improvement','High edit rate exceeds acceptance rate',`Edits: ${edited}, Accepted: ${accepted}`,'medium','agent_prompts','Adjust prompt templates to reduce manual edits.');
      }
      if (wfFailed>2) await tauri.createImprovementProposal('workflow_improvement','Multiple workflow failures',`${wfFailed} workflow failures`,'high','workflow_execution','Add retry logic for failed steps.');
      if (wfCompleted>=3 && total===0) await tauri.createImprovementProposal('ui_ux_improvement','Workflows completed but no user feedback',`${wfCompleted} workflows with no accept/reject actions.`,'low','agent_workspace_ui','Add review notification to highlight un-reviewed outputs.');

      await loadProposals();
    } catch(e){}
    setLoading(false);
  }

  async function handleReview(proposalId: string, newStatus: string) { try { await tauri.reviewImprovementProposal(proposalId, newStatus); await loadProposals(); } catch(e){} }

  const pendingProposals = proposals.filter(p => p.status==='proposed');
  const reviewedProposals = proposals.filter(p => p.status!=='proposed' && p.status!=='draft');

  return (
    <div className="page">
      <h2 className="page-title">{t.iteration.title}</h2>
      <div className="card"><h3 className="form-title">{t.iteration.generate}</h3><p className="text-secondary" style={{marginBottom:'1rem'}}>{t.iteration.desc}</p><button className="btn btn-primary" onClick={handleGenerateProposals} disabled={loading||!currentProject}>{loading?t.iteration.analyzing:t.iteration.generate}</button>{!currentProject&&<p className="text-secondary" style={{marginTop:'0.5rem'}}>{t.iteration.openProject}</p>}</div>
      <div className="card"><h3 className="form-title">{t.iteration.pending} ({pendingProposals.length})</h3>{pendingProposals.length===0?<p className="text-secondary">{t.iteration.noPending}</p>:pendingProposals.map(p=><div key={p.id} style={{marginBottom:'1rem',padding:'0.75rem',border:'1px solid var(--border)',borderRadius:'4px'}}><div style={{display:'flex',justifyContent:'space-between',marginBottom:'0.5rem'}}><span className="badge">{p.proposal_type}</span><span className={`badge ${p.risk_level==='high'?'status-failed':p.risk_level==='medium'?'status-running':''}`}>{t.iteration.risk}: {p.risk_level||'low'}</span>{p.requires_human_approval&&<span className="badge" style={{background:'var(--primary)'}}>{t.iteration.requiresApproval}</span>}</div><p><strong>{p.summary}</strong></p>{p.evidence&&<p className="text-sm text-secondary">{p.evidence}</p>}{p.target_area&&<p className="text-sm text-secondary">{p.target_area}</p>}<div style={{marginTop:'0.75rem',display:'flex',gap:'0.5rem'}}><button className="btn btn-success btn-sm" onClick={()=>handleReview(p.id,'accepted')}>{t.iteration.accept}</button><button className="btn btn-danger btn-sm" onClick={()=>handleReview(p.id,'rejected')}>{t.iteration.reject}</button></div></div>)}</div>
      {reviewedProposals.length>0 && <div className="card"><h3 className="form-title">{t.iteration.reviewed} ({reviewedProposals.length})</h3>{reviewedProposals.map(p=><div key={p.id} style={{marginBottom:'0.5rem',padding:'0.5rem',opacity:0.7}}><div style={{display:'flex',justifyContent:'space-between'}}><span className="badge">{p.proposal_type}</span><span className={`badge status-${p.status}`}>{p.status}</span></div><p className="text-sm">{p.summary}</p>{p.reviewed_at&&<p className="text-sm text-secondary">{new Date(p.reviewed_at).toLocaleString()}</p>}</div>)}</div>}
      <div className="card"><h3 className="form-title">{t.iteration.safety}</h3><p className="text-secondary">{t.iteration.safetyDesc}</p></div>
    </div>
  );
}
