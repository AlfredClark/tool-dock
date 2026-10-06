//! 示例命令：演示页（文件 / 对话框 / 剪贴板 / 通知 / 快捷键 / 拖放）薄封装。
//!
//! 本文件是唯一同时委托两层的命令：问候语等纯函数走 `features::demo`，
//! 其余需 `Tauri` 运行时的能力走 `cores::demo`。

use crate::cores::demo::{self, DemoAppPaths};
use crate::cores::types::CommandResult;
use crate::features::demo as demo_features;

/// 演示命令：把入参转交给 `features::demo::greet` 处理。
///
/// `anyhow` 错误经 `From` 自动转为 `CommandError::Internal`。
#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> CommandResult<String> {
    Ok(demo_features::greet(name)?)
}

/// 解析三处应用目录供演示页展示与复制
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_app_paths(app: tauri::AppHandle) -> CommandResult<DemoAppPaths> {
    Ok(demo::app_paths(&app)?)
}

/// 写演示沙盒文件，返回绝对路径
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_write_file(
    app: tauri::AppHandle,
    filename: String,
    contents: String,
) -> CommandResult<String> {
    Ok(demo::write_demo_file(&app, &filename, &contents)?)
}

/// 读演示沙盒文件
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_read_file(app: tauri::AppHandle, filename: String) -> CommandResult<String> {
    Ok(demo::read_demo_file(&app, &filename)?)
}

/// 系统选文件框；用户取消返回 `None`
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn demo_pick_file(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    Ok(demo::pick_file(&app).await)
}

/// 系统选目录框；用户取消返回 `None`（移动端无目录选择，报错走 `.failed()`）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn demo_pick_folder(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    Ok(demo::pick_folder(&app).await)
}

/// 系统选目录框；用户取消返回 `None`（移动端无目录选择，报错走 `.failed()`）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn demo_pick_folder(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    Ok(demo::pick_folder(&app).await?)
}

/// 系统存文件框；只取路径不写盘，用户取消返回 `None`
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn demo_save_file(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    Ok(demo::save_file(&app).await)
}

/// 写剪贴板纯文本
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_clipboard_write(app: tauri::AppHandle, text: String) -> CommandResult<()> {
    Ok(demo::clipboard_write(&app, &text)?)
}

/// 读剪贴板纯文本；空剪贴板按空串返回
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_clipboard_read(app: tauri::AppHandle) -> CommandResult<String> {
    Ok(demo::clipboard_read(&app)?)
}

/// 发送一条本地通知
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_notify(app: tauri::AppHandle, title: String, body: String) -> CommandResult<()> {
    Ok(demo::notify(&app, &title, &body)?)
}

/// 演示快捷键固定键（`features::demo::DEMO_SHORTCUT`），前端展示用，不开放写入
#[tauri::command]
#[specta::specta]
pub fn demo_shortcut_key() -> CommandResult<String> {
    Ok(demo_features::DEMO_SHORTCUT.to_owned())
}

/// 注册演示快捷键；幂等，已注册直接成功（移动端报错走 `.failed()`）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_shortcut_register(app: tauri::AppHandle) -> CommandResult<bool> {
    Ok(demo::shortcut_register(&app)?)
}

/// 查询演示快捷键是否已注册（移动端恒 `false`）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_shortcut_is_registered(app: tauri::AppHandle) -> CommandResult<bool> {
    Ok(demo::shortcut_is_registered(&app))
}

/// 注销演示快捷键；幂等（移动端报错走 `.failed()`）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_shortcut_unregister(app: tauri::AppHandle) -> CommandResult<()> {
    Ok(demo::shortcut_unregister(&app)?)
}

/// 鉴别拖放批次：只取元信息不读内容，任一项失败整批失败
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_inspect_drop(paths: Vec<String>) -> CommandResult<Vec<demo::DropFileInfo>> {
    Ok(demo::inspect_drop(&paths)?)
}

/// 存拖放文件到沙盒：仅常规文件可拷，返回沙盒绝对路径列表
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn demo_import_drop(app: tauri::AppHandle, paths: Vec<String>) -> CommandResult<Vec<String>> {
    Ok(demo::import_drop(&app, &paths)?)
}
