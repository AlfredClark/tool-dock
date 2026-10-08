//! 视频元数据编辑业务逻辑（工具路由 `video/metadata`）：标签模型 + ffprobe 解析 +
//! ffmpeg 参数组装与执行的纯函数实现（`std::process` 直调，不依赖 Tauri 运行时）。
//!
//! 二进制路径由调用方经 `cores::ffmpeg` 解析后传入；首版字段锁定通用标题系六项
//! （标题/作者/专辑/流派/日期/注释），封面图放二期
//! （各容器的挂载图语义差异大，单独一轮）。
//! 输出路径由后端按输出目录 + 原名计算（`overwrite` 三策略与图片侧同语义），
//! 前端只下发目录与策略，不拼路径。

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::features::image_resize::{ExpandDropOutcome, OverwritePolicy};

/// 支持的视频扩展名：与文件对话框过滤器同源（对话框接线时复用）
pub const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "mkv", "webm", "avi"];

/// 无后缀封面名的尝试后缀（按序首个存在即用；显式后缀不在此限，见 `COVER_IMAGE_EXTENSIONS`）
const COVER_PROBE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

/// 封面图允许的显式后缀（解码校验由 `image` 承担，此处只做门禁）
const COVER_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

/// 单个标签值上限：500 字符，超限视为误填直接拒绝
pub const MAX_TAG_CHARS: usize = 500;

/// 海报 PNG 上限：20MiB，超限视为坏帧直接拒绝
const MAX_THUMBNAIL_BYTES: usize = 20 * 1024 * 1024;

/// 通用标题系标签：`None` 即保持原值不动，`Some("")` 即清空该标签
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub comment: Option<String>,
}

impl VideoTags {
    /// 是否空标签集（无操作即误调用，argv 组装时拒绝）
    pub const fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.genre.is_none()
            && self.date.is_none()
            && self.comment.is_none()
    }

    /// 字段迭代：`(ffmpeg 键名, 值)`，`None` 跳过
    fn entries(&self) -> Vec<(&'static str, &str)> {
        [
            ("title", self.title.as_deref()),
            ("artist", self.artist.as_deref()),
            ("album", self.album.as_deref()),
            ("genre", self.genre.as_deref()),
            ("date", self.date.as_deref()),
            ("comment", self.comment.as_deref()),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|text| (key, text)))
        .collect()
    }

    /// 标签校验：NUL（argv 传参会直接失败）/ 换行（部分播放器截断）/ 超长拒绝；
    /// 键名映射交给 ffmpeg 按容器归一化，此处不做容器特判
    pub fn validate(&self) -> anyhow::Result<()> {
        for (key, text) in self.entries() {
            if text.contains('\0') {
                anyhow::bail!("tag {key} contains NUL byte");
            }
            if text.contains('\n') || text.contains('\r') {
                anyhow::bail!("tag {key} contains line break");
            }
            if text.chars().count() > MAX_TAG_CHARS {
                anyhow::bail!("tag {key} exceeds {MAX_TAG_CHARS} chars");
            }
        }
        Ok(())
    }
}

/// 首个视频流信息：占位符 `%width%`/`%height%` 与中栏技术行的数据源；
/// 无视频流（如纯音频 MP4）即 `None`，占位符展开时按未知处理
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoStreamInfo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec_name: Option<String>,
}

/// 单文件元数据读取结果：`ffprobe -print_format json -show_format -show_streams` 的子集
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
pub struct VideoFileMetadata {
    /// 容器名原文（如 `mov,mp4,m4a,3gp,3g2,mj2`），展示层自行简化
    pub format_name: Option<String>,
    /// 时长秒数
    pub duration_seconds: Option<f64>,
    /// 文件字节数（展示用，超 4GiB 饱和为 `u32::MAX`；`specta` 禁止导出 `u64`）
    pub file_size: Option<u32>,
    pub stream: Option<VideoStreamInfo>,
    /// 是否已有封面（`attached_pic` 视频流或图片附件任一存在）
    pub has_cover: bool,
    pub tags: VideoTags,
}

/// 业务错误码：前端按码映射 i18n 文案，`message` 只做诊断补充
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum VideoMetadataErrorCode {
    UnsupportedFormat,
    InvalidTags,
    OutputNotWritable,
    FfmpegFailed,
    Skipped,
}

/// 业务错误体：扁平结构便于 `specta` 导出，前端按 `code` 分支（与图片侧同约定）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoMetadataError {
    pub code: VideoMetadataErrorCode,
    pub message: Option<String>,
}

/// 单文件写入结果：`ok` 为真读 `output`，为假读 `error`（失败跳过继续，与图片侧同约定）；
/// 封面 outcome 单独表达（`cover` 跳过不影响 `ok`，标签照写）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoApplyOutcome {
    pub ok: bool,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<VideoMetadataError>,
    pub cover: CoverResult,
}

impl VideoApplyOutcome {
    const fn success(input: String, output: String, cover: CoverResult) -> Self {
        Self {
            ok: true,
            input,
            output: Some(output),
            error: None,
            cover,
        }
    }

    fn failure(
        input: &str,
        code: VideoMetadataErrorCode,
        message: impl Into<String>,
        cover: CoverResult,
    ) -> Self {
        Self {
            ok: false,
            input: input.to_owned(),
            output: None,
            error: Some(VideoMetadataError {
                code,
                message: Some(message.into()),
            }),
            cover,
        }
    }
}

/// 封面意图（前端展开后下发）：`file` 为展开后的文件名（相对视频同目录、绝对均可，
/// 无后缀自动识别），`None` 即保持；`clear` 置位即清除（与 `file` 互斥，置位优先）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CoverSpec {
    pub file: Option<String>,
    pub clear: bool,
}

/// 封面结果：跳过类均不影响 `ok`（标签照写），前端按值展示备注
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CoverResult {
    /// 未动（保持原封面）
    #[default]
    Kept,
    /// 已嵌入
    Embedded,
    /// 已清除
    Cleared,
    /// 无文件可嵌（模板解析无命中）
    SkippedNoFile,
    /// 容器不支持封面（如 AVI）
    SkippedUnsupported,
    /// 封面文件损坏不可解码
    SkippedInvalid,
}

/// 单文件写入选项（前端参数面板全量下发；输出路径由后端按输出目录 + 原名计算）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoApplyOptions {
    pub tags: VideoTags,
    pub output_dir: String,
    pub overwrite: OverwritePolicy,
    pub cover: CoverSpec,
}

