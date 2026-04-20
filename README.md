# web_fs — Web 文件系统管理器

一个基于浏览器的文件系统管理工具，编译为**单个静态二进制文件**，无 glibc 依赖，专为 Linux/aarch64 嵌入式设备设计。

## 功能特性

- 📂 **目录浏览** — 列表 / 网格双视图，支持按名称、大小、修改时间排序
- 🔍 **实时搜索** — 在当前目录下递归搜索文件名（防抖 300ms）
- 📤 **文件上传** — 拖拽或点击上传，支持批量上传并显示进度条
- 📥 **文件下载** — 流式下载，无内存缓冲，支持大文件
- 👁️ **文件预览** — 文本/代码、图片、音频、视频内联预览
- ✏️ **文件操作** — 重命名、复制、剪切/粘贴、删除（含递归删除目录）
- 📁 **新建目录** — 支持多级路径自动创建
- 🌑 **深色主题** — 默认深色，支持明暗切换
- 🔒 **路径安全** — 严格的 path traversal 防护，不允许跨越根目录

## 技术栈

| 层     | 技术                                              |
|--------|---------------------------------------------------|
| 后端   | Rust 2021 + Axum 0.8 + Tokio (async)              |
| 前端   | SvelteKit 2 + Svelte 5 (runes) + TypeScript       |
| 样式   | Tailwind CSS v4 (CSS-first, vite plugin)          |
| 组件   | bits-ui v1 + lucide-svelte                        |
| 嵌入   | rust-embed (前端静态文件编译进二进制)             |
| 目标   | `aarch64-unknown-linux-musl` (纯静态，无 glibc)  |

## 项目结构

```
web_fs/
├── .cargo/
│   └── config.toml          # 跨编译配置与 alias
├── src/
│   ├── main.rs              # 服务器入口，静态文件嵌入
│   └── api/
│       ├── mod.rs           # API 路由注册
│       ├── files.rs         # 所有文件系统操作 handler
│       └── error.rs         # 统一错误类型
├── frontend/
│   ├── src/
│   │   ├── routes/
│   │   │   ├── +layout.svelte  # 根布局 (Toaster, ModeWatcher)
│   │   │   └── +page.svelte    # 主页面
│   │   └── lib/
│   │       ├── api.ts           # API 客户端封装
│   │       ├── types.ts         # TypeScript 类型定义
│   │       ├── utils.ts         # cn() 工具函数
│   │       └── components/
│   │           ├── FileManager.svelte    # 主容器，状态管理
│   │           ├── FileBrowser.svelte   # 文件列表/网格
│   │           ├── Toolbar.svelte       # 工具栏 + 面包屑 + 搜索
│   │           ├── FilePreview.svelte   # 右侧预览面板
│   │           ├── UploadZone.svelte    # 拖拽上传区
│   │           ├── NewFolderDialog.svelte
│   │           ├── RenameDialog.svelte
│   │           └── DeleteDialog.svelte
│   ├── package.json
│   ├── svelte.config.js     # adapter-static, 输出到 dist/
│   └── vite.config.ts       # @tailwindcss/vite + /api 代理
├── Cargo.toml
├── build.rs                 # 编译前自动执行 npm run build
└── README.md
```

## 快速开始（开发模式）

### 前提条件

- Rust 1.80+ (`rustup update stable`)
- Node.js 20+ 和 npm

### 启动后端

```bash
# 编译并运行（跳过前端构建，前端单独启动）
SKIP_FRONTEND_BUILD=1 cargo run

# 自定义配置
FS_ROOT=/data LISTEN_ADDR=0.0.0.0:9000 cargo run
```

### 启动前端开发服务器

```bash
cd frontend
npm install
npm run dev
# 访问 http://localhost:5173
# /api/* 请求自动代理到 http://localhost:8080
```

## 生产构建（前端嵌入二进制）

```bash
# 构建前端（在 host 上执行，与目标架构无关）
cd frontend && npm install && npm run build && cd ..

# 本机架构（调试用）
SKIP_FRONTEND_BUILD=1 cargo build --release

# 运行
./target/release/web_fs
```

## 交叉编译到 aarch64-unknown-linux-musl

目标：完全静态链接，无 glibc 依赖，可在 ARM64 嵌入式 Linux 上直接运行。

### 方案 A — cross（推荐，需要 Docker）

```bash
# 安装工具
cargo install cross --git https://github.com/cross-rs/cross
rustup target add aarch64-unknown-linux-musl

# 先在 host 构建前端
cd frontend && npm run build && cd ..

# 交叉编译（cross 会自动拉取 Docker 镜像）
SKIP_FRONTEND_BUILD=1 cross build --release --target aarch64-unknown-linux-musl

# 输出文件
ls -lh target/aarch64-unknown-linux-musl/release/web_fs
```

### 方案 B — cargo-zigbuild（不需要 Docker，需要 zig）

```bash
# 安装 zig: https://ziglang.org/download/
cargo install cargo-zigbuild
rustup target add aarch64-unknown-linux-musl

cd frontend && npm run build && cd ..

SKIP_FRONTEND_BUILD=1 cargo zigbuild --release --target aarch64-unknown-linux-musl
```

