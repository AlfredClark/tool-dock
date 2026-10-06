use tauri::Runtime;
use tauri::plugin::Plugin;

/// 外部打开插件：让前端用系统默认程序打开链接或文件
///
/// 安全网关在前端 `src/libs/utils/opener.ts` 的 `openExternal`（先剥 `git+` 前缀再限 `http(s)`），
/// 禁止绕过该网关直引 `@tauri-apps/plugin-opener` 的 `openUrl`，后端此处不做二次校验。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_opener::Builder::default().build()
}
