import type {
  DirListing,
  FileInfo,
  RootsResponse,
  SearchResponse,
  SortField,
  SortOrder,
  UploadProgress,
  UploadResult,
} from "./types";
import { ApiException } from "./types";

// ============================================================
// Base helpers
// ============================================================

const BASE = "/api";

/**
 * Build a URL string from a relative API path and optional query parameters.
 * Undefined / null values are omitted from the query string.
 */
function buildUrl(
  path: string,
  params?: Record<string, string | number | boolean | null | undefined>,
): string {
  const url = new URL(BASE + path, location.origin);
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== null) {
        url.searchParams.set(key, String(value));
      }
    }
  }
  return url.toString();
}

/**
 * Parse a fetch Response, throwing ApiException for non-2xx status codes.
 *
 * The backend returns errors as `{ "error": "<message>" }`.
 * We normalise that into our `ApiError` shape internally.
 */
async function parseResponse<T>(res: Response): Promise<T> {
  if (res.ok) {
    if (res.status === 204) return undefined as unknown as T;
    const ct = res.headers.get("content-type") ?? "";
    if (ct.includes("application/json")) {
      return res.json() as Promise<T>;
    }
    // For text endpoints (file content preview) return the raw text.
    return res.text() as unknown as Promise<T>;
  }

  // Extract error message from `{ "error": "..." }` body or status text.
  let message = res.statusText;
  const ct = res.headers.get("content-type") ?? "";
  if (ct.includes("application/json")) {
    try {
      const body = (await res.json()) as Record<string, unknown>;
      if (typeof body["error"] === "string") message = body["error"];
      else if (typeof body["message"] === "string") message = body["message"];
    } catch {
      // ignore
    }
  } else {
    const text = await res.text().catch(() => "");
    if (text) message = text;
  }

  throw new ApiException(res.status, {
    code: String(res.status),
    message,
  });
}

/**
 * Thin wrapper for JSON-body requests (POST / PUT / DELETE with body).
 */
async function request<T>(
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
  url: string,
  body?: unknown,
): Promise<T> {
  const res = await fetch(url, {
    method,
    headers:
      body !== undefined ? { "Content-Type": "application/json" } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  return parseResponse<T>(res);
}

// ============================================================
// Roots
//   GET /api/roots
// ============================================================

/**
 * Fetch the list of configured accessible roots.
 * Used by the frontend to know which top-level roots are available.
 */
export async function getRoots(): Promise<RootsResponse> {
  return request<RootsResponse>("GET", buildUrl("/roots"));
}

// ============================================================
// Directory listing
//   GET /api/files?path=<p>&sort=<f>&order=<o>&show_hidden=<b>
// ============================================================

/**
 * Fetch a directory listing.
 *
 * @param path       Absolute server-side path (e.g. "/home/user/docs")
 * @param sort       Column to sort by (default: "name")
 * @param order      Sort direction (default: "asc")
 * @param showHidden Include hidden entries whose names start with "." (default: false)
 */
export async function listDir(
  path: string,
  sort: SortField = "name",
  order: SortOrder = "asc",
  showHidden = false,
): Promise<DirListing> {
  const url = buildUrl("/files", {
    path,
    sort,
    order,
    show_hidden: showHidden,
  });
  return request<DirListing>("GET", url);
}

// ============================================================
// File content preview
//   GET /api/file?path=<p>
// ============================================================

/**
 * Fetch the text content of a file for in-browser preview.
 * The server limits the response to 10 MiB.
 *
 * @param path Absolute server-side path to the file
 * @returns    Raw text content of the file
 */
export async function getFileContent(path: string): Promise<string> {
  const url = buildUrl("/file", { path });
  const res = await fetch(url);
  if (res.ok) return res.text();
  return parseResponse<string>(res); // throws ApiException
}

// ============================================================
// File download
//   GET /api/download?path=<p>
// ============================================================

/**
 * Trigger a browser file download for the given server-side path.
 * Navigates the browser to the download endpoint; the server sets
 * Content-Disposition: attachment so the browser saves the file.
 *
 * @param path Absolute server-side path to the file
 */
export function downloadFile(path: string): void {
  const url = buildUrl("/download", { path });
  const a = document.createElement("a");
  a.href = url;
  a.setAttribute("download", "");
  a.style.display = "none";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
}

/**
 * Fetch the raw bytes of a file as a Blob.
 * Useful for creating an object URL for media preview.
 *
 * @param path Absolute server-side path to the file
 */
export async function fetchFileBlob(path: string): Promise<Blob> {
  const url = buildUrl("/download", { path });
  const res = await fetch(url);
  if (res.ok) return res.blob();
  return parseResponse<Blob>(res); // throws ApiException
}

// ============================================================
// File upload
//   POST /api/upload?path=<dir>   multipart/form-data
// ============================================================

/**
 * Upload one or more files into the given directory.
 *
 * Uses XMLHttpRequest so we can report per-file upload progress.
 * Each file is sent as a separate multipart/form-data request.
 *
 * @param dirPath    Absolute destination directory path
 * @param files      File objects from an <input type="file"> or drag-drop
 * @param onProgress Optional progress callback, called during each upload
 * @returns          Array of results in the same order as `files`
 */
export async function uploadFiles(
  dirPath: string,
  files: File[],
  onProgress?: (progress: UploadProgress) => void,
): Promise<UploadResult[]> {
  const url = buildUrl("/upload", { path: dirPath });
  const results: UploadResult[] = [];
  for (let i = 0; i < files.length; i++) {
    results.push(
      await uploadSingleFile(url, files[i], i, files.length, onProgress),
    );
  }
  return results;
}

function uploadSingleFile(
  url: string,
  file: File,
  fileIndex: number,
  totalFiles: number,
  onProgress?: (p: UploadProgress) => void,
): Promise<UploadResult> {
  return new Promise((resolve) => {
    const xhr = new XMLHttpRequest();
    const form = new FormData();
    form.append("file", file, file.name);

    xhr.upload.addEventListener("progress", (evt) => {
      if (!evt.lengthComputable || !onProgress) return;
      onProgress({
        filename: file.name,
        fileIndex,
        totalFiles,
        loaded: evt.loaded,
        total: evt.total,
        percent: Math.round((evt.loaded / evt.total) * 100),
      });
    });

    xhr.addEventListener("load", () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        let path = "";
        try {
          const body = JSON.parse(xhr.responseText) as { path?: string };
          path = body.path ?? "";
        } catch {
          /* ignore */
        }
        resolve({ filename: file.name, path, success: true });
      } else {
        let message = `HTTP ${xhr.status}`;
        try {
          const body = JSON.parse(xhr.responseText) as {
            error?: string;
            message?: string;
          };
          message = body.error ?? body.message ?? message;
        } catch {
          /* ignore */
        }
        resolve({
          filename: file.name,
          path: "",
          success: false,
          error: message,
        });
      }
    });

    xhr.addEventListener("error", () =>
      resolve({
        filename: file.name,
        path: "",
        success: false,
        error: "Network error",
      }),
    );
    xhr.addEventListener("abort", () =>
      resolve({
        filename: file.name,
        path: "",
        success: false,
        error: "Upload aborted",
      }),
    );

    xhr.open("POST", url);
    xhr.send(form);
  });
}

