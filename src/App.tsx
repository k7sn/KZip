import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  DirectoryListing,
  DriveInfo,
  EngineStatus,
  LayoutMode,
  PaneTab,
  SearchQuery,
  SearchResponse,
  VfsEntry,
  ViewMode,
} from "./types/vfs";
import { TopToolbar } from "./components/TopToolbar";
import { FilePane } from "./components/FilePane";
import { BottomBar } from "./components/BottomBar";
import { QuickViewModal } from "./components/QuickViewModal";
import { ThemeModal } from "./components/ThemeModal";
import { ArchiveModal } from "./components/ArchiveModal";

interface PaneState {
  tabs: PaneTab[];
  activeTabId: string;
  viewMode: ViewMode;
  listing: DirectoryListing | null;
  selectedPaths: Set<string>;
  lastRealFolder: string;
}

function createInitialTab(id: string, path: string): PaneTab {
  const folderName =
    path.replace(/\\$/, "").split("\\").pop() || path || "Root";
  return {
    id,
    title: folderName,
    path,
    history: [path],
    historyIndex: 0,
  };
}

export const App: React.FC = () => {
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>(() => {
    const saved = localStorage.getItem("kzip.layoutMode");
    return saved === "single" || saved === "dual" ? saved : "dual";
  });
  const [activePane, setActivePane] = useState<"left" | "right">("left");
  const [splitRatio, setSplitRatio] = useState<number>(50);
  const [isDraggingSplitter, setIsDraggingSplitter] = useState<boolean>(false);
  const [showHidden, setShowHidden] = useState<boolean>(false);
  const [engineStatus, setEngineStatus] = useState<EngineStatus | null>(null);

  const [searchQuery, setSearchQuery] = useState<string>("");
  const [searchScopeGlobal, setSearchScopeGlobal] = useState<boolean>(true);

  const [leftPane, setLeftPane] = useState<PaneState>(() => ({
    tabs: [createInitialTab("L-1", "C:\\")],
    activeTabId: "L-1",
    viewMode: "list",
    listing: null,
    selectedPaths: new Set(),
    lastRealFolder: "C:\\",
  }));

  const [rightPane, setRightPane] = useState<PaneState>(() => ({
    tabs: [createInitialTab("R-1", "C:\\")],
    activeTabId: "R-1",
    viewMode: "list",
    listing: null,
    selectedPaths: new Set(),
    lastRealFolder: "C:\\",
  }));

  const [themeModalOpen, setThemeModalOpen] = useState(false);
  const [archiveModalOpen, setArchiveModalOpen] = useState(false);
  const [quickViewData, setQuickViewData] = useState<{
    title: string;
    content: string;
    isDiagnostics?: boolean;
  } | null>(null);

  const workspaceRef = useRef<HTMLDivElement>(null);
  const searchDebounceRef = useRef<number | null>(null);

  const fetchDirectory = useCallback(
    async (pane: "left" | "right", targetPath: string, pushHistory = true) => {
      try {
        const listing = await invoke<DirectoryListing>("read_vfs_directory", {
          path: targetPath,
        });
        const cleanTitle = listing.currentPath.startsWith("search://")
          ? `🔍 ${listing.currentPath.replace("search://", "")}`
          : listing.currentPath.replace(/\\$/, "").split("\\").pop() ||
            listing.currentPath;

        const isRealLocal = listing.providerScheme === "local";
        if (isRealLocal) {
          localStorage.setItem(`kzip.lastPath.${pane}`, listing.currentPath);
        }

        const updater = (prev: PaneState): PaneState => {
          const updatedTabs = prev.tabs.map((tab) => {
            if (tab.id !== prev.activeTabId) return tab;
            if (!pushHistory) {
              return { ...tab, path: listing.currentPath, title: cleanTitle };
            }
            const sliced = tab.history.slice(0, tab.historyIndex + 1);
            const nextHistory = [...sliced, listing.currentPath];
            return {
              ...tab,
              title: cleanTitle,
              path: listing.currentPath,
              history: nextHistory,
              historyIndex: nextHistory.length - 1,
            };
          });
          return {
            ...prev,
            tabs: updatedTabs,
            listing,
            selectedPaths: new Set(),
            lastRealFolder: isRealLocal
              ? listing.currentPath
              : prev.lastRealFolder,
          };
        };

        if (pane === "left") setLeftPane(updater);
        else setRightPane(updater);
      } catch (err) {
        alert(`Errore accesso al percorso:\n${String(err)}`);
      }
    },
    []
  );

  // Avvio intelligente: riparte dall'ultimo percorso salvato o da C:\Users\<utente>
  useEffect(() => {
    (async () => {
      try {
        const [drvList, defaultHome, status] = await Promise.all([
          invoke<DriveInfo[]>("list_drives"),
          invoke<string>("get_startup_paths"),
          invoke<EngineStatus>("get_engine_status"),
        ]);
        setDrives(drvList);
        setEngineStatus(status);

        const savedLeft =
          localStorage.getItem("kzip.lastPath.left") || defaultHome;
        const savedRight =
          localStorage.getItem("kzip.lastPath.right") || defaultHome;

        await fetchDirectory("left", savedLeft, false);
        await fetchDirectory("right", savedRight, false);
      } catch (e) {
        console.error("Errore inizializzazione:", e);
      }
    })();
  }, [fetchDirectory]);

  // Gestione Splitter Dual-Pane
  useEffect(() => {
    if (!isDraggingSplitter) return;
    const handleMove = (e: MouseEvent) => {
      if (!workspaceRef.current) return;
      const rect = workspaceRef.current.getBoundingClientRect();
      const pct = ((e.clientX - rect.left) / rect.width) * 100;
      setSplitRatio(Math.min(78, Math.max(22, pct)));
    };
    const handleUp = () => setIsDraggingSplitter(false);
    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
    return () => {
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    };
  }, [isDraggingSplitter]);

  const currentPaneState = activePane === "left" ? leftPane : rightPane;
  const targetPaneState =
    layoutMode === "dual"
      ? activePane === "left"
        ? rightPane
        : leftPane
      : currentPaneState;

  const activeTab =
    currentPaneState.tabs.find((t) => t.id === currentPaneState.activeTabId) ||
    currentPaneState.tabs[0];

  const refreshBothPanes = useCallback(async () => {
    const leftPath = leftPane.listing?.currentPath || leftPane.lastRealFolder;
    const rightPath =
      rightPane.listing?.currentPath || rightPane.lastRealFolder;
    await fetchDirectory("left", leftPath, false);
    if (layoutMode === "dual") {
      await fetchDirectory("right", rightPath, false);
    }
  }, [
    leftPane.listing,
    leftPane.lastRealFolder,
    rightPane.listing,
    rightPane.lastRealFolder,
    layoutMode,
    fetchDirectory,
  ]);

  // Ricerca istantanea stile Everything direttamente nel pannello attivo
  const runInstantSearch = useCallback(
    async (rawQuery: string) => {
      const q = rawQuery.trim();
      if (!q) {
        fetchDirectory(activePane, currentPaneState.lastRealFolder, false);
        return;
      }

      try {
        const params: SearchQuery = {
          query: q,
          scopePath: searchScopeGlobal
            ? null
            : currentPaneState.lastRealFolder,
          extensions: null,
          minSize: null,
          maxSize: null,
          useRegex: false,
          matchCase: false,
          maxResults: 1000,
        };
        const res = await invoke<SearchResponse>("execute_search", { params });
        await fetchDirectory(activePane, res.vfsUri, false);
      } catch (err) {
        console.error("Search error:", err);
      }
    },
    [activePane, currentPaneState.lastRealFolder, searchScopeGlobal, fetchDirectory]
  );

  const handleSearchInputChange = (val: string) => {
    setSearchQuery(val);
    if (searchDebounceRef.current) {
      window.clearTimeout(searchDebounceRef.current);
    }
    if (!val.trim()) {
      fetchDirectory(activePane, currentPaneState.lastRealFolder, false);
      return;
    }
    searchDebounceRef.current = window.setTimeout(() => {
      runInstantSearch(val);
    }, 220);
  };

  const formatDiagnosticsReport = (st: EngineStatus) => {
    return [
      "=== STATO MOTORI KZIP ===",
      "",
      `[Everything Search Engine]`,
      `• Connesso a Everything IPC/Pipe: ${
        st.everythingServiceConnected ? "SÌ" : "NO (Fallback Rust attivo)"
      }`,
      `• Versione rilevata: ${st.everythingVersion || "N/D"}`,
      `• Motore attivo: ${st.activeSearchEngine}`,
      "",
      `[Motore Archivi 7-Zip / NanaZip COM (KZip.Core.dll)]`,
      `• DLL Caricata: ${st.archiveDllLoaded ? "SÌ" : "NO"}`,
      `• Percorso attivo: ${st.archiveLoadedPath || "Nessuno"}`,
      `• Formati supportati registrati: ${st.archiveFormatsCount}`,
      "",
      "--- Log Diagnostico Caricamento KZip.Core.dll & Codecs ---",
      ...(st.archiveLoadDiagnostics.length > 0
        ? st.archiveLoadDiagnostics
        : ["Nessun tentativo registrato"]),
    ].join("\n");
  };

  const handleOpenDiagnostics = useCallback(async () => {
    const st = await invoke<EngineStatus>("get_engine_status");
    setEngineStatus(st);
    setQuickViewData({
      title: "Diagnostica Motori KZip (Everything IPC & KZip.Core.dll)",
      content: formatDiagnosticsReport(st),
      isDiagnostics: true,
    });
  }, []);

  const handleViewF3 = useCallback(async () => {
    const selected = Array.from(currentPaneState.selectedPaths);
    if (selected.length === 0) {
      await handleOpenDiagnostics();
      return;
    }
    try {
      const text = await invoke<string>("read_file_preview", {
        path: selected[0],
      });
      setQuickViewData({
        title: `Anteprima (F3) — ${selected[0]}`,
        content: text,
      });
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.selectedPaths, handleOpenDiagnostics]);

  const handleRenameEntry = useCallback(
    async (entry?: VfsEntry) => {
      const targetPath =
        entry?.path || Array.from(currentPaneState.selectedPaths)[0];
      if (!targetPath) return;
      const currentName = targetPath.split("\\").pop() || targetPath;
      const newName = window.prompt("Rinomina elemento in:", currentName);
      if (!newName || newName === currentName) return;
      try {
        await invoke("rename_item", { oldPath: targetPath, newName });
        await refreshBothPanes();
      } catch (err) {
        alert(String(err));
      }
    },
    [currentPaneState.selectedPaths, refreshBothPanes]
  );

  const handleCopyF5 = useCallback(async () => {
    const sources = Array.from(currentPaneState.selectedPaths);
    const destDir = targetPaneState.lastRealFolder;
    if (sources.length === 0 || !destDir) return;
    try {
      await invoke("copy_items", { sources, destinationDir: destDir });
      await refreshBothPanes();
    } catch (err) {
      alert(String(err));
    }
  }, [
    currentPaneState.selectedPaths,
    targetPaneState.lastRealFolder,
    refreshBothPanes,
  ]);

  const handleMoveF6 = useCallback(async () => {
    const sources = Array.from(currentPaneState.selectedPaths);
    const destDir = targetPaneState.lastRealFolder;
    if (sources.length === 0 || !destDir) return;
    try {
      await invoke("move_items", { sources, destinationDir: destDir });
      await refreshBothPanes();
    } catch (err) {
      alert(String(err));
    }
  }, [
    currentPaneState.selectedPaths,
    targetPaneState.lastRealFolder,
    refreshBothPanes,
  ]);

  const handleNewFolderF7 = useCallback(async () => {
    const parentPath = currentPaneState.lastRealFolder;
    if (!parentPath) return;
    const folderName = window.prompt(
      "Nome della nuova cartella:",
      "Nuova cartella"
    );
    if (!folderName) return;
    try {
      await invoke("create_directory", { parentPath, folderName });
      await fetchDirectory(activePane, parentPath, false);
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.lastRealFolder, activePane, fetchDirectory]);

  const handleDeleteF8 = useCallback(async () => {
    const paths = Array.from(currentPaneState.selectedPaths);
    if (paths.length === 0) return;
    const ok = window.confirm(
      `Confermi l'eliminazione definitiva di ${paths.length} elemento/i?`
    );
    if (!ok) return;
    try {
      await invoke("delete_items", { paths });
      await refreshBothPanes();
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.selectedPaths, refreshBothPanes]);

  const handleExtractHere = useCallback(
    async (entry?: VfsEntry) => {
      const archivePath =
        entry?.path || Array.from(currentPaneState.selectedPaths)[0];
      if (!archivePath) {
        alert("Seleziona prima un file archivio (.7z, .zip, .rar, .tar) da estrarre.");
        return;
      }
      const destDir = currentPaneState.lastRealFolder;
      try {
        await invoke("extract_archive", {
          archivePath,
          destinationDir: destDir,
        });
        await refreshBothPanes();
      } catch (err) {
        alert(String(err));
      }
    },
    [
      currentPaneState.selectedPaths,
      currentPaneState.lastRealFolder,
      refreshBothPanes,
    ]
  );

  const handleExtractToSubfolder = useCallback(
    async (entry: VfsEntry) => {
      const sep = currentPaneState.lastRealFolder.endsWith("\\") ? "" : "\\";
      const destDir = `${currentPaneState.lastRealFolder}${sep}${entry.stem}`;
      try {
        await invoke("extract_archive", {
          archivePath: entry.path,
          destinationDir: destDir,
        });
        await refreshBothPanes();
      } catch (err) {
        alert(String(err));
      }
    },
    [currentPaneState.lastRealFolder, refreshBothPanes]
  );

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F3") {
        e.preventDefault();
        handleViewF3();
      } else if (e.key === "F4") {
        e.preventDefault();
        handleRenameEntry();
      } else if (e.key === "F5") {
        e.preventDefault();
        handleCopyF5();
      } else if (e.key === "F6") {
        e.preventDefault();
        handleMoveF6();
      } else if (e.key === "F7") {
        e.preventDefault();
        handleNewFolderF7();
      } else if (e.key === "F8") {
        e.preventDefault();
        handleDeleteF8();
      } else if (e.key === "Escape") {
        setThemeModalOpen(false);
        setArchiveModalOpen(false);
        setQuickViewData(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    handleViewF3,
    handleRenameEntry,
    handleCopyF5,
    handleMoveF6,
    handleNewFolderF7,
    handleDeleteF8,
  ]);

  const handlePaneHistoryStep = (pane: "left" | "right", delta: -1 | 1) => {
    const state = pane === "left" ? leftPane : rightPane;
    const tab =
      state.tabs.find((t) => t.id === state.activeTabId) || state.tabs[0];
    if (!tab) return;
    const nextIdx = tab.historyIndex + delta;
    if (nextIdx < 0 || nextIdx >= tab.history.length) return;
    const targetPath = tab.history[nextIdx];

    const setter = pane === "left" ? setLeftPane : setRightPane;
    setter((prev) => ({
      ...prev,
      tabs: prev.tabs.map((t) =>
        t.id === prev.activeTabId ? { ...t, historyIndex: nextIdx } : t
      ),
    }));
    fetchDirectory(pane, targetPath, false);
  };

  const handleOpenEntry = (pane: "left" | "right", entry: VfsEntry) => {
    if (entry.isDir || entry.isArchive) {
      fetchDirectory(pane, entry.path, true);
    } else {
      invoke<string>("read_file_preview", { path: entry.path })
        .then((content) =>
          setQuickViewData({ title: entry.name, content })
        )
        .catch((err) => alert(String(err)));
    }
  };

  const leftActiveTab =
    leftPane.tabs.find((t) => t.id === leftPane.activeTabId) ||
    leftPane.tabs[0];
  const rightActiveTab =
    rightPane.tabs.find((t) => t.id === rightPane.activeTabId) ||
    rightPane.tabs[0];

  const selectedArray = Array.from(currentPaneState.selectedPaths);
  const defaultArchiveBase =
    selectedArray.length === 1
      ? (selectedArray[0].split("\\").pop() || "Archivio").replace(
          /\.[^.]+$/,
          ""
        )
      : currentPaneState.lastRealFolder
          .replace(/\\$/, "")
          .split("\\")
          .pop() || "Archivio";

  return (
    <div className="kzip-shell">
      <TopToolbar
        layoutMode={layoutMode}
        onToggleLayoutMode={() => {
          const next = layoutMode === "dual" ? "single" : "dual";
          setLayoutMode(next);
          localStorage.setItem("kzip.layoutMode", next);
        }}
        activeViewMode={currentPaneState.viewMode}
        onChangeViewMode={(mode) => {
          const setter = activePane === "left" ? setLeftPane : setRightPane;
          setter((prev) => ({ ...prev, viewMode: mode }));
        }}
        showHidden={showHidden}
        onToggleHidden={() => setShowHidden((v) => !v)}
        searchQuery={searchQuery}
        onSearchQueryChange={handleSearchInputChange}
        onTriggerInstantSearch={runInstantSearch}
        onClearSearch={() => {
          setSearchQuery("");
          fetchDirectory(activePane, currentPaneState.lastRealFolder, false);
        }}
        searchScopeGlobal={searchScopeGlobal}
        onToggleSearchScope={() => {
          const next = !searchScopeGlobal;
          setSearchScopeGlobal(next);
          if (searchQuery.trim()) {
            runInstantSearch(searchQuery);
          }
        }}
        onOpenCompressModal={() => {
          if (selectedArray.length === 0) {
            alert("Seleziona almeno un file o una cartella da comprimere.");
            return;
          }
          setArchiveModalOpen(true);
        }}
        onExtractSelected={() => handleExtractHere()}
        onNewFolder={handleNewFolderF7}
        onOpenTerminal={() => {
          invoke("open_in_terminal", {
            path: currentPaneState.lastRealFolder,
          }).catch((err) => alert(String(err)));
        }}
        onOpenDiagnostics={handleOpenDiagnostics}
        onOpenThemeModal={() => setThemeModalOpen(true)}
        engineStatus={engineStatus}
      />

      <main ref={workspaceRef} className="dual-pane-workspace">
        <div
          style={{
            width: layoutMode === "single" ? "100%" : `${splitRatio}%`,
            height: "100%",
          }}
        >
          <FilePane
            paneId="left"
            isActive={activePane === "left"}
            onActivate={() => setActivePane("left")}
            drives={drives}
            tabs={leftPane.tabs}
            activeTabId={leftPane.activeTabId}
            onSelectTab={(tabId) => {
              const t = leftPane.tabs.find((x) => x.id === tabId);
              setLeftPane((p) => ({ ...p, activeTabId: tabId }));
              if (t) fetchDirectory("left", t.path, false);
            }}
            onAddTab={() => {
              const id = `L-${Date.now()}`;
              const cur = leftPane.lastRealFolder;
              const newTab = createInitialTab(id, cur);
              setLeftPane((p) => ({
                ...p,
                tabs: [...p.tabs, newTab],
                activeTabId: id,
              }));
              fetchDirectory("left", cur, false);
            }}
            onCloseTab={(tabId) => {
              setLeftPane((p) => {
                const filtered = p.tabs.filter((t) => t.id !== tabId);
                const nextActive =
                  p.activeTabId === tabId ? filtered[0].id : p.activeTabId;
                return { ...p, tabs: filtered, activeTabId: nextActive };
              });
            }}
            canGoBack={Boolean(leftActiveTab && leftActiveTab.historyIndex > 0)}
            canGoForward={Boolean(
              leftActiveTab &&
                leftActiveTab.historyIndex < leftActiveTab.history.length - 1
            )}
            onBack={() => handlePaneHistoryStep("left", -1)}
            onForward={() => handlePaneHistoryStep("left", 1)}
            onUp={() => {
              if (leftPane.listing?.parentPath) {
                fetchDirectory("left", leftPane.listing.parentPath, true);
              }
            }}
            listing={leftPane.listing}
            viewMode={leftPane.viewMode}
            showHidden={showHidden}
            selectedPaths={leftPane.selectedPaths}
            onSelectionChange={(next) =>
              setLeftPane((p) => ({ ...p, selectedPaths: next }))
            }
            onNavigate={(path) => fetchDirectory("left", path, true)}
            onOpenEntry={(entry) => handleOpenEntry("left", entry)}
            onCompressRequest={() => setArchiveModalOpen(true)}
            onExtractHereRequest={(entry) => handleExtractHere(entry)}
            onExtractToFolderRequest={(entry) => handleExtractToSubfolder(entry)}
            onRenameRequest={(entry) => handleRenameEntry(entry)}
            onDeleteRequest={handleDeleteF8}
            onNewFolderRequest={handleNewFolderF7}
          />
        </div>

        {layoutMode === "dual" && (
          <>
            <div
              className={`pane-splitter ${
                isDraggingSplitter ? "dragging" : ""
              }`}
              onMouseDown={() => setIsDraggingSplitter(true)}
            />

            <div style={{ width: `${100 - splitRatio}%`, height: "100%" }}>
              <FilePane
                paneId="right"
                isActive={activePane === "right"}
                onActivate={() => setActivePane("right")}
                drives={drives}
                tabs={rightPane.tabs}
                activeTabId={rightPane.activeTabId}
                onSelectTab={(tabId) => {
                  const t = rightPane.tabs.find((x) => x.id === tabId);
                  setRightPane((p) => ({ ...p, activeTabId: tabId }));
                  if (t) fetchDirectory("right", t.path, false);
                }}
                onAddTab={() => {
                  const id = `R-${Date.now()}`;
                  const cur = rightPane.lastRealFolder;
                  const newTab = createInitialTab(id, cur);
                  setRightPane((p) => ({
                    ...p,
                    tabs: [...p.tabs, newTab],
                    activeTabId: id,
                  }));
                  fetchDirectory("right", cur, false);
                }}
                onCloseTab={(tabId) => {
                  setRightPane((p) => {
                    const filtered = p.tabs.filter((t) => t.id !== tabId);
                    const nextActive =
                      p.activeTabId === tabId ? filtered[0].id : p.activeTabId;
                    return { ...p, tabs: filtered, activeTabId: nextActive };
                  });
                }}
                canGoBack={Boolean(
                  rightActiveTab && rightActiveTab.historyIndex > 0
                )}
                canGoForward={Boolean(
                  rightActiveTab &&
                    rightActiveTab.historyIndex <
                      rightActiveTab.history.length - 1
                )}
                onBack={() => handlePaneHistoryStep("right", -1)}
                onForward={() => handlePaneHistoryStep("right", 1)}
                onUp={() => {
                  if (rightPane.listing?.parentPath) {
                    fetchDirectory("right", rightPane.listing.parentPath, true);
                  }
                }}
                listing={rightPane.listing}
                viewMode={rightPane.viewMode}
                showHidden={showHidden}
                selectedPaths={rightPane.selectedPaths}
                onSelectionChange={(next) =>
                  setRightPane((p) => ({ ...p, selectedPaths: next }))
                }
                onNavigate={(path) => fetchDirectory("right", path, true)}
                onOpenEntry={(entry) => handleOpenEntry("right", entry)}
                onCompressRequest={() => setArchiveModalOpen(true)}
                onExtractHereRequest={(entry) => handleExtractHere(entry)}
                onExtractToFolderRequest={(entry) =>
                  handleExtractToSubfolder(entry)
                }
                onRenameRequest={(entry) => handleRenameEntry(entry)}
                onDeleteRequest={handleDeleteF8}
                onNewFolderRequest={handleNewFolderF7}
              />
            </div>
          </>
        )}
      </main>

      <BottomBar
        onViewF3={handleViewF3}
        onEditF4={() => handleRenameEntry()}
        onCopyF5={handleCopyF5}
        onMoveF6={handleMoveF6}
        onNewFolderF7={handleNewFolderF7}
        onDeleteF8={handleDeleteF8}
      />

      {themeModalOpen && (
        <ThemeModal onClose={() => setThemeModalOpen(false)} />
      )}

      {archiveModalOpen && (
        <ArchiveModal
          defaultArchiveName={defaultArchiveBase}
          destinationDir={currentPaneState.lastRealFolder}
          selectedCount={selectedArray.length}
          onClose={() => setArchiveModalOpen(false)}
          onConfirmCompress={async (fullArchivePath) => {
            try {
              await invoke("compress_Selection", {
                sources: selectedArray,
                destinationArchive: fullArchivePath,
              });
              await refreshBothPanes();
            } catch (err) {
              alert(String(err));
            }
          }}
        />
      )}

      {quickViewData && (
        <QuickViewModal
          title={quickViewData.title}
          content={quickViewData.content}
          onReloadDll={
            quickViewData.isDiagnostics
              ? async () => {
                  const st = await invoke<EngineStatus>("reload_archive_engine");
                  setEngineStatus(st);
                  setQuickViewData({
                    title:
                      "Diagnostica Motori KZip (Everything IPC & KZip.Core.dll)",
                    content: formatDiagnosticsReport(st),
                    isDiagnostics: true,
                  });
                }
              : undefined
          }
          onClose={() => setQuickViewData(null)}
        />
      )}
    </div>
  );
};
