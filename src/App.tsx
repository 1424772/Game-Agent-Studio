import React, { useEffect } from 'react';
import Layout from './app/components/Layout';
import { useAppStore } from './app/stores/useAppStore';
import { useModelStore } from './app/stores/useModelStore';
import ProjectDashboard from './app/routes/ProjectDashboard';
import ModelSettings from './app/routes/ModelSettings';
import AgentWorkspace from './app/routes/AgentWorkspace';
import MemoryCenter from './app/routes/MemoryCenter';
import ExportCenter from './app/routes/ExportCenter';
import SelfIterationPanel from './app/routes/SelfIterationPanel';

export default function App() {
  const currentRoute = useAppStore((s) => s.currentRoute);
  const loadConfig = useModelStore((s) => s.loadConfig);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  function renderPage() {
    switch (currentRoute) {
      case 'dashboard':
        return <ProjectDashboard />;
      case 'settings':
        return <ModelSettings />;
      case 'workspace':
        return <AgentWorkspace />;
      case 'memory':
        return <MemoryCenter />;
      case 'knowledge':
        return (
          <div className="page">
            <h2 className="page-title">Knowledge Base</h2>
            <p className="text-secondary">Knowledge base management coming soon.</p>
          </div>
        );
      case 'export':
        return <ExportCenter />;
      case 'iteration':
        return <SelfIterationPanel />;
      default:
        return <ProjectDashboard />;
    }
  }

  return (
    <Layout>
      {renderPage()}
    </Layout>
  );
}
