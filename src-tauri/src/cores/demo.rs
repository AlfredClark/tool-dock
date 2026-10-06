//! 演示页能力：应用目录解析、沙盒文件读写、对话框、剪贴板、通知、全局快捷键与文件拖放。
//!
//! 需要 Tauri 运行时（插件状态与系统 API），故放在 `cores` 而非纯函数的 `features`；
//! 输入校验（文件名 / 文本长度 / 通知长度 / 拖放批次）仍在 `features::demo`，此处只做路径装配与调用。
//! 删除演示页时本文件整体删除，命令层对应项同步下线。
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::features::demo as demo_features;

/// 演示页展示的应用目录：只含前端展示所需字段，`PathBuf` 句柄不出命令边界
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DemoAppPaths {
    /// 应用数据目录（演示沙盒 `demo/` 即落于此下）
    pub app_data: String,
    /// 应用缓存目录
    pub app_cache: String,
    /// 系统临时目录
    pub temp: String,
}

/// 解析三处应用目录；任一解析失败即整条命令失败，不回落空串（空串展示会误导复制路径）
pub fn app_paths(app: &tauri::AppHandle) -> anyhow::Result<DemoAppPaths> {
    use tauri::Manager;

    let path = app.path();
    let display = |value: PathBuf| value.display().to_string();
    Ok(DemoAppPaths {
        app_data: display(
            path.app_data_dir()
                .context("failed to resolve app data dir")?,
        ),
        app_cache: display(
            path.app_cache_dir()
                .context("failed to resolve app cache dir")?,
        ),
        temp: display(path.temp_dir().context("failed to resolve temp dir")?),
    })
}

/// 装配沙盒文件绝对路径：`app_data/demo/<规整文件名>`；
/// 规整与收敛断言由 `features` 完成，此处只提供 `base`
fn demo_file_path(app: &tauri::AppHandle, filename: &str) -> anyhow::Result<PathBuf> {
    use tauri::Manager;

    let base = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data dir")?;
    demo_features::resolve_demo_file(&base, filename)
}

/// 沙盒 containment 门禁：路径必须收敛在 `app_data/demo/` 内。
///
/// capability 的 `fs` 域只约束 JS 直接调用（IPC 层），Rust 侧调用不走 IPC，
/// 故此处以后缀前缀断言做二次校验；先门禁、再建目录落盘，避免拒绝前产生副作用
fn ensure_inside_sandbox(app: &tauri::AppHandle, path: &std::path::Path) -> anyhow::Result<()> {
    use tauri::Manager;

    let root = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data dir")?
        .join(demo_features::DEMO_DIR_NAME);
    if path.starts_with(&root) {
        Ok(())
    } else {
        anyhow::bail!("path is outside the demo sandbox: {}", path.display())
    }
}

/// 写演示文件：建沙盒目录 → 门禁 → 落盘，返回绝对路径供前端展示与复制
pub fn write_demo_file(
    app: &tauri::AppHandle,
    filename: &str,
    contents: &str,
) -> anyhow::Result<String> {
    demo_features::validate_demo_text(contents)?;
    let path = demo_file_path(app, filename)?;
    ensure_inside_sandbox(app, &path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create demo folder")?;
    }
    std::fs::write(&path, contents).context("failed to write demo file")?;
    Ok(path.display().to_string())
}

/// 读演示文件：门禁 → 读文本；缺文件与超大文件都经错误分支，前端演示失败态
pub fn read_demo_file(app: &tauri::AppHandle, filename: &str) -> anyhow::Result<String> {
    let path = demo_file_path(app, filename)?;
    ensure_inside_sandbox(app, &path)?;
    std::fs::read_to_string(&path).context("failed to read demo file")
}

/// 拖放文件元信息：只取 `symlink_metadata`（不跟随符号链接），不读内容；
/// 目录仅展示不导入，符号链接直接拒绝（目标可能指向沙盒之外）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct DropFileInfo {
    /// 末段文件名（展示用，不含目录部分）
    pub name: String,
    /// 文件字节数；目录按 `0` 返回（`f64`：`specta` 禁止导出 `u64`，展示精度足够）
    pub size: f64,
    /// 是否为目录
    pub is_dir: bool,
}

