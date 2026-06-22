import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { ExportRecord } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function ExportCenter() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const [exporting, setExporting] = useState(false);
  const [exportType, setExportType] = useState<'markdown' | 'json'>('markdown');
  const [exportStatus, setExportStatus] = useState<string | null>(null);
  const [exportHistory, setExportHistory] = useState<ExportRecord[]>([]);

  useEffect(() => {
    if (currentProject) {
      loadExports();
    }
  }, [currentProject]);

  async function loadExports() {
    if (!currentProject) return;
    try {
      const records = await tauri.getExports(currentProject.id);
      setExportHistory(records);
    } catch (e: any) {
      console.error('Failed to load export history:', e);
    }
  }

  async function handleExport() {
    if (!currentProject) return;
    setExporting(true);
    setExportStatus(null);
    try {
      let record: ExportRecord;
      if (exportType === 'markdown') {
        record = await tauri.exportMarkdown(currentProject.id);
      } else {
        record = await tauri.exportJson(currentProject.id);
      }
      setExportStatus(`Exported successfully to: ${record.file_path}`);
      await tauri.logEvent(currentProject.id,
        exportType === 'markdown' ? 'export_markdown' : 'export_json',
        JSON.stringify({ filePath: record.file_path, exportType })
      );
      loadExports();
    } catch (e: any) {
      setExportStatus(`Export failed: ${e.toString()}`);
    }
    setExporting(false);
  }

  return (
    <div className="page">
      <h2 className="page-title">Export Center</h2>

      {!currentProject ? (
        <div className="empty-state">
          <p className="text-secondary">Please open a project first from the Project Dashboard.</p>
        </div>
      ) : (
        <>
          <div className="card form-card">
            <h3 className="form-title">Export Project: {currentProject.name}</h3>

            <div className="form-group">
              <label className="form-label">Export Format</label>
              <div className="form-radio-group">
                <label className="form-radio">
                  <input
                    type="radio"
                    value="markdown"
                    checked={exportType === 'markdown'}
                    onChange={() => setExportType('markdown')}
                  />
                  <span>Markdown (.md)</span>
                </label>
                <label className="form-radio">
                  <input
                    type="radio"
                    value="json"
                    checked={exportType === 'json'}
                    onChange={() => setExportType('json')}
                  />
                  <span>JSON (.json)</span>
                </label>
              </div>
            </div>

            <p className="text-secondary" style={{marginBottom: '1rem'}}>
              Files will be exported to the application data directory.
            </p>

            <button
              className="btn btn-primary"
              onClick={handleExport}
              disabled={exporting}
            >
              {exporting ? 'Exporting...' : exportType === 'markdown' ? 'Export Markdown' : 'Export JSON'}
            </button>

            {exportStatus && (
              <div className={`alert ${exportStatus.startsWith('Exported') ? 'alert-success' : 'alert-error'}`}>
                {exportStatus}
              </div>
            )}
          </div>

          <div className="card">
            <h3 className="form-title">Export History</h3>
            {exportHistory.length === 0 ? (
              <p className="text-secondary">No exports yet.</p>
            ) : (
              <div className="export-list">
                {exportHistory.map((record) => (
                  <div key={record.id} className="export-item">
                    <div className="export-item-info">
                      <span className="badge">{record.export_type}</span>
                      <span className="export-path">{record.file_path}</span>
                    </div>
                    <span className="text-sm text-secondary">
                      {new Date(record.created_at).toLocaleString()}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
