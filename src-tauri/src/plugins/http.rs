//! 网络请求插件初始化：前端经 `@tauri-apps/plugin-http` 的 `fetch` 出网。
//!
//! 出网域约束在 `capabilities/plugins.json` 的 `http` 域（仅放行 `https://timeapi.io/*`），
//! `WebView` 侧另受 `tauri.conf.json` 的 CSP `connect-src` 约束，两处同步放行缺一不可。

use tauri::{Runtime, plugin::Plugin};

/// 网络请求插件：桌面 / 移动通用，故在 `lib.rs` 直接注册，无需 `BuilderExt` 空实现。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_http::init()
}
