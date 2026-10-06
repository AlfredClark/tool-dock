//! 系统信息插件初始化：供后端做系统语言探测。

use tauri::{Runtime, plugin::Plugin};

/// 系统信息插件：后端语言探测直接调用 Rust 侧函数（见 `cores::config`），不经过命令层；
/// 注册插件是为了与文档保持一致，并允许前端在 capability 最小授权（仅 `os:allow-locale`）下查询系统语言
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_os::init()
}
