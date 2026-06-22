import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { MEMORY_TYPES } from '../../shared/constants';
import { useT } from '../../shared/i18n';
import type { ProjectMemory, MemoryVersion } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function MemoryCenter() {
  const t = useT();
  const currentProject = useProjectStore((s) => s.currentProject);
  const [memories, setMemories] = useState<ProjectMemory[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeType, setActiveType] = useState<string | null>(null);
  const [selectedMemoryId, setSelectedMemoryId] = useState<string | null>(null);
  const [versions, setVersions] = useState<MemoryVersion[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);

  useEffect(() => { if (currentProject) loadMemory(activeType ?? undefined); }, [currentProject, activeType]);

  async function loadMemory(memoryType?: string) { if (!currentProject) return; setLoading(true); try { setMemories(await tauri.getProjectMemory(currentProject.id, memoryType)); } catch (e) {} setLoading(false); }

  async function handleShowVersions(memoryId: string) {
    if (selectedMemoryId === memoryId) { setSelectedMemoryId(null); setVersions([]); return; }
    setSelectedMemoryId(memoryId); setVersionsLoading(true);
    try { setVersions(await tauri.getMemoryVersions(memoryId)); } catch (e) { setVersions([]); }
    setVersionsLoading(false);
  }

  if (!currentProject) return <div className="page"><div className="empty-state"><h2 className="page-title">{t.memory.title}</h2><p className="text-secondary">{t.memory.noProject}</p></div></div>;

  const grouped = memories.reduce<Record<string, ProjectMemory[]>>((acc, mem) => { if (!acc[mem.memory_type]) acc[mem.memory_type] = []; acc[mem.memory_type].push(mem); return acc; }, {});

  return (
    <div className="page">
      <h2 className="page-title">{t.memory.title}: {currentProject.name}</h2>
      <div className="memory-filters">
        <button className={`btn btn-sm ${activeType===null?'btn-primary':'btn-secondary'}`} onClick={() => setActiveType(null)}>{t.memory.all}</button>
        {MEMORY_TYPES.map(type => <button key={type} className={`btn btn-sm ${activeType===type?'btn-primary':'btn-secondary'}`} onClick={() => setActiveType(activeType===type?null:type)}>{type.replace(/_/g,' ')} ({(grouped[type]?.length||0)})</button>)}
      </div>
      {loading && <p className="text-secondary">{t.memory.loading}</p>}
      {!loading && memories.length === 0 && <div className="empty-state"><p className="text-secondary">{t.memory.empty}</p></div>}
      {Object.entries(grouped).map(([type, items]) => <div key={type}><h3 className="memory-type-title">{type.replace(/_/g,' ')}</h3>{items.map(mem => <div key={mem.id}><div className="card memory-card" style={{cursor:'pointer'}} onClick={() => handleShowVersions(mem.id)}><div className="memory-card-header"><span className="memory-key">{mem.key.replace(/_/g,' ')}</span><span className="text-sm text-secondary">{mem.layer} · v{mem.version} · {(mem.confidence*100).toFixed(0)}%</span><span className="text-sm text-secondary">{new Date(mem.updated_at).toLocaleDateString()}</span></div><pre className="memory-value">{mem.value}</pre></div>{selectedMemoryId===mem.id && <div className="card" style={{marginLeft:'1rem',marginTop:'-0.5rem',marginBottom:'1rem'}}><h4>{t.memory.revisionHistory} ({versions.length})</h4>{versionsLoading?<p className="text-secondary">{t.memory.loading}</p>:versions.length===0?<p className="text-secondary">{t.memory.noVersions}</p>:versions.map(v=><div key={v.id} style={{padding:'0.5rem',borderLeft:'2px solid var(--primary)'}}><div className="text-sm text-secondary">{new Date(v.created_at).toLocaleString()}{v.source&&` · ${v.source}`}</div><pre style={{fontSize:'0.8rem',maxHeight:'150px',overflow:'auto'}}>{v.old_value}</pre></div>)}</div>}</div>)}</div>)}
    </div>
  );
}
