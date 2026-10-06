//! 剪贴板插件初始化：演示页经命令读写系统剪贴板纯文本。

use tauri::{Runtime, plugin::Plugin};

/// 剪贴板插件：命令经 Rust 侧 `ClipboardExt::write_text` 写纯文本，不碰图片与 HTML；
/// 前端无剪贴板 JS 直调，故 capability 不放行任何 `clipboard-manager` 权限
/// （能力项只管 `WebView` 的 JS 调用，Rust 侧调用不受其约束）。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_clipboard_manager::init()
}
