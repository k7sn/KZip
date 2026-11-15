import React from "react";
import {
  AlignJustify,
  Columns,
  Eye,
  FileArchive,
  FolderOutput,
  FolderPlus,
  Info,
  LayoutGrid,
  List,
  Palette,
  Search,
  Square,
  Terminal,
  X,
} from "lucide-react";
import { EngineStatus, LayoutMode, ViewMode } from "../types/vfs";

interface TopToolbarProps {
  layoutMode: LayoutMode;
  onToggleLayoutMode: () => void;
  activeViewMode: ViewMode;
  onChangeViewMode: (mode: ViewMode) => void;
  showHidden: boolean;
  onToggleHidden: () => void;
  searchQuery: string;
  onSearchQueryChange: (q: string) => void;
  onTriggerInstantSearch: (q: string) => void;
  onClearSearch: () => void;
  searchScopeGlobal: boolean;
  onToggleSearchScope: () => void;
  onOpenCompressModal: () => void;
  onExtractSelected: () => void;
  onNewFolder: () => void;
  onOpenTerminal: () => void;
  onOpenDiagnostics: () => void;
  onOpenThemeModal: () => void;
  engineStatus: EngineStatus | null;
}

export const TopToolbar: React.FC<TopToolbarProps> = ({
  layoutMode,
  onToggleLayoutMode,
  activeViewMode,
  onChangeViewMode,
  showHidden,
  onToggleHidden,
  searchQuery,
  onSearchQueryChange,
  onTriggerInstantSearch,
  onClearSearch,
  searchScopeGlobal,
  onToggleSearchScope,
  onOpenCompressModal,
  onExtractSelected,
  onNewFolder,
  onOpenTerminal,
  onOpenDiagnostics,
  onOpenThemeModal,
  engineStatus,
}) => {
  return (
    <header className="top-toolbar">
      <div className="toolbar-left">
        {/* Toggle Vista Singola / Doppia Vista */}
        <button
          className="toolbar-btn active"
          onClick={onToggleLayoutMode}
          title="Passa tra Vista Singola (1 Pannello) e Doppia Vista (2 Pannelli)"
        >
          {layoutMode === "dual" ? <Columns size={15} /> : <Square size={15} />}
          <span>{layoutMode === "dual" ? "2 Pannelli" : "1 Pannello"}</span>
        </button>

        {/* Selettore Modalità Vista */}
        <div className="toolbar-view-group" role="group" aria-label="View Mode">
          <button
            className={activeViewMode === "list" ? "active" : ""}
            onClick={() => onChangeViewMode("list")}
            title="Vista Lista Dettagliata"
          >
            <AlignJustify size={14} />
          </button>
          <button
            className={activeViewMode === "columns" ? "active" : ""}
            onClick={() => onChangeViewMode("columns")}
            title="Vista Compatta"
          >
            <List size={14} />
          </button>
          <button
            className={activeViewMode === "grid" ? "active" : ""}
            onClick={() => onChangeViewMode("grid")}
            title="Vista Griglia Icone"
          >
            <LayoutGrid size={14} />
          </button>
        </div>
      </div>

      {/* BARRA DI RICERCA ISTANTANEA EVERYTHING (Zero popup nascosti) */}
      <div className="instant-search-bar">
        <Search size={14} color="var(--accent-color)" />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => onSearchQueryChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              onTriggerInstantSearch(searchQuery);
            } else if (e.key === "Escape") {
              onClearSearch();
            }
          }}
          placeholder={
            engineStatus?.everythingServiceConnected
              ? "Cerca istantaneamente con Everything (es. ciao, ext:zip;7z)..."
              : "Cerca file nel pannello attivo..."
          }
        />
        {searchQuery && (
          <button
            className="tab-close-btn"
            onClick={onClearSearch}
            title="Pulisci ricerca e torna alla cartella"
          >
            <X size={14} />
          </button>
        )}
        <button
          type="button"
          className={`search-scope-chip ${searchScopeGlobal ? "active" : ""}`}
          onClick={onToggleSearchScope}
          title="Cambia ambito: Tutto il PC (Everything) oppure Cartella corrente"
        >
          {searchScopeGlobal ? "Tutto il PC" : "Cartella"}
        </button>
      </div>

      <div className="toolbar-right">
        <button
          className="toolbar-btn primary-accent"
          onClick={onOpenCompressModal}
          title="Comprimi elementi selezionati in .7z / .zip"
        >
          <FileArchive size={15} />
          <span>Comprimi</span>
        </button>

        <button
          className="toolbar-btn"
          onClick={onExtractSelected}
          title="Estrai archivio selezionato"
        >
          <FolderOutput size={15} />
          <span>Estrai</span>
        </button>

        <button
          className="toolbar-btn"
          onClick={onNewFolder}
          title="Nuova Cartella (F7)"
        >
          <FolderPlus size={15} />
        </button>

        <button
          className={`toolbar-btn ${showHidden ? "active" : ""}`}
          onClick={onToggleHidden}
          title="Mostra/Nascondi file nascosti"
        >
          <Eye size={15} />
        </button>

        <button
          className="toolbar-btn"
          onClick={onOpenTerminal}
          title="Apri Terminale nel percorso attivo"
        >
          <Terminal size={15} />
        </button>

        <button
          className="toolbar-btn"
          onClick={onOpenDiagnostics}
          title="Diagnostica Motori FFI / Anteprima F3"
        >
          <Info size={15} />
        </button>

        <button
          className="toolbar-btn"
          onClick={onOpenThemeModal}
          title="Selettore Temi e Personalizzazione JSON"
        >
          <Palette size={15} />
          <span>Temi</span>
        </button>
      </div>
    </header>
  );
};
