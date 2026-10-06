//! 网络请求插件初始化：前端经 `@tauri-apps/plugin-http` 的 `fetch` 出网。
//!
//! 当前未放行任何外部域名（最小授权默认拒绝）：`capabilities/plugins.json` 无 `http` 域条目，
//! `WebView` 侧 `tauri.conf.json` 的 CSP `connect-src` 亦仅收敛到 IPC 与本地开发源。
//! 新增出网域名时两处必须同步放行，缺一不可。

use tauri::{Runtime, plugin::Plugin};

/// 网络请求插件：桌面 / 移动通用，故在 `lib.rs` 直接注册，无需 `BuilderExt` 空实现。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_http::init()
}