/// 解析 ffprobe JSON：缺 key 即 `None` 不抛错（各容器标签完备度本就参差）；
/// 标签键大小写不敏感（MKV 为全大写 `TITLE`，MP4/AVI 为小写），未知键忽略；
/// 视频流取首个 `codec_type == "video"` 者（纯音频文件即无流）
pub fn parse_ffprobe_output(json: &str) -> anyhow::Result<VideoFileMetadata> {
    let value: serde_json::Value = serde_json::from_str(json).context("invalid ffprobe json")?;
    let format = value
        .get("format")
        .context("ffprobe output missing format")?;
    let tags = format
        .get("tags")
        .and_then(serde_json::Value::as_object)
        .map_or_else(VideoTags::default, |map| {
            let mut collected = VideoTags::default();
            for (key, text) in map {
                let Some(text) = text.as_str() else {
                    continue;
                };
                match key.to_lowercase().as_str() {
                    "title" => collected.title = Some(text.to_owned()),
                    "artist" => collected.artist = Some(text.to_owned()),
                    "album" => collected.album = Some(text.to_owned()),
                    "genre" => collected.genre = Some(text.to_owned()),
                    "date" => collected.date = Some(text.to_owned()),
                    "comment" => collected.comment = Some(text.to_owned()),
                    _ => {}
                }
            }
            collected
        });
    let streams = value.get("streams").and_then(serde_json::Value::as_array);
    let stream = streams
        .and_then(|streams| {
            streams
                .iter()
                .find(|stream| {
                    stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
                })
                .or_else(|| streams.first())
        })
        .filter(|stream| {
            stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
        })
        .map(|stream| VideoStreamInfo {
            width: stream
                .get("width")
                .and_then(serde_json::Value::as_u64)
                .and_then(|width| u32::try_from(width).ok()),
            height: stream
                .get("height")
                .and_then(serde_json::Value::as_u64)
                .and_then(|height| u32::try_from(height).ok()),
            codec_name: stream
                .get("codec_name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        });
    Ok(VideoFileMetadata {
        format_name: format
            .get("format_name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        duration_seconds: format.get("duration").and_then(|duration| {
            duration
                .as_str()
                .and_then(|text| text.parse::<f64>().ok())
                .or_else(|| duration.as_f64())
        }),
        file_size: format.get("size").and_then(|size| {
            size.as_str()
                .and_then(|text| text.parse::<u64>().ok())
                .or_else(|| size.as_u64())
                .and_then(|bytes| u32::try_from(bytes).ok().or(Some(u32::MAX)))
        }),
        stream,
        has_cover: streams.is_some_and(|streams| streams.iter().any(stream_is_cover)),
        tags,
    })
}

/// 是否为封面流：`attached_pic` 视频流，或图片附件（MKV/WebM 标准附件机制）；
/// ffmpeg 会把 MKV 图片附件误识别为 mjpeg 视频流（无 `disposition`），此时按 mimetype 回补
fn stream_is_cover(stream: &serde_json::Value) -> bool {
    if is_image_mimetype(stream) {
        return true;
    }
    let codec_type = stream.get("codec_type").and_then(serde_json::Value::as_str);
    if codec_type == Some("video") {
        return stream
            .get("disposition")
            .and_then(|disposition| disposition.get("attached_pic"))
            .and_then(serde_json::Value::as_u64)
            == Some(1);
    }
    false
}

/// 流标签的 mimetype 是否为图片
fn is_image_mimetype(stream: &serde_json::Value) -> bool {
    stream
        .get("tags")
        .and_then(|tags| tags.get("mimetype"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mimetype| mimetype.to_lowercase().starts_with("image/"))
}

/// 组装标签参数片段（纯函数）：`-metadata k=v…` 键值对；空标签集即空片段
/// （清除场景无标签也合法，调用方 `apply` 的无操作门禁另行拦截）
fn tag_args(tags: &VideoTags) -> anyhow::Result<Vec<String>> {
    tags.validate()?;
    let mut args = Vec::new();
    for (key, text) in tags.entries() {
        args.push("-metadata".to_owned());
        args.push(format!("{key}={text}"));
    }
    Ok(args)
}

/// 取文件扩展名（小写）：白名单校验用
fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// 扩展名是否在视频白名单内
fn is_supported_video(path: &str) -> bool {
    SUPPORTED_VIDEO_EXTENSIONS.contains(&extension_of(path).as_str())
}

/// 读取单文件元数据：`ffprobe -v quiet -print_format json -show_format -show_streams`；
/// 非白名单扩展名直接拒绝（防误选）；ffprobe 非零退出时附 `stderr` 尾部便于排障
pub fn read_video_metadata(ffprobe: &Path, path: &str) -> anyhow::Result<VideoFileMetadata> {
    if !is_supported_video(path) {
        anyhow::bail!("unsupported video format: {path}");
    }
    let output = std::process::Command::new(ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path,
        ])
        .output()
        .with_context(|| format!("failed to run ffprobe on {path}"))?;
    if !output.status.success() {
        anyhow::bail!("ffprobe failed on {path}: {}", stderr_tail(&output.stderr));
    }
    let text = String::from_utf8(output.stdout)
        .with_context(|| format!("ffprobe output not utf-8: {path}"))?;
    parse_ffprobe_output(&text)
}

/// 读取海报帧：`seek_seconds` 处单帧 PNG，以 `data:` URL 返回（前端 `<img>` 直显，
/// 与图片缩略图同最小授权口径）；首选时刻失败回退 0 秒；输出封顶 20MiB 防坏帧刷屏
pub fn read_video_thumbnail(
    ffmpeg: &Path,
    path: &str,
    max_side: u32,
    seek_seconds: f64,
) -> anyhow::Result<String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    if !is_supported_video(path) {
        anyhow::bail!("unsupported video format: {path}");
    }
    let scale = format!("scale=w='if(gt(iw,ih),{max_side},-2)':h='if(gt(iw,ih),-2,{max_side})'");
    for seek in [seek_seconds.max(0.0), 0.0] {
        let output = std::process::Command::new(ffmpeg)
            .args([
                "-ss",
                &seek.to_string(),
                "-i",
                path,
                "-vframes",
                "1",
                "-vf",
                scale.as_str(),
                "-f",
                "image2pipe",
                "-vcodec",
                "png",
                "-",
            ])
            .output()
            .with_context(|| format!("failed to run ffmpeg on {path}"))?;
        if output.status.success() && !output.stdout.is_empty() {
            if output.stdout.len() > MAX_THUMBNAIL_BYTES {
                anyhow::bail!("thumbnail too large for {path}");
            }
            return Ok(format!(
                "data:image/png;base64,{}",
                STANDARD.encode(&output.stdout)
            ));
        }
    }
    anyhow::bail!("thumbnail failed for {path}")
}

/// 写入标签与封面：输出路径按输出目录 + 输入原名计算（同后缀，`-c copy` 不换容器），
/// `overwrite` 三策略与图片侧同语义（跳过即 `Skipped`，不算失败）；
/// 封面走独立分支（找不到/不支持/损坏一律跳过封面、标签照写，不影响 `ok`）；
/// 业务失败全部装进 `VideoApplyOutcome` 返回，不抛错（单文件失败跳过继续是常态 UI 状态）
pub fn apply_video_metadata(
    ffmpeg: &Path,
    ffprobe: &Path,
    input: &str,
    options: &VideoApplyOptions,
) -> VideoApplyOutcome {
    use VideoMetadataErrorCode::{
        FfmpegFailed, InvalidTags, OutputNotWritable, Skipped, UnsupportedFormat,
    };

    if !is_supported_video(input) {
        return VideoApplyOutcome::failure(
            input,
            UnsupportedFormat,
            format!("unsupported video format: {input}"),
            CoverResult::default(),
        );
    }
    let cover_plan = match check_apply_request(input, options) {
        Ok(plan) => plan,
        Err(outcome) => return outcome,
    };
    let output = match reserve_video_output(input, &options.output_dir, options.overwrite) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return VideoApplyOutcome::failure(
                input,
                Skipped,
                format!(
                    "output already exists: {}",
                    skip_candidate(input, &options.output_dir)
                ),
                CoverResult::default(),
            );
        }
        Err(err) => {
            return VideoApplyOutcome::failure(
                input,
                OutputNotWritable,
                format!("{err:#}"),
                CoverResult::default(),
            );
        }
    };
    // 封面分支先行结算（文件解析纯本地，不拉起进程）
    let (cover_inputs, cover_args, cover_result) = match cover_stage(
        ffmpeg,
        ffprobe,
        input,
        &output,
        &options.tags,
        options.overwrite,
        cover_plan,
    ) {
        Ok(stage) => stage,
        Err(outcome) => return outcome,
    };
    // 同路径覆写（输出目录即源目录 + 直接覆盖）：ffmpeg 禁止原地编辑，
    // 先写临时文件再原子替换；临时名保留后缀（无后缀 ffmpeg 无法判定容器）
    let work_output = work_output_for(input, &output);
    let args = match build_ffmpeg_args_covered(
        input,
        &work_output,
        &options.tags,
        options.overwrite == OverwritePolicy::Overwrite,
        &cover_inputs,
        &cover_args,
    ) {
        Ok(args) => args,
        Err(err) => {
            return VideoApplyOutcome::failure(
                input,
                InvalidTags,
                format!("{err:#}"),
                cover_result,
            );
        }
    };
    let outcome = match std::process::Command::new(ffmpeg).args(&args).output() {
        Ok(result) if result.status.success() => {
            VideoApplyOutcome::success(input.to_owned(), output.clone(), cover_result)
        }
        Ok(result) => VideoApplyOutcome::failure(
            input,
            FfmpegFailed,
            format!("ffmpeg failed: {}", stderr_tail(&result.stderr)),
            cover_result,
        ),
        Err(err) => VideoApplyOutcome::failure(
            input,
            FfmpegFailed,
            format!("failed to run ffmpeg: {err:#}"),
            cover_result,
        ),
    };
    if work_output != output {
        if outcome.ok {
            if let Err(err) = std::fs::rename(&work_output, &output) {
                return VideoApplyOutcome::failure(
                    input,
                    FfmpegFailed,
                    format!("failed to replace output: {err:#}"),
                    cover_result,
                );
            }
        } else {
            let _ = std::fs::remove_file(&work_output);
        }
    }
    outcome
}