/// 鉴别拖放批次：先过数量门禁，再逐项取元信息；任一项失败整批失败，
/// 前端收到错误后按通用失败提示处理，不做部分回显
// `u64 as f64` 在 2^53 以上丢精度，但此处仅做展示，无需精确值
#[allow(clippy::cast_precision_loss)]
pub fn inspect_drop(paths: &[String]) -> anyhow::Result<Vec<DropFileInfo>> {
    demo_features::validate_drop_paths(paths)?;
    paths
        .iter()
        .map(|raw| {
            let path = PathBuf::from(raw);
            // 不跟随链接：链接本身一律拒绝，不解析目标，避免目标越界不可见
            let meta = std::fs::symlink_metadata(&path)
                .with_context(|| format!("failed to stat dropped path: {raw}"))?;
            if meta.is_symlink() {
                anyhow::bail!("symbolic links are not accepted: {raw}");
            }
            let name = path
                .file_name()
                .context("dropped path has no file name")?
                .to_string_lossy()
                .into_owned();
            Ok(DropFileInfo {
                name,
                size: if meta.is_dir() {
                    0.0
                } else {
                    meta.len() as f64
                },
                is_dir: meta.is_dir(),
            })
        })
        .collect()
}

/// 存拖放文件到沙盒：仅常规文件可拷，目录与链接拒绝；单个超大拒绝；
/// 文件名取末段后走沙盒装配与门禁，与 `write_demo_file` 同一收敛点
pub fn import_drop(app: &tauri::AppHandle, paths: &[String]) -> anyhow::Result<Vec<String>> {
    demo_features::validate_drop_paths(paths)?;
    paths
        .iter()
        .map(|raw| {
            let source = PathBuf::from(raw);
            let meta = std::fs::symlink_metadata(&source)
                .with_context(|| format!("failed to stat dropped path: {raw}"))?;
            if meta.is_symlink() {
                anyhow::bail!("symbolic links are not accepted: {raw}");
            }
            if !meta.is_file() {
                anyhow::bail!("only files can be imported: {raw}");
            }
            if meta.len() > demo_features::MAX_DROP_FILE_SIZE {
                anyhow::bail!("dropped file is too large: {raw}");
            }
            let name = source
                .file_name()
                .context("dropped path has no file name")?
                .to_string_lossy()
                .into_owned();
            // 末段天然无分隔符，仍走文件名校验与沙盒门禁，收敛点与写文件一致
            let target = demo_file_path(app, &name)?;
            ensure_inside_sandbox(app, &target)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).context("failed to create demo folder")?;
            }
            std::fs::copy(&source, &target).context("failed to import dropped file")?;
            Ok(target.display().to_string())
        })
        .collect()
}

/// 系统选文件框：异步回调版经主线程弹窗，取消返回 `None`（正常分支，前端展示"已取消"而非报错）。
///
/// 同步命令跑在主线程，`blocking_*` 的 `recv` 会与 `run_on_main_thread` 排队的弹窗任务死锁，
//  此处禁止使用阻塞版；回调经 `spawn_blocking` 转回异步，无需新增 channel 依赖
pub async fn pick_file(app: &tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_file(move |selected| {
        let _ = tx.send(selected.map(|path| path.to_string()));
    });
    tauri::async_runtime::spawn_blocking(move || rx.recv().ok().flatten())
        .await
        .ok()
        .flatten()
}

/// 系统选目录框：同上，取消返回 `None`（仅桌面端，移动端直接报错）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn pick_folder(app: &tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |selected| {
        let _ = tx.send(selected.map(|path| path.to_string()));
    });
    tauri::async_runtime::spawn_blocking(move || rx.recv().ok().flatten())
        .await
        .ok()
        .flatten()
}

