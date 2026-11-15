import React, { useMemo, useRef, useState } from "react";
import {
  HardDrive,
  Globe,
  Cpu,
  Folder,
  FileText,
  FileArchive,
  Image as ImageIcon,
  Film,
  Music,
  Code,
  Star,
  AlignJustify,
  ChevronRight,
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
  listing: DirectoryListing | null;
  viewMode: ViewMode;
  showHidden: boolean;
  selectedPaths: Set<string>;
  onSelectionChange: (next: Set<string>) => void;
  onNavigate: (path: string) => void;
  onOpenEntry: (entry: VfsEntry) => void;
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
  listing,
  viewMode,
  showHidden,
  selectedPaths,
  onSelectionChange,
  onNavigate,
  onOpenEntry,
}) => {
  const [sortField, setSortField] = useState<SortField>("name");
  const [sortAsc, setSortAsc] = useState<boolean>(true);
  const [focusedIndex, setFocusedIndex] = useState<number>(0);
  const [lastClickedIndex, setLastClickedIndex] = useState<number>(0);
  const viewportRef = useRef<HTMLDivElement>(null);

  const currentPath = listing?.currentPath || "C:\\";

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
        case "extension":
          cmp = a.extension.localeCompare(b.extension, undefined, { sensitivity: "base" });
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
    if (currentPath.startsWith("search://") || currentPath.startsWith("archive://")) {
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

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (visibleEntries.length === 0) return;

    if (e.key === "ArrowDown" || e.key === "ArrowRight") {
      e.preventDefault();
      const step = viewMode === "grid" && e.key === "ArrowDown" ? 3 : 1;
      const nextIdx = Math.min(visibleEntries.length - 1, focusedIndex + step);
      setFocusedIndex(nextIdx);
      const target = visibleEntries[nextIdx];
      if (target && !target.isParent) {
        onSelectionChange(new Set([target.path]));
      }
    } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
      e.preventDefault();
      const step = viewMode === "grid" && e.key === "ArrowUp" ? 3 : 1;
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
      if (listing?.parentPath) {
        onNavigate(listing.parentPath);
      }
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

  const focusedEntry = visibleEntries[focusedIndex] || visibleEntries[0] || null;

  return (
    <section
      className={`file-pane ${isActive ? "active-pane" : ""}`}
      onMouseDown={onActivate}
    >
      <div className="quick-volumes-bar">
        {drives.map((drv) => (
          <button
            key={drv.mountPoint}
            className={`quick-vol-pill ${
              currentDrive?.mountPoint === drv.mountPoint ? "active" : ""
            }`}
            onClick={() => onNavigate(drv.mountPoint)}
          >
            <HardDrive size={12} />
            <span>{drv.name}</span>
          </button>
        ))}
        <button className="quick-vol-pill" onClick={() => onNavigate("C:\\Users")}>
          <Globe size={12} color="#0284c7" />
          <span>Users</span>
        </button>
        <button className="quick-vol-pill" onClick={() => onNavigate("C:\\Windows")}>
          <Cpu size={12} color="#0284c7" />
          <span>System</span>
        </button>
      </div>

      <div className="drive-selector-bar">
        <select
          className="drive-select-combo"
          value={currentDrive?.mountPoint || ""}
          onChange={(e) => onNavigate(e.target.value)}
        >
          {drives.map((d) => (
            <option key={d.mountPoint} value={d.mountPoint}>
              {d.name}
            </option>
          ))}
        </select>
        {currentDrive && (
          <span className="drive-free-text">
            {formatGigabytes(currentDrive.availableSpace)} of{" "}
            {formatGigabytes(currentDrive.totalSpace)} free
          </span>
        )}
      </div>

      <div className="pane-tab-bar">
        <div className="pane-tabs-scroll">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`tab-item ${tab.id === activeTabId ? "active" : ""}`}
              onClick={() => onSelectTab(tab.id)}
            >
              <span>{tab.title}</span>
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

      <div className="breadcrumb-bar">
        <div className="breadcrumb-segments">
          {breadcrumbParts.map((part, idx) => (
            <React.Fragment key={part.target + idx}>
              {idx > 0 && <ChevronRight size={11} color="#8e8e93" />}
              <button
                className="crumb-btn"
                onClick={() => onNavigate(part.target)}
              >
                {idx === 0 ? (
                  <HardDrive size={12} color="#5c5c63" />
                ) : (
                  <Folder size={12} fill="#52baff" color="#268fd8" />
                )}
                <span>{part.label}</span>
              </button>
            </React.Fragment>
          ))}
        </div>
        <div className="breadcrumb-actions">
          <button
            className="crumb-icon-btn"
            onClick={() => onNavigate(currentPath)}
            title="Ricarica percorso"
          >
            <AlignJustify size={13} />
          </button>
          <button
            className="crumb-icon-btn"
            onClick={() => onNavigate("C:\\")}
            title="Vai alla radice disco"
          >
            <Star size={13} />
          </button>
        </div>
      </div>

      {viewMode === "grid" ? (
        <div className="column-headers-bar grid-mode-headers">
          {(
            [
              ["name", "name"],
              ["extension", "extension"],
              ["size", "size"],
              ["modified", "modified"],
              ["created", "created"],
              ["modified", "added"],
              ["kind", "kind"],
            ] as [SortField, string][]
          ).map(([field, label], i) => (
            <div
              key={label + i}
              className="col-header-cell"
              onClick={() => handleHeaderSort(field)}
            >
              <span>{label}</span>
              {sortField === field && <span>{sortAsc ? "▲" : "▼"}</span>}
            </div>
          ))}
        </div>
      ) : (
        <div className="column-headers-bar">
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("name")}
          >
            <span>name</span>
            {sortField === "name" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("extension")}
          >
            <span>ext</span>
            {sortField === "extension" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("size")}
          >
            <span>size</span>
            {sortField === "size" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("modified")}
          >
            <span>modified</span>
            {sortField === "modified" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
          <div
            className="col-header-cell"
            onClick={() => handleHeaderSort("kind")}
          >
            <span>kind</span>
            {sortField === "kind" && <span>{sortAsc ? "▲" : "▼"}</span>}
          </div>
        </div>
      )}

      <div
        ref={viewportRef}
        className="pane-entries-viewport"
        tabIndex={0}
        onKeyDown={handleKeyDown}
      >
        {viewMode === "grid" ? (
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
                >
                  <div
                    className={`grid-tile-icon-wrap ${
                      entry.isParent ? "is-parent-tile" : ""
                    }`}
                  >
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
                (entry.isParent && focusedIndex === idx && selectedPaths.size === 0);
              return (
                <div
                  key={entry.path + idx}
                  className={`list-row ${isSelected ? "selected" : ""} ${
                    focusedIndex === idx ? "focused" : ""
                  }`}
                  onClick={(e) => handleEntryClick(e, entry, idx)}
                  onDoubleClick={() => onOpenEntry(entry)}
                >
                  <div className="list-cell">
                    {renderSmallIcon(entry, isSelected)}
                    <span>{entry.isDir ? entry.name : entry.stem}</span>
                  </div>
                  <div className="list-cell">{entry.extension}</div>
                  <div className="list-cell size-col">
                    {entry.isDir ? "DIR" : formatBytesShort(entry.size)}
                  </div>
                  <div className="list-cell">{formatTimestamp(entry.modified)}</div>
                  <div className="list-cell">{entry.kind}</div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <footer className="pane-status-bar">
        {viewMode === "grid" && focusedEntry ? (
          <>
            <span>{focusedEntry.name}</span>
            <span>
              {focusedEntry.isDir ? "DIR" : formatBytesShort(focusedEntry.size)}{" "}
              {formatTimestampWithSeconds(focusedEntry.modified)}
            </span>
          </>
        ) : (
          <span style={{ margin: "0 auto" }}>
            {formatBytesShort(selectionStats.selBytes)} /{" "}
            {formatBytesShort(listing?.totalBytes || 0)} in{" "}
            {selectionStats.selFiles} / {listing?.totalFiles || 0} file(s),{" "}
            {selectionStats.selDirs} / {listing?.totalDirs || 0} dir(s)
          </span>
        )}
      </footer>
    </section>
  );
};

function renderSmallIcon(entry: VfsEntry, isSelected: boolean) {
  const color = isSelected ? "#ffffff" : "#268fd8";
  if (entry.isDir) {
    return <Folder size={14} fill={isSelected ? "#76d0ff" : "#52baff"} color={color} />;
  }
  if (entry.isArchive) return <FileArchive size={14} color={isSelected ? "#fff" : "#d97706"} />;
  switch (entry.kind) {
    case "image":
      return <ImageIcon size={14} color={isSelected ? "#fff" : "#10b981"} />;
    case "video":
      return <Film size={14} color={isSelected ? "#fff" : "#8b5cf6"} />;
    case "audio":
      return <Music size={14} color={isSelected ? "#fff" : "#ec4899"} />;
    case "code":
      return <Code size={14} color={isSelected ? "#fff" : "#0284c7"} />;
    default:
      return <FileText size={14} color={isSelected ? "#fff" : "#64748b"} />;
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
        <rect
          x="6"
          y="18"
          width="60"
          height="3"
          rx="1.5"
          fill="rgba(255,255,255,0.45)"
        />
      </svg>
    );
  }

  if (entry.kind === "image") {
    return (
      <div className="photo-frame-preview">
        <div className="photo-frame-art">
          <ImageIcon size={20} />
        </div>
      </div>
    );
  }

  if (entry.isArchive) {
    return <FileArchive size={44} color="#d97706" />;
  }

  return <FileText size={44} color="#3b82f6" />;
}

function formatGigabytes(bytes: number): string {
  const gb = bytes / (1024 * 1024 * 1024);
  return `${gb.toFixed(2).replace(".", ",")} GB`;
}

function formatBytesShort(bytes: number): string {
  if (bytes === 0) return "0 bytes";
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
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()}, ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

function formatTimestampWithSeconds(unixSecs: number): string {
  if (!unixSecs) return "";
  const d = new Date(unixSecs * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()}, ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}
