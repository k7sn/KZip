export type ViewMode = "list" | "columns" | "grid";
export type LayoutMode = "single" | "dual";

export type SortField =
  | "name"
  | "path"
  | "extension"
  | "size"
  | "modified"
  | "created"
  | "kind";

export interface VfsEntry {
  name: string;
  stem: string;
  extension: string;
  path: string;
  parentDir: string;
  isDir: boolean;
  isParent: boolean;
  isArchive: boolean;
  size: number;
  modified: number;
  created: number;
  kind: string;
  attributes: number;
}

export interface DriveInfo {
  name: string;
  mountPoint: string;
  fileSystem: string;
  totalSpace: number;
  availableSpace: number;
  isRemovable: boolean;
}

export interface DirectoryListing {
  currentPath: string;
  parentPath: string | null;
  providerScheme: string;
  entries: VfsEntry[];
  totalFiles: number;
  totalDirs: number;
  totalBytes: number;
}

export interface PaneTab {
  id: string;
  title: string;
  path: string;
  history: string[];
  historyIndex: number;
}

export interface SearchQuery {
  query: string;
  scopePath: string | null;
  extensions: string | null;
  minSize: number | null;
  maxSize: number | null;
  useRegex: boolean;
  matchCase: boolean;
  maxResults: number;
}

export interface SearchResponse {
  engineUsed: string;
  elapsedMs: number;
  vfsUri: string;
  results: VfsEntry[];
}

export interface EngineStatus {
  everythingDllLoaded: boolean;
  everythingServiceConnected: boolean;
  everythingVersion: string | null;
  activeSearchEngine: string;
  archiveDllLoaded: boolean;
  archiveLoadedPath: string | null;
  archiveFormatsCount: number;
  archiveLoadDiagnostics: string[];
}

export interface CompressOptions {
  sources: string[];
  destinationArchive: string;
  format: string;
  compressionLevel: number;
  method: string;
  dictionarySize: string;
  wordSize: number;
  solidBlockSize: string;
  numThreads: number;
  splitVolumeSize: string | null;
  extraParameters: string | null;
  updateMode: string;
  pathMode: string;
  createSfx: boolean;
  compressSharedFiles: boolean;
  deleteAfterCompression: boolean;
  password: string | null;
  encryptionMethod: string;
  encryptFileNames: boolean;
}

export interface ExtractOptions {
  archivePath: string;
  destinationDir: string;
  overwriteMode: string;
  password: string | null;
}

export interface ProgressPayload {
  operation: string;
  archiveName: string;
  currentFile: string;
  processedBytes: number;
  totalBytes: number;
  percent: number;
  speedMbPerSec: number;
  elapsedSecs: number;
  etaSecs: number;
  status: "running" | "completed" | "cancelled" | "error";
  message: string | null;
}
