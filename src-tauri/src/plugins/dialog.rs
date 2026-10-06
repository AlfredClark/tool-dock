//! 文件对话框插件初始化：演示页经命令弹出系统选文件 / 存文件 / 选目录框。

use tauri::{Runtime, plugin::Plugin};

/// 文件对话框插件：只经后端命令调用（`blocking_pick_file` 等阻塞式 API 在命令线程等待用户选择），
/// 前端不直调 JS API，保持调用链收敛于 `libs/commands`。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_dialog::init()
}