/// 同路径覆写的工作路径：ffmpeg 禁止原地编辑，命中时返回同目录临时名
/// （保留后缀供容器判定）；不命中即原文
fn work_output_for(input: &str, output: &str) -> String {
    if Path::new(output) != Path::new(input) {
        return output.to_owned();
    }
    let stem = Path::new(output)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output");
    let extension = Path::new(output)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp4");
    Path::new(output)
        .with_file_name(format!("{stem}.tmp.tool-dock.{extension}"))
        .to_string_lossy()
        .into_owned()
}

/// 前置门禁：标签校验 + 封面意图结算 + 无操作拦截；失败直接装好 Outcome
fn check_apply_request(
    input: &str,
    options: &VideoApplyOptions,
) -> Result<CoverPlan, VideoApplyOutcome> {
    use VideoMetadataErrorCode::InvalidTags;

    if let Err(err) = options.tags.validate() {
        return Err(VideoApplyOutcome::failure(
            input,
            InvalidTags,
            format!("{err:#}"),
            CoverResult::default(),
        ));
    }
    let cover_plan = plan_cover(&options.cover);
    if options.tags.is_empty() && matches!(cover_plan, CoverPlan::Keep) {
        return Err(VideoApplyOutcome::failure(
            input,
            InvalidTags,
            "nothing to do",
            CoverResult::default(),
        ));
    }
    Ok(cover_plan)
}

/// 封面分支结算：返回待拼接的 `(cover_inputs, cover_args, cover_result)`；
/// 清除无旧封面 / 嵌入前跳过时直接回落纯标签管线，其 Outcome 即最终返回
fn cover_stage(
    ffmpeg: &Path,
    ffprobe: &Path,
    input: &str,
    output: &str,
    tags: &VideoTags,
    overwrite: OverwritePolicy,
    plan: CoverPlan,
) -> Result<(Vec<String>, Vec<String>, CoverResult), VideoApplyOutcome> {
    use CoverResult::{Cleared, Embedded, Kept};

    let plain_overwrite = overwrite == OverwritePolicy::Overwrite;
    match plan {
        CoverPlan::Keep => Ok((Vec::new(), Vec::new(), Kept)),
        CoverPlan::Clear => {
            let layout = probe_cover_layout(ffprobe, input).unwrap_or_default();
            if layout.old_cover_indices.is_empty() {
                // 无旧封面即无操作成功（AVI 等无封面容器同样落此分支）
                return Err(run_tags_only(
                    ffmpeg,
                    input,
                    output,
                    tags,
                    plain_overwrite,
                    Cleared,
                ));
            }
            Ok((Vec::new(), clear_cover_args(&layout), Cleared))
        }
        CoverPlan::Embed { path } => match prepare_cover_embed(ffprobe, input, &path) {
            CoverEmbed::Ready(parts) => Ok((parts.inputs, parts.args, Embedded)),
            CoverEmbed::Skip(result) => Err(run_tags_only(
                ffmpeg,
                input,
                output,
                tags,
                plain_overwrite,
                result,
            )),
        },
    }
}

