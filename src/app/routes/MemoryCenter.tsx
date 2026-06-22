import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { MEMORY_TYPES } from '../../shared/constants';
import type { ProjectMemory } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function MemoryCenter() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const [memories, setMemories] = useState<ProjectMemory[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeType, setActiveType] = useState<string | null>(null);

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

  function handleTypeFilter(type: string | null) {
    setActiveType(activeType === type ? null : type);
  }

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
            <div key={mem.id} className="card memory-card">
              <div className="memory-card-header">
                <span className="memory-key">{mem.key.replace(/_/g, ' ')}</span>
                <span className="text-sm text-secondary">
                  {new Date(mem.updated_at).toLocaleDateString()}
                </span>
              </div>
              <pre className="memory-value">{mem.value}</pre>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
