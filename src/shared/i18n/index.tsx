import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';
import { zh, en, type Translations } from './translations';

const LANG_KEY = 'game_agent_studio_lang';

const translations: Record<string, Translations> = { zh, en };

function getStoredLang(): string {
  try { return localStorage.getItem(LANG_KEY) || 'zh'; } catch { return 'zh'; }
}

interface LanguageCtx {
  lang: string;
  t: Translations;
  setLang: (lang: string) => void;
}

const LanguageContext = createContext<LanguageCtx>({
  lang: 'zh', t: zh, setLang: () => {},
});

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLangState] = useState(getStoredLang);

  const setLang = useCallback((value: string) => {
    setLangState(value);
    try { localStorage.setItem(LANG_KEY, value); } catch {}
  }, []);

  const t = translations[lang] || zh;

  useEffect(() => {
    const handler = () => setLangState(getStoredLang());
    window.addEventListener('storage', handler);
    return () => window.removeEventListener('storage', handler);
  }, []);

  return (
    <LanguageContext.Provider value={{ lang, t, setLang }}>
      {children}
    </LanguageContext.Provider>
  );
}

export function useT(): Translations {
  return useContext(LanguageContext).t;
}

export function useLang() {
  const ctx = useContext(LanguageContext);
  return { lang: ctx.lang, setLang: ctx.setLang };
}
