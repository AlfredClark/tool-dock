//! 视频引擎命令：状态查询与托管版下载安装的薄封装，逻辑在 `cores::ffmpeg`。

use crate::cores::ffmpeg::{self, FfmpegStatus};
use crate::cores::types::{CommandError, CommandResult};

/// 查询 ffmpeg/ffprobe 可用性与版本；永不抛错，不可用即 `available: false`
/// （前端据此展示下载入口）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn get_ffmpeg_status(app: tauri::AppHandle) -> CommandResult<FfmpegStatus> {
    Ok(ffmpeg::query_status(&app))
}

/// 确保可用：已可用直接返回状态，否则下载托管版（进度经事件推送）后复查返回；
/// 网络 + 解包密集活经 `spawn_blocking` 隔离，不饿死异步运行时（同 `resize_image`）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn ensure_ffmpeg(app: tauri::AppHandle) -> CommandResult<FfmpegStatus> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || ffmpeg::ensure_binaries(&handle))
        .await
        .map_err(|err| CommandError::Internal(format!("ffmpeg task failed: {err}")))??;
    Ok(ffmpeg::query_status(&app))
}

/// 强制重装托管版：FFmpeg 管理页修复入口，无视系统版，清空后全量重装；
/// 调度语义同 `ensure_ffmpeg`
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn reinstall_managed_ffmpeg(app: tauri::AppHandle) -> CommandResult<FfmpegStatus> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || ffmpeg::reinstall_managed(&handle))
        .await
        .map_err(|err| CommandError::Internal(format!("ffmpeg task failed: {err}")))??;
    Ok(ffmpeg::query_status(&app))
}
