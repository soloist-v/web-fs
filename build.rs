use std::{env, path::Path, process::Command};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let frontend_dir = Path::new(&manifest_dir).join("frontend");
    let dist_dir = frontend_dir.join("dist");
    let pkg_json = frontend_dir.join("package.json");

    // 监听这些文件/目录的变化，变化时重新触发 build.rs
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/svelte.config.js");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");

    // 允许通过环境变量跳过前端构建（用于纯后端开发或 CI 场景）
    if env::var("SKIP_FRONTEND_BUILD").is_ok() {
        println!("cargo:warning=Skipping frontend build (SKIP_FRONTEND_BUILD is set)");
        // 确保 dist 目录存在，否则 rust-embed 会在编译时报错
        std::fs::create_dir_all(&dist_dir).ok();
        return;
    }

    // 若 frontend/package.json 不存在，说明前端尚未初始化，跳过
    if !pkg_json.exists() {
        println!("cargo:warning=frontend/package.json not found, skipping frontend build");
        std::fs::create_dir_all(&dist_dir).ok();
        return;
    }

    // Windows 上 npm 的可执行文件名为 npm.cmd
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    // ── Step 1: npm install ──────────────────────────────────────────────────
    println!("cargo:warning=Running npm install in frontend/");
    let status = Command::new(npm_cmd)
        .args(["install"])
        .current_dir(&frontend_dir)
        .status()
        .expect("Failed to spawn npm install. Is npm installed and in PATH?");

    if !status.success() {
        panic!("npm install failed with exit code: {:?}", status.code());
    }

    // ── Step 2: npm run build ────────────────────────────────────────────────
    println!("cargo:warning=Running npm run build in frontend/");
    let status = Command::new(npm_cmd)
        .args(["run", "build"])
        .current_dir(&frontend_dir)
        .status()
        .expect("Failed to spawn npm run build. Is npm installed and in PATH?");

    if !status.success() {
        panic!("npm run build failed with exit code: {:?}", status.code());
    }

    println!("cargo:warning=Frontend build completed successfully");
}
