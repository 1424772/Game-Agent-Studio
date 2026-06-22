import React from 'react';
import { useAppStore } from '../stores/useAppStore';

const NAV_ITEMS = [
  { route: 'dashboard', label: 'Project Dashboard', icon: '\u{1F3E0}' },
  { route: 'settings', label: 'Model Settings', icon: '\u{2699}\uFE0F' },
  { route: 'workspace', label: 'Agent Workspace', icon: '\u{1F916}' },
  { route: 'memory', label: 'Memory Center', icon: '\u{1F9E0}' },
  { route: 'knowledge', label: 'Knowledge Base', icon: '\u{1F4DA}' },
  { route: 'export', label: 'Export Center', icon: '\u{1F4E4}' },
  { route: 'iteration', label: 'Self-Iteration', icon: '\u{1F504}' },
];

export default function Sidebar() {
  const currentRoute = useAppStore((s) => s.currentRoute);
  const setRoute = useAppStore((s) => s.setRoute);

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <h1 className="sidebar-logo">Game Agent Studio</h1>
        <span className="sidebar-subtitle">Multi-Agent Game Creator</span>
      </div>
      <nav className="sidebar-nav">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.route}
            className={`sidebar-nav-item ${currentRoute === item.route ? 'active' : ''}`}
            onClick={() => setRoute(item.route)}
          >
            <span className="nav-icon">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
          </button>
        ))}
      </nav>
      <div className="sidebar-footer">
        <span className="sidebar-version">v0.1.0</span>
      </div>
    </aside>
  );
}
