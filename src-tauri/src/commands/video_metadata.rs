//! 视频元数据命令：单文件读取 / 海报帧 / 标签写入 / 路径展开 / 系统视频目录的
//! 薄封装，逻辑在 `features::video_metadata`，二进制路径经 `cores::ffmpeg` 解析。

use crate::cores::ffmpeg;
use crate::cores::types::{CommandError, CommandResult};
use crate::features::image_resize::ExpandDropOutcome;
use crate::features::video_metadata::{
    VideoApplyOptions, VideoApplyOutcome, VideoBatchItem, VideoFileMetadata,
};

/// 子线程执行同步阻塞活（进程拉起 + 流式拷贝），不饿死异步运行时（同 `resize_image`）
async fn run_blocking<T>(
    task: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> CommandResult<T>
where
    T: Send + 'static,
{
    Ok(tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|err| CommandError::Internal(format!("video metadata task failed: {err}")))??)
}

/// 读取单文件元数据（容器/时长/大小/首视频流/标签）；路径来自对话框，后端校验扩展名
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn read_video_metadata(
    app: tauri::AppHandle,
    path: String,
) -> CommandResult<VideoFileMetadata> {
    let ffprobe = ffmpeg::resolve_binaries(&app)?.ffprobe;
    run_blocking(move || crate::features::video_metadata::read_video_metadata(&ffprobe, &path))
        .await
}

/// 读取海报帧：`seek_seconds` 处单帧 PNG，以 `data:` URL 返回（前端 `<img>` 直显，
/// 与图片缩略图同最小授权口径）；失败前端降级为空预览，不降级条目
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn get_video_thumbnail(
    app: tauri::AppHandle,
    path: String,
    max_side: u32,
    seek_seconds: f64,
) -> CommandResult<String> {
    let ffmpeg = ffmpeg::resolve_binaries(&app)?.ffmpeg;
    run_blocking(move || {
        crate::features::video_metadata::read_video_thumbnail(
            &ffmpeg,
            &path,
            max_side,
            seek_seconds,
        )
    })
    .await
}

/// 写入单文件标签；输出路径由后端按输出目录 + 原名计算，单文件失败装进
/// `VideoApplyOutcome` 跳过继续（前端逐项调用以驱动进度）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn apply_video_metadata(
    app: tauri::AppHandle,
    input: String,
    options: VideoApplyOptions,
) -> CommandResult<VideoApplyOutcome> {
    let bins = ffmpeg::resolve_binaries(&app)?;
    let (ffmpeg, ffprobe) = (bins.ffmpeg, bins.ffprobe);
    tauri::async_runtime::spawn_blocking(move || {
        crate::features::video_metadata::apply_video_metadata(&ffmpeg, &ffprobe, &input, &options)
    })
    .await
    .map_err(|err| CommandError::Internal(format!("video metadata task failed: {err}")))
}

/// 批量读取元信息 + 列表小海报：一次调用返回整块结果（顺序与入参一致），单项失败
/// 装进条目不中断其余。前端按 200 切块调用，后端文件间 4 线程并发；768 大海报仍走
/// 单文件命令按选中项懒加载。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn read_video_batch(
    app: tauri::AppHandle,
    paths: Vec<String>,
    list_side: u32,
) -> CommandResult<Vec<VideoBatchItem>> {
    let bins = ffmpeg::resolve_binaries(&app)?;
    let (ffmpeg, ffprobe) = (bins.ffmpeg, bins.ffprobe);
    run_blocking(move || {
        crate::features::video_metadata::read_video_batch(&ffmpeg, &ffprobe, &paths, list_side)
    })
    .await
}

/// 展开拖放路径：文件直通 + 文件夹展平为视频 + 首个文件夹的 `edited/` 建议。
/// 枚举失败抛错走 `CommandError`，前端回落原路径逐项处理。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn expand_dropped_video_paths(paths: Vec<String>) -> CommandResult<ExpandDropOutcome> {
    Ok(crate::features::video_metadata::expand_dropped_video_paths(
        &paths,
    )?)
}

/// 系统视频目录：输出目录的默认值；取不到返回 `None`，前端保持空（不阻断）。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_video_dir(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    use tauri::Manager;

    Ok(app
        .path()
        .video_dir()
        .ok()
        .and_then(|path| path.to_str().map(str::to_string)))
}
