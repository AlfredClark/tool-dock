//! 系统通知插件初始化：演示页经命令发送一条本地通知。

use tauri::{Runtime, plugin::Plugin};

/// 系统通知插件：标题正文长度由 `features::demo` 在命令入口前校验，
/// Linux 无通知守护时发送失败走命令错误分支，前端演示失败态。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_notification::init()
}
