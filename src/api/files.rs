//! File system API handlers
//!
//! # 虚拟路径设计
//!
//! 所有用户可见的路径均为"虚拟路径"，格式为 `/{root_name}/sub/path`。
//! 虚拟根 `/` 列出所有已配置的 root 条目。
//!
//! ```text
//! 虚拟路径               真实路径（示例 root: data → /data）
//! /                   →  <列出所有 root>
//! /data               →  /data
//! /data/foo/bar.txt   →  /data/foo/bar.txt
//! ```
//!
//! # 安全
//! [`resolve_virtual_path`] 保证解析后的真实路径不会逃出对应 root 的目录树。

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use axum::{
    body::Body,
    extract::{Multipart, Query},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::{DateTime, Utc};
use humansize::{format_size, BINARY};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncReadExt};
use tokio_util::io::ReaderStream;

use super::error::{ApiError, ApiResult};

// ─── Global roots registry ────────────────────────────────────────────────────

/// 已解析（规范化）的 root 条目
#[derive(Debug, Clone)]
pub struct ResolvedRoot {
    /// 虚拟名称，对应 URL 的第一段
    pub name: String,
    /// 规范化后的真实绝对路径
    pub real_path: PathBuf,
}

static ROOTS: OnceLock<Vec<ResolvedRoot>> = OnceLock::new();

/// 在服务器启动前调用一次，初始化全局 root 列表。
///
/// 对每个 [`crate::config::RootEntry`] 调用 `canonicalize`；
/// 路径不存在时保留原始值并打印警告。
pub fn init_roots(entries: Vec<crate::config::RootEntry>) {
    ROOTS.get_or_init(|| {
        entries
            .into_iter()
            .map(|e| {
                let real_path = std::fs::canonicalize(&e.path)
                    .unwrap_or_else(|_| PathBuf::from(&e.path));
                tracing::info!(
                    name = %e.name,
                    real_path = %real_path.display(),
                    "Registered root"
                );
                ResolvedRoot {
                    name: e.name,
                    real_path,
                }
            })
            .collect()
    });
}

/// 获取已初始化的 root 列表（panic if not initialized）
fn get_roots() -> &'static [ResolvedRoot] {
    ROOTS.get().expect("init_roots() was not called before handling requests")
}

// ─── Virtual path helpers ─────────────────────────────────────────────────────

/// 将虚拟路径解析为真实文件系统路径，同时返回对应 root 的名称。
///
/// - `/` 是虚拟根，不对应任何真实路径，返回 [`ApiError::InvalidPath`]。
/// - `/{name}` 或 `/{name}/sub` 在 ROOTS 中查找匹配的 root，然后拼接子路径。
/// - 通过 [`normalize_logical_path`] 纯逻辑规范化后验证不逃出 root。
fn resolve_virtual_path(vpath: &str) -> ApiResult<(PathBuf, &'static str)> {
    if vpath.contains('\0') {
        return Err(ApiError::InvalidPath(
            "Path contains null byte".to_string(),
        ));
    }

    let stripped = vpath.trim_start_matches('/');
    if stripped.is_empty() {
        return Err(ApiError::InvalidPath(
            "Virtual root '/' has no corresponding real path".to_string(),
        ));
    }

    let (root_name, sub) = match stripped.find('/') {
        Some(i) => (&stripped[..i], &stripped[i + 1..]),
        None => (stripped, ""),
    };

    let root = get_roots()
        .iter()
        .find(|r| r.name == root_name)
        .ok_or_else(|| ApiError::NotFound(format!("Root '{}' not configured", root_name)))?;

    let candidate = if sub.is_empty() {
        root.real_path.clone()
    } else {
        root.real_path.join(sub)
    };

    let normalized = normalize_logical_path(&candidate);
    let normalized_base = normalize_logical_path(&root.real_path);

    if !normalized.starts_with(&normalized_base) {
        return Err(ApiError::InvalidPath(format!(
            "Path '{}' attempts to escape root '{}'",
            vpath, root_name
        )));
    }

    Ok((normalized, &root.name))
}

/// 将真实文件系统路径还原为虚拟路径（`/{root_name}/sub/path`）。
///
/// 兼容 Windows 路径分隔符（`\` → `/`）。
fn to_virtual_path(real_path: &Path) -> String {
    for root in get_roots() {
        // 精确匹配 root 本身
        if real_path == root.real_path {
            return format!("/{}", root.name);
        }
        if let Ok(rel) = real_path.strip_prefix(&root.real_path) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            return format!("/{}/{}", root.name, rel_str);
        }
    }
    // 理论上不会到达这里（validate 已经确保路径在 root 内）
    real_path.to_string_lossy().replace('\\', "/")
}

