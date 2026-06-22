import { create } from 'zustand';

interface AppState {
  currentRoute: string;
  currentProjectId: string | null;
  setRoute: (route: string) => void;
  setCurrentProject: (projectId: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentRoute: 'dashboard',
  currentProjectId: null,
  setRoute: (route) => set({ currentRoute: route }),
  setCurrentProject: (projectId) => set({ currentProjectId: projectId }),
}));
