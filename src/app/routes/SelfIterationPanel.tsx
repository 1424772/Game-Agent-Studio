import React, { useEffect, useState } from 'react';
import { useProjectStore } from '../stores/useProjectStore';
import type { Event, UserPreference } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

export default function SelfIterationPanel() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const [events, setEvents] = useState<Event[]>([]);
  const [preferences, setPreferences] = useState<UserPreference[]>([]);
  const [insights, setInsights] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadData();
  }, [currentProject]);

  async function loadData() {
    setLoading(true);
    try {
      const evts = await tauri.getEvents(currentProject?.id || null, 50);
      setEvents(evts.reverse());

      const prefs = await tauri.getUserPreferences();
      setPreferences(prefs);

      const generated = generateInsights(evts, prefs);
      setInsights(generated);
    } catch (e: any) {
      console.error('Failed to load iteration data:', e);
    }
    setLoading(false);
  }

  function generateInsights(events: Event[], prefs: UserPreference[]): string[] {
    const result: string[] = [];

    const accepted = events.filter((e) => e.event_type === 'output_accepted').length;
    const rejected = events.filter((e) => e.event_type === 'output_rejected').length;
    const edited = events.filter((e) => e.event_type === 'output_edited').length;
    const total = accepted + rejected + edited;

    if (total > 0) {
      const acceptRate = Math.round((accepted / total) * 100);
      result.push(`Acceptance rate: ${acceptRate}% (${accepted} accepted, ${rejected} rejected, ${edited} edited)`);

      if (acceptRate < 50) {
        result.push('Low acceptance rate detected. Consider adjusting prompt templates or model parameters.');
      }
      if (edited > accepted) {
        result.push('High edit rate suggests output quality needs improvement. Review agent prompts.');
      }
    }

    const failedEvents = events.filter((e) => e.event_type === 'agent_workflow_failed');
    if (failedEvents.length > 2) {
      result.push(`Multiple workflow failures (${failedEvents.length}). Check model connection and prompt configuration.`);
    }

    const completedEvents = events.filter((e) => e.event_type === 'agent_workflow_completed');
    if (completedEvents.length >= 3) {
      result.push(`${completedEvents.length} workflows completed. Consider reviewing accumulated knowledge in Memory Center.`);
    }

    if (prefs.length > 0) {
      result.push(`${prefs.length} user preferences recorded. The system is learning your preferences over time.`);
    }

    const projectCreated = events.filter((e) => e.event_type === 'project_created');
    if (projectCreated.length > 1 && completedEvents.length === 0) {
      result.push('Multiple projects created but no workflows completed. Try running an agent workflow on a project.');
    }

    if (result.length === 0) {
      result.push('Not enough data for insights yet. Create a project and run some agent workflows to get started.');
    }

    return result;
  }

  return (
    <div className="page">
      <h2 className="page-title">Self-Iteration Panel</h2>

      {!currentProject && (
        <p className="text-secondary">Open a project to see project-specific insights, or view global analytics below.</p>
      )}

      <div className="card">
        <h3 className="form-title">Insights & Recommendations</h3>
        {loading ? (
          <p className="text-secondary">Analyzing...</p>
        ) : (
          <ul className="insights-list">
            {insights.map((insight, i) => (
              <li key={i} className="insight-item">{insight}</li>
            ))}
          </ul>
        )}
        <button className="btn btn-secondary" onClick={loadData} style={{ marginTop: '12px' }}>
          Refresh Insights
        </button>
      </div>

      <div className="card">
        <h3 className="form-title">Recent Events</h3>
        {events.length === 0 ? (
          <p className="text-secondary">No events recorded yet.</p>
        ) : (
          <div className="events-list">
            {events.slice(0, 20).map((event) => (
              <div key={event.id} className="event-item">
                <span className="badge">{event.event_type}</span>
                <span className="text-sm text-secondary">
                  {new Date(event.created_at).toLocaleString()}
                </span>
                <span className="text-sm text-secondary" style={{ marginLeft: '8px' }}>
                  {event.event_data && event.event_data.length > 80
                    ? event.event_data.slice(0, 80) + '...'
                    : event.event_data}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <h3 className="form-title">Learned Preferences</h3>
        {preferences.length === 0 ? (
          <p className="text-secondary">No preferences learned yet. Continue using the app to build preference data.</p>
        ) : (
          <div className="preferences-list">
            {preferences.map((pref) => (
              <div key={pref.id} className="preference-item">
                <div className="preference-header">
                  <span className="preference-key">{pref.preference_key}</span>
                  <span className="badge">confidence: {(pref.confidence * 100).toFixed(0)}%</span>
                </div>
                <p className="text-sm">{pref.preference_value}</p>
                {pref.evidence && (
                  <p className="text-sm text-secondary">Evidence: {pref.evidence}</p>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
