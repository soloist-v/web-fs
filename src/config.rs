//! 配置文件加载模块
//!
//! 优先读取 `config.yaml`（或 `CONFIG_FILE` 环境变量指定的路径），
//! 若不存在则回退到旧版环境变量（`LISTEN_ADDR` / `FS_ROOT`），
//! 最终兜底为内置默认值。
//!
//! # 配置文件示例
//! ```yaml
//! host: "0.0.0.0"
//! port: 8080
//! log_level: "info"
//!
//! roots:
//!   - name: data
//!     path: /data
//!   - name: media
//!     path: /mnt/media
//! ```

use std::path::PathBuf;

use serde::Deserialize;

// ─── Defaults ────────────────────────────────────────────────────────────────

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_roots() -> Vec<RootEntry> {
    vec![RootEntry {
        name: "root".to_string(),
        path: "/".to_string(),
    }]
}

// ─── Data types ───────────────────────────────────────────────────────────────

/// 单个可访问根路径的配置项
#[derive(Debug, Clone, Deserialize)]
pub struct RootEntry {
    /// 在虚拟文件系统中显示的名称（URL 路径第一段）
    pub name: String,
    /// 宿主机上的真实路径
    pub path: String,
}

/// 完整应用配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 监听 IP，默认 `0.0.0.0`
    pub host: String,
    /// 监听端口，默认 `8080`
    pub port: u16,
    /// tracing 日志级别过滤器，例如 `"info"`、`"web_fs=debug,tower_http=warn"`
    pub log_level: String,
    /// 可访问的根路径列表，至少需要一个
    pub roots: Vec<RootEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
            roots: default_roots(),
        }
    }
}

impl Config {
    /// 加载配置，搜索顺序：
    /// 1. `CONFIG_FILE` 环境变量指定的文件
    /// 2. 当前目录下的 `config.yaml`
    /// 3. 当前目录下的 `config.yml`
    /// 4. 旧版环境变量（`LISTEN_ADDR`、`FS_ROOT`、`RUST_LOG`）
    /// 5. 内置默认值
    pub fn load() -> Self {
        // ── 候选配置文件路径 ──────────────────────────────────────────────────
        let candidates: Vec<PathBuf> = {
            let mut v: Vec<PathBuf> = Vec::new();
            if let Ok(p) = std::env::var("CONFIG_FILE") {
                v.push(PathBuf::from(p));
            }
            v.push(PathBuf::from("config.yaml"));
            v.push(PathBuf::from("config.yml"));
            v
        };

        for path in &candidates {
            if !path.exists() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(content) => match serde_yaml::from_str::<Config>(&content) {
                    Ok(mut cfg) => {
                        // roots 不能为空
                        if cfg.roots.is_empty() {
                            eprintln!(
                                "Warning: 'roots' list in {} is empty; \
                                 using default root '/'",
                                path.display()
                            );
                            cfg.roots = default_roots();
                        }
                        // 去掉 root name 中的非法字符（/ 和空格）
                        for r in &mut cfg.roots {
                            r.name = sanitize_root_name(&r.name);
                        }
                        eprintln!("Config loaded from {}", path.display());
                        return cfg;
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: failed to parse {}: {}",
                            path.display(),
                            e
                        );
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Warning: failed to read {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        // ── 旧版环境变量回退 ───────────────────────────────────────────────────
        let (host, port) = parse_listen_addr(
            &std::env::var("LISTEN_ADDR").unwrap_or_default(),
        );
        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| default_log_level());
        let fs_root =
            std::env::var("FS_ROOT").unwrap_or_else(|_| "/".to_string());

        Config {
            host,
            port,
            log_level,
            roots: vec![RootEntry {
                name: "root".to_string(),
                path: fs_root,
            }],
        }
    }

    /// 返回 `host:port` 格式的监听地址字符串
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// 从 `host:port` 格式字符串中拆分主机和端口。
/// 解析失败时分别回退到 `"0.0.0.0"` 和 `8080`。
fn parse_listen_addr(addr: &str) -> (String, u16) {
    // rsplit_once 处理 IPv6 地址中的冒号
    if let Some((h, p)) = addr.rsplit_once(':') {
        if let Ok(port) = p.parse::<u16>() {
            return (h.to_string(), port);
        }
    }
    (default_host(), default_port())
}

/// 将 root name 中的 `/` 和空格替换为 `_`，防止路径注入。
fn sanitize_root_name(name: &str) -> String {
    name.chars()
        .map(|c| if c == '/' || c.is_whitespace() { '_' } else { c })
        .collect()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = Config::default();
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 8080);
        assert!(!cfg.roots.is_empty());
    }

    #[test]
    fn parse_listen_addr_ipv4() {
        let (h, p) = parse_listen_addr("127.0.0.1:9000");
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 9000);
    }

    #[test]
    fn parse_listen_addr_fallback() {
        let (h, p) = parse_listen_addr("invalid");
        assert_eq!(h, "0.0.0.0");
        assert_eq!(p, 8080);
    }

    #[test]
    fn parse_from_yaml() {
        let yaml = r#"
host: "127.0.0.1"
port: 9090
log_level: "debug"
roots:
  - name: home
    path: /home/user
  - name: data
    path: /data
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.roots.len(), 2);
        assert_eq!(cfg.roots[0].name, "home");
        assert_eq!(cfg.roots[1].path, "/data");
    }

    #[test]
    fn sanitize_root_name_strips_slash() {
        assert_eq!(sanitize_root_name("foo/bar"), "foo_bar");
        assert_eq!(sanitize_root_name("my root"), "my_root");
        assert_eq!(sanitize_root_name("ok"), "ok");
    }
}
