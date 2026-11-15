import React, { useState } from "react";
import { Cpu, Search, X } from "lucide-react";
import { EngineStatus, SearchQuery, SearchResponse } from "../types/vfs";

interface SearchModalProps {
  currentFolder: string;
  engineStatus: EngineStatus | null;
  onClose: () => void;
  onRunSearch: (query: SearchQuery) => Promise<SearchResponse | null>;
  onMountSearchToPane: (vfsUri: string) => void;
}

export const SearchModal: React.FC<SearchModalProps> = ({
  currentFolder,
  engineStatus,
  onClose,
  onRunSearch,
  onMountSearchToPane,
}) => {
  const [queryText, setQueryText] = useState("");
  const [extensions, setExtensions] = useState("");
  const [scopeMode, setScopeMode] = useState<"current" | "system">("current");
  const [minSizeKb, setMinSizeKb] = useState("");
  const [maxSizeKb, setMaxSizeKb] = useState("");
  const [useRegex, setUseRegex] = useState(false);
  const [matchCase, setMatchCase] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [lastResponse, setLastResponse] = useState<SearchResponse | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSearching(true);
    try {
      const payload: SearchQuery = {
        query: queryText,
        scopePath: scopeMode === "current" ? currentFolder : null,
        extensions: extensions.trim() ? extensions.trim() : null,
        minSize: minSizeKb ? Number(minSizeKb) * 1024 : null,
        maxSize: maxSizeKb ? Number(maxSizeKb) * 1024 : null,
        useRegex,
        matchCase,
        maxResults: 500,
      };
      const res = await onRunSearch(payload);
      setLastResponse(res);
    } finally {
      setIsSearching(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-window" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Search size={16} />
            <span>Ricerca Dual-Engine (Everything SDK3 FFI + Rust Fallback)</span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span className="engine-badge">
              <Cpu size={13} />
              Motore Attivo: {engineStatus?.activeSearchEngine || "Rilevamento..."}
            </span>
            {engineStatus?.everythingVersion && (
              <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                Everything v{engineStatus.everythingVersion}
              </span>
            )}
          </div>

          <div className="search-form-grid">
            <div className="form-field" style={{ gridColumn: "1 / -1" }}>
              <label>Nome file o pattern di ricerca</label>
              <input
                type="text"
                value={queryText}
                onChange={(e) => setQueryText(e.target.value)}
                placeholder="Es. report, *.rs, vacanze..."
                autoFocus
              />
            </div>

            <div className="form-field">
              <label>Filtro Estensioni (separate da ; )</label>
              <input
                type="text"
                value={extensions}
                onChange={(e) => setExtensions(e.target.value)}
                placeholder="es. zip;7z;rar;jpg"
              />
            </div>

            <div className="form-field">
              <label>Ambito di ricerca</label>
              <select
                value={scopeMode}
                onChange={(e) => setScopeMode(e.target.value as "current" | "system")}
              >
                <option value="current">Cartella Corrente ({currentFolder})</option>
                <option value="system">Tutto il Sistema</option>
              </select>
            </div>

            <div className="form-field">
              <label>Dimensione Minima (KB)</label>
              <input
                type="number"
                min={0}
                value={minSizeKb}
                onChange={(e) => setMinSizeKb(e.target.value)}
                placeholder="Opzionale"
              />
            </div>

            <div className="form-field">
              <label>Dimensione Massima (KB)</label>
              <input
                type="number"
                min={0}
                value={maxSizeKb}
                onChange={(e) => setMaxSizeKb(e.target.value)}
                placeholder="Opzionale"
              />
            </div>
          </div>

          <div style={{ display: "flex", gap: 18 }}>
            <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <input
                type="checkbox"
                checked={useRegex}
                onChange={(e) => setUseRegex(e.target.checked)}
              />
              <span>Espressione Regolare (Regex)</span>
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <input
                type="checkbox"
                checked={matchCase}
                onChange={(e) => setMatchCase(e.target.checked)}
              />
              <span>Case Sensitive</span>
            </label>
          </div>

          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button type="button" className="secondary-btn" onClick={onClose}>
              Chiudi
            </button>
            <button type="submit" className="primary-btn" disabled={isSearching}>
              {isSearching ? "Ricerca in corso..." : "Cerca Ora"}
            </button>
          </div>

          {lastResponse && (
            <div
              style={{
                borderTop: "1px solid var(--pane-border)",
                paddingTop: 10,
                display: "flex",
                flexDirection: "column",
                gap: 8,
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span>
                  Trovati <strong>{lastResponse.results.length}</strong> risultati in{" "}
                  <strong>{lastResponse.elapsedMs} ms</strong> tramite{" "}
                  <em>{lastResponse.engineUsed}</em>
                </span>
                <button
                  type="button"
                  className="primary-btn"
                  onClick={() => {
                    onMountSearchToPane(lastResponse.vfsUri);
                    onClose();
                  }}
                >
                  Apri Risultati nel Pannello Attivo
                </button>
              </div>
            </div>
          )}
        </form>
      </div>
    </div>
  );
};
