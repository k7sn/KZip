export type ThemeId = "commander-light" | "frutiger-aero";

export interface KZipTheme {
  id: ThemeId;
  name: string;
  description: string;
  variables: Record<string, string>;
}

export const THEMES: Record<ThemeId, KZipTheme> = {
  "commander-light": {
    id: "commander-light",
    name: "Commander Light",
    description: "Classic Commander One / macOS dual-pane light theme (Reference 1 & 2)",
    variables: {
      "--app-bg": "#e7e7e9",
      "--top-toolbar-bg": "linear-gradient(180deg, #f3f3f5 0%, #dedee2 100%)",
      "--top-toolbar-border": "#bebec2",
      "--sub-bar-bg": "linear-gradient(180deg, #ececee 0%, #dcdce0 100%)",
      "--tab-bar-bg": "linear-gradient(180deg, #d7d7db 0%, #c8c8cd 100%)",
      "--tab-active-bg": "linear-gradient(180deg, #e5e5e8 0%, #d4d4d8 100%)",
      "--breadcrumb-bg": "linear-gradient(180deg, #f7f7f9 0%, #e9e9ec 100%)",
      "--pane-bg": "#ffffff",
      "--pane-alt-row": "#f4f6f9",
      "--pane-border": "#c4c4c8",
      "--pane-active-ring": "#0060df",
      "--col-header-bg": "linear-gradient(180deg, #ffffff 0%, #efefef 100%)",
      "--col-header-text": "#424247",
      "--text-primary": "#1d1d1f",
      "--text-secondary": "#5c5c63",
      "--text-muted": "#86868b",
      "--selection-row-bg": "#0058d0",
      "--selection-row-text": "#ffffff",
      "--selection-grid-box": "rgba(0, 98, 225, 0.14)",
      "--selection-grid-border": "#f0c419",
      "--folder-icon-top": "#76d0ff",
      "--folder-icon-bottom": "#42b5ff",
      "--folder-icon-glyph": "#268fd8",
      "--folder-selected-bg": "#0058d0",
      "--button-bg": "linear-gradient(180deg, #ffffff 0%, #ebebee 100%)",
      "--button-hover-bg": "linear-gradient(180deg, #ffffff 0%, #dfdfe5 100%)",
      "--button-border": "#b5b5ba",
      "--accent-color": "#0066cc",
      "--accent-highlight": "#f2c94c",
      "--status-bar-bg": "#f5f5f7",
      "--bottom-cmd-bg": "#e4e4e7",
      "--fn-bar-bg": "linear-gradient(180deg, #ededf0 0%, #dcdce0 100%)",
      "--glass-blur": "none",
      "--gloss-overlay": "none",
      "--shadow-elevated": "0 4px 16px rgba(0, 0, 0, 0.12)"
    }
  },
  "frutiger-aero": {
    id: "frutiger-aero",
    name: "Frutiger Aero Glass",
    description: "Windows Vista / 7 Glossy Aurora Glass & Aquamarine gradients",
    variables: {
      "--app-bg": "linear-gradient(135deg, #0b3954 0%, #087e8b 48%, #1b998b 100%)",
      "--top-toolbar-bg": "linear-gradient(180deg, rgba(235, 250, 255, 0.88) 0%, rgba(188, 230, 248, 0.78) 50%, rgba(145, 208, 242, 0.85) 51%, rgba(195, 238, 255, 0.90) 100%)",
      "--top-toolbar-border": "rgba(120, 205, 250, 0.85)",
      "--sub-bar-bg": "linear-gradient(180deg, rgba(228, 247, 255, 0.85) 0%, rgba(176, 224, 248, 0.78) 100%)",
      "--tab-bar-bg": "linear-gradient(180deg, rgba(162, 215, 242, 0.78) 0%, rgba(122, 190, 228, 0.82) 100%)",
      "--tab-active-bg": "linear-gradient(180deg, rgba(255, 255, 255, 0.95) 0%, rgba(215, 243, 255, 0.92) 50%, rgba(180, 229, 252, 0.94) 51%, rgba(235, 250, 255, 0.96) 100%)",
      "--breadcrumb-bg": "linear-gradient(180deg, rgba(250, 254, 255, 0.92) 0%, rgba(214, 241, 255, 0.86) 100%)",
      "--pane-bg": "rgba(250, 254, 255, 0.92)",
      "--pane-alt-row": "rgba(222, 244, 255, 0.55)",
      "--pane-border": "rgba(110, 194, 238, 0.8)",
      "--pane-active-ring": "#00b4d8",
      "--col-header-bg": "linear-gradient(180deg, rgba(255, 255, 255, 0.96) 0%, rgba(218, 242, 255, 0.9) 50%, rgba(190, 230, 250, 0.92) 51%, rgba(232, 248, 255, 0.95) 100%)",
      "--col-header-text": "#073b54",
      "--text-primary": "#05293c",
      "--text-secondary": "#1c5875",
      "--text-muted": "#467d99",
      "--selection-row-bg": "linear-gradient(180deg, #38bdf8 0%, #0284c7 50%, #0369a1 51%, #0ea5e9 100%)",
      "--selection-row-text": "#ffffff",
      "--selection-grid-box": "rgba(14, 165, 233, 0.20)",
      "--selection-grid-border": "#00e5ff",
      "--folder-icon-top": "#72efdd",
      "--folder-icon-bottom": "#00b4d8",
      "--folder-icon-glyph": "#0077b6",
      "--folder-selected-bg": "#0284c7",
      "--button-bg": "linear-gradient(180deg, rgba(255,255,255,0.95) 0%, rgba(212,241,255,0.88) 50%, rgba(178,226,250,0.9) 51%, rgba(230,248,255,0.95) 100%)",
      "--button-hover-bg": "linear-gradient(180deg, #ffffff 0%, #b8e8ff 50%, #7dd3fc 51%, #e0f7ff 100%)",
      "--button-border": "#5bc0eb",
      "--accent-color": "#0096c7",
      "--accent-highlight": "#48cae4",
      "--status-bar-bg": "linear-gradient(180deg, rgba(235, 249, 255, 0.9) 0%, rgba(202, 236, 252, 0.9) 100%)",
      "--bottom-cmd-bg": "linear-gradient(180deg, rgba(210, 240, 254, 0.9) 0%, rgba(175, 224, 248, 0.9) 100%)",
      "--fn-bar-bg": "linear-gradient(180deg, rgba(242, 252, 255, 0.92) 0%, rgba(202, 236, 252, 0.86) 50%, rgba(168, 220, 246, 0.9) 51%, rgba(220, 245, 255, 0.94) 100%)",
      "--glass-blur": "blur(12px)",
      "--gloss-overlay": "inset 0 1px 0 rgba(255, 255, 255, 0.85)",
      "--shadow-elevated": "0 8px 28px rgba(3, 45, 70, 0.32)"
    }
  }
};
