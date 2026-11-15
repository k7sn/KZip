import React, { useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  ChevronRight,
  Code,
  Copy,
  Edit3,
  FileArchive,
  FileText,
  Film,
  Folder,
  FolderOutput,
  FolderPlus,
  HardDrive,
  Image as ImageIcon,
  Music,
  RefreshCw,
  Trash2,
} from "lucide-react";
import {
  DirectoryListing,
  DriveInfo,
  PaneTab,
  SortField,
  VfsEntry,
  ViewMode,
} from "../types/vfs";

interface FilePaneProps {
  paneId: "left" | "right";
  isActive: boolean;
  onActivate: () => void;
  drives: DriveInfo[];
  tabs: PaneTab[];
  activeTabId: string;
  onSelectTab: (tabId: string) => void;
  onAddTab: () => void;
  onCloseTab: (tabId: string) => void;
  canGoBack: boolean;
  canGoForward: boolean;
  onBack: () => void;
  onForward: () => void;
  onUp: () => void;
  listing: DirectoryListing | null;
  viewMode: ViewMode;
  showHidden: boolean;
  selectedPaths: Set<string>;
  onSelectionChange: (next: Set<string>) => void;
  onNavigate: (path: string) => void;
  onOpenEntry: (entry: VfsEntry) => void;
  onCompressRequest: () => void;
  onExtractHereRequest: (entry: VfsEntry) => void;
  onExtractToFolderRequest: (entry: VfsEntry) => void;
  onRenameRequest: (entry: VfsEntry) => void;
  onDeleteRequest: () => void;
  onNewFolderRequest: () => void;
}

