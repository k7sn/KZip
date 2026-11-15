import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  DirectoryListing,
  DriveInfo,
  EngineStatus,
  PaneTab,
  SearchQuery,
  SearchResponse,
  VfsEntry,
  ViewMode,
} from "./types/vfs";
import { TopToolbar } from "./components/TopToolbar";
import { FilePane } from "./components/FilePane";
import { BottomBar } from "./components/BottomBar";
import { SearchModal } from "./components/SearchModal";
import { QuickViewModal } from "./components/QuickViewModal";

interface PaneState {
  tabs: PaneTab[];
  activeTabId: string;
  viewMode: ViewMode;
  listing: DirectoryListing | null;
  selectedPaths: Set<string>;
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
  const [activePane, setActivePane] = useState<"left" | "right">("left");
  const [splitRatio, setSplitRatio] = useState<number>(50);
  const [isDraggingSplitter, setIsDraggingSplitter] = useState<boolean>(false);
  const [showHidden, setShowHidden] = useState<boolean>(false);
  const [engineStatus, setEngineStatus] = useState<EngineStatus | null>(null);

  const [leftPane, setLeftPane] = useState<PaneState>(() => ({
    tabs: [createInitialTab("L-1", "C:\\")],
    activeTabId: "L-1",
    viewMode: "list",
    listing: null,
    selectedPaths: new Set(),
  }));

  const [rightPane, setRightPane] = useState<PaneState>(() => ({
    tabs: [createInitialTab("R-1", "C:\\")],
    activeTabId: "R-1",
    viewMode: "grid",
    listing: null,
    selectedPaths: new Set(),
  }));

  const [searchModalOpen, setSearchModalOpen] = useState(false);
  const [quickViewData, setQuickViewData] = useState<{
    title: string;
    content: string;
  } | null>(null);

  const workspaceRef = useRef<HTMLDivElement>(null);

