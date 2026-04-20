//! web_fs — Web 文件系统管理工具后端入口
//!
//! # 架构概览
//! ```text
//! 请求
//!  │
//!  ├─ /api/*  ──► api::router()   (文件系统 REST API)
//!  │
//!  └─ /*      ──► static_handler  (rust-embed 嵌入的前端 SPA)
//!                   └─ 404 → index.html  (SPA fallback)
//! ```
//!
//! # 配置
//! 优先读取 `config.yaml`（或 `CONFIG_FILE` 环境变量指定的路径）。
//! 若不存在则回退到 `LISTEN_ADDR` / `FS_ROOT` / `RUST_LOG` 环境变量，
//! 最终兜底为内置默认值。

mod api;
mod config;

use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use rust_embed::RustEmbed;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ─── 嵌入前端静态资源 ─────────────────────────────────────────────────────────

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

// ─── 静态文件处理器 ───────────────────────────────────────────────────────────

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let raw_path = uri.path().trim_start_matches('/');

    let lookup_path = if raw_path.is_empty() {
        "index.html".to_string()
    } else {
        raw_path.to_string()
    };

    if let Some(content) = Assets::get(&lookup_path) {
        return serve_embedded_file(&lookup_path, content);
    }

    let with_html = format!("{}.html", lookup_path.trim_end_matches('/'));
    if let Some(content) = Assets::get(&with_html) {
        return serve_embedded_file(&with_html, content);
    }

    if let Some(content) = Assets::get("index.html") {
        return serve_embedded_file("index.html", content);
    }

    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Body::from(
            r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>web_fs</title></head>
<body>
  <h1>Frontend not built</h1>
  <p>Run <code>npm run build</code> in the <code>frontend/</code> directory,
     or set <code>SKIP_FRONTEND_BUILD=1</code> and rebuild the server.</p>
  <p>The API is available at <a href="/api/files">/api/files</a>.</p>
</body>
</html>"#,
        ),
    )
        .into_response()
}

fn serve_embedded_file(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from(file.data.to_vec()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn fallback_handler(req: Request) -> impl IntoResponse {
    static_handler(req.uri().clone()).await
}

// ─── 应用程序入口 ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── 加载配置 ───────────────────────────────────────────────────────────────
    // 注意：此时 tracing 尚未初始化，config 内部用 eprintln! 输出警告
    let cfg = config::Config::load();

    // ── 初始化 tracing ─────────────────────────────────────────────────────────
    // RUST_LOG 环境变量优先于配置文件中的 log_level
    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .unwrap_or_else(|| cfg.log_level.clone());

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_new(&env_filter).unwrap_or_else(|_| {
                eprintln!(
                    "Warning: invalid log filter {:?}, falling back to 'info'",
                    env_filter
                );
                EnvFilter::new("info")
            }),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    // ── 初始化全局 root 列表 ───────────────────────────────────────────────────
    // 必须在处理任何请求之前完成
    api::files::init_roots(cfg.roots.clone());

    // ── CORS 中间件 ────────────────────────────────────────────────────────────
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    // ── 构建路由树 ────────────────────────────────────────────────────────────
    let app = Router::new()
        .nest("/api", api::router())
        .fallback(fallback_handler)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors);

    // ── 解析监听地址 ──────────────────────────────────────────────────────────
    let listen_addr_str = cfg.listen_addr();
    let addr: SocketAddr = listen_addr_str.parse().map_err(|e| {
        anyhow::anyhow!(
            "Invalid listen address '{}': {}",
            listen_addr_str,
            e
        )
    })?;

    // ── 打印启动信息 ──────────────────────────────────────────────────────────
    tracing::info!(
        listen    = %addr,
        log_level = %cfg.log_level,
        roots     = cfg.roots.len(),
        version   = env!("CARGO_PKG_VERSION"),
        "web_fs server starting"
    );
    for root in &cfg.roots {
        tracing::info!(
            name = %root.name,
            path = %root.path,
            "Accessible root"
        );
    }

    // ── 启动服务 ──────────────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        anyhow::anyhow!("Failed to bind to {}: {}", addr, e)
    })?;

    tracing::info!("Listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

// ─── 优雅关闭 ─────────────────────────────────────────────────────────────────

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c   => tracing::info!("Received Ctrl-C, shutting down..."),
        _ = sigterm  => tracing::info!("Received SIGTERM, shutting down..."),
    }
}
