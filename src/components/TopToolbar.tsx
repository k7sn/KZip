import React from "react";
import {
  ChevronLeft,
  ChevronRight,
  AlignJustify,
  List,
  LayoutGrid,
  Info,
  Eye,
  Search,
  FileArchive,
  FolderPlus,
  Terminal,
  ArrowDownCircle,
  HardDrive,
} from "lucide-react";
import { EngineStatus, ViewMode } from "../types/vfs";
import { useTheme } from "../themes/ThemeContext";

interface TopToolbarProps {
  activeGlobalPath: string;
  canGoBack: boolean;
  canGoForward: boolean;
  onBack: () => void;
  onForward: () => void;
  activeViewMode: ViewMode;
  onChangeViewMode: (mode: ViewMode) => void;
  showHidden: boolean;
  onToggleHidden: () => void;
  onOpenSearch: () => void;
  onQuickView: () => void;
  onNewFolder: () => void;
  onOpenTerminal: () => void;
  onArchiveAction: () => void;
  engineStatus: EngineStatus | null;
}

export const TopToolbar: React.FC<TopToolbarProps> = ({
  activeGlobalPath,
  canGoBack,
  canGoForward,
  onBack,
  onForward,
  activeViewMode,
  onChangeViewMode,
  showHidden,
  onToggleHidden,
  onOpenSearch,
  onQuickView,
  onNewFolder,
  onOpenTerminal,
  onArchiveAction,
  engineStatus,
}) => {
  const { themeId, toggleTheme } = useTheme();

  return (
    <header className="top-toolbar">
      <div className="toolbar-left">
        <button
          className="toolbar-nav-btn"
          disabled={!canGoBack}
          onClick={onBack}
          title="Indietro (Alt+Left)"
        >
          <ChevronLeft size={16} />
        </button>
        <button
          className="toolbar-nav-btn"
          disabled={!canGoForward}
          onClick={onForward}
          title="Avanti (Alt+Right)"
        >
          <ChevronRight size={16} />
        </button>

        <div className="toolbar-global-title" title={activeGlobalPath}>
          <HardDrive size={14} color="#0066cc" />
          <span>{activeGlobalPath}</span>
        </div>
      </div>

      <div className="toolbar-right">
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
            title="Vista Griglia Icone e Miniature"
          >
            <LayoutGrid size={14} />
          </button>
        </div>

        <button
          className={`theme-pill-switch ${themeId === "frutiger-aero" ? "aero" : ""}`}
          onClick={toggleTheme}
          title={`Tema attivo: ${themeId}. Clicca per cambiare in ${
            themeId === "commander-light" ? "Frutiger Aero" : "Commander Light"
          }`}
        >
          <span className="theme-pill-knob" />
        </button>

        <button
          className="toolbar-action-btn"
          onClick={onQuickView}
          title="Informazioni / Anteprima Rapida (F3)"
        >
          <Info size={15} />
        </button>

        <button
          className={`toolbar-action-btn ${showHidden ? "active" : ""}`}
          onClick={onToggleHidden}
          title="Mostra/Nascondi file nascosti"
        >
          <Eye size={15} />
        </button>

        <button
          className="toolbar-action-btn"
          onClick={onOpenSearch}
          title={`Ricerca Dual-Engine (${
            engineStatus?.everythingServiceConnected ? "Everything SDK3" : "Rust Fallback"
          }) - Ctrl+F`}
        >
          <Search size={15} />
        </button>

        <button
          className="toolbar-action-btn highlight-zip"
          onClick={onArchiveAction}
          title="KZip.Core.dll COM Archive Manager (Comprimi / Apri Archivio)"
        >
          <FileArchive size={15} />
        </button>

        <button
          className="toolbar-action-btn"
          onClick={onNewFolder}
          title="Nuova Cartella (F7)"
        >
          <FolderPlus size={15} />
        </button>

        <button
          className="toolbar-action-btn"
          onClick={onOpenTerminal}
          title="Apri Terminale nel percorso attivo"
        >
          <Terminal size={15} />
        </button>

        <button
          className="toolbar-action-btn"
          onClick={onQuickView}
          title="Coda Trasferimenti / Stato Motori FFI"
        >
          <ArrowDownCircle size={15} />
        </button>
      </div>
    </header>
  );
};
