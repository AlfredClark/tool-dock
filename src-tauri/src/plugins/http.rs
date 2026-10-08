//! 网络请求插件初始化：前端经 `@tauri-apps/plugin-http` 的 `fetch` 出网。
//!
//! 出网域名白名单（含 Rust 侧直连、仅作审计备查）。
//!
//! - `github.com` + `objects.githubusercontent.com`：应用更新器与 ffmpeg 托管包（`BtbN` 构建）下载
//! - `evermeet.cx` + `e.deolaha.ca:4242`：macOS ffmpeg/ffprobe 包（evermeet 主站及重定向镜像）
//!
//! ffmpeg 下载走 Rust 侧 `ureq` 直连（`cores::ffmpeg`），不受 `WebView` CSP 约束
//! （与 `updater` 下载端点同类例外）；前端 `fetch` 若新增以上域名，`capabilities/plugins.json`
//! 与 `tauri.conf.json` 的 CSP `connect-src` 仍须同步放行，缺一不可。

use tauri::{Runtime, plugin::Plugin};

/// 网络请求插件：桌面 / 移动通用，故在 `lib.rs` 直接注册，无需 `BuilderExt` 空实现。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_http::init()
}
