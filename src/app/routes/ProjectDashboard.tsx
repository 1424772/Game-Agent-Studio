import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import { useAppStore } from '../stores/useAppStore';
import { GAME_TYPES } from '../../shared/constants';
import { useT } from '../../shared/i18n';
import type { Project } from '../../shared/types';

export default function ProjectDashboard() {
  const t = useT();
  const { projects, loading, error, loadProjects, createProject, openProject, deleteProject } = useProjectStore();
  const setRoute = useAppStore((s) => s.setRoute);
  const setCurrentProjectId = useAppStore((s) => s.setCurrentProject);

  const [showForm, setShowForm] = useState(false);
  const [formName, setFormName] = useState('');
  const [formGameType, setFormGameType] = useState<string>(GAME_TYPES[0].value);
  const [formDescription, setFormDescription] = useState('');
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => { loadProjects(); }, [loadProjects]);

  function handleCreateProject(e: React.FormEvent) {
    e.preventDefault();
    if (!formName.trim()) return;
    setSubmitting(true);
    createProject(formName, formGameType, formDescription)
      .then(() => { setShowForm(false); setFormName(''); setFormDescription(''); })
      .finally(() => setSubmitting(false));
  }

  function handleOpen(project: Project) {
    openProject(project);
    setCurrentProjectId(project.id);
    setRoute('workspace');
  }

  function handleDelete(id: string, name: string) {
    if (window.confirm(`${t.dashboard.confirmDelete} "${name}"?`)) deleteProject(id);
  }

  return (
    <div className="page">
      <div className="page-header">
        <h2 className="page-title">{t.dashboard.title}</h2>
        <button className="btn btn-primary" onClick={() => setShowForm(!showForm)}>
          {showForm ? t.common.cancel : t.dashboard.newProject}
        </button>
      </div>
      {error && <div className="alert alert-error">{error}</div>}
      {showForm && (
        <div className="card form-card">
          <h3 className="form-title">{t.dashboard.createTitle}</h3>
          <form onSubmit={handleCreateProject}>
            <div className="form-group">
              <label className="form-label">{t.dashboard.projectName}</label>
              <input type="text" className="form-input" value={formName} onChange={(e) => setFormName(e.target.value)} placeholder={t.dashboard.placeholder.name} required />
            </div>
            <div className="form-group">
              <label className="form-label">{t.dashboard.gameType}</label>
              <select className="form-input" value={formGameType} onChange={(e) => setFormGameType(e.target.value)}>
                {GAME_TYPES.map((gt) => (<option key={gt.value} value={gt.value}>{gt.label}</option>))}
              </select>
            </div>
            <div className="form-group">
              <label className="form-label">{t.dashboard.description}</label>
              <textarea className="form-input form-textarea" value={formDescription} onChange={(e) => setFormDescription(e.target.value)} placeholder={t.dashboard.placeholder.desc} rows={4} />
            </div>
            <button type="submit" className="btn btn-primary" disabled={submitting}>
              {submitting ? t.dashboard.creating : t.dashboard.create}
            </button>
          </form>
        </div>
      )}
      {loading && <p className="text-secondary">{t.dashboard.loading}</p>}
      {!loading && projects.length === 0 && (
        <div className="empty-state"><p className="text-secondary">{t.dashboard.noProjects}</p></div>
      )}
      <div className="project-grid">
        {projects.map((project) => (
          <div key={project.id} className="card project-card">
            <div className="project-card-header">
              <h3 className="project-card-title">{project.name}</h3>
              <span className="badge">{project.game_type}</span>
            </div>
            <p className="project-card-desc">{project.description || '-'}</p>
            <p className="text-secondary text-sm">{t.dashboard.created}: {new Date(project.created_at).toLocaleDateString()}</p>
            <div className="project-card-actions">
              <button className="btn btn-primary btn-sm" onClick={() => handleOpen(project)}>{t.dashboard.open}</button>
              <button className="btn btn-danger btn-sm" onClick={() => handleDelete(project.id, project.name)}>{t.dashboard.delete}</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