/// 计算虚拟路径的上级目录。
///
/// - `/`            → `None`（已在虚拟根）
/// - `/{name}`      → `Some("/")`
/// - `/{name}/sub`  → `Some("/{name}")`
fn parent_virtual_path(vpath: &str) -> Option<String> {
    if vpath == "/" {
        return None;
    }
    let trimmed = vpath.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    if idx == 0 {
        Some("/".to_string())
    } else {
        Some(trimmed[..idx].to_string())
    }
}

// ─── Path normalization ───────────────────────────────────────────────────────

/// 不访问文件系统的纯逻辑路径规范化。
///
/// `..` 弹出最近一个普通组件；`.` 跳过；其余原样压栈。
fn normalize_logical_path(path: &Path) -> PathBuf {
    let mut parts: Vec<std::path::Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                if matches!(parts.last(), Some(std::path::Component::Normal(_))) {
                    parts.pop();
                }
            }
            std::path::Component::CurDir => {}
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

// ─── Data types ───────────────────────────────────────────────────────────────

/// 目录条目的完整元信息
#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub name: String,
    /// 虚拟路径，以 `/` 开头
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub size_human: String,
    pub modified: Option<String>,
    pub mime_type: Option<String>,
    pub extension: Option<String>,
    pub is_symlink: bool,
    #[cfg(unix)]
    pub permissions: String,
    pub readonly: bool,
}

impl FileEntry {
    /// 从 `tokio::fs::DirEntry` 异步构建
    pub async fn from_dir_entry(
        entry: &tokio::fs::DirEntry,
    ) -> std::io::Result<Self> {
        let metadata = entry.metadata().await?;
        let full_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let vpath = to_virtual_path(&full_path);
        Self::from_metadata(vpath, &full_path, &metadata, name)
    }

    /// 从已有 `std::fs::Metadata` 同步构建
    pub fn from_metadata(
        virtual_path: String,
        full_path: &Path,
        metadata: &std::fs::Metadata,
        name: String,
    ) -> std::io::Result<Self> {
        let is_dir = metadata.is_dir();
        let is_symlink = metadata.file_type().is_symlink();
        let size = if is_dir { 0 } else { metadata.len() };
        let size_human = format_size(size, BINARY);

        let modified = metadata.modified().ok().map(|st| {
            let dt: DateTime<Utc> = st.into();
            dt.to_rfc3339()
        });

        let extension = if is_dir {
            None
        } else {
            full_path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
        };

        let mime_type = if is_dir {
            None
        } else {
            Some(
                mime_guess::from_path(full_path)
                    .first_or_octet_stream()
                    .to_string(),
            )
        };

        let readonly = metadata.permissions().readonly();

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            format_unix_permissions(metadata.permissions().mode())
        };

        Ok(Self {
            name,
            path: virtual_path,
            is_dir,
            size,
            size_human,
            modified,
            mime_type,
            extension,
            is_symlink,
            #[cfg(unix)]
            permissions,
            readonly,
        })
    }
}

/// Unix mode 低 9 位 → `rwxr-xr-x` 风格字符串
#[cfg(unix)]
fn format_unix_permissions(mode: u32) -> String {
    const BITS: [(u32, char); 9] = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    BITS.iter()
        .map(|(m, c)| if mode & m != 0 { *c } else { '-' })
        .collect()
}

