import React, { useEffect, useState } from 'react';
import { useModelStore } from '../stores/useModelStore';
import { useT } from '../../shared/i18n';

export default function ModelSettings() {
  const t = useT();
  const { config, testing, testResult, error, loadConfig, saveConfig, testConnection } = useModelStore();
  const [baseUrl, setBaseUrl] = useState('https://api.openai.com/v1');
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState('gpt-4o');
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(4096);
  const [saving, setSaving] = useState(false);

  useEffect(() => { loadConfig(); }, []);
  useEffect(() => {
    if (config) { setBaseUrl(config.base_url); setModel(config.model); setTemperature(config.temperature); setMaxTokens(config.max_tokens); }
  }, [config]);

  return (
    <div className="page">
      <h2 className="page-title">{t.settings.title}</h2>
      {testResult && <div className={`alert ${testResult.success ? 'alert-success' : 'alert-error'}`}>{testResult.message}</div>}
      {error && <div className="alert alert-error">{error}</div>}
      {config && (
        <div className="card config-summary">
          <h3 className="form-title">{t.settings.current}</h3>
          <div className="config-details">
            <div className="config-row"><span className="text-secondary">{t.settings.model}:</span><span>{config.model}</span></div>
            <div className="config-row"><span className="text-secondary">{t.settings.baseUrl}:</span><span>{config.base_url}</span></div>
            <div className="config-row"><span className="text-secondary">{t.settings.apiKey}:</span><span>{config.has_api_key ? config.masked_api_key : t.settings.notSet}</span></div>
            <div className="config-row"><span className="text-secondary">{t.settings.temperature}:</span><span>{config.temperature}</span></div>
            <div className="config-row"><span className="text-secondary">{t.settings.maxTokens}:</span><span>{config.max_tokens}</span></div>
          </div>
        </div>
      )}
      <div className="card form-card">
        <h3 className="form-title">{config ? t.settings.update : t.settings.configure}</h3>
        <form onSubmit={(e) => { e.preventDefault(); if (!apiKey && !config?.has_api_key) return; setSaving(true); saveConfig(baseUrl, apiKey || null, model, temperature, maxTokens).then(() => setApiKey('')).finally(() => setSaving(false)); }}>
          <div className="form-group"><label className="form-label">{t.settings.baseUrl}</label><input type="text" className="form-input" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} /></div>
          <div className="form-group"><label className="form-label">{t.settings.apiKey} {config && `(${t.settings.leaveEmpty})`}</label><input type="password" className="form-input" value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder="sk-..." /></div>
          <div className="form-group"><label className="form-label">{t.settings.model}</label><input type="text" className="form-input" value={model} onChange={(e) => setModel(e.target.value)} /></div>
          <div className="form-row"><div className="form-group"><label className="form-label">{t.settings.temperature} ({temperature})</label><input type="range" className="form-range" min={0} max={2} step={0.1} value={temperature} onChange={(e) => setTemperature(parseFloat(e.target.value))} /></div>
          <div className="form-group"><label className="form-label">{t.settings.maxTokens}</label><input type="number" className="form-input" value={maxTokens} onChange={(e) => setMaxTokens(parseInt(e.target.value) || 4096)} min={1} max={32768} /></div></div>
          <div className="form-actions">
            <button type="submit" className="btn btn-primary" disabled={saving}>{saving ? t.settings.saving : t.settings.save}</button>
            <button type="button" className="btn btn-secondary" onClick={() => { if (!apiKey && !config?.has_api_key) return; testConnection(baseUrl, apiKey || null, model); }} disabled={testing}>{testing ? t.settings.testing : t.settings.test}</button>
          </div>
        </form>
      </div>
    </div>
  );
}
