//! 文件对话框插件初始化：图片工具的“选择图片 / 选择输出目录”原生对话框入口，此处只做注册。

use tauri::Runtime;
use tauri::plugin::Plugin;

/// 文件对话框插件：前端经 `@tauri-apps/plugin-dialog` 的 `open` / `save` 调用，
/// capability 仅放行 `dialog:allow-open` 与 `dialog:allow-save`（最小授权）
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_dialog::init()
}
