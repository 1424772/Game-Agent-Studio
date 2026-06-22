import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { Document as DocDocument, DocumentChunk, RetrievalRun, HitExcerpt } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function KnowledgeBase() {
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

  async function loadDocuments() {
    if (!currentProject) return; setLoading(true);
    try { setDocuments(await tauri.listDocuments(currentProject.id)); } catch(e) { console.error(e); }
    setLoading(false);
  }

  async function loadRetrievalRuns() {
    if (!currentProject) return;
    try { setRetrievalRuns(await tauri.getRetrievalRuns(currentProject.id)); } catch(e) {}
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!currentProject || !formTitle.trim() || !formContent.trim()) return;
    try {
      await tauri.createDocument(currentProject.id, formTitle, formContent, formType, null);
      setShowCreate(false); setFormTitle(''); setFormContent('');
      await loadDocuments();
    } catch(e) { console.error(e); }
  }

  async function handleChunk(docId: string) {
    try {
      await tauri.chunkDocument(docId);
      await loadDocuments();
    } catch(e) { console.error(e); }
  }

  async function handleSelectDoc(doc: DocDocument) {
    setSelectedDoc(doc);
    try { setChunks(await tauri.getDocumentChunks(doc.id)); } catch(e) { setChunks([]); }
  }

  async function handleSearch() {
    if (!currentProject || !searchQuery.trim()) return;
    try {
      const { run } = await tauri.searchDocuments(currentProject.id, searchQuery, 10);
      const excerpts = await tauri.getRetrievalHitExcerpts(run.id);
      setSearchResults({ run, excerpts });
    } catch(e) { console.error(e); }
  }

  if (!currentProject) {
    return <div className="page"><div className="empty-state"><h2 className="page-title">Knowledge Base</h2><p className="text-secondary">Please open a project first.</p></div></div>;
  }

  return (
    <div className="page">
      <h2 className="page-title">Knowledge Base: {currentProject.name}</h2>

      <div className="card form-card">
        <h3 className="form-title">Search Documents</h3>
        <div style={{display:'flex', gap:'0.5rem'}}>
          <input className="form-input" value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder="Search across all document chunks..." onKeyDown={e => e.key==='Enter' && handleSearch()} />
          <button className="btn btn-primary" onClick={handleSearch}>Search</button>
        </div>
        {searchResults && (
          <div style={{marginTop:'1rem'}}>
            <p className="text-sm text-secondary">{searchResults.excerpts.length} results in {searchResults.run.duration_ms}ms</p>
            {searchResults.excerpts.map(h => (
              <div key={h.id} style={{padding:'0.5rem', borderBottom:'1px solid var(--border)'}}>
                <div className="text-sm" style={{display:'flex', justifyContent:'space-between'}}>
                  <span><strong>{h.doc_title}</strong> <span className="badge">{h.doc_type}</span></span>
                  <span className="text-secondary">Score: {h.score.toFixed(1)} | Rank: {h.rank}</span>
                </div>
                <pre style={{fontSize:'0.8rem', maxHeight:'80px', overflow:'auto', marginTop:'0.25rem', whiteSpace:'pre-wrap'}}>{h.chunk_excerpt}{h.chunk_excerpt.length >= 200 ? '...' : ''}</pre>
                <div className="text-sm text-secondary" style={{marginTop:'0.25rem'}}>
                  {h.source && <span>Source: {h.source}</span>}
                  {h.provenance && <span style={{marginLeft:'1rem'}}>Provenance: {h.provenance.slice(0, 80)}</span>}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <div style={{display:'flex', justifyContent:'space-between', alignItems:'center'}}>
          <h3 className="form-title">Documents ({documents.length})</h3>
          <button className="btn btn-primary btn-sm" onClick={() => setShowCreate(!showCreate)}>{showCreate ? 'Cancel' : '+ New Document'}</button>
        </div>

        {showCreate && (
          <form onSubmit={handleCreate} style={{marginBottom:'1rem'}}>
            <div className="form-group">
              <input className="form-input" value={formTitle} onChange={e => setFormTitle(e.target.value)} placeholder="Document title" required />
            </div>
            <div className="form-group">
              <select className="form-input" value={formType} onChange={e => setFormType(e.target.value)}>
                <option value="game_design">Game Design</option>
                <option value="template">Template</option>
                <option value="reference">Reference</option>
                <option value="export_guide">Export Guide</option>
              </select>
            </div>
            <div className="form-group">
              <textarea className="form-input form-textarea" value={formContent} onChange={e => setFormContent(e.target.value)} placeholder="Document content..." rows={6} required />
            </div>
            <button type="submit" className="btn btn-primary btn-sm">Create Document</button>
          </form>
        )}

        {loading && <p className="text-secondary">Loading...</p>}
        {!loading && documents.length === 0 && <p className="text-secondary">No documents. Create one to start building your knowledge base.</p>}

        {documents.map(doc => (
          <div key={doc.id} className="card" style={{cursor:'pointer', padding:'0.75rem', marginBottom:'0.5rem'}} onClick={() => handleSelectDoc(doc)}>
            <div style={{display:'flex', justifyContent:'space-between'}}>
              <span><strong>{doc.title}</strong></span>
              <span className="badge">{doc.doc_type}</span>
            </div>
            <div className="text-sm text-secondary">
              Chunks: {doc.chunk_count} | {new Date(doc.created_at).toLocaleDateString()}
              {doc.chunk_count === 0 && <button className="btn btn-secondary btn-sm" style={{marginLeft:'1rem'}} onClick={e => {e.stopPropagation(); handleChunk(doc.id);}}>Chunk</button>}
            </div>
            {selectedDoc?.id === doc.id && (
              <div style={{marginTop:'0.5rem'}}>
                {chunks.length === 0 ? <p className="text-secondary">No chunks. Click "Chunk" to split this document.</p> :
                  chunks.map(c => (
                    <div key={c.id} style={{padding:'0.25rem 0', borderTop:'1px solid var(--border)'}}>
                      <span className="text-sm text-secondary">Chunk #{c.chunk_index}</span>
                      <pre style={{fontSize:'0.75rem', maxHeight:'80px', overflow:'auto'}}>{c.content.slice(0, 300)}{c.content.length > 300 ? '...' : ''}</pre>
                    </div>
                  ))}
              </div>
            )}
          </div>
        ))}
      </div>

      {retrievalRuns.length > 0 && (
        <div className="card">
          <h3 className="form-title">Retrieval History</h3>
          {retrievalRuns.map(r => (
            <div key={r.id} style={{padding:'0.5rem', borderBottom:'1px solid var(--border)'}}>
              <span className="badge">{r.strategy || 'keyword'}</span>
              <span style={{marginLeft:'0.5rem'}}>"{r.query_text}"</span>
              <span className="text-sm text-secondary" style={{marginLeft:'1rem'}}>{r.result_count} hits · {r.duration_ms}ms</span>
              <span className="text-sm text-secondary" style={{marginLeft:'1rem'}}>{new Date(r.created_at).toLocaleString()}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
