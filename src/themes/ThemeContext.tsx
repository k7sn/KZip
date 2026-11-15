import React, { createContext, useContext, useEffect, useState } from "react";
import { THEMES, ThemeId } from "./themes";

interface ThemeContextValue {
  themeId: ThemeId;
  setThemeId: (id: ThemeId) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  themeId: "commander-light",
  setThemeId: () => {},
  toggleTheme: () => {},
});

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [themeId, setThemeIdState] = useState<ThemeId>(() => {
    const saved = localStorage.getItem("kzip.theme") as ThemeId | null;
    return saved && THEMES[saved] ? saved : "commander-light";
  });

  useEffect(() => {
    const theme = THEMES[themeId];
    const root = document.documentElement;
    Object.entries(theme.variables).forEach(([key, val]) => {
      root.style.setProperty(key, val);
    });
    root.setAttribute("data-theme", themeId);
    localStorage.setItem("kzip.theme", themeId);
  }, [themeId]);

  const setThemeId = (id: ThemeId) => setThemeIdState(id);

  const toggleTheme = () => {
    setThemeIdState((prev) =>
      prev === "commander-light" ? "frutiger-aero" : "commander-light"
    );
  };

  return (
    <ThemeContext.Provider value={{ themeId, setThemeId, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => useContext(ThemeContext);