// ============================================================
// Create directory
//   POST /api/mkdir   body: { path }
// ============================================================

/**
 * Create a new directory (with all intermediate parents) at the given path.
 *
 * @param path Absolute server-side path for the new directory
 */
export async function createDir(path: string): Promise<void> {
  return request<void>("POST", buildUrl("/mkdir"), { path });
}

// ============================================================
// Delete
//   DELETE /api/files?path=<p>
// ============================================================

/**
 * Delete a file or directory (directories are deleted recursively).
 * Deleting the root is forbidden by the server.
 *
 * @param path Absolute server-side path to delete
 */
export async function deleteEntry(path: string): Promise<void> {
  const url = buildUrl("/files", { path });
  const res = await fetch(url, { method: "DELETE" });
  return parseResponse<void>(res);
}

// ============================================================
// Rename / Move
//   PUT /api/rename   body: { from, to }
// ============================================================

/**
 * Rename or move a file/directory on the server.
 * Moving across file-system boundaries is supported (copy + delete fallback).
 *
 * @param from Absolute source path
 * @param to   Absolute destination path (may be in a different directory)
 */
export async function renameEntry(from: string, to: string): Promise<void> {
  return request<void>("PUT", buildUrl("/rename"), { from, to });
}

// ============================================================
// Copy
//   POST /api/copy   body: { from, to }
// ============================================================

/**
 * Copy a file or directory to a new location.
 * Directories are copied recursively.
 *
 * @param from Absolute source path
 * @param to   Absolute destination path
 */
export async function copyEntry(from: string, to: string): Promise<void> {
  return request<void>("POST", buildUrl("/copy"), { from, to });
}

// ============================================================
// File info
//   GET /api/info?path=<p>
// ============================================================

/**
 * Retrieve extended metadata for a single file or directory.
 *
 * @param path Absolute server-side path
 */
export async function getFileInfo(path: string): Promise<FileInfo> {
  return request<FileInfo>("GET", buildUrl("/info", { path }));
}

// ============================================================
// Search
//   GET /api/search?path=<p>&q=<query>&limit=<n>
// ============================================================

/**
 * Recursively search for files and directories under a root path.
 *
 * @param path  Absolute server-side root to search within
 * @param query Search query string (matched against file names)
 * @param limit Maximum number of results to return (default: 200)
 */
export async function searchFiles(
  path: string,
  query: string,
  limit = 200,
): Promise<SearchResponse> {
  // Backend query parameter is `q`, not `query`
  return request<SearchResponse>(
    "GET",
    buildUrl("/search", { path, q: query, limit }),
  );
}

// ============================================================
// Re-exports
// ============================================================
export { ApiException } from "./types";