/// 纯标签 fallback：封面保持/跳过时走原标签管线（空标签在清除场景合法，此处经
/// `covered` 构建器放行，与无封面历史行为一致）
fn run_tags_only(
    ffmpeg: &Path,
    input: &str,
    output: &str,
    tags: &VideoTags,
    overwrite: bool,
    cover: CoverResult,
) -> VideoApplyOutcome {
    use VideoMetadataErrorCode::{FfmpegFailed, InvalidTags};

    let args = match build_ffmpeg_args_covered(input, output, tags, overwrite, &[], &[]) {
        Ok(args) => args,
        Err(err) => {
            return VideoApplyOutcome::failure(input, InvalidTags, format!("{err:#}"), cover);
        }
    };
    match std::process::Command::new(ffmpeg).args(&args).output() {
        Ok(result) if result.status.success() => {
            VideoApplyOutcome::success(input.to_owned(), output.to_owned(), cover)
        }
        Ok(result) => VideoApplyOutcome::failure(
            input,
            FfmpegFailed,
            format!("ffmpeg failed: {}", stderr_tail(&result.stderr)),
            cover,
        ),
        Err(err) => VideoApplyOutcome::failure(
            input,
            FfmpegFailed,
            format!("failed to run ffmpeg: {err:#}"),
            cover,
        ),
    }
}

/// 封面容器：MP4 系走 `attached_pic` 视频流，MKV/WebM 系走标准附件，其余无封面概念
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoverContainer {
    Mp4,
    Mkv,
}

/// 按输入扩展名判定封面容器：`None` 即无封面概念（AVI 等），嵌入/清除均按跳过或无操作处理
fn cover_container(input: &str) -> Option<CoverContainer> {
    match extension_of(input).as_str() {
        "mp4" | "m4v" | "mov" => Some(CoverContainer::Mp4),
        "mkv" | "webm" => Some(CoverContainer::Mkv),
        _ => None,
    }
}

/// 封面执行计划（文件解析后）：保持 / 清除 / 嵌入（路径为展开后的模板值）
enum CoverPlan {
    Keep,
    Clear,
    Embed { path: String },
}

/// 封面意图结算：清除优先于文件；无有效文件即保持
fn plan_cover(spec: &CoverSpec) -> CoverPlan {
    if spec.clear {
        return CoverPlan::Clear;
    }
    match spec
        .file
        .as_deref()
        .map(str::trim)
        .filter(|file| !file.is_empty())
    {
        Some(_) => CoverPlan::Embed {
            path: spec.file.clone().unwrap_or_default(),
        },
        None => CoverPlan::Keep,
    }
}

/// 旧封面版式：需剔除的旧封面流索引 + 新封面序号推导用的计数
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CoverLayout {
    /// 需剔除的旧封面流全局索引（MP4 系 `attached_pic` 视频流，MKV 系图片附件）
    old_cover_indices: Vec<u32>,
    /// 视频流总数（MP4 新封面序号用）
    video_count: usize,
    /// 附件总数（MKV 新附件序号用）
    attachment_count: usize,
}

/// 解析版式（纯函数，单测覆盖）：从 `show_streams` 数组提取旧封面索引与计数
fn parse_cover_layout(streams: &serde_json::Value) -> CoverLayout {
    let mut layout = CoverLayout::default();
    let Some(entries) = streams.as_array() else {
        return layout;
    };
    for stream in entries {
        let codec_type = stream.get("codec_type").and_then(serde_json::Value::as_str);
        let index = stream
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .and_then(|index| u32::try_from(index).ok());
        match codec_type {
            Some("video") => {
                layout.video_count += 1;
                let attached = stream
                    .get("disposition")
                    .and_then(|disposition| disposition.get("attached_pic"))
                    .and_then(serde_json::Value::as_u64)
                    == Some(1);
                if attached && let Some(index) = index {
                    layout.old_cover_indices.push(index);
                }
            }
            Some("attachment") => {
                layout.attachment_count += 1;
                if stream_is_cover(stream)
                    && let Some(index) = index
                {
                    layout.old_cover_indices.push(index);
                }
            }
            _ => {}
        }
    }
    layout
}

/// 探测旧封面版式：失败回落空版式（不阻断主流程；嵌入可能多带旧封面，清除退化为无操作）
fn probe_cover_layout(ffprobe: &Path, input: &str) -> anyhow::Result<CoverLayout> {
    let output = std::process::Command::new(ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            input,
        ])
        .output()
        .with_context(|| format!("failed to probe {input}"))?;
    if !output.status.success() {
        anyhow::bail!("ffprobe failed on {input}");
    }
    let value: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .context("invalid ffprobe json")?;
    Ok(value
        .get("streams")
        .map_or_else(CoverLayout::default, parse_cover_layout))
}

/// 封面文件解析：绝对路径直接用；相对路径以输入视频所在目录为基准；
/// 无后缀按 `jpg→jpeg→png` 首个存在者；命中后只做存在性判定（门禁与解码在后）
fn resolve_cover_file(input: &str, file: &str) -> Option<PathBuf> {
    let candidate = Path::new(file);
    if candidate.is_absolute() {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }
    let base = Path::new(input)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let base = base.map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    if extension_of(file).is_empty() {
        return COVER_PROBE_EXTENSIONS.iter().find_map(|extension| {
            let probe = base.join(format!("{file}.{extension}"));
            probe.is_file().then_some(probe)
        });
    }
    let probe = base.join(file);
    probe.is_file().then_some(probe)
}

/// 嵌入就绪态：额外输入与输出参数均已组装
struct CoverArgs {
    inputs: Vec<String>,
    args: Vec<String>,
}

/// 嵌入准备结果：就绪或跳过（跳过携带原因，调用方回落纯标签管线）
enum CoverEmbed {
    Ready(CoverArgs),
    Skip(CoverResult),
}

