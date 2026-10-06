//! 文件系统插件初始化：演示页经命令读写应用数据目录下的 `demo/` 沙盒。

use tauri::{Runtime, plugin::Plugin};

/// 文件系统插件： capability 的 `fs` 域把 JS 直调限定在 `$APPDATA/demo/*`；
/// 后端命令不走 IPC，故落盘前另做路径收敛校验（见 `cores::demo::ensure_inside_sandbox`），
/// 先门禁、再建目录写盘。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    tauri_plugin_fs::init()
}
