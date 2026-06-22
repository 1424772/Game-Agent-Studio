import { create } from 'zustand';
import type { ModelConfigPublic } from '../../shared/types';
import * as tauri from '../../shared/utils/tauri';

interface ModelStore {
  config: ModelConfigPublic | null;
  loading: boolean;
  testing: boolean;
  testResult: { success: boolean; message: string } | null;
  error: string | null;

  loadConfig: () => Promise<void>;
  saveConfig: (baseUrl: string, apiKey: string, model: string, temperature: number, maxTokens: number) => Promise<void>;
  testConnection: (baseUrl: string, apiKey: string, model: string) => Promise<boolean>;
}

export const useModelStore = create<ModelStore>((set) => ({
  config: null,
  loading: false,
  testing: false,
  testResult: null,
  error: null,

  loadConfig: async () => {
    set({ loading: true });
    try {
      const config = await tauri.getModelConfig();
      set({ config, loading: false });
    } catch (e: any) {
      set({ error: e.toString(), loading: false });
    }
  },

  saveConfig: async (baseUrl, apiKey, model, temperature, maxTokens) => {
    const config = await tauri.saveModelConfig(baseUrl, apiKey, model, temperature, maxTokens);
    set({ config });
    await tauri.logEvent(null, 'model_config_saved', JSON.stringify({ model, baseUrl }));
  },

  testConnection: async (baseUrl, apiKey, model) => {
    set({ testing: true, testResult: null });
    try {
      const success = await tauri.testModelConnection(baseUrl, apiKey, model);
      set({
        testing: false,
        testResult: {
          success,
          message: success ? 'Connection successful!' : 'Connection failed. Check your settings.',
        },
      });
      return success;
    } catch (e: any) {
      set({
        testing: false,
        testResult: { success: false, message: e.toString() },
      });
      return false;
    }
  },
}));
