import React from 'react';
import { useAppStore } from '../stores/useAppStore';
import { LANGUAGES } from '../../shared/constants';
import { useT, useLang } from '../../shared/i18n';

export default function Sidebar() {
  const currentRoute = useAppStore((s) => s.currentRoute);
  const setRoute = useAppStore((s) => s.setRoute);
  const t = useT();
  const { lang, setLang } = useLang();

  const NAV_ITEMS = [
    { route: 'dashboard', label: t.nav.dashboard, icon: '\u{1F3E0}' },
    { route: 'settings', label: t.nav.settings, icon: '\u{2699}\uFE0F' },
    { route: 'workspace', label: t.nav.workspace, icon: '\u{1F916}' },
    { route: 'memory', label: t.nav.memory, icon: '\u{1F9E0}' },
    { route: 'knowledge', label: t.nav.knowledge, icon: '\u{1F4DA}' },
    { route: 'export', label: t.nav.export, icon: '\u{1F4E4}' },
    { route: 'iteration', label: t.nav.iteration, icon: '\u{1F504}' },
  ];

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <h1 className="sidebar-logo">{t.app.title}</h1>
        <span className="sidebar-subtitle">{t.app.subtitle}</span>
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
        <select className="lang-select" value={lang} onChange={(e) => setLang(e.target.value)}>
          {LANGUAGES.map((l) => (
            <option key={l.value} value={l.value}>{l.label}</option>
          ))}
        </select>
        <span className="sidebar-version">v0.2.0</span>
      </div>
    </aside>
  );
}