/// 嵌入准备：容器门禁 → 文件解析 → 后缀门禁 → 解码校验 → 参数组装
fn prepare_cover_embed(ffprobe: &Path, input: &str, file: &str) -> CoverEmbed {
    use CoverResult::{SkippedInvalid, SkippedNoFile, SkippedUnsupported};

    let Some(container) = cover_container(input) else {
        return CoverEmbed::Skip(SkippedUnsupported);
    };
    let Some(cover) = resolve_cover_file(input, file) else {
        return CoverEmbed::Skip(SkippedNoFile);
    };
    if !COVER_IMAGE_EXTENSIONS.contains(&extension_of(&cover.to_string_lossy()).as_str()) {
        return CoverEmbed::Skip(SkippedInvalid);
    }
    // 解码校验（`image` 全量解码，损坏即拒收）
    let decodable = (|| {
        let reader = image::ImageReader::open(&cover).ok()?;
        let reader = reader.with_guessed_format().ok()?;
        reader.decode().ok()
    })()
    .is_some();
    if !decodable {
        return CoverEmbed::Skip(SkippedInvalid);
    }
    let layout = probe_cover_layout(ffprobe, input).unwrap_or_default();
    CoverEmbed::Ready(cover_ffmpeg_parts(&cover, container, &layout))
}

/// 封面 ffmpeg 片段（纯函数，单测锁定 argv 形状）：
/// MP4 系追加第二输入 + 末视频流 `attached_pic`；MKV 系 `-attach` + 新附件元信息
/// （只标新附件的 mimetype，字体等其它附件不动）
fn cover_ffmpeg_parts(cover: &Path, container: CoverContainer, layout: &CoverLayout) -> CoverArgs {
    let cover_text = cover.to_string_lossy().into_owned();
    let drops: Vec<String> = layout
        .old_cover_indices
        .iter()
        .flat_map(|index| ["-map".to_owned(), format!("-0:{index}")])
        .collect();
    match container {
        CoverContainer::Mp4 => {
            // 新封面为输出末视频流：已映射视频数即其序号
            let dropped = layout.old_cover_indices.len().min(layout.video_count);
            let fresh = layout.video_count.saturating_sub(dropped);
            let codec = if extension_of(&cover_text) == "png" {
                "png"
            } else {
                "mjpeg"
            };
            let mut args = vec!["-map".to_owned(), "0".to_owned()];
            args.extend(drops);
            args.extend(["-map".to_owned(), "1".to_owned()]);
            args.extend(["-c".to_owned(), "copy".to_owned()]);
            args.extend([
                format!("-c:v:{fresh}"),
                codec.to_owned(),
                format!("-disposition:v:{fresh}"),
                "attached_pic".to_owned(),
            ]);
            CoverArgs {
                inputs: vec![cover_text],
                args,
            }
        }
        CoverContainer::Mkv => {
            // 新附件为输出末附件：总数减已删即其序号
            let fresh = layout
                .attachment_count
                .saturating_sub(layout.old_cover_indices.len());
            let mimetype = if extension_of(&cover_text) == "png" {
                "image/png"
            } else {
                "image/jpeg"
            };
            let mut args = vec![
                "-attach".to_owned(),
                cover_text.clone(),
                "-map".to_owned(),
                "0".to_owned(),
            ];
            args.extend(drops);
            args.extend(["-c".to_owned(), "copy".to_owned()]);
            args.extend([
                "-metadata:s:t".to_owned(),
                format!("mimetype={mimetype}"),
                format!("-metadata:s:t:{fresh}"),
                format!("filename=cover.{}", extension_of(&cover_text)),
            ]);
            CoverArgs {
                inputs: Vec::new(),
                args,
            }
        }
    }
}

/// 清除参数（纯函数）：剔除旧封面流；调用方保证非空（空即无操作成功，不走此分支）
fn clear_cover_args(layout: &CoverLayout) -> Vec<String> {
    let mut args = vec!["-map".to_owned(), "0".to_owned()];
    for index in &layout.old_cover_indices {
        args.extend(["-map".to_owned(), format!("-0:{index}")]);
    }
    args.extend(["-c".to_owned(), "copy".to_owned()]);
    args
}

/// 组装完整 ffmpeg 参数：`-i 输入 [-i 封面] -map… -c copy [-metadata…] -y/-n 输出`，
/// 流式拷贝不重编码；全部 argv 传参，无 shell 拼接，标签含特殊字符也安全
fn build_ffmpeg_args_covered(
    input: &str,
    output: &str,
    tags: &VideoTags,
    overwrite: bool,
    cover_inputs: &[String],
    cover_args: &[String],
) -> anyhow::Result<Vec<String>> {
    if input.is_empty() || output.is_empty() {
        anyhow::bail!("input and output must not be empty");
    }
    let mut args = vec!["-i".to_owned(), input.to_owned()];
    for extra in cover_inputs {
        args.extend(["-i".to_owned(), extra.clone()]);
    }
    if cover_args.is_empty() {
        args.extend([
            "-c".to_owned(),
            "copy".to_owned(),
            "-map".to_owned(),
            "0".to_owned(),
        ]);
    } else {
        // 封面片段自带 `-map` 与 `-c copy`（MP4 含第二输入映射，MKV 含 `-attach`）
        args.extend(cover_args.iter().cloned());
    }
    args.extend(tag_args(tags)?);
    args.push(if overwrite { "-y" } else { "-n" }.to_owned());
    args.push(output.to_owned());
    Ok(args)
}

/// 预留输出路径：输出目录自动建，`Overwrite` 直用首候选，`Skip` 遇存在即 `None`，
/// `Increment` 按 `名-2.后缀` 递增（与图片侧同规则）
fn reserve_video_output(
    input: &str,
    output_dir: &str,
    overwrite: OverwritePolicy,
) -> anyhow::Result<Option<String>> {
    let out_dir = Path::new(output_dir);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("cannot create output dir: {output_dir}"))?;
    let file_name = Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .with_context(|| format!("invalid file name: {input}"))?;
    let first = out_dir.join(file_name);
    if overwrite == OverwritePolicy::Overwrite {
        return first
            .to_str()
            .map(str::to_string)
            .with_context(|| "output path is not valid UTF-8".to_string())
            .map(Some);
    }
    if overwrite == OverwritePolicy::Skip && first.exists() {
        return Ok(None);
    }
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output");
    let extension = Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp4");
    let mut candidate = first;
    let mut index = 1;
    while candidate.exists() {
        index += 1;
        candidate = out_dir.join(format!("{stem}-{index}.{extension}"));
    }
    candidate
        .to_str()
        .map(str::to_string)
        .with_context(|| "output path is not valid UTF-8".to_string())
        .map(Some)
}

/// 跳过诊断用的首候选路径：与 `reserve_video_output` 同规则组装（仅展示，不预留）
fn skip_candidate(input: &str, output_dir: &str) -> String {
    Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(
            || output_dir.to_owned(),
            |name| {
                Path::new(output_dir)
                    .join(name)
                    .to_string_lossy()
                    .into_owned()
            },
        )
}

