import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { MEMORY_TYPES, LAYER_DEFINITIONS } from '../../shared/constants';
import type { ProjectMemory, MemoryVersion } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function MemoryCenter() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const [memories, setMemories] = useState<ProjectMemory[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeType, setActiveType] = useState<string | null>(null);
  const [selectedMemoryId, setSelectedMemoryId] = useState<string | null>(null);
  const [versions, setVersions] = useState<MemoryVersion[]>([]);
  const [versionsLoading, setVersionsLoading] = useState(false);

  useEffect(() => {
    if (currentProject) {
      loadMemory(activeType ?? undefined);
    }
  }, [currentProject, activeType]);

  async function loadMemory(memoryType?: string) {
    if (!currentProject) return;
    setLoading(true);
    try {
      const result = await tauri.getProjectMemory(currentProject.id, memoryType);
      setMemories(result);
    } catch (e: any) {
      console.error('Failed to load memory:', e);
    }
    setLoading(false);
  }

  async function handleShowVersions(memoryId: string) {
    if (selectedMemoryId === memoryId) {
      setSelectedMemoryId(null);
      setVersions([]);
      return;
    }
    setSelectedMemoryId(memoryId);
    setVersionsLoading(true);
    try {
      const v = await tauri.getMemoryVersions(memoryId);
      setVersions(v);
    } catch (e: any) {
      console.error('Failed to load versions:', e);
      setVersions([]);
    }
    setVersionsLoading(false);
  }

  function handleTypeFilter(type: string | null) {
    setActiveType(activeType === type ? null : type);
  }

  const layerLabel = (layer: string) => {
    const def = LAYER_DEFINITIONS.find(d => d.layer === layer);
    return def ? `${def.layer}: ${def.name}` : layer;
  };

  if (!currentProject) {
    return (
      <div className="page">
        <div className="empty-state">
          <h2 className="page-title">Memory Center</h2>
          <p className="text-secondary">Please open a project first from the Project Dashboard.</p>
        </div>
      </div>
    );
  }

  const grouped = memories.reduce<Record<string, ProjectMemory[]>>((acc, mem) => {
    if (!acc[mem.memory_type]) acc[mem.memory_type] = [];
    acc[mem.memory_type].push(mem);
    return acc;
  }, {});

  return (
    <div className="page">
      <h2 className="page-title">Memory Center: {currentProject.name}</h2>

      <div className="memory-filters">
        <button
          className={`btn btn-sm ${activeType === null ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => handleTypeFilter(null)}
        >
          All
        </button>
        {MEMORY_TYPES.map((type) => {
          const count = grouped[type]?.length || 0;
          return (
            <button
              key={type}
              className={`btn btn-sm ${activeType === type ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => handleTypeFilter(type)}
            >
              {type.replace(/_/g, ' ')} ({count})
            </button>
          );
        })}
      </div>

      {loading && <p className="text-secondary">Loading...</p>}

      {!loading && memories.length === 0 && (
        <div className="empty-state">
          <p className="text-secondary">No memories stored yet. Run an agent workflow to generate content.</p>
        </div>
      )}

      {Object.entries(grouped).map(([type, items]) => (
        <div key={type} className="memory-section">
          <h3 className="memory-type-title">{type.replace(/_/g, ' ')}</h3>
          {items.map((mem) => (
            <div key={mem.id}>
              <div
                className="card memory-card"
                style={{cursor: 'pointer'}}
                onClick={() => handleShowVersions(mem.id)}
              >
                <div className="memory-card-header">
                  <span className="memory-key">{mem.key.replace(/_/g, ' ')}</span>
                  <span className="text-sm text-secondary">
                    {layerLabel(mem.layer)} · v{mem.version} · confidence: {mem.confidence.toFixed(2)}
                  </span>
                  <span className="text-sm text-secondary">
                    {new Date(mem.updated_at).toLocaleDateString()}
                  </span>
                </div>
                <pre className="memory-value">{mem.value}</pre>
              </div>
              {selectedMemoryId === mem.id && (
                <div className="card" style={{marginLeft: '1rem', marginTop: '-0.5rem', marginBottom: '1rem'}}>
                  <h4 className="form-title">Revision History ({versions.length})</h4>
                  {versionsLoading && <p className="text-secondary">Loading...</p>}
                  {!versionsLoading && versions.length === 0 && (
                    <p className="text-secondary">No previous versions.</p>
                  )}
                  {versions.map((v) => (
                    <div key={v.id} className="memory-version-item" style={{marginBottom: '0.5rem', padding: '0.5rem', borderLeft: '2px solid var(--primary)'}}>
                      <div className="text-sm text-secondary">
                        {new Date(v.created_at).toLocaleString()}
                        {v.source && ` · source: ${v.source}`}
                      </div>
                      <pre style={{fontSize: '0.8rem', maxHeight: '150px', overflow: 'auto', whiteSpace: 'pre-wrap'}}>{v.old_value}</pre>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
