//! 系统信息插件初始化：供后端做系统语言探测。

use tauri::{Runtime, plugin::Plugin};

/// 系统信息插件：后端语言探测与平台信息采集直接调用 Rust 侧函数（见 `cores::config` /
///
/// `cores::system`），不经过命令层；前端无 `plugin-os` JS 直调，故 capability 不放行任何
/// `os` 权限（能力项只管 `WebView` 的 JS 调用，Rust 侧调用不受其约束）
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_os::init()
}