/// 展开拖放路径：不存在/非视频文件原样透传（前端按单项标红），
/// 文件夹展开为顶层视频文件（按文件名排序，不递归，避免误扫深层目录）；
/// 首个文件夹给出 `edited/` 建议输出目录（复用图片侧结果类型）
pub fn expand_dropped_video_paths(paths: &[String]) -> anyhow::Result<ExpandDropOutcome> {
    let mut files = Vec::new();
    let mut output_dir = None;
    for path in paths {
        let candidate = Path::new(path);
        if !candidate.is_dir() {
            files.push(path.clone());
            continue;
        }
        let mut inner: Vec<String> = std::fs::read_dir(candidate)
            .with_context(|| format!("cannot list directory: {path}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|child| child.is_file() && is_supported_video(&child.to_string_lossy()))
            .filter_map(|child| child.to_str().map(str::to_string))
            .collect();
        inner.sort();
        files.extend(inner);
        if output_dir.is_none() {
            output_dir = candidate.join("edited").to_str().map(str::to_string);
        }
    }
    Ok(ExpandDropOutcome { files, output_dir })
}

/// 取 `stderr` 末 2KB：既保留诊断信息，又防损坏文件刷出海量日志
fn stderr_tail(stderr: &[u8]) -> String {
    const TAIL_BYTES: usize = 2048;
    let tail = if stderr.len() > TAIL_BYTES {
        &stderr[stderr.len() - TAIL_BYTES..]
    } else {
        stderr
    };
    String::from_utf8_lossy(tail).trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mp4_style_lowercase_tags() {
        let metadata = parse_ffprobe_output(
            r#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"12.500","size":"4525","tags":{"title":"片名","artist":"作者"}},"streams":[{"codec_type":"video","codec_name":"mpeg4","width":128,"height":128}]}"#,
        )
        .unwrap();
        assert_eq!(
            metadata.format_name.as_deref(),
            Some("mov,mp4,m4a,3gp,3g2,mj2")
        );
        assert_eq!(metadata.duration_seconds, Some(12.5));
        assert_eq!(metadata.file_size, Some(4525));
        assert_eq!(
            metadata.stream,
            Some(VideoStreamInfo {
                width: Some(128),
                height: Some(128),
                codec_name: Some("mpeg4".to_owned()),
            })
        );
        assert_eq!(metadata.tags.title.as_deref(), Some("片名"));
        assert_eq!(metadata.tags.artist.as_deref(), Some("作者"));
        assert_eq!(metadata.tags.album, None);
    }

    #[test]
    fn parses_mkv_style_uppercase_tags_and_numeric_duration() {
        let metadata = parse_ffprobe_output(
            r#"{"format":{"format_name":"matroska,webm","duration":60.0,"tags":{"TITLE":"大写标题","DATE":"2024"}},"streams":[{"codec_type":"audio","codec_name":"opus"}]}"#,
        )
        .unwrap();
        assert_eq!(metadata.tags.title.as_deref(), Some("大写标题"));
        assert_eq!(metadata.tags.date.as_deref(), Some("2024"));
        assert_eq!(metadata.duration_seconds, Some(60.0));
        // 无视频流即无流信息（纯音频文件）
        assert_eq!(metadata.stream, None);
    }

    #[test]
    fn tolerates_missing_format_fields() {
        let metadata = parse_ffprobe_output("{\"format\":{}}").unwrap();
        assert_eq!(metadata, VideoFileMetadata::default());
    }

    #[test]
    fn rejects_output_without_format() {
        assert!(parse_ffprobe_output("{}").is_err());
        assert!(parse_ffprobe_output("not json").is_err());
    }

    #[test]
    fn builds_copy_args_with_overwrite_flag() {
        let tags = VideoTags {
            title: Some("新标题".to_owned()),
            artist: Some("作者".to_owned()),
            ..VideoTags::default()
        };
        assert_eq!(
            build_ffmpeg_args_covered("in.mp4", "out.mp4", &tags, true, &[], &[]).unwrap(),
            vec![
                "-i",
                "in.mp4",
                "-c",
                "copy",
                "-map",
                "0",
                "-metadata",
                "title=新标题",
                "-metadata",
                "artist=作者",
                "-y",
                "out.mp4",
            ]
        );
        let args = build_ffmpeg_args_covered("in.mp4", "out.mp4", &tags, false, &[], &[]).unwrap();
        assert!(args.contains(&"-n".to_owned()));
        assert!(!args.contains(&"-y".to_owned()));
    }

    #[test]
    fn rejects_empty_paths_and_allows_empty_tags() {
        let tags = VideoTags {
            title: Some("t".to_owned()),
            ..VideoTags::default()
        };
        // 空标签集在清除场景合法，此处只断路径门禁
        assert!(build_ffmpeg_args_covered("", "out.mp4", &tags, true, &[], &[]).is_err());
        assert!(build_ffmpeg_args_covered("in.mp4", "", &tags, true, &[], &[]).is_err());
        let cleared =
            build_ffmpeg_args_covered("in.mp4", "out.mp4", &VideoTags::default(), true, &[], &[])
                .unwrap();
        assert!(!cleared.iter().any(|arg| arg == "-metadata"));
    }

    #[test]
    fn validates_tag_values() {
        let nul = VideoTags {
            title: Some("a\0b".to_owned()),
            ..VideoTags::default()
        };
        assert!(nul.validate().is_err());
        let breaks = VideoTags {
            title: Some("a\nb".to_owned()),
            ..VideoTags::default()
        };
        assert!(breaks.validate().is_err());
        let long = VideoTags {
            title: Some("标".repeat(MAX_TAG_CHARS + 1)),
            ..VideoTags::default()
        };
        assert!(long.validate().is_err());
        let ok = VideoTags {
            title: Some(String::new()),
            ..VideoTags::default()
        };
        // 空串即清空标签，允许
        assert!(ok.validate().is_ok());
        assert!(!ok.is_empty());
        assert!(VideoTags::default().is_empty());
    }

    #[test]
    fn supported_extensions_cover_target_containers() {
        for extension in ["mp4", "m4v", "mov", "mkv", "webm", "avi"] {
            assert!(SUPPORTED_VIDEO_EXTENSIONS.contains(&extension));
        }
    }

    #[test]
    fn rejects_unsupported_extensions_before_spawning() {
        let fake = Path::new("/nonexistent/ffprobe");
        assert!(read_video_metadata(fake, "clip.txt").is_err());
        assert!(read_video_thumbnail(fake, "clip.txt", 768, 1.0).is_err());
    }

    #[test]
    fn stderr_tail_keeps_last_bytes() {
        assert_eq!(stderr_tail(b"abc"), "abc");
        let long = vec![b'x'; 3000];
        assert_eq!(stderr_tail(&long).len(), 2048);
    }

    #[test]
    fn apply_reports_business_outcomes_without_spawning() {
        let fake = Path::new("/nonexistent/ffmpeg");
        let tags = VideoTags {
            title: Some("t".to_owned()),
            ..VideoTags::default()
        };
        let options = |overwrite| VideoApplyOptions {
            tags: tags.clone(),
            output_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            overwrite,
            cover: CoverSpec::default(),
        };
        // 非白名单扩展名直接拒收（不拉起进程）
        let outcome =
            apply_video_metadata(fake, fake, "clip.txt", &options(OverwritePolicy::Overwrite));
        assert!(!outcome.ok);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoMetadataErrorCode::UnsupportedFormat)
        );
        // 空标签集 + 保持封面即无操作，拒绝
        let outcome = apply_video_metadata(
            fake,
            fake,
            "clip.mp4",
            &VideoApplyOptions {
                tags: VideoTags::default(),
                output_dir: std::env::temp_dir().to_string_lossy().into_owned(),
                overwrite: OverwritePolicy::Overwrite,
                cover: CoverSpec::default(),
            },
        );
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoMetadataErrorCode::InvalidTags)
        );
        // 空标签集 + 清除封面即合法意图（后段需真机验证，此处只断不断言拒绝）
        let outcome = apply_video_metadata(
            fake,
            fake,
            "clip.mp4",
            &VideoApplyOptions {
                tags: VideoTags::default(),
                output_dir: std::env::temp_dir().to_string_lossy().into_owned(),
                overwrite: OverwritePolicy::Skip,
                cover: CoverSpec {
                    file: None,
                    clear: true,
                },
            },
        );
        // 跳过策略 + 输出目录可建 → 走到封面清除分支；
        // `ffprobe` 不存在即探测回落空版式 → 无旧封面 → 纯标签管线拉起真进程 → 二进制缺失
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoMetadataErrorCode::FfmpegFailed)
        );
        assert_eq!(outcome.cover, CoverResult::Cleared);
    }

    #[test]
    fn reserve_increments_and_skips() {
        let dir = std::env::temp_dir().join(format!("tool-dock-video-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dir_text = dir.to_string_lossy().into_owned();
        std::fs::write(dir.join("clip.mp4"), b"x").unwrap();

        // 跳过策略：已存在即 `None`
        assert!(
            reserve_video_output("src/clip.mp4", &dir_text, OverwritePolicy::Skip)
                .unwrap()
                .is_none()
        );
        // 覆盖策略：直用首候选
        assert!(
            reserve_video_output("src/clip.mp4", &dir_text, OverwritePolicy::Overwrite)
                .unwrap()
                .is_some()
        );
        // 递增策略：`clip-2.mp4`
        let reserved =
            reserve_video_output("src/clip.mp4", &dir_text, OverwritePolicy::Increment).unwrap();
        assert!(reserved.is_some_and(|path| path.ends_with("clip-2.mp4")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cover_plan_clear_wins_and_empty_keeps() {
        assert!(matches!(
            plan_cover(&CoverSpec {
                file: Some("a.jpg".to_owned()),
                clear: true
            }),
            CoverPlan::Clear
        ));
        assert!(matches!(
            plan_cover(&CoverSpec {
                file: Some("a.jpg".to_owned()),
                clear: false
            }),
            CoverPlan::Embed { .. }
        ));
        assert!(matches!(plan_cover(&CoverSpec::default()), CoverPlan::Keep));
        assert!(matches!(
            plan_cover(&CoverSpec {
                file: Some("   ".to_owned()),
                clear: false
            }),
            CoverPlan::Keep
        ));
    }

    #[test]
    fn cover_container_maps_families() {
        assert_eq!(cover_container("a.mp4"), Some(CoverContainer::Mp4));
        assert_eq!(cover_container("a.MOV"), Some(CoverContainer::Mp4));
        assert_eq!(cover_container("a.mkv"), Some(CoverContainer::Mkv));
        assert_eq!(cover_container("a.webm"), Some(CoverContainer::Mkv));
        assert_eq!(cover_container("a.avi"), None);
    }

    #[test]
    fn parses_cover_layout_and_has_cover() {
        let streams = serde_json::json!([
            {"index": 0, "codec_type": "video", "codec_name": "h264", "width": 1920, "height": 1080},
            {"index": 1, "codec_type": "audio", "codec_name": "aac"},
            {"index": 2, "codec_type": "video", "codec_name": "mjpeg",
             "disposition": {"attached_pic": 1}},
            {"index": 3, "codec_type": "attachment",
             "tags": {"filename": "font.ttf", "mimetype": "application/x-truetype"}},
            {"index": 4, "codec_type": "attachment",
             "tags": {"filename": "cover.jpg", "mimetype": "image/jpeg"}},
        ]);
        let layout = parse_cover_layout(&streams);
        assert_eq!(layout.old_cover_indices, vec![2, 4]);
        assert_eq!(layout.video_count, 2);
        assert_eq!(layout.attachment_count, 2);
        assert!(stream_is_cover(&streams[2]));
        assert!(stream_is_cover(&streams[4]));
        assert!(!stream_is_cover(&streams[0]));
        assert!(!stream_is_cover(&streams[3]));
    }

    #[test]
    fn detects_real_world_cover_shapes() {
        // 真机 ffprobe 实录形状：MP4 的 `attached_pic` 视频流；
        // MKV `-attach` 产物被识别为 mjpeg 视频流（带 `attached_pic` + 文件名/mimetype 标签）
        let mp4 = serde_json::json!([
            {"index": 0, "codec_type": "video", "codec_name": "mpeg4",
             "width": 320, "height": 240, "disposition": {"default": 1, "attached_pic": 0}},
            {"index": 1, "codec_type": "video", "codec_name": "mjpeg",
             "width": 160, "height": 160, "disposition": {"default": 0, "attached_pic": 1}},
        ]);
        let layout = parse_cover_layout(&mp4);
        assert_eq!(layout.old_cover_indices, vec![1]);
        let mkv = serde_json::json!([
            {"index": 0, "codec_type": "video", "codec_name": "h264",
             "width": 320, "height": 240, "disposition": {"default": 0, "attached_pic": 0}},
            {"index": 1, "codec_type": "video", "codec_name": "mjpeg",
             "width": 160, "height": 160, "disposition": {"default": 0, "attached_pic": 1},
             "tags": {"filename": "cover.jpg", "mimetype": "image/jpeg"}},
        ]);
        let layout = parse_cover_layout(&mkv);
        // 误识别为视频流的图片附件同样命中（mimetype 回补 + disposition 双保险）
        assert_eq!(layout.old_cover_indices, vec![1]);
    }

    #[test]
    fn detects_has_cover_in_metadata() {
        let with_pic = parse_ffprobe_output(
            r#"{"format":{},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":128,"height":128},{"index":1,"codec_type":"video","codec_name":"mjpeg","disposition":{"attached_pic":1}}]}"#,
        )
        .unwrap();
        assert!(with_pic.has_cover);
        let without = parse_ffprobe_output(
            r#"{"format":{},"streams":[{"index":0,"codec_type":"video","codec_name":"h264"}]}"#,
        )
        .unwrap();
        assert!(!without.has_cover);
    }

    #[test]
    fn mp4_cover_argv_appends_attached_pic_last() {
        let layout = CoverLayout {
            old_cover_indices: vec![2],
            video_count: 2,
            attachment_count: 0,
        };
        let parts = cover_ffmpeg_parts(Path::new("/v/cover.jpg"), CoverContainer::Mp4, &layout);
        assert_eq!(parts.inputs, vec!["/v/cover.jpg".to_owned()]);
        // 旧封面 2 号流剔除，已映射视频剩 1 → 新封面序号 1
        assert!(parts.args.contains(&"-0:2".to_owned()));
        assert!(parts.args.contains(&"-c:v:1".to_owned()));
        assert!(parts.args.contains(&"mjpeg".to_owned()));
        assert!(parts.args.contains(&"-disposition:v:1".to_owned()));
        // png 封面走 png 编码
        let parts = cover_ffmpeg_parts(
            Path::new("/v/cover.png"),
            CoverContainer::Mp4,
            &CoverLayout::default(),
        );
        assert!(parts.args.contains(&"png".to_owned()));
        assert!(parts.args.contains(&"-disposition:v:0".to_owned()));
    }

    #[test]
    fn mkv_cover_argv_attaches_with_mimetype() {
        let layout = CoverLayout {
            old_cover_indices: vec![3],
            video_count: 1,
            attachment_count: 2,
        };
        let parts = cover_ffmpeg_parts(Path::new("/v/cover.jpg"), CoverContainer::Mkv, &layout);
        assert!(parts.inputs.is_empty(), "mkv attach needs no extra input");
        assert!(parts.args.contains(&"-attach".to_owned()));
        assert!(parts.args.contains(&"-0:3".to_owned()));
        assert!(parts.args.contains(&"mimetype=image/jpeg".to_owned()));
        // 附件剩 1（字体）→ 新附件序号 1
        assert!(parts.args.contains(&"-metadata:s:t:1".to_owned()));
    }

    #[test]
    fn clear_args_drop_old_indices() {
        let layout = CoverLayout {
            old_cover_indices: vec![2, 4],
            video_count: 2,
            attachment_count: 1,
        };
        let args = clear_cover_args(&layout);
        assert!(args.contains(&"-0:2".to_owned()));
        assert!(args.contains(&"-0:4".to_owned()));
        assert!(args.contains(&"copy".to_owned()));
    }

    /// 1x1 BMP 构造（24 位，三字节像素 + 一字节补齐，共 58 字节）
    fn tiny_bmp() -> Vec<u8> {
        let mut bytes = vec![
            0x42, 0x4D, 58, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
            1, 0, 24, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        bytes.extend([0xFF, 0, 0, 0]);
        bytes
    }

    #[test]
    fn resolve_cover_file_probes_extensions_in_order() {
        let dir = std::env::temp_dir().join(format!("tool-dock-cover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("clip.mp4");
        std::fs::write(&input, b"x").unwrap();
        let input_text = input.to_string_lossy().into_owned();

        // 无命中即 `None`
        assert!(resolve_cover_file(&input_text, "clip").is_none());
        assert!(resolve_cover_file(&input_text, "clip.webp").is_none());

        // `jpg` 优先于 `png`
        std::fs::write(dir.join("clip.png"), tiny_bmp()).unwrap();
        std::fs::write(dir.join("clip.jpg"), tiny_bmp()).unwrap();
        assert_eq!(
            resolve_cover_file(&input_text, "clip").map(|path| path
                .extension()
                .unwrap()
                .to_string_lossy()
                .into_owned()),
            Some("jpg".to_owned())
        );
        // 显式后缀直达
        assert!(resolve_cover_file(&input_text, "clip.png").is_some());
        // 绝对路径直达
        let absolute = dir.join("cover.png");
        std::fs::write(&absolute, tiny_bmp()).unwrap();
        assert_eq!(
            resolve_cover_file(&input_text, &absolute.to_string_lossy()),
            Some(absolute)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn work_output_preserves_extension_on_same_path() {
        assert_eq!(
            work_output_for("/v/a.mp4", "/v/a.mp4"),
            "/v/a.tmp.tool-dock.mp4"
        );
        assert_eq!(work_output_for("/v/a.mp4", "/o/a.mp4"), "/o/a.mp4");
    }

    #[test]
    fn prepare_skips_without_spawning() {
        let fake = Path::new("/nonexistent/ffprobe");
        // 不支持的容器直接跳过（不碰文件系统）
        assert!(matches!(
            prepare_cover_embed(fake, "clip.avi", "clip"),
            CoverEmbed::Skip(CoverResult::SkippedUnsupported)
        ));
        // 无文件直接跳过
        assert!(matches!(
            prepare_cover_embed(fake, "clip.mp4", "missing"),
            CoverEmbed::Skip(CoverResult::SkippedNoFile)
        ));
    }

    #[test]
    fn prepare_rejects_corrupt_cover_before_probing() {
        let dir = std::env::temp_dir().join(format!("tool-dock-cover-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("clip.mp4");
        std::fs::write(&input, b"x").unwrap();
        std::fs::write(dir.join("clip.jpg"), b"not an image").unwrap();
        let input_text = input.to_string_lossy().into_owned();
        // 损坏封面在探测前被拦截（`ffprobe` 不存在也不会被调用到）
        assert!(matches!(
            prepare_cover_embed(Path::new("/nonexistent/ffprobe"), &input_text, "clip"),
            CoverEmbed::Skip(CoverResult::SkippedInvalid)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