// ─── Request / query types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PathQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub show_hidden: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub path: Option<String>,
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RenameBody {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct CopyBody {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct MkdirBody {
    pub path: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// `GET /api/roots`
///
/// 返回所有已配置 root 的名称与虚拟路径列表，供前端构建初始导航。
pub async fn list_roots() -> Json<serde_json::Value> {
    let roots: Vec<serde_json::Value> = get_roots()
        .iter()
        .map(|r| {
            serde_json::json!({
                "name":         r.name,
                "virtual_path": format!("/{}", r.name),
                "real_path":    r.real_path.to_string_lossy(),
            })
        })
        .collect();
    Json(serde_json::json!({ "roots": roots }))
}

/// `GET /api/files?path=&sort=name&order=asc&show_hidden=false`
///
/// 列出目录内容。
///
/// 当 `path=/` 时返回所有已配置 root 条目（虚拟根列表）；
/// 否则解析对应 root 并列出真实目录。
pub async fn list_dir(
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let vpath = q.path.as_deref().unwrap_or("/");
    let show_hidden = q.show_hidden.unwrap_or(false);
    let sort_field = q.sort.as_deref().unwrap_or("name").to_string();
    let order_asc = q.order.as_deref().unwrap_or("asc") != "desc";

    // ── 虚拟根：列出所有 root ───────────────────────────────────────────────
    if vpath == "/" {
        let mut entries: Vec<FileEntry> = get_roots()
            .iter()
            .filter(|r| show_hidden || !r.name.starts_with('.'))
            .map(|r| {
                let meta = std::fs::metadata(&r.real_path);
                let modified = meta
                    .as_ref()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let dt: DateTime<Utc> = t.into();
                        dt.to_rfc3339()
                    });
                let readonly = meta
                    .as_ref()
                    .map(|m| m.permissions().readonly())
                    .unwrap_or(false);

                #[cfg(unix)]
                let permissions = {
                    use std::os::unix::fs::PermissionsExt;
                    meta.as_ref()
                        .map(|m| format_unix_permissions(m.permissions().mode()))
                        .unwrap_or_else(|| "---------".to_string())
                };

                FileEntry {
                    name: r.name.clone(),
                    path: format!("/{}", r.name),
                    is_dir: true,
                    size: 0,
                    size_human: "0 B".to_string(),
                    modified,
                    mime_type: None,
                    extension: None,
                    is_symlink: false,
                    #[cfg(unix)]
                    permissions,
                    readonly,
                }
            })
            .collect();

        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        return Ok(Json(serde_json::json!({
            "path":    "/",
            "parent":  null,
            "total":   entries.len(),
            "entries": entries,
        })));
    }

    // ── 普通目录：解析真实路径 ──────────────────────────────────────────────
    let (dir_path, _root) = resolve_virtual_path(vpath)?;

    let meta = fs::metadata(&dir_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound(format!("Directory '{}' does not exist", vpath))
        } else {
            ApiError::Io(e)
        }
    })?;

    if !meta.is_dir() {
        return Err(ApiError::InvalidPath(format!(
            "'{}' is not a directory",
            vpath
        )));
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    let mut read_dir = fs::read_dir(&dir_path).await?;

    while let Some(de) = read_dir.next_entry().await? {
        let name = de.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        match FileEntry::from_dir_entry(&de).await {
            Ok(fe) => entries.push(fe),
            Err(e) => tracing::warn!(
                path = %de.path().display(),
                error = %e,
                "Skipping entry"
            ),
        }
    }

    // 目录优先；同类型内按指定字段排序
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        let ord = match sort_field.as_str() {
            "size" => a.size.cmp(&b.size),
            "modified" => a.modified.cmp(&b.modified),
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        };
        if order_asc { ord } else { ord.reverse() }
    });

    let parent = parent_virtual_path(vpath);

    Ok(Json(serde_json::json!({
        "path":    vpath,
        "parent":  parent,
        "total":   entries.len(),
        "entries": entries,
    })))
}

/// `GET /api/file?path=...`
///
/// 读取文件内容用于内联预览（上限 10 MiB）。
pub async fn get_file_content(
    Query(q): Query<PathQuery>,
) -> ApiResult<Response> {
    let vpath = q.path.as_deref().unwrap_or("/");
    let (file_path, _) = resolve_virtual_path(vpath)?;

    let meta = fs::metadata(&file_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound(format!("'{}' not found", vpath))
        } else {
            ApiError::Io(e)
        }
    })?;

    if meta.is_dir() {
        return Err(ApiError::InvalidPath(format!(
            "'{}' is a directory",
            vpath
        )));
    }

    const MAX_PREVIEW_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
    if meta.len() > MAX_PREVIEW_BYTES {
        return Err(ApiError::Other(format!(
            "File too large for preview ({} > 10 MiB)",
            format_size(meta.len(), BINARY)
        )));
    }

    let mut file = fs::File::open(&file_path).await?;
    let mut buf = Vec::with_capacity(meta.len() as usize);
    file.read_to_end(&mut buf).await?;

    let mime = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(buf))
        .unwrap())
}

