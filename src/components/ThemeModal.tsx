import React, { useRef } from "react";
import { Download, Palette, Upload, X } from "lucide-react";
import { useTheme } from "../themes/ThemeContext";

interface ThemeModalProps {
  onClose: () => void;
}

export const ThemeModal: React.FC<ThemeModalProps> = ({ onClose }) => {
  const { themeId, availableThemes, setThemeId, importCustomThemeJson } =
    useTheme();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        importCustomThemeJson(String(reader.result));
      } catch (err) {
        alert(`Errore caricamento tema JSON:\n${String(err)}`);
      }
    };
    reader.readAsText(file);
  };

  const handleExportCurrent = () => {
    const current = availableThemes[themeId];
    if (!current) return;
    const blob = new Blob([JSON.stringify(current, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${current.id}.kzip-theme.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-window" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Palette size={16} />
            <span>Impostazioni Temi e Aspetto</span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <div className="theme-cards-grid">
            {Object.values(availableThemes).map((t) => (
              <div
                key={t.id}
                className={`theme-card ${t.id === themeId ? "active" : ""}`}
                onClick={() => setThemeId(t.id)}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                  }}
                >
                  <strong>{t.name}</strong>
                  <span
                    style={{
                      width: 18,
                      height: 18,
                      borderRadius: "50%",
                      background: t.variables["--accent-color"] || "#0067c0",
                      border: "1px solid rgba(0,0,0,0.2)",
                    }}
                  />
                </div>
                <span style={{ fontSize: 11.5, color: "var(--text-secondary)" }}>
                  {t.description}
                </span>
              </div>
            ))}
          </div>

          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              borderTop: "1px solid var(--pane-border)",
              paddingTop: 12,
            }}
          >
            <div style={{ display: "flex", gap: 8 }}>
              <input
                ref={fileInputRef}
                type="file"
                accept=".json"
                style={{ display: "none" }}
                onChange={handleFileUpload}
              />
              <button
                type="button"
                className="secondary-btn"
                onClick={() => fileInputRef.current?.click()}
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <Upload size={14} />
                <span>Carica Tema .JSON Custom</span>
              </button>

              <button
                type="button"
                className="secondary-btn"
                onClick={handleExportCurrent}
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <Download size={14} />
                <span>Esporta Tema Attivo</span>
              </button>
            </div>

            <button type="button" className="primary-btn" onClick={onClose}>
              Fatto
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
