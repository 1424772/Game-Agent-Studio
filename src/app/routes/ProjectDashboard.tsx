import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { useAppStore } from '../stores/useAppStore';
import { GAME_TYPES } from '../../shared/constants';
import type { Project } from '../../shared/types';

export default function ProjectDashboard() {
  const { projects, loading, error, loadProjects, createProject, openProject, deleteProject } = useProjectStore();
  const setRoute = useAppStore((s) => s.setRoute);
  const setCurrentProjectId = useAppStore((s) => s.setCurrentProject);

  const [showForm, setShowForm] = useState(false);
  const [formName, setFormName] = useState('');
  const [formGameType, setFormGameType] = useState<string>(GAME_TYPES[0].value);
  const [formDescription, setFormDescription] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  function handleCreateProject(e: React.FormEvent) {
    e.preventDefault();
    if (!formName.trim()) return;
    setSubmitting(true);
    createProject(formName, formGameType, formDescription)
      .then(() => {
        setShowForm(false);
        setFormName('');
        setFormGameType(GAME_TYPES[0].value);
        setFormDescription('');
      })
      .finally(() => setSubmitting(false));
  }

  function handleOpen(project: Project) {
    openProject(project);
    setCurrentProjectId(project.id);
    setRoute('workspace');
  }

  function handleDelete(id: string, name: string) {
    if (window.confirm(`Delete project "${name}"? This cannot be undone.`)) {
      deleteProject(id);
    }
  }

  return (
    <div className="page">
      <div className="page-header">
        <h2 className="page-title">Project Dashboard</h2>
        <button className="btn btn-primary" onClick={() => setShowForm(!showForm)}>
          {showForm ? 'Cancel' : '+ New Project'}
        </button>
      </div>

      {error && <div className="alert alert-error">{error}</div>}

      {showForm && (
        <div className="card form-card">
          <h3 className="form-title">Create New Project</h3>
          <form onSubmit={handleCreateProject}>
            <div className="form-group">
              <label className="form-label">Project Name</label>
              <input
                type="text"
                className="form-input"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="My Awesome Game"
                required
              />
            </div>
            <div className="form-group">
              <label className="form-label">Game Type</label>
              <select
                className="form-input"
                value={formGameType}
                onChange={(e) => setFormGameType(e.target.value)}
              >
                {GAME_TYPES.map((gt) => (
                  <option key={gt.value} value={gt.value}>
                    {gt.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label className="form-label">Description</label>
              <textarea
                className="form-input form-textarea"
                value={formDescription}
                onChange={(e) => setFormDescription(e.target.value)}
                placeholder="Brief description of your game concept..."
                rows={4}
              />
            </div>
            <button type="submit" className="btn btn-primary" disabled={submitting}>
              {submitting ? 'Creating...' : 'Create Project'}
            </button>
          </form>
        </div>
      )}

      {loading && <p className="text-secondary">Loading projects...</p>}

      {!loading && projects.length === 0 && (
        <div className="empty-state">
          <p className="text-secondary">No projects yet. Create your first project to get started.</p>
        </div>
      )}

      <div className="project-grid">
        {projects.map((project) => (
          <div key={project.id} className="card project-card">
            <div className="project-card-header">
              <h3 className="project-card-title">{project.name}</h3>
              <span className="badge">{project.game_type}</span>
            </div>
            <p className="project-card-desc">{project.description || 'No description'}</p>
            <p className="text-secondary text-sm">
              Created: {new Date(project.created_at).toLocaleDateString()}
            </p>
            <div className="project-card-actions">
              <button className="btn btn-primary btn-sm" onClick={() => handleOpen(project)}>
                Open
              </button>
              <button className="btn btn-danger btn-sm" onClick={() => handleDelete(project.id, project.name)}>
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