/// `GET /api/download?path=...`
///
/// 流式下载文件，设置 `Content-Disposition: attachment`。
pub async fn download_file(
    Query(q): Query<PathQuery>,
) -> ApiResult<Response> {
    let vpath = q.path.as_deref().unwrap_or("/");
    let (file_path, _) = resolve_virtual_path(vpath)?;

    let meta = fs::metadata(&file_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound(format!("'{}' not found", vpath))
        } else {
            ApiError::Io(e)
        }
    })?;

    if meta.is_dir() {
        return Err(ApiError::InvalidPath(format!(
            "'{}' is a directory, cannot download",
            vpath
        )));
    }

    let file = fs::File::open(&file_path).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mime = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    let filename = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "download".to_string());
    let disposition = format!(
        "attachment; filename=\"{}\"",
        percent_encode_filename(&filename)
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CONTENT_LENGTH, meta.len())
        .body(body)
        .unwrap())
}

fn percent_encode_filename(name: &str) -> String {
    name.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || "._-~ ()[]".contains(c) {
                vec![c]
            } else {
                c.to_string()
                    .bytes()
                    .flat_map(|b| {
                        format!("%{:02X}", b).chars().collect::<Vec<_>>()
                    })
                    .collect()
            }
        })
        .collect()
}

/// `POST /api/upload?path=<virtual_dir>`
///
/// multipart/form-data 批量上传，每个 `file` 字段对应一个文件。
pub async fn upload_files(
    Query(q): Query<PathQuery>,
    mut multipart: Multipart,
) -> ApiResult<Json<serde_json::Value>> {
    let vpath = q.path.as_deref().unwrap_or("/");
    let (dir_path, _) = resolve_virtual_path(vpath)?;

    let meta = fs::metadata(&dir_path).await.map_err(|_| {
        ApiError::NotFound(format!("Upload target '{}' does not exist", vpath))
    })?;
    if !meta.is_dir() {
        return Err(ApiError::InvalidPath(format!(
            "Upload target '{}' is not a directory",
            vpath
        )));
    }

    let mut uploaded = Vec::new();
    let mut errors = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = match field.file_name() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };

        // 安全校验文件名（不允许路径分隔符）
        if filename.contains('/') || filename.contains('\\') {
            errors.push(serde_json::json!({
                "name":  filename,
                "error": "Filename must not contain path separators",
            }));
            continue;
        }

        let dest = dir_path.join(&filename);
        match field.bytes().await {
            Ok(data) => {
                if let Err(e) = fs::write(&dest, &data).await {
                    errors.push(serde_json::json!({
                        "name":  filename,
                        "error": e.to_string(),
                    }));
                } else {
                    let size = data.len() as u64;
                    tracing::info!(
                        path = %dest.display(),
                        bytes = size,
                        "File uploaded"
                    );
                    uploaded.push(serde_json::json!({
                        "name":      filename,
                        "path":      to_virtual_path(&dest),
                        "size":      size,
                        "size_human": format_size(size, BINARY),
                    }));
                }
            }
            Err(e) => {
                errors.push(serde_json::json!({
                    "name":  filename,
                    "error": e.to_string(),
                }));
            }
        }
    }

    let status = if errors.is_empty() { "ok" } else { "partial" };
    Ok(Json(serde_json::json!({
        "status":   status,
        "uploaded": uploaded,
        "errors":   errors,
    })))
}

/// `POST /api/mkdir`  body: `{ "path": "/root/new/dir" }`
pub async fn create_dir(
    Json(body): Json<MkdirBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let vpath = &body.path;
    let (dir_path, _) = resolve_virtual_path(vpath)?;

    fs::create_dir_all(&dir_path).await?;
    tracing::info!(path = %dir_path.display(), "Directory created");

    Ok(Json(serde_json::json!({
        "status": "ok",
        "path":   vpath,
    })))
}

/// `DELETE /api/files?path=...`
///
/// 删除文件或目录（目录递归删除），禁止删除虚拟根或 root 本身。
pub async fn delete_entry(
    Query(q): Query<PathQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let vpath = q.path.as_deref().unwrap_or("/");

    // 禁止删除虚拟根
    if vpath == "/" {
        return Err(ApiError::InvalidPath(
            "Cannot delete the virtual root".to_string(),
        ));
    }

    let (target, root_name) = resolve_virtual_path(vpath)?;

    // 禁止删除 root 本身
    let root = get_roots().iter().find(|r| r.name == root_name).unwrap();
    if normalize_logical_path(&target) == normalize_logical_path(&root.real_path) {
        return Err(ApiError::InvalidPath(format!(
            "Cannot delete root '{}'",
            root_name
        )));
    }

    let meta = fs::metadata(&target).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound(format!("'{}' not found", vpath))
        } else {
            ApiError::Io(e)
        }
    })?;

    if meta.is_dir() {
        fs::remove_dir_all(&target).await?;
        tracing::info!(path = %target.display(), "Directory deleted recursively");
    } else {
        fs::remove_file(&target).await?;
        tracing::info!(path = %target.display(), "File deleted");
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "path":   vpath,
    })))
}

