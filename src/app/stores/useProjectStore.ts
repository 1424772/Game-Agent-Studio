import { create } from 'zustand';
import type { Project } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

interface ProjectStore {
  projects: Project[];
  currentProject: Project | null;
  loading: boolean;
  error: string | null;
  loadProjects: () => Promise<void>;
  createProject: (name: string, gameType: string, description: string) => Promise<Project>;
  openProject: (project: Project) => void;
  deleteProject: (id: string) => Promise<void>;
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  currentProject: null,
  loading: false,
  error: null,
  loadProjects: async () => {
    set({ loading: true, error: null });
    try {
      const projects = await tauri.listProjects();
      set({ projects, loading: false });
    } catch (e: any) {
      set({ error: e.toString(), loading: false });
    }
  },
  createProject: async (name, gameType, description) => {
    const project = await tauri.createProject(name, gameType, description);
    await tauri.logEvent(project.id, 'project_created', JSON.stringify({ name, gameType }));
    await get().loadProjects();
    return project;
  },
  openProject: (project) => {
    set({ currentProject: project });
  },
  deleteProject: async (id) => {
    await tauri.deleteProject(id);
    await get().loadProjects();
  },
}));
