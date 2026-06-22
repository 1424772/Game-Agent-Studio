import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { ExportRecord } from '../../shared/types';
import { useT } from '../../shared/i18n';
import * as tauri from '../../shared/utils/tauri';

export default function ExportCenter() {
  const t = useT();
  const currentProject = useProjectStore((s) => s.currentProject);
  const [exporting, setExporting] = useState(false);
  const [exportType, setExportType] = useState<'markdown' | 'json'>('markdown');
  const [exportStatus, setExportStatus] = useState<string | null>(null);
  const [exportHistory, setExportHistory] = useState<ExportRecord[]>([]);

  useEffect(() => { if (currentProject) loadExports(); }, [currentProject]);

  async function loadExports() { if (!currentProject) return; try { setExportHistory(await tauri.getExports(currentProject.id)); } catch (e) {} }

  async function handleExport() {
    if (!currentProject) return; setExporting(true); setExportStatus(null);
    try {
      const r = exportType === 'markdown' ? await tauri.exportMarkdown(currentProject.id) : await tauri.exportJson(currentProject.id);
      setExportStatus(`${t.export.exported}${r.file_path}`);
      await tauri.logEvent(currentProject.id, exportType==='markdown'?'export_markdown':'export_json', JSON.stringify({filePath:r.file_path}));
      loadExports();
    } catch (e: any) { setExportStatus(`${t.export.exportFailed}${e}`); }
    setExporting(false);
  }

  if (!currentProject) return <div className="page"><div className="empty-state"><h2 className="page-title">{t.export.title}</h2><p className="text-secondary">{t.export.noProject}</p></div></div>;

  return (
    <div className="page">
      <h2 className="page-title">{t.export.title}</h2>
      <div className="card form-card">
        <h3 className="form-title">{currentProject.name}</h3>
        <div className="form-group"><label className="form-label">{t.export.format}</label>
          <div className="form-radio-group">
            <label className="form-radio"><input type="radio" value="markdown" checked={exportType==='markdown'} onChange={()=>setExportType('markdown')}/><span>Markdown (.md)</span></label>
            <label className="form-radio"><input type="radio" value="json" checked={exportType==='json'} onChange={()=>setExportType('json')}/><span>JSON (.json)</span></label>
          </div>
        </div>
        <p className="text-secondary" style={{marginBottom:'1rem'}}>{t.export.dir}</p>
        <button className="btn btn-primary" onClick={handleExport} disabled={exporting}>{exporting ? t.export.exporting : exportType==='markdown' ? t.export.exportMd : t.export.exportJson}</button>
        {exportStatus && <div className={`alert ${exportStatus.startsWith(t.export.exported) ? 'alert-success' : 'alert-error'}`}>{exportStatus}</div>}
      </div>
      <div className="card"><h3 className="form-title">{t.export.history}</h3>{exportHistory.length===0?<p className="text-secondary">{t.export.noHistory}</p>:exportHistory.map(r=><div key={r.id} className="export-item"><span className="badge">{r.export_type}</span><span>{r.file_path}</span><span className="text-sm text-secondary">{new Date(r.created_at).toLocaleString()}</span></div>)}</div>
    </div>
  );
}
