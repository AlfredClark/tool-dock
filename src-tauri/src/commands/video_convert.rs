//! 视频格式转换命令：单文件转码的薄封装，逻辑在 `features::video_convert`，
//! 二进制路径经 `cores::ffmpeg` 解析。文件接入复用元数据侧命令
//! （`expand_dropped_video_paths` / `get_video_dir` / `read_video_metadata` /
//! `get_video_thumbnail`），此处只新增实际转码命令。

use crate::cores::ffmpeg;
use crate::cores::types::{CommandError, CommandResult};
use crate::features::video_convert::{VideoConvertOptions, VideoConvertOutcome};

/// 转换单文件视频到目标容器；输出路径由后端按输出目录 + 原名计算，
/// 单文件失败装进 `VideoConvertOutcome` 跳过继续（前端逐项调用以驱动进度）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn convert_video_format(
    app: tauri::AppHandle,
    input: String,
    options: VideoConvertOptions,
) -> CommandResult<VideoConvertOutcome> {
    let bins = ffmpeg::resolve_binaries(&app)?;
    let ffmpeg = bins.ffmpeg;
    tauri::async_runtime::spawn_blocking(move || {
        crate::features::video_convert::convert_video_format(&ffmpeg, &input, &options)
    })
    .await
    .map_err(|err| CommandError::Internal(format!("video convert task failed: {err}")))
}