export const FilePane: React.FC<FilePaneProps> = ({
  isActive,
  onActivate,
  drives,
  tabs,
  activeTabId,
  onSelectTab,
  onAddTab,
  onCloseTab,
  canGoBack,
  canGoForward,
  onBack,
  onForward,
  onUp,
  listing,
  viewMode,
  showHidden,
  selectedPaths,
  onSelectionChange,
  onNavigate,
  onOpenEntry,
  onCompressRequest,
  onExtractHereRequest,
  onExtractToFolderRequest,
  onRenameRequest,
  onDeleteRequest,
  onNewFolderRequest,
}) => {
  const [sortField, setSortField] = useState<SortField>("name");
  const [sortAsc, setSortAsc] = useState<boolean>(true);
  const [focusedIndex, setFocusedIndex] = useState<number>(0);
  const [lastClickedIndex, setLastClickedIndex] = useState<number>(0);
  const [editingPath, setEditingPath] = useState<boolean>(false);
  const [pathInput, setPathInput] = useState<string>("");

  const [ctxMenu, setCtxMenu] = useState<{
    x: number;
    y: number;
    entry: VfsEntry | null;
  } | null>(null);

  const viewportRef = useRef<HTMLDivElement>(null);

  const currentPath = listing?.currentPath || "C:\\";
  const isSearchMode = listing?.providerScheme === "search";

  const currentDrive = useMemo(() => {
    const matched = drives.find((d) =>
      currentPath.toUpperCase().startsWith(d.mountPoint.toUpperCase())
    );
    return matched || drives[0] || null;
  }, [drives, currentPath]);

  const visibleEntries = useMemo(() => {
    if (!listing) return [];
    const raw = listing.entries.filter((e) => {
      if (e.isParent) return true;
      if (!showHidden && (e.name.startsWith(".") || (e.attributes & 0x2) !== 0)) {
        return false;
      }
      return true;
    });

    const parentEntry = raw.find((e) => e.isParent);
    const rest = raw.filter((e) => !e.isParent);

    rest.sort((a, b) => {
      if (a.isDir !== b.isDir) {
        return a.isDir ? -1 : 1;
      }
      let cmp = 0;
      switch (sortField) {
        case "name":
          cmp = a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
          break;
        case "path":
          cmp = a.parentDir.localeCompare(b.parentDir, undefined, {
            sensitivity: "base",
          });
          break;
        case "extension":
          cmp = a.extension.localeCompare(b.extension, undefined, {
            sensitivity: "base",
          });
          break;
        case "size":
          cmp = a.size - b.size;
          break;
        case "modified":
          cmp = a.modified - b.modified;
          break;
        case "created":
          cmp = a.created - b.created;
          break;
        case "kind":
          cmp = a.kind.localeCompare(b.kind);
          break;
      }
      return sortAsc ? cmp : -cmp;
    });

    return parentEntry ? [parentEntry, ...rest] : rest;
  }, [listing, showHidden, sortField, sortAsc]);

  const breadcrumbParts = useMemo(() => {
    if (currentPath.startsWith("search://")) {
      return [
        {
          label: `Risultati Ricerca: "${currentPath.replace("search://", "")}"`,
          target: currentPath,
        },
      ];
    }
    if (currentPath.startsWith("archive://")) {
      return [{ label: currentPath, target: currentPath }];
    }
    const normalized = currentPath.replace(/\//g, "\\");
    const segments = normalized.split("\\").filter(Boolean);
    const acc: { label: string; target: string }[] = [];
    let running = "";
    segments.forEach((seg, idx) => {
      running = idx === 0 ? `${seg}\\` : `${running.replace(/\\$/, "")}\\${seg}`;
      acc.push({ label: seg, target: running });
    });
    return acc;
  }, [currentPath]);

  const handleHeaderSort = (field: SortField) => {
    if (sortField === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortField(field);
      setSortAsc(true);
    }
  };

  const handleEntryClick = (
    e: React.MouseEvent,
    entry: VfsEntry,
    index: number
  ) => {
    e.stopPropagation();
    setCtxMenu(null);
    onActivate();
    setFocusedIndex(index);

    if (e.shiftKey && visibleEntries.length > 0) {
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      const next = new Set(selectedPaths);
      for (let i = start; i <= end; i++) {
        const item = visibleEntries[i];
        if (item && !item.isParent) {
          next.add(item.path);
        }
      }
      onSelectionChange(next);
      return;
    }

    if (e.ctrlKey || e.metaKey) {
      const next = new Set(selectedPaths);
      if (next.has(entry.path)) {
        next.delete(entry.path);
      } else if (!entry.isParent) {
        next.add(entry.path);
      }
      onSelectionChange(next);
      setLastClickedIndex(index);
      return;
    }

    if (entry.isParent) {
      onSelectionChange(new Set());
    } else {
      onSelectionChange(new Set([entry.path]));
    }
    setLastClickedIndex(index);
  };

  const handleContextMenu = (
    e: React.MouseEvent,
    entry: VfsEntry | null,
    index?: number
  ) => {
    e.preventDefault();
    e.stopPropagation();
    onActivate();
    if (entry && !entry.isParent && !selectedPaths.has(entry.path)) {
      onSelectionChange(new Set([entry.path]));
      if (index !== undefined) setFocusedIndex(index);
    }
    setCtxMenu({
      x: Math.min(e.clientX, window.innerWidth - 240),
      y: Math.min(e.clientY, window.innerHeight - 260),
      entry,
    });
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (visibleEntries.length === 0) return;

    if (e.key === "ArrowDown" || e.key === "ArrowRight") {
      e.preventDefault();
      const step = viewMode === "grid" && e.key === "ArrowDown" ? 4 : 1;
      const nextIdx = Math.min(visibleEntries.length - 1, focusedIndex + step);
      setFocusedIndex(nextIdx);
      const target = visibleEntries[nextIdx];
      if (target && !target.isParent) {
        onSelectionChange(new Set([target.path]));
      }
    } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
      e.preventDefault();
      const step = viewMode === "grid" && e.key === "ArrowUp" ? 4 : 1;
      const nextIdx = Math.max(0, focusedIndex - step);
      setFocusedIndex(nextIdx);
      const target = visibleEntries[nextIdx];
      if (target && !target.isParent) {
        onSelectionChange(new Set([target.path]));
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      const target = visibleEntries[focusedIndex];
      if (target) {
        onOpenEntry(target);
      }
    } else if (e.key === "Backspace") {
      e.preventDefault();
      onUp();
    } else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      const all = new Set<string>();
      visibleEntries.forEach((item) => {
        if (!item.isParent) all.add(item.path);
      });
      onSelectionChange(all);
    }
  };

  const selectionStats = useMemo(() => {
    let selFiles = 0;
    let selDirs = 0;
    let selBytes = 0;
    visibleEntries.forEach((e) => {
      if (selectedPaths.has(e.path)) {
        if (e.isDir) selDirs += 1;
        else {
          selFiles += 1;
          selBytes += e.size;
        }
      }
    });
    return { selFiles, selDirs, selBytes };
  }, [visibleEntries, selectedPaths]);

  return (
    <section
      className={`file-pane ${isActive ? "active-pane" : ""}`}
      onMouseDown={() => {
        setCtxMenu(null);
        onActivate();
      }}
    >
      {/* 1. Barra Schede Compatta */}
      <div className="pane-tab-bar">
        <div className="pane-tabs-scroll">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`tab-item ${tab.id === activeTabId ? "active" : ""}`}
              onClick={() => onSelectTab(tab.id)}
            >
              <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
                {tab.title}
              </span>
              {tabs.length > 1 && (
                <button
                  className="tab-close-btn"
                  onClick={(ev) => {
                    ev.stopPropagation();
                    onCloseTab(tab.id);
                  }}
                  title="Chiudi scheda"
                >
                  ×
                </button>
              )}
            </div>
          ))}
        </div>
        <button className="tab-add-btn" onClick={onAddTab} title="Nuova scheda">
          +
        </button>
      </div>

      {/* 2. UNICA BARRA DI NAVIGAZIONE COMPATTA (Indietro/Avanti/Su + Unità con spazio libero + Breadcrumb) */}
      <div className="pane-single-navbar">
        <div className="pane-nav-group">
          <button
            className="pane-icon-btn"
            disabled={!canGoBack}
            onClick={onBack}
            title="Indietro"
          >
            <ArrowLeft size={14} />
          </button>
          <button
            className="pane-icon-btn"
            disabled={!canGoForward}
            onClick={onForward}
            title="Avanti"
          >
            <ArrowRight size={14} />
          </button>
          <button
            className="pane-icon-btn"
            disabled={!listing?.parentPath}
            onClick={onUp}
            title="Cartella Superiore (Backspace)"
          >
            <ArrowUp size={14} />
          </button>
        </div>

        {/* Dropdown compatto unità disco con spazio disponibile */}
        <select
          className="compact-drive-select"
          value={currentDrive?.mountPoint || ""}
          onChange={(e) => onNavigate(e.target.value)}
          title={
            currentDrive
              ? `${currentDrive.name} - ${formatGigabytes(
                  currentDrive.availableSpace
                )} liberi su ${formatGigabytes(currentDrive.totalSpace)}`
              : "Seleziona Unità"
          }
        >
          {drives.map((d) => (
            <option key={d.mountPoint} value={d.mountPoint}>
              {d.mountPoint.replace("\\", "")} ({formatGigabytes(d.availableSpace)} free)
            </option>
          ))}
        </select>

        {/* Breadcrumb cliccabile (doppio click o click sullo sfondo per digitare percorso) */}
        <div
          className="breadcrumb-box"
          onDoubleClick={() => {
            setPathInput(currentPath);
            setEditingPath(true);
          }}
        >
          {editingPath ? (
            <input
              className="path-direct-input"
              value={pathInput}
              autoFocus
              onChange={(e) => setPathInput(e.target.value)}
              onBlur={() => setEditingPath(false)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && pathInput.trim()) {
                  setEditingPath(false);
                  onNavigate(pathInput.trim());
                } else if (e.key === "Escape") {
                  setEditingPath(false);
                }
              }}
            />
          ) : (
            breadcrumbParts.map((part, idx) => (
              <React.Fragment key={part.target + idx}>
                {idx > 0 && <ChevronRight size={12} color="var(--text-muted)" />}
                <button
                  className="crumb-btn"
                  onClick={() => onNavigate(part.target)}
                >
                  {idx === 0 && <HardDrive size={12} color="var(--accent-color)" />}
                  <span>{part.label}</span>
                </button>
              </React.Fragment>
            ))
          )}
        </div>

        <button
          className="pane-icon-btn"
          onClick={() => onNavigate(currentPath)}
          title="Aggiorna (F5)"
        >
          <RefreshCw size={13} />
        </button>
      </div>

      {/* 3. Intestazioni Colonne (in modalità Ricerca mostra la colonna Percorso) */}
      {isSearchMode ? (
        <div className="column-headers-bar search-columns">
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("name")}
          >
            <span>Nome</span>
            {sortField === "name" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("path")}
          >
            <span>Percorso</span>
            {sortField === "path" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("size")}
          >
            <span>Dimensione</span>
            {sortField === "size" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("modified")}
          >
            <span>Data modifica</span>
            {sortField === "modified" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
        </div>
      ) : (
        <div className="column-headers-bar">
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("name")}
          >
            <span>Nome</span>
            {sortField === "name" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("extension")}
          >
            <span>Est.</span>
            {sortField === "extension" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("size")}
          >
            <span>Dimensione</span>
            {sortField === "size" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("modified")}
          >
            <span>Data modifica</span>
            {sortField === "modified" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("kind")}
          >
            <span>Tipo</span>
            {sortField === "kind" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
        </div>
      )}

      {/* 4. Area Principale File (Lista / Griglia) */}
      <div
        ref={viewportRef}
        className="pane-entries-viewport"
        tabIndex={0}
        onKeyDown={handleKeyDown}
        onContextMenu={(e) => handleContextMenu(e, null)}
      >
        {viewMode === "grid" && !isSearchMode ? (
          <div className="entries-grid-container">
            {visibleEntries.map((entry, idx) => {
              const isSelected =
                selectedPaths.has(entry.path) ||
                (entry.isParent && focusedIndex === idx);
              return (
                <div
                  key={entry.path + idx}
                  className={`grid-tile ${isSelected ? "selected" : ""}`}
                  onClick={(e) => handleEntryClick(e, entry, idx)}
                  onDoubleClick={() => onOpenEntry(entry)}
                  onContextMenu={(e) => handleContextMenu(e, entry, idx)}
                >
                  <div className="grid-tile-icon-wrap">
                    {renderGridArtwork(entry)}
                  </div>
                  <div className="grid-tile-label" title={entry.name}>
                    {entry.name}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <div>
            {visibleEntries.map((entry, idx) => {
              const isSelected =
                selectedPaths.has(entry.path) ||
                (entry.isParent &&
                  focusedIndex === idx &&
                  selectedPaths.size === 0);
              return (
                <div
                  key={entry.path + idx}
                  className={`list-row ${isSearchMode ? "search-row" : ""} ${
                    isSelected ? "selected" : ""
                  } ${focusedIndex === idx ? "focused" : ""}`}
                  onClick={(e) => handleEntryClick(e, entry, idx)}
                  onDoubleClick={() => onOpenEntry(entry)}
                  onContextMenu={(e) => handleContextMenu(e, entry, idx)}
                >
                  {isSearchMode ? (
                    <>
                      <div className="list-cell" title={entry.name}>
                        {renderSmallIcon(entry, isSelected)}
                        <span>{entry.name}</span>
                      </div>
                      <div
                        className="list-cell"
                        title={entry.parentDir}
                        style={{ opacity: 0.85 }}
                      >
                        {entry.parentDir}
                      </div>
                      <div className="list-cell size-col">
                        {entry.isDir ? "DIR" : formatBytesShort(entry.size)}
                      </div>
                      <div className="list-cell">
                        {formatTimestamp(entry.modified)}
                      </div>
                    </>
                  ) : (
                    <>
                      <div className="list-cell" title={entry.name}>
                        {renderSmallIcon(entry, isSelected)}
                        <span>{entry.isDir ? entry.name : entry.stem}</span>
                      </div>
                      <div className="list-cell">{entry.extension}</div>
                      <div className="list-cell size-col">
                        {entry.isDir ? "DIR" : formatBytesShort(entry.size)}
                      </div>
                      <div className="list-cell">
                        {formatTimestamp(entry.modified)}
                      </div>
                      <div className="list-cell">{entry.kind}</div>
                    </>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* 5. Menu Contestuale Tasto Destro */}
      {ctxMenu && (
        <div
          className="context-menu-popup"
          style={{ left: ctxMenu.x, top: ctxMenu.y }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          {ctxMenu.entry && (
            <>
              <button
                className="context-menu-item"
                onClick={() => {
                  const target = ctxMenu.entry!;
                  setCtxMenu(null);
                  onOpenEntry(target);
                }}
              >
                <Folder size={14} />
                <span>
                  {ctxMenu.entry.isArchive
                    ? "Naviga dentro l'archivio"
                    : "Apri"}
                </span>
              </button>

              {isSearchMode && ctxMenu.entry.parentDir && (
                <button
                  className="context-menu-item"
                  onClick={() => {
                    const dir = ctxMenu.entry!.parentDir;
                    setCtxMenu(null);
                    onNavigate(dir);
                  }}
                >
                  <HardDrive size={14} />
                  <span>Apri cartella contenitore</span>
                </button>
              )}

              {ctxMenu.entry.isArchive && (
                <>
                  <div className="context-menu-sep" />
                  <button
                    className="context-menu-item"
                    onClick={() => {
                      const target = ctxMenu.entry!;
                      setCtxMenu(null);
                      onExtractHereRequest(target);
                    }}
                  >
                    <FolderOutput size={14} />
                    <span>Estrai qui</span>
                  </button>
                  <button
                    className="context-menu-item"
                    onClick={() => {
                      const target = ctxMenu.entry!;
                      setCtxMenu(null);
                      onExtractToFolderRequest(target);
                    }}
                  >
                    <FolderOutput size={14} />
                    <span>Estrai in "{ctxMenu.entry.stem}\"...</span>
                  </button>
                </>
              )}

              <div className="context-menu-sep" />
              <button
                className="context-menu-item"
                onClick={() => {
                  setCtxMenu(null);
                  onCompressRequest();
                }}
              >
                <FileArchive size={14} />
                <span>Comprimi in .7z / .zip...</span>
              </button>

              <button
                className="context-menu-item"
                onClick={() => {
                  const target = ctxMenu.entry!;
                  setCtxMenu(null);
                  onRenameRequest(target);
                }}
              >
                <Edit3 size={14} />
                <span>Rinomina</span>
              </button>

              <button
                className="context-menu-item"
                onClick={() => {
                  navigator.clipboard.writeText(ctxMenu.entry!.path);
                  setCtxMenu(null);
                }}
              >
                <Copy size={14} />
                <span>Copia percorso completo</span>
              </button>

              <div className="context-menu-sep" />
              <button
                className="context-menu-item"
                onClick={() => {
                  setCtxMenu(null);
                  onDeleteRequest();
                }}
              >
                <Trash2 size={14} />
                <span>Elimina (F8)</span>
              </button>
            </>
          )}

          {!ctxMenu.entry && (
            <>
              <button
                className="context-menu-item"
                onClick={() => {
                  setCtxMenu(null);
                  onNewFolderRequest();
                }}
              >
                <FolderPlus size={14} />
                <span>Nuova Cartella (F7)</span>
              </button>
              <button
                className="context-menu-item"
                onClick={() => {
                  setCtxMenu(null);
                  onNavigate(currentPath);
                }}
              >
                <RefreshCw size={14} />
                <span>Aggiorna vista</span>
              </button>
            </>
          )}
        </div>
      )}

      {/* 6. Status Bar del Pannello */}
      <footer className="pane-status-bar">
        <span>
          Selezionati: {selectionStats.selFiles} file, {selectionStats.selDirs}{" "}
          cartelle ({formatBytesShort(selectionStats.selBytes)})
        </span>
        <span>
          Totale: {listing?.totalFiles || 0} file, {listing?.totalDirs || 0}{" "}
          cartelle ({formatBytesShort(listing?.totalBytes || 0)})
        </span>
      </footer>
    </section>
  );
};

function renderSmallIcon(entry: VfsEntry, isSelected: boolean) {
  const color = isSelected ? "#ffffff" : "var(--accent-color)";
  if (entry.isDir) {
    return (
      <Folder
        size={15}
        fill={isSelected ? "#ffffff" : "var(--folder-icon-bottom)"}
        color={isSelected ? "#ffffff" : "var(--folder-icon-glyph)"}
      />
    );
  }
  if (entry.isArchive)
    return <FileArchive size={15} color={isSelected ? "#fff" : "#d97706"} />;
  switch (entry.kind) {
    case "image":
      return <ImageIcon size={15} color={isSelected ? "#fff" : "#10b981"} />;
    case "video":
      return <Film size={15} color={isSelected ? "#fff" : "#8b5cf6"} />;
    case "audio":
      return <Music size={15} color={isSelected ? "#fff" : "#ec4899"} />;
    case "code":
      return <Code size={15} color={isSelected ? "#fff" : color} />;
    default:
      return <FileText size={15} color={isSelected ? "#fff" : "#64748b"} />;
  }
}

function renderGridArtwork(entry: VfsEntry) {
  if (entry.isDir) {
    return (
      <svg className="sky-folder-svg" viewBox="0 0 72 58" fill="none">
        <path
          d="M4 11C4 7.68629 6.68629 5 10 5H25.5C27.4 5 29.1 6.1 30 7.8L32.2 12H62C65.3137 12 68 14.6863 68 18V48C68 51.3137 65.3137 54 62 54H10C6.68629 54 4 51.3137 4 48V11Z"
          fill="var(--folder-icon-top)"
        />
        <rect
          x="4"
          y="16"
          width="64"
          height="38"
          rx="6"
          fill="var(--folder-icon-bottom)"
        />
      </svg>
    );
  }
  if (entry.isArchive) return <FileArchive size={42} color="#d97706" />;
  if (entry.kind === "image") return <ImageIcon size={42} color="#10b981" />;
  return <FileText size={42} color="var(--accent-color)" />;
}

function formatGigabytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(1).replace(".", ",")} GB`;
}

function formatBytesShort(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

function formatTimestamp(unixSecs: number): string {
  if (!unixSecs) return "--";
  const d = new Date(unixSecs * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}
