// ============================================================
// Roots
// ============================================================

/** A configured accessible root returned by GET /api/roots */
export interface RootInfo {
  /** Display name, also the first segment of the virtual path */
  name: string;
  /** Virtual path, always "/{name}" */
  virtual_path: string;
  /** Real path on the server (informational only) */
  real_path: string;
}

/** Response body for GET /api/roots */
export interface RootsResponse {
  roots: RootInfo[];
}

// ============================================================
// File System Entries
// ============================================================

/** A single file or directory entry returned by the API */
export interface FileEntry {
  /** Display name (basename only) */
  name: string;
  /** Absolute server-side path, always starts with "/" */
  path: string;
  /** True when the entry is a directory */
  is_dir: boolean;
  /** Raw size in bytes (0 for directories) */
  size: number;
  /** Human-readable size string, e.g. "1.23 MB" */
  size_human: string;
  /** ISO-8601 last-modified timestamp, or null if unavailable */
  modified: string | null;
  /** MIME type, e.g. "image/png", or null for directories */
  mime_type: string | null;
  /** Lowercase file extension without the dot, e.g. "rs", or null */
  extension: string | null;
  /** True when the entry is a symbolic link */
  is_symlink: boolean;
  /** Unix permission string, e.g. "rwxr-xr-x", or null on Windows */
  permissions: string | null;
  /** True when the entry cannot be written to */
  readonly: boolean;
}

/** Directory listing returned by GET /api/files */
export interface DirListing {
  /** Virtual path of the listed directory */
  path: string;
  /** All entries inside the directory */
  entries: FileEntry[];
  /** Parent virtual path, or null when at the virtual root "/" */
  parent: string | null;
  /** Total number of entries */
  total: number;
}

/** Extended file metadata returned by GET /api/info */
export interface FileInfo extends FileEntry {
  /** Full absolute path on the server */
  absolute_path: string;
  /** Creation time as ISO-8601, or null if unavailable */
  created: string | null;
  /** Last-accessed time as ISO-8601, or null if unavailable */
  accessed: string | null;
  /** Number of hard links pointing to this inode */
  hard_links: number | null;
  /** Inode number (Unix only) */
  inode: number | null;
  /** Device ID (Unix only) */
  device: number | null;
}

// ============================================================
// Search
// ============================================================

/** A single search result */
export interface SearchResult {
  /** The matching entry */
  entry: FileEntry;
  /** Relevance score (higher = better match) */
  score: number;
}

/** Response body for GET /api/search */
export interface SearchResponse {
  /** The query string that was searched */
  query: string;
  /** Root path where the search was performed */
  path: string;
  /** Matching results, sorted by relevance */
  results: SearchResult[];
  /** Total number of matches before any limit */
  total: number;
}

// ============================================================
// Upload
// ============================================================

/** Progress event emitted during file upload */
export interface UploadProgress {
  /** Name of the file currently being uploaded */
  filename: string;
  /** Index of the current file (0-based) */
  fileIndex: number;
  /** Total number of files in this upload batch */
  totalFiles: number;
  /** Bytes uploaded so far for the current file */
  loaded: number;
  /** Total bytes for the current file */
  total: number;
  /** Upload percentage for the current file (0–100) */
  percent: number;
}

/** Result of a single file upload */
export interface UploadResult {
  /** Original filename */
  filename: string;
  /** Server-side path where the file was saved */
  path: string;
  /** Whether the upload succeeded */
  success: boolean;
  /** Error message if success is false */
  error?: string;
}

// ============================================================
// API Error
// ============================================================

/** Error response body returned by the API */
export interface ApiError {
  /** Short machine-readable error code, e.g. "NOT_FOUND" */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Optional extra detail */
  detail?: string;
}

/** Thrown when the API returns a non-2xx response */
export class ApiException extends Error {
  public readonly status: number;
  public readonly code: string;
  public readonly detail?: string;

  constructor(status: number, error: ApiError) {
    super(error.message);
    this.name = "ApiException";
    this.status = status;
    this.code = error.code;
    this.detail = error.detail;
  }
}

// ============================================================
// UI State
// ============================================================

/** Column used for sorting the file list */
export type SortField = "name" | "size" | "modified" | "type";

/** Sort direction */
export type SortOrder = "asc" | "desc";

/** How the file list is rendered */
export type ViewMode = "list" | "grid";

/** Current sort state */
export interface SortState {
  field: SortField;
  order: SortOrder;
}

/** Application-level navigation breadcrumb */
export interface Breadcrumb {
  /** Display label */
  label: string;
  /** Absolute path this crumb links to */
  path: string;
}

/** State of a file-rename operation */
export interface RenameState {
  /** The entry being renamed */
  entry: FileEntry;
  /** Current value of the rename input */
  newName: string;
}

/** Clipboard entry for copy/move operations */
export interface ClipboardEntry {
  /** Source path */
  path: string;
  /** Whether this is a cut (move) or copy */
  operation: "copy" | "cut";
  /** The original entry metadata */
  entry: FileEntry;
}

/** State tracked per-entry during bulk operations */
export interface BulkOperation {
  /** Paths selected for the operation */
  paths: string[];
  /** Type of bulk operation */
  type: "delete" | "copy" | "move" | "download";
}

// ============================================================
// File Preview
// ============================================================

/** Categories of file content for preview rendering */
export type PreviewType =
  | "text" // plain text / source code
  | "image" // raster / vector image
  | "video" // video file
  | "audio" // audio file
  | "pdf" // PDF document
  | "archive" // zip / tar etc. (show metadata only)
  | "binary" // unrecognised binary (hex dump)
  | "directory" // directory (should not normally be previewed)
  | "unsupported";

/** Data passed to a file-preview panel */
export interface PreviewState {
  entry: FileEntry;
  type: PreviewType;
  /** Text content, set only when type === 'text' */
  textContent?: string;
  /** Object URL for media blobs, set for image / video / audio */
  objectUrl?: string;
  /** Whether the preview is still loading */
  loading: boolean;
  /** Error message if the preview failed to load */
  error?: string;
}

// ============================================================
// Settings
// ============================================================

/** Persisted user preferences */
export interface UserSettings {
  viewMode: ViewMode;
  sortField: SortField;
  sortOrder: SortOrder;
  /** Number of entries per page (list view) */
  pageSize: number;
  /** Show hidden files (names starting with ".") */
  showHidden: boolean;
  /** Confirm before deleting */
  confirmDelete: boolean;
}

export const DEFAULT_SETTINGS: UserSettings = {
  viewMode: "list",
  sortField: "name",
  sortOrder: "asc",
  pageSize: 100,
  showHidden: false,
  confirmDelete: true,
};
