import React, { createContext, useContext, useEffect, useState } from "react";
import { BUILTIN_THEMES, KZipTheme, ThemeId } from "./themes";

interface ThemeContextValue {
  themeId: ThemeId;
  availableThemes: Record<string, KZipTheme>;
  setThemeId: (id: ThemeId) => void;
  importCustomThemeJson: (jsonContent: string) => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  themeId: "win11-light",
  availableThemes: BUILTIN_THEMES,
  setThemeId: () => {},
  importCustomThemeJson: () => {},
});

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [customThemes, setCustomThemes] = useState<Record<string, KZipTheme>>(() => {
    try {
      const raw = localStorage.getItem("kzip.customThemes");
      return raw ? JSON.parse(raw) : {};
    } catch {
      return {};
    }
  });

  const availableThemes = { ...BUILTIN_THEMES, ...customThemes };

  const [themeId, setThemeIdState] = useState<ThemeId>(() => {
    const saved = localStorage.getItem("kzip.theme");
    return saved && availableThemes[saved] ? saved : "win11-light";
  });

  useEffect(() => {
    const theme = availableThemes[themeId] || BUILTIN_THEMES["win11-light"];
    const root = document.documentElement;
    Object.entries(theme.variables).forEach(([key, val]) => {
      root.style.setProperty(key, val);
    });
    root.setAttribute("data-theme", theme.id);
    localStorage.setItem("kzip.theme", theme.id);
  }, [themeId, availableThemes]);

  const setThemeId = (id: ThemeId) => {
    if (availableThemes[id]) {
      setThemeIdState(id);
    }
  };

  const importCustomThemeJson = (jsonContent: string) => {
    const parsed = JSON.parse(jsonContent) as KZipTheme;
    if (!parsed.id || !parsed.name || !parsed.variables) {
      throw new Error("Il file JSON deve contenere 'id', 'name' e 'variables'.");
    }
    const next = { ...customThemes, [parsed.id]: parsed };
    setCustomThemes(next);
    localStorage.setItem("kzip.customThemes", JSON.stringify(next));
    setThemeIdState(parsed.id);
  };

  return (
    <ThemeContext.Provider
      value={{ themeId, availableThemes, setThemeId, importCustomThemeJson }}
    >
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => useContext(ThemeContext);
