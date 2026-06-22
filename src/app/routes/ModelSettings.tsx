import React, { useEffect, useState } from 'react';
import { useModelStore } from '../stores/useModelStore';

export default function ModelSettings() {
  const { config, loading, testing, testResult, error, loadConfig, saveConfig, testConnection } = useModelStore();

  const [baseUrl, setBaseUrl] = useState('https://api.openai.com/v1');
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState('gpt-4o');
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(4096);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    if (config) {
      setBaseUrl(config.base_url);
      setModel(config.model);
      setTemperature(config.temperature);
      setMaxTokens(config.max_tokens);
    }
  }, [config]);

  function handleSave(e: React.FormEvent) {
    e.preventDefault();
    if (!apiKey && !config?.has_api_key) return;
    setSaving(true);
    saveConfig(baseUrl, apiKey || null, model, temperature, maxTokens)
      .then(() => setApiKey(''))
      .finally(() => setSaving(false));
  }

  function handleTest() {
    if (!apiKey && !config?.has_api_key) return;
    testConnection(baseUrl, apiKey || null, model);
  }

  return (
    <div className="page">
      <h2 className="page-title">Model Settings</h2>

      {testResult && (
        <div className={`alert ${testResult.success ? 'alert-success' : 'alert-error'}`}>
          {testResult.message}
        </div>
      )}

      {error && <div className="alert alert-error">{error}</div>}

      {config && (
        <div className="card config-summary">
          <h3 className="form-title">Current Configuration</h3>
          <div className="config-details">
            <div className="config-row">
              <span className="text-secondary">Model:</span>
              <span>{config.model}</span>
            </div>
            <div className="config-row">
              <span className="text-secondary">Base URL:</span>
              <span>{config.base_url}</span>
            </div>
            <div className="config-row">
              <span className="text-secondary">API Key:</span>
              <span>{config.has_api_key ? config.masked_api_key : 'Not set'}</span>
            </div>
            <div className="config-row">
              <span className="text-secondary">Temperature:</span>
              <span>{config.temperature}</span>
            </div>
            <div className="config-row">
              <span className="text-secondary">Max Tokens:</span>
              <span>{config.max_tokens}</span>
            </div>
          </div>
        </div>
      )}

      <div className="card form-card">
        <h3 className="form-title">{config ? 'Update Configuration' : 'Configure Model'}</h3>
        <form onSubmit={handleSave}>
          <div className="form-group">
            <label className="form-label">Base URL</label>
            <input
              type="text"
              className="form-input"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="https://api.openai.com/v1"
            />
          </div>
          <div className="form-group">
            <label className="form-label">
              API Key {config && '(leave empty to keep existing)'}
            </label>
            <input
              type="password"
              className="form-input"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-..."
            />
          </div>
          <div className="form-group">
            <label className="form-label">Model</label>
            <input
              type="text"
              className="form-input"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="gpt-4o"
            />
          </div>
          <div className="form-row">
            <div className="form-group">
              <label className="form-label">Temperature ({temperature})</label>
              <input
                type="range"
                className="form-range"
                min={0}
                max={2}
                step={0.1}
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
              />
            </div>
            <div className="form-group">
              <label className="form-label">Max Tokens</label>
              <input
                type="number"
                className="form-input"
                value={maxTokens}
                onChange={(e) => setMaxTokens(parseInt(e.target.value) || 4096)}
                min={1}
                max={32768}
              />
            </div>
          </div>
          <div className="form-actions">
            <button type="submit" className="btn btn-primary" disabled={saving}>
              {saving ? 'Saving...' : 'Save Configuration'}
            </button>
            <button type="button" className="btn btn-secondary" onClick={handleTest} disabled={testing}>
              {testing ? 'Testing...' : 'Test Connection'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