  const fetchDirectory = useCallback(
    async (pane: "left" | "right", targetPath: string, pushHistory = true) => {
      try {
        const listing = await invoke<DirectoryListing>("read_vfs_directory", {
          path: targetPath,
        });
        const cleanTitle =
          listing.currentPath.replace(/\\$/, "").split("\\").pop() ||
          listing.currentPath;

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

  useEffect(() => {
    (async () => {
      try {
        const drvList = await invoke<DriveInfo[]>("list_drives");
        setDrives(drvList);
        const defaultRoot = drvList[0]?.mountPoint || "C:\\";
        await fetchDirectory("left", defaultRoot, false);
        await fetchDirectory("right", defaultRoot, false);

        const status = await invoke<EngineStatus>("get_engine_status");
        setEngineStatus(status);
      } catch (e) {
        console.error("Init error:", e);
      }
    })();
  }, [fetchDirectory]);

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
  const targetPaneState = activePane === "left" ? rightPane : leftPane;
  const activeTab =
    currentPaneState.tabs.find((t) => t.id === currentPaneState.activeTabId) ||
    currentPaneState.tabs[0];

  const refreshBothPanes = useCallback(async () => {
    const leftPath = leftPane.listing?.currentPath || "C:\\";
    const rightPath = rightPane.listing?.currentPath || "C:\\";
    await fetchDirectory("left", leftPath, false);
    await fetchDirectory("right", rightPath, false);
  }, [leftPane.listing, rightPane.listing, fetchDirectory]);

  const handleViewF3 = useCallback(async () => {
    const selected = Array.from(currentPaneState.selectedPaths);
    if (selected.length === 0) {
      if (engineStatus) {
        setQuickViewData({
          title: "Stato Motori KZip (FFI & VFS)",
          content: JSON.stringify(engineStatus, null, 2),
        });
      }
      return;
    }
    try {
      const text = await invoke<string>("read_file_preview", {
        path: selected[0],
      });
      setQuickViewData({ title: selected[0], content: text });
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.selectedPaths, engineStatus]);

  const handleCopyF5 = useCallback(async () => {
    const sources = Array.from(currentPaneState.selectedPaths);
    const destDir = targetPaneState.listing?.currentPath;
    if (sources.length === 0 || !destDir) return;
    try {
      await invoke("copy_items", { sources, destinationDir: destDir });
      await refreshBothPanes();
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.selectedPaths, targetPaneState.listing, refreshBothPanes]);

  const handleMoveF6 = useCallback(async () => {
    const sources = Array.from(currentPaneState.selectedPaths);
    const destDir = targetPaneState.listing?.currentPath;
    if (sources.length === 0 || !destDir) return;
    try {
      await invoke("move_items", { sources, destinationDir: destDir });
      await refreshBothPanes();
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.selectedPaths, targetPaneState.listing, refreshBothPanes]);

  const handleNewFolderF7 = useCallback(async () => {
    const parentPath = currentPaneState.listing?.currentPath;
    if (!parentPath) return;
    const folderName = window.prompt("Nome della nuova cartella:", "New Folder");
    if (!folderName) return;
    try {
      await invoke("create_directory", { parentPath, folderName });
      await fetchDirectory(activePane, parentPath, false);
    } catch (err) {
      alert(String(err));
    }
  }, [currentPaneState.listing, activePane, fetchDirectory]);

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

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F3") {
        e.preventDefault();
        handleViewF3();
      } else if (e.key === "F4") {
        e.preventDefault();
        handleViewF3();
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
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setSearchModalOpen(true);
      } else if (e.key === "Escape") {
        setSearchModalOpen(false);
        setQuickViewData(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    handleViewF3,
    handleCopyF5,
    handleMoveF6,
    handleNewFolderF7,
    handleDeleteF8,
  ]);

  const handleHistoryStep = (delta: -1 | 1) => {
    if (!activeTab) return;
    const nextIdx = activeTab.historyIndex + delta;
    if (nextIdx < 0 || nextIdx >= activeTab.history.length) return;
    const targetPath = activeTab.history[nextIdx];

    const setState = activePane === "left" ? setLeftPane : setRightPane;
    setState((prev) => ({
      ...prev,
      tabs: prev.tabs.map((t) =>
        t.id === prev.activeTabId ? { ...t, historyIndex: nextIdx } : t
      ),
    }));
    fetchDirectory(activePane, targetPath, false);
  };

  const handleOpenEntry = (pane: "left" | "right", entry: VfsEntry) => {
    if (entry.isDir || entry.isArchive) {
      fetchDirectory(pane, entry.path, true);
    } else {
      invoke<string>("read_file_preview", { path: entry.path })
        .then((content) => setQuickViewData({ title: entry.name, content }))
        .catch((err) => alert(String(err)));
    }
  };

  return (
    <div className="kzip-shell">
      <TopToolbar
        activeGlobalPath={currentPaneState.listing?.currentPath || "C:\\"}
        canGoBack={Boolean(activeTab && activeTab.historyIndex > 0)}
        canGoForward={Boolean(
          activeTab && activeTab.historyIndex < activeTab.history.length - 1
        )}
        onBack={() => handleHistoryStep(-1)}
        onForward={() => handleHistoryStep(1)}
        activeViewMode={currentPaneState.viewMode}
        onChangeViewMode={(mode) => {
          const setter = activePane === "left" ? setLeftPane : setRightPane;
          setter((prev) => ({ ...prev, viewMode: mode }));
        }}
        showHidden={showHidden}
        onToggleHidden={() => setShowHidden((v) => !v)}
        onOpenSearch={() => setSearchModalOpen(true)}
        onQuickView={handleViewF3}
        onNewFolder={handleNewFolderF7}
        onOpenTerminal={() => {
          const p = currentPaneState.listing?.currentPath || "C:\\";
          invoke("open_in_terminal", { path: p }).catch((err) =>
            alert(String(err))
          );
        }}
        onArchiveAction={() => {
          const sel = Array.from(currentPaneState.selectedPaths)[0];
          if (sel) {
            fetchDirectory(activePane, `archive://${sel}!/`, true);
          } else {
            alert(
              `Motore Archivi COM KZip.Core.dll: ${
                engineStatus?.archiveDllLoaded
                  ? `Caricato (${engineStatus.archiveFormatsCount} formati registrati)`
                  : "In attesa di KZip.Core.dll nella cartella binaries/"
              }\nSeleziona un archivio (.7z, .zip, .rar, .tar) per navigarlo nel VFS.`
            );
          }
        }}
        engineStatus={engineStatus}
      />

      <main ref={workspaceRef} className="dual-pane-workspace">
        <div style={{ width: `${splitRatio}%`, height: "100%" }}>
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
              const cur = leftPane.listing?.currentPath || "C:\\";
              const newTab = createInitialTab(id, cur);
              setLeftPane((p) => ({
                ...p,
                tabs: [...p.tabs, newTab],
                activeTabId: id,
              }));
            }}
            onCloseTab={(tabId) => {
              setLeftPane((p) => {
                const filtered = p.tabs.filter((t) => t.id !== tabId);
                const nextActive =
                  p.activeTabId === tabId ? filtered[0].id : p.activeTabId;
                return { ...p, tabs: filtered, activeTabId: nextActive };
              });
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
          />
        </div>

        <div
          className={`pane-splitter ${isDraggingSplitter ? "dragging" : ""}`}
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
              const cur = rightPane.listing?.currentPath || "C:\\";
              const newTab = createInitialTab(id, cur);
              setRightPane((p) => ({
                ...p,
                tabs: [...p.tabs, newTab],
                activeTabId: id,
              }));
            }}
            onCloseTab={(tabId) => {
              setRightPane((p) => {
                const filtered = p.tabs.filter((t) => t.id !== tabId);
                const nextActive =
                  p.activeTabId === tabId ? filtered[0].id : p.activeTabId;
                return { ...p, tabs: filtered, activeTabId: nextActive };
              });
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
          />
        </div>
      </main>

      <BottomBar
        activePath={currentPaneState.listing?.currentPath || "C:\\"}
        onExecutePathOrCommand={(input) =>
          fetchDirectory(activePane, input, true)
        }
        onViewF3={handleViewF3}
        onEditF4={handleViewF3}
        onCopyF5={handleCopyF5}
        onMoveF6={handleMoveF6}
        onNewFolderF7={handleNewFolderF7}
        onDeleteF8={handleDeleteF8}
      />

      {searchModalOpen && (
        <SearchModal
          currentFolder={currentPaneState.listing?.currentPath || "C:\\"}
          engineStatus={engineStatus}
          onClose={() => setSearchModalOpen(false)}
          onRunSearch={async (params: SearchQuery) => {
            try {
              return await invoke<SearchResponse>("execute_search", { params });
            } catch (err) {
              alert(String(err));
              return null;
            }
          }}
          onMountSearchToPane={(vfsUri) =>
            fetchDirectory(activePane, vfsUri, true)
          }
        />
      )}

      {quickViewData && (
        <QuickViewModal
          title={quickViewData.title}
          content={quickViewData.content}
          onClose={() => setQuickViewData(null)}
        />
      )}
    </div>
  );
};
