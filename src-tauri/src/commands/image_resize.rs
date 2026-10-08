//! 图片尺寸命令（工具路由 `image/resize`）：元信息 / 缩略图 / 路径展开 / 系统图片目录 /
//! 单张缩放薄封装，逻辑在 `features`。

use crate::cores::types::{CommandError, CommandResult};
use crate::features::image_resize::{
    ExpandDropOutcome, ImageInfo, ResizeOptions, ResizeSingleOutcome,
};

/// 读取单张图片元信息（尺寸/格式/大小）；路径来自拖放或对话框，后端二次校验。
/// 文件打不开抛错走 `CommandError`，前端按单张标红处理，不中断批量。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn read_image_info(path: String) -> CommandResult<ImageInfo> {
    Ok(crate::features::image_resize::read_image_info(&path)?)
}

/// 读取预览缩略图：等比压到 `max_side` 内，以 `data:` URL 返回（前端 `<img>` 直显）。
/// `asset` 协议需配 scope 才放行本地路径，回 data URL 可保持最小授权。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_image_thumbnail(path: String, max_side: u32) -> CommandResult<String> {
    Ok(crate::features::image_resize::read_image_thumbnail(
        &path, max_side,
    )?)
}
/// 展开拖放路径：文件直通 + 文件夹展平为图片 + 首个文件夹的 `resized/` 建议。
/// 枚举失败抛错走 `CommandError`，前端回落原路径逐张处理。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn expand_dropped_paths(paths: Vec<String>) -> CommandResult<ExpandDropOutcome> {
    Ok(crate::features::image_resize::expand_dropped_paths(&paths)?)
}

/// 系统图片目录：输出目录的默认值；取不到返回 `None`，前端保持空（不阻断）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_picture_dir(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    use tauri::Manager;

    Ok(app
        .path()
        .picture_dir()
        .ok()
        .and_then(|path| path.to_str().map(str::to_string)))
}

/// 执行单张缩放；前端逐张调用以驱动进度，单张失败装进 `ResizeSingleOutcome` 跳过继续。
/// CPU 密集活经 `spawn_blocking` 隔离：同步解码/Lanczos3/编码若直接跑在异步运行时上，
/// 大图批量时会饿死执行器线程，窗口拖拽等主循环事件跟着卡顿。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn resize_image(
    input: String,
    options: ResizeOptions,
) -> CommandResult<ResizeSingleOutcome> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::features::image_resize::resize_image(&input, &options)
    })
    .await
    .map_err(|err| CommandError::Internal(format!("resize task failed: {err}")))
}
