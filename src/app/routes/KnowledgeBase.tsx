import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { Document as DocDocument, DocumentChunk, RetrievalRun, HitExcerpt } from '../../shared/types';
import { useT } from '../../shared/i18n';
import * as tauri from '../../shared/utils/tauri';

export default function KnowledgeBase() {
  const t = useT();
  const currentProject = useProjectStore((s) => s.currentProject);
  const [documents, setDocuments] = useState<DocDocument[]>([]);
  const [loading, setLoading] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [formTitle, setFormTitle] = useState('');
  const [formContent, setFormContent] = useState('');
  const [formType, setFormType] = useState('game_design');
  const [selectedDoc, setSelectedDoc] = useState<DocDocument | null>(null);
  const [chunks, setChunks] = useState<DocumentChunk[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<{run: RetrievalRun; excerpts: HitExcerpt[]} | null>(null);
  const [retrievalRuns, setRetrievalRuns] = useState<RetrievalRun[]>([]);

  useEffect(() => { if (currentProject) { loadDocuments(); loadRetrievalRuns(); } }, [currentProject]);

  async function loadDocuments() { if (!currentProject) return; setLoading(true); try { setDocuments(await tauri.listDocuments(currentProject.id)); } catch(e){} setLoading(false); }
  async function loadRetrievalRuns() { if (!currentProject) return; try { setRetrievalRuns(await tauri.getRetrievalRuns(currentProject.id)); } catch(e){} }

  async function handleCreate(e: React.FormEvent) { e.preventDefault(); if (!currentProject) return; try { await tauri.createDocument(currentProject.id, formTitle, formContent, formType, null); setShowCreate(false); setFormTitle(''); setFormContent(''); await loadDocuments(); } catch(e){} }
  async function handleChunk(docId: string) { try { await tauri.chunkDocument(docId); await loadDocuments(); } catch(e){} }
  async function handleSelectDoc(doc: DocDocument) { setSelectedDoc(doc); try { setChunks(await tauri.getDocumentChunks(doc.id)); } catch(e){} }
  async function handleSearch() { if (!currentProject || !searchQuery.trim()) return; try { const { run } = await tauri.searchDocuments(currentProject.id, searchQuery, 10); const excerpts = await tauri.getRetrievalHitExcerpts(run.id); setSearchResults({run,excerpts}); } catch(e){} }

  if (!currentProject) return <div className="page"><div className="empty-state"><h2 className="page-title">{t.knowledge.title}</h2><p className="text-secondary">{t.knowledge.noProject}</p></div></div>;

  return (
    <div className="page">
      <h2 className="page-title">{t.knowledge.title}: {currentProject.name}</h2>
      <div className="card form-card"><h3 className="form-title">{t.knowledge.search}</h3><div style={{display:'flex',gap:'0.5rem'}}><input className="form-input" value={searchQuery} onChange={e=>setSearchQuery(e.target.value)} placeholder={t.knowledge.searchPlaceholder} onKeyDown={e=>e.key==='Enter'&&handleSearch()}/><button className="btn btn-primary" onClick={handleSearch}>{t.knowledge.search}</button></div>
        {searchResults && <div style={{marginTop:'1rem'}}><p className="text-sm text-secondary">{searchResults.excerpts.length} {t.knowledge.results} / {searchResults.run.duration_ms}{t.knowledge.ms} · {searchResults.run.strategy||'keyword'}</p>{searchResults.excerpts.map(h=><div key={h.id} style={{padding:'0.5rem',borderBottom:'1px solid var(--border)'}}><div style={{display:'flex',justifyContent:'space-between'}}><span><strong>{h.doc_title}</strong> <span className="badge">{h.doc_type}</span></span><span className="text-secondary">Score: {h.score.toFixed(1)} | Rank: {h.rank}</span></div><pre style={{fontSize:'0.8rem',maxHeight:'80px',overflow:'auto',marginTop:'0.25rem'}}>{h.chunk_excerpt}</pre><div className="text-sm text-secondary">{h.source && <span>Source: {h.source}</span>}{h.provenance && <span style={{marginLeft:'1rem'}}>{h.provenance.slice(0,80)}</span>}{h.score_breakdown && <span style={{marginLeft:'1rem'}}>BD: {h.score_breakdown.slice(0,60)}</span>}</div></div>)}</div>}
      </div>
      <div className="card"><div style={{display:'flex',justifyContent:'space-between',alignItems:'center'}}><h3 className="form-title">{t.knowledge.title} ({documents.length})</h3><button className="btn btn-primary btn-sm" onClick={()=>setShowCreate(!showCreate)}>{showCreate?t.knowledge.cancel:t.knowledge.newDoc}</button></div>
        <div style={{display:'flex',gap:'0.5rem',marginBottom:'1rem'}}>
          <button className="btn btn-secondary btn-sm" onClick={async ()=>{if(currentProject) try{await tauri.embedPendingChunks(currentProject.id,10);await loadDocuments();}catch(e){}}}>Embed Pending Chunks</button>
        </div>
        {showCreate && <form onSubmit={handleCreate} style={{marginBottom:'1rem'}}><div className="form-group"><input className="form-input" value={formTitle} onChange={e=>setFormTitle(e.target.value)} placeholder={t.knowledge.title_} required/></div><div className="form-group"><select className="form-input" value={formType} onChange={e=>setFormType(e.target.value)}><option value="game_design">{t.knowledge.gameDesign}</option><option value="template">{t.knowledge.template}</option><option value="reference">{t.knowledge.reference}</option><option value="export_guide">{t.knowledge.exportGuide}</option></select></div><div className="form-group"><textarea className="form-input form-textarea" value={formContent} onChange={e=>setFormContent(e.target.value)} placeholder={t.knowledge.content} rows={6} required/></div><button type="submit" className="btn btn-primary btn-sm">{t.knowledge.create}</button></form>}
        {loading && <p className="text-secondary">{t.common.loading}</p>}
        {!loading && documents.length===0 && <p className="text-secondary">{t.knowledge.noDocs}</p>}
        {documents.map(doc=><div key={doc.id} className="card" style={{cursor:'pointer',padding:'0.75rem',marginBottom:'0.5rem'}} onClick={()=>handleSelectDoc(doc)}><div style={{display:'flex',justifyContent:'space-between'}}><span><strong>{doc.title}</strong></span><span className="badge">{doc.doc_type}</span></div><div className="text-sm text-secondary">{t.knowledge.chunks}: {doc.chunk_count} | {new Date(doc.created_at).toLocaleDateString()}{doc.chunk_count===0 && <button className="btn btn-secondary btn-sm" style={{marginLeft:'1rem'}} onClick={e=>{e.stopPropagation();handleChunk(doc.id);}}>{t.knowledge.chunk}</button>}</div>{selectedDoc?.id===doc.id && <div style={{marginTop:'0.5rem'}}>{chunks.length===0?<p className="text-secondary">{t.knowledge.noDocs}</p>:chunks.map(c=><div key={c.id} style={{padding:'0.25rem 0',borderTop:'1px solid var(--border)'}}><div style={{display:'flex',justifyContent:'space-between'}}><span className="text-sm text-secondary">#{c.chunk_index}</span><span className={`badge ${c.embedding_status==='embedded'?'status-completed':c.embedding_status==='failed'?'status-failed':'status-pending'}`}>{c.embedding_status||'pending'}</span></div><pre style={{fontSize:'0.75rem',maxHeight:'80px',overflow:'auto'}}>{c.content.slice(0,300)}</pre>{c.embedded_at&&<div className="text-sm text-secondary">Embedded: {new Date(c.embedded_at).toLocaleString()}</div>}{c.embedding_error&&<div className="text-sm text-secondary" style={{color:'var(--danger)'}}>{c.embedding_error.slice(0,100)}</div>}</div>)}</div>}</div>)}
      </div>
      {retrievalRuns.length>0 && <div className="card"><h3 className="form-title">{t.knowledge.retrievalHistory}</h3>{retrievalRuns.map(r=><div key={r.id} style={{padding:'0.5rem',borderBottom:'1px solid var(--border)'}}><span className="badge">{r.strategy||'keyword'}</span><span style={{marginLeft:'0.5rem'}}>"{r.query_text}"</span><span className="text-sm text-secondary" style={{marginLeft:'1rem'}}>{r.result_count} {t.knowledge.hits} · {r.duration_ms}{t.knowledge.ms}</span><span className="text-sm text-secondary" style={{marginLeft:'1rem'}}>{new Date(r.created_at).toLocaleString()}</span></div>)}</div>}
    </div>
  );
}
