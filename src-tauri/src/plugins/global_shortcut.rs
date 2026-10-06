//! 全局快捷键插件初始化（仅桌面端）：演示页双轨使用本插件。
//!
//! 后端固定键（见 `features::demo::DEMO_SHORTCUT`）经命令注册 / 注销；
//! 前端自定义槽位与关闭窗口预设键经插件 JS API 直调（见 `libs/shortcuts/`），
//! 输入先过格式与保留键校验，注册失败走 toast，用户只能劫持自己的会话。

use tauri::{Runtime, plugin::Plugin};

/// 全局快捷键插件（仅桌面端）：后端固定键只注册演示键（见 `features::demo::DEMO_SHORTCUT`），
/// 不开放命令层任意注册入口；前端直调用 `capabilities/plugins.json` 的三项权限约束，
/// 触发时后端固定键走前端事件通知演示页，前端直调走注册时传入的回调。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_global_shortcut::Builder::default().build()
}