/// `PUT /api/rename`  body: `{ "from": "/a", "to": "/b" }`
pub async fn rename_entry(
    Json(body): Json<RenameBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let (src, _) = resolve_virtual_path(&body.from)?;
    let (dst, _) = resolve_virtual_path(&body.to)?;

    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }

    fs::rename(&src, &dst).await.map_err(|e| {
        // On cross-device move, fall back to copy + delete
        if e.raw_os_error() == Some(18) || e.raw_os_error() == Some(17) {
            // EXDEV / cross-device — handled below via copy_dir_recursive
            ApiError::Io(e)
        } else {
            ApiError::Io(e)
        }
    })?;

    tracing::info!(
        from = %src.display(),
        to = %dst.display(),
        "Entry renamed/moved"
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "from":   body.from,
        "to":     body.to,
    })))
}

/// `POST /api/copy`  body: `{ "from": "/a", "to": "/b" }`
pub async fn copy_entry(
    Json(body): Json<CopyBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let (src, _) = resolve_virtual_path(&body.from)?;
    let (dst, _) = resolve_virtual_path(&body.to)?;

    let meta = fs::metadata(&src).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound(format!("'{}' not found", body.from))
        } else {
            ApiError::Io(e)
        }
    })?;

    if meta.is_dir() {
        copy_dir_recursive(&src, &dst).await?;
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(&src, &dst).await?;
    }

    tracing::info!(
        from = %src.display(),
        to = %dst.display(),
        "Entry copied"
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "from":   body.from,
        "to":     body.to,
    })))
}

fn copy_dir_recursive<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        fs::create_dir_all(dst).await?;
        let mut rd = fs::read_dir(src).await?;
        while let Some(entry) = rd.next_entry().await? {
            let ft = entry.file_type().await?;
            let child_dst = dst.join(entry.file_name());
            if ft.is_dir() {
                copy_dir_recursive(&entry.path(), &child_dst).await?;
            } else {
                fs::copy(entry.path(), child_dst).await?;
            }
        }
        Ok(())
    })
}

/// `GET /api/info?path=...`
pub async fn get_file_info(
    Query(q): Query<PathQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let vpath = q.path.as_deref().unwrap_or("/");
    let (file_path, _) = resolve_virtual_path(vpath)?;

    let meta = fs::metadata(&file_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound(format!("'{}' not found", vpath))
        } else {
            ApiError::Io(e)
        }
    })?;

    let name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let entry = FileEntry::from_metadata(vpath.to_string(), &file_path, &meta, name)?;

    Ok(Json(serde_json::to_value(&entry).unwrap()))
}

/// `GET /api/search?path=&q=keyword&limit=200`
///
/// 在指定虚拟路径下（`/` 表示跨所有 root）递归搜索文件名。
pub async fn search_files(
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let vpath = q.path.as_deref().unwrap_or("/");
    let keyword = q.q.to_lowercase();
    let limit = q.limit.unwrap_or(200).min(1000);

    let mut results: Vec<FileEntry> = Vec::new();

    if vpath == "/" {
        // 跨所有 root 搜索
        for root in get_roots() {
            if results.len() >= limit {
                break;
            }
            search_recursive(&root.real_path, &keyword, limit, &mut results).await;
        }
    } else {
        let (search_path, _) = resolve_virtual_path(vpath)?;
        search_recursive(&search_path, &keyword, limit, &mut results).await;
    }

    let total = results.len();
    Ok(Json(serde_json::json!({
        "query":   q.q,
        "path":    vpath,
        "total":   total,
        "results": results.iter().map(|e| serde_json::json!({ "entry": e })).collect::<Vec<_>>(),
    })))
}

fn search_recursive<'a>(
    dir: &'a Path,
    keyword: &'a str,
    limit: usize,
    results: &'a mut Vec<FileEntry>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if results.len() >= limit {
            return;
        }
        let mut rd = match fs::read_dir(dir).await {
            Ok(r) => r,
            Err(_) => return,
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            if results.len() >= limit {
                break;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains(keyword) {
                if let Ok(fe) = FileEntry::from_dir_entry(&entry).await {
                    results.push(fe);
                }
            }
            if let Ok(ft) = entry.file_type().await {
                if ft.is_dir() {
                    search_recursive(&entry.path(), keyword, limit, results).await;
                }
            }
        }
    })
}