/// 系统选目录框：同上，取消返回 `None`（仅桌面端，移动端直接报错）
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn pick_folder(_app: &tauri::AppHandle) -> anyhow::Result<Option<String>> {
    Err(anyhow::anyhow!("folder picker is not supported on mobile"))
}

/// 系统存文件框：返回用户确认的保存路径（本命令只取路径不写盘，写盘走沙盒命令）
pub async fn save_file(app: &tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().save_file(move |selected| {
        let _ = tx.send(selected.map(|path| path.to_string()));
    });
    tauri::async_runtime::spawn_blocking(move || rx.recv().ok().flatten())
        .await
        .ok()
        .flatten()
}

/// 写剪贴板纯文本：长度先行校验，不碰图片与 HTML
pub fn clipboard_write(app: &tauri::AppHandle, text: &str) -> anyhow::Result<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    demo_features::validate_demo_text(text)?;
    app.clipboard()
        .write_text(text)
        .context("failed to write clipboard")?;
    Ok(())
}

/// 读剪贴板纯文本：无文本或非文本内容时按空串返回，不报错（空剪贴板是常态）
pub fn clipboard_read(app: &tauri::AppHandle) -> anyhow::Result<String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    app.clipboard()
        .read_text()
        .context("failed to read clipboard")
}

/// 发送一条本地通知：标题正文先行校验；Linux 无通知守护等失败走错误分支
pub fn notify(app: &tauri::AppHandle, title: &str, body: &str) -> anyhow::Result<()> {
    use tauri_plugin_notification::NotificationExt;

    demo_features::validate_notify(title, body)?;
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .context("failed to send notification")?;
    Ok(())
}

/// 注册演示快捷键；已注册时直接成功（幂等，前端可重复点击）；
/// 触发时回一条通知做跨插件反馈，全程 Rust 侧，无需前端事件订阅权限
/// （仅桌面端，移动端直接报错）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn shortcut_register(app: &tauri::AppHandle) -> anyhow::Result<bool> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let manager = app.global_shortcut();
    if manager.is_registered(demo_features::DEMO_SHORTCUT) {
        return Ok(true);
    }
    manager
        .on_shortcut(demo_features::DEMO_SHORTCUT, |handle, _shortcut, _event| {
            use tauri_plugin_notification::NotificationExt;

            // 回调内失败只记日志：通知能力缺失不该反过来扰动快捷键注册态
            if let Err(err) = handle
                .notification()
                .builder()
                .title("demo shortcut")
                .body("shortcut pressed")
                .show()
            {
                log::warn!("failed to notify shortcut press: {err:#}");
            }
        })
        .context("failed to register demo shortcut")?;
    Ok(true)
}

/// 注册演示快捷键；已注册时直接成功（幂等，前端可重复点击）；
/// 触发时回一条通知做跨插件反馈，全程 Rust 侧，无需前端事件订阅权限
/// （仅桌面端，移动端直接报错）
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn shortcut_register(_app: &tauri::AppHandle) -> anyhow::Result<bool> {
    Err(anyhow::anyhow!(
        "global shortcut is not supported on mobile"
    ))
}

/// 查询演示快捷键是否已注册（移动端恒为 `false`，前端据此禁用注册按钮）
pub fn shortcut_is_registered(app: &tauri::AppHandle) -> bool {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;

        app.global_shortcut()
            .is_registered(demo_features::DEMO_SHORTCUT)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        false
    }
}

/// 注销演示快捷键；未注册时直接成功（幂等）
/// （仅桌面端，移动端直接报错）
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn shortcut_unregister(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    app.global_shortcut()
        .unregister(demo_features::DEMO_SHORTCUT)
        .context("failed to unregister demo shortcut")?;
    Ok(())
}

/// 注销演示快捷键；未注册时直接成功（幂等）
/// （仅桌面端，移动端直接报错）
#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn shortcut_unregister(_app: &tauri::AppHandle) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "global shortcut is not supported on mobile"
    ))
}