### 方案 C — 原生 musl 工具链（仅限 Linux host）

```bash
# Debian/Ubuntu
sudo apt install gcc-aarch64-linux-gnu musl-tools

rustup target add aarch64-unknown-linux-musl

cd frontend && npm run build && cd ..

SKIP_FRONTEND_BUILD=1 cargo build --release --target aarch64-unknown-linux-musl
```

### 验证无 glibc 依赖

```bash
# 在目标机器上（或用 qemu）
file target/aarch64-unknown-linux-musl/release/web_fs
# → ELF 64-bit LSB executable, ARM aarch64, statically linked

# 确认没有动态依赖
readelf -d target/aarch64-unknown-linux-musl/release/web_fs | grep NEEDED
# → (无输出)
```

### 一键构建脚本（使用 cross）

```bash
#!/usr/bin/env bash
set -e

echo "==> Building frontend..."
(cd frontend && npm ci && npm run build)

echo "==> Cross-compiling Rust backend..."
SKIP_FRONTEND_BUILD=1 cross build --release --target aarch64-unknown-linux-musl

BINARY="target/aarch64-unknown-linux-musl/release/web_fs"
echo "==> Done: ${BINARY}"
echo "    Size: $(du -h "${BINARY}" | cut -f1)"
file "${BINARY}"
```

## 部署

将单个二进制文件复制到目标设备即可运行：

```bash
# 复制到设备
scp target/aarch64-unknown-linux-musl/release/web_fs user@device:/usr/local/bin/

# 在设备上运行
FS_ROOT=/data LISTEN_ADDR=0.0.0.0:8080 web_fs
```

### systemd 服务单元示例

```ini
[Unit]
Description=Web File System Manager
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/web_fs
Environment="FS_ROOT=/data"
Environment="LISTEN_ADDR=0.0.0.0:8080"
Environment="RUST_LOG=info"
Restart=on-failure
RestartSec=5s
# 安全加固
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/data

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now web_fs
```

## 环境变量

| 变量            | 默认值          | 说明                                             |
|-----------------|-----------------|--------------------------------------------------|
| `LISTEN_ADDR`   | `0.0.0.0:8080`  | 监听地址，格式 `host:port`                       |
| `FS_ROOT`       | `/`             | 文件系统根目录，用户只能访问此目录及其子目录     |
| `RUST_LOG`      | `info`          | tracing 日志过滤器，如 `web_fs=debug,tower_http=warn` |
| `SKIP_FRONTEND_BUILD` | (未设置) | 设为任意值时跳过 `npm run build`（build.rs）     |

## REST API

所有接口路径均以 `/api` 为前缀：

| 方法     | 路径                   | 查询参数 / 请求体                           | 说明               |
|----------|------------------------|---------------------------------------------|--------------------|
| `GET`    | `/api/files`           | `?path=&sort=name&order=asc&show_hidden=false` | 列出目录           |
| `GET`    | `/api/file`            | `?path=`                                    | 获取文件内容（预览，限 10 MiB） |
| `GET`    | `/api/download`        | `?path=`                                    | 流式下载文件       |
| `POST`   | `/api/upload`          | `?path=`，multipart body                    | 上传文件           |
| `POST`   | `/api/mkdir`           | `{ "path": "/new/dir" }`                    | 创建目录           |
| `DELETE` | `/api/files`           | `?path=`                                    | 删除文件或目录     |
| `PUT`    | `/api/rename`          | `{ "from": "/old", "to": "/new" }`          | 重命名 / 移动      |
| `POST`   | `/api/copy`            | `{ "from": "/src", "to": "/dst" }`          | 复制文件或目录     |
| `GET`    | `/api/info`            | `?path=`                                    | 获取文件元信息     |
| `GET`    | `/api/search`          | `?path=&q=keyword&limit=200`                | 递归搜索文件名     |

错误响应格式：
```json
{ "error": "错误描述信息" }
```

## 键盘快捷键

| 快捷键         | 功能         |
|----------------|--------------|
| `Ctrl+A`       | 全选         |
| `Ctrl+C`       | 复制选中项   |
| `Ctrl+X`       | 剪切选中项   |
| `Ctrl+V`       | 粘贴         |
| `Delete`       | 删除选中项   |
| `F2`           | 重命名选中项 |
| `Escape`       | 取消选择     |
| `Alt+Left`     | 后退         |
| `Alt+Right`    | 前进         |
| `Alt+Up`       | 上级目录     |
| `F5`           | 刷新         |

## 安全说明

- **路径遍历防护**：所有用户提供的路径都经过规范化处理，确保不能访问 `FS_ROOT` 之外的文件
- **根目录保护**：不允许删除 `FS_ROOT` 本身
- **CORS**：当前配置为开发友好（允许所有来源），生产环境建议通过反向代理（nginx/caddy）限制来源
- **认证**：工具本身不包含认证机制，建议通过反向代理添加 Basic Auth 或其他认证方案

## License

MIT