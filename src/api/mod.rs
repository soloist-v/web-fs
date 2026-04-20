//! API 模块入口
//!
//! 负责将所有文件系统 API 处理器注册到 axum [`Router`] 上，
//! 并通过 `router()` 函数暴露给 `main.rs` 挂载。
//!
//! 路由表：
//! ```
//! GET    /api/roots    → list_roots
//! GET    /api/files    → list_dir
//! GET    /api/file     → get_file_content
//! GET    /api/download → download_file
//! POST   /api/upload   → upload_files
//! POST   /api/mkdir    → create_dir
//! DELETE /api/files    → delete_entry
//! PUT    /api/rename   → rename_entry
//! POST   /api/copy     → copy_entry
//! GET    /api/info     → get_file_info
//! GET    /api/search   → search_files
//! ```

pub mod error;
pub mod files;

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use files::{
    copy_entry, create_dir, delete_entry, download_file, get_file_content, get_file_info,
    list_dir, list_roots, rename_entry, search_files, upload_files,
};

/// 构建并返回所有 `/api/*` 路由
///
/// 返回的 [`Router`] 会被 `main.rs` 以 `.nest("/api", api::router())` 的方式挂载。
pub fn router() -> Router {
    Router::new()
        // ── 已配置的 root 列表 ─────────────────────────────────────────────────
        // GET /api/roots
        .route("/roots", get(list_roots))
        // ── 目录列表 ──────────────────────────────────────────────────────────
        // GET /api/files?path=/&sort=name&order=asc&show_hidden=false
        .route("/files", get(list_dir))
        // ── 文件内容（内联预览，最大 10 MiB）──────────────────────────────────
        // GET /api/file?path=/foo/bar.txt
        .route("/file", get(get_file_content))
        // ── 文件下载（流式传输，无大小限制）──────────────────────────────────
        // GET /api/download?path=/foo/bar.txt
        .route("/download", get(download_file))
        // ── 文件上传（multipart/form-data，支持批量）─────────────────────────
        // POST /api/upload?path=/target/dir
        .route("/upload", post(upload_files))
        // ── 创建目录（mkdir -p 语义）──────────────────────────────────────────
        // POST /api/mkdir  body: { "path": "/new/dir" }
        .route("/mkdir", post(create_dir))
        // ── 删除文件或目录（目录递归删除）────────────────────────────────────
        // DELETE /api/files?path=/foo/bar
        .route("/files", delete(delete_entry))
        // ── 重命名 / 移动 ─────────────────────────────────────────────────────
        // PUT /api/rename  body: { "from": "/old", "to": "/new" }
        .route("/rename", put(rename_entry))
        // ── 复制文件或目录（目录递归复制）────────────────────────────────────
        // POST /api/copy  body: { "from": "/src", "to": "/dst" }
        .route("/copy", post(copy_entry))
        // ── 单条目元信息 ──────────────────────────────────────────────────────
        // GET /api/info?path=/foo/bar.txt
        .route("/info", get(get_file_info))
        // ── 递归文件名搜索 ────────────────────────────────────────────────────
        // GET /api/search?path=/&q=keyword&limit=200
        .route("/search", get(search_files))
}
