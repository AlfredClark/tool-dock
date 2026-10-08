//! 视频格式转换业务逻辑（工具路由 `video/convert`）：目标容器枚举 + remux/重编码参数组装与执行的纯函数实现（`std::process` 直调，不依赖 Tauri 运行时）。
//!
//! 二进制路径由调用方经 `cores::ffmpeg` 解析后传入；输入白名单复用
//! `features::video_metadata::SUPPORTED_VIDEO_EXTENSIONS`（七容器与文件对话框过滤器同源）。
//! 输出路径由后端按输出目录 + 原名茎 + 目标后缀计算，`overwrite` 三策略与图片/元数据侧同语义，
//! 前端只下发目录与策略，不拼路径；已是目标格式的文件允许重处理（统一编码/修复场景），不拦截。
//! 智能模式先试流复制只换容器，失败删残缺输出后回落重编码；字幕/附件整轨携带，
//! 不兼容的边角组合以外层 `FfmpegFailed` 如实返回，不做第三程特判。

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::cores::ffmpeg::Accelerator;
use crate::features::image_resize::OverwritePolicy;
use crate::features::video_metadata::SUPPORTED_VIDEO_EXTENSIONS;

/// 目标容器：七种全量，与输入白名单对齐（线上传 `lowercase`，与前端下拉取值一致）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VideoTarget {
    /// 默认目标：兼容性最广
    #[default]
    Mp4,
    M4v,
    Mov,
    Mkv,
    Webm,
    Avi,
    /// 广播录制常见流容器（`mpegts` 复用器，无 `faststart` 概念）
    Ts,
}

impl VideoTarget {
    /// 目标后缀（输出命名用）
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::M4v => "m4v",
            Self::Mov => "mov",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
            Self::Avi => "avi",
            Self::Ts => "ts",
        }
    }

    /// 是否 mp4 系复用器（`+faststart` 适用：moov 前置，渐进播放更快）
    const fn uses_faststart(self) -> bool {
        matches!(self, Self::Mp4 | Self::M4v | Self::Mov)
    }

    /// 该容器是否有 NVENC 对应（webm/avi 无硬编码器）
    pub const fn supports_hw(self) -> bool {
        matches!(
            self,
            Self::Mp4 | Self::M4v | Self::Mov | Self::Mkv | Self::Ts
        )
    }

    /// 该容器可选的视频编码器（首项为默认；前端下拉与后端校验同源）
    pub const fn allowed_encoders(self) -> &'static [VideoEncoder] {
        match self {
            Self::Mp4 | Self::M4v | Self::Mov | Self::Mkv => {
                &[VideoEncoder::Libx264, VideoEncoder::Libx265]
            }
            Self::Webm => &[VideoEncoder::Vp9, VideoEncoder::Av1],
            Self::Avi => &[VideoEncoder::Mpeg4],
            Self::Ts => &[
                VideoEncoder::Libx264,
                VideoEncoder::Libx265,
                VideoEncoder::Mpeg2Video,
            ],
        }
    }
}

/// 转码模式：智能先复制失败回落重编码；其余两档只跑对应一程
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ConvertMode {
    /// 先流复制，失败回落重编码
    #[default]
    Auto,
    /// 仅流复制换容器，失败直接报错
    CopyOnly,
    /// 直接重编码，不试复制
    Reencode,
}

/// 重编码画质档：按编码器映射为 `CRF`/`q` 值（`Standard` 复刻各容器的旧默认值）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    High,
    #[default]
    Standard,
    Compact,
}

/// 编码速度：仅 264/265 系拼 `-preset`，其余编码器忽略（前端按编码器显隐）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VideoPreset {
    Ultrafast,
    #[default]
    Veryfast,
    Medium,
    Slow,
}

impl VideoPreset {
    /// 传给 ffmpeg 的取值（与变体同名）
    const fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::Ultrafast => "ultrafast",
            Self::Veryfast => "veryfast",
            Self::Medium => "medium",
            Self::Slow => "slow",
        }
    }
}

/// 视频编码器：线上传 `lowercase`，264/265 系取值即 ffmpeg 编码器名
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VideoEncoder {
    #[default]
    Libx264,
    Libx265,
    Vp9,
    Av1,
    Mpeg4,
    Mpeg2Video,
}

impl VideoEncoder {
    /// 传给 `-c:v` 的取值
    const fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::Libx264 => "libx264",
            Self::Libx265 => "libx265",
            Self::Vp9 => "libvpx-vp9",
            Self::Av1 => "libaom-av1",
            Self::Mpeg4 => "mpeg4",
            Self::Mpeg2Video => "mpeg2video",
        }
    }

    /// 是否 264/265 系（`-preset` 与 `CRF` 适用）
    const fn uses_preset(self) -> bool {
        matches!(self, Self::Libx264 | Self::Libx265)
    }
}

/// 音频码率：`Default` 即不传 `-b:a`（复刻旧行为，由 ffmpeg 按编码器取默认）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum AudioBitrate {
    #[default]
    Default,
    Kb128,
    Kb192,
    Kb320,
}

impl AudioBitrate {
    /// 传给 `-b:a` 的取值（`Default` 即不传）
    const fn ffmpeg_value(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Kb128 => Some("128k"),
            Self::Kb192 => Some("192k"),
            Self::Kb320 => Some("320k"),
        }
    }
}

/// 输出分辨率：`Source` 即不传 `-vf`（复刻旧行为）；其余按高缩放、宽自适应保比例
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum OutputResolution {
    #[default]
    Source,
    P1080,
    P720,
    P480,
}

impl OutputResolution {
    /// 目标高度（`Source` 即不缩放）
    const fn target_height(self) -> Option<u32> {
        match self {
            Self::Source => None,
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::P480 => Some(480),
        }
    }
}

/// 单文件转换选项（前端参数面板全量下发；输出路径由后端按输出目录 + 原名计算）。
/// 新增五字段均有 `serde` 默认值：默认值组合复刻旧 argv，旧前端/旧状态反序列化不炸
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoConvertOptions {
    pub target: VideoTarget,
    pub mode: ConvertMode,
    pub output_dir: String,
    pub overwrite: OverwritePolicy,
    #[serde(default)]
    pub quality: VideoQuality,
    #[serde(default)]
    pub preset: VideoPreset,
    #[serde(default)]
    pub video_encoder: VideoEncoder,
    #[serde(default)]
    pub audio_bitrate: AudioBitrate,
    #[serde(default)]
    pub resolution: OutputResolution,
    /// 硬件加速：`Cpu` 为默认（旧行为）；`Nvenc` 只影响重编码程，`CopyOnly` 下忽略
    #[serde(default)]
    pub accelerator: Accelerator,
}

/// 业务错误码：前端按码映射 i18n 文案，`message` 只做诊断补充
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum VideoConvertErrorCode {
    UnsupportedFormat,
    OutputNotWritable,
    FfmpegFailed,
    Skipped,
    /// 编码器与目标容器不兼容（前端切目标即重置，正常走不到；裸调契约的防御）
    InvalidOptions,
}

/// 业务错误体：扁平结构便于 `specta` 导出，前端按 `code` 分支（与视频元数据侧同约定）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoConvertError {
    pub code: VideoConvertErrorCode,
    pub message: Option<String>,
}

/// 单文件转换结果：`ok` 为真读 `output`，为假读 `error`（失败跳过继续，与元数据侧同约定）；
/// `tried_copy` 标识产物是否来自流复制（成功 remux 为真，其余为假；失败时为假）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct VideoConvertOutcome {
    pub ok: bool,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<VideoConvertError>,
    pub target: VideoTarget,
    pub tried_copy: bool,
    /// 产物是否实际走硬加速（硬编成功才为真；回落 CPU 成功即为假，前端据此如实展示）
    #[serde(default)]
    pub used_hw: bool,
}

impl VideoConvertOutcome {
    const fn success(
        input: String,
        output: String,
        target: VideoTarget,
        tried_copy: bool,
        used_hw: bool,
    ) -> Self {
        Self {
            ok: true,
            input,
            output: Some(output),
            error: None,
            target,
            tried_copy,
            used_hw,
        }
    }

    fn failure(
        input: &str,
        code: VideoConvertErrorCode,
        message: impl Into<String>,
        target: VideoTarget,
    ) -> Self {
        Self {
            ok: false,
            input: input.to_owned(),
            output: None,
            error: Some(VideoConvertError {
                code,
                message: Some(message.into()),
            }),
            target,
            tried_copy: false,
            used_hw: false,
        }
    }
}

/// 取文件扩展名（小写）：白名单校验用（与 `video_metadata` 同规则，复用同源常量）
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

/// 组装流复制参数：`-i 输入 -map 0 -c copy [-movflags +faststart] -y/-n 输出`，
/// 全轨携带（字幕/附件尽量保留）；全部 argv 传参，无 shell 拼接
fn build_remux_args(
    input: &str,
    output: &str,
    overwrite: bool,
    target: VideoTarget,
) -> anyhow::Result<Vec<String>> {
    if input.is_empty() || output.is_empty() {
        anyhow::bail!("input and output must not be empty");
    }
    let mut args = vec![
        "-i".to_owned(),
        input.to_owned(),
        "-map".to_owned(),
        "0".to_owned(),
        "-c".to_owned(),
        "copy".to_owned(),
    ];
    if target.uses_faststart() {
        args.extend(["-movflags".to_owned(), "+faststart".to_owned()]);
    }
    args.push(if overwrite { "-y" } else { "-n" }.to_owned());
    args.push(output.to_owned());
    Ok(args)
}

/// 重编码的映射与剥离片段（纯函数，两条重编码组装共用）：
/// `-map` 按类型显式映射（`V` 大写排除内嵌封面，无需 ffprobe；`?` 保缺流不报错），
/// `-map_metadata -1 -map_chapters -1` 清文本元数据与章节；
/// 输出流按类型分组（不再与输入交错），播放器无影响；
/// `remux` 路径仍 `-map 0` 全量复制（无损语义，封面/元数据全留）
fn reencode_map_args() -> Vec<String> {
    let mut args = Vec::new();
    for spec in ["0:V?", "0:a?", "0:s?", "0:d?", "0:t?"] {
        args.extend(["-map".to_owned(), spec.to_owned()]);
    }
    args.extend([
        "-map_metadata".to_owned(),
        "-1".to_owned(),
        "-map_chapters".to_owned(),
        "-1".to_owned(),
    ]);
    args
}

/// 组装重编码参数：视频编码器按 `options.video_encoder` 取（已由调用方校验兼容），
/// 画质档按编码器映射为 `CRF`/`q`（`Standard` 即各编码器旧默认值）；
/// 音频编码器仍按目标容器定（mp4 系/ts 走 `aac`，webm 走 `opus`，avi 走 `mp3`），
/// 码率显式指定才拼 `-b:a`；分辨率非 `Source` 才拼 `-vf scale` 保比例缩放；
/// 内嵌封面随映射丢弃，文本元数据与章节全清（见 `reencode_map_args`）
fn build_reencode_args(
    input: &str,
    output: &str,
    overwrite: bool,
    options: &VideoConvertOptions,
) -> anyhow::Result<Vec<String>> {
    if input.is_empty() || output.is_empty() {
        anyhow::bail!("input and output must not be empty");
    }
    let target = options.target;
    let encoder = options.video_encoder;
    let mut args = vec!["-i".to_owned(), input.to_owned()];
    args.extend(reencode_map_args());
    args.extend(["-c:v".to_owned(), encoder.ffmpeg_name().to_owned()]);
    if encoder.uses_preset() {
        args.extend([
            "-preset".to_owned(),
            options.preset.ffmpeg_name().to_owned(),
        ]);
    }
    match quality_args(encoder, options.quality) {
        QualityArgs::Crf(value) => {
            args.extend(["-crf".to_owned(), value.to_string()]);
            if matches!(encoder, VideoEncoder::Vp9 | VideoEncoder::Av1) {
                // 可变质量模式：不限码率才按 CRF 走（复刻 webm 旧行为）
                args.extend(["-b:v".to_owned(), "0".to_owned()]);
            }
        }
        QualityArgs::Quantizer(value) => {
            args.extend(["-q:v".to_owned(), value.to_string()]);
        }
    }
    args.extend(["-c:a".to_owned(), audio_codec_for(target).to_owned()]);
    if let Some(bitrate) = options.audio_bitrate.ffmpeg_value() {
        args.extend(["-b:a".to_owned(), bitrate.to_owned()]);
    } else if target == VideoTarget::Avi {
        // avi 默认挡复刻旧行为（`mp3` 固定质量；显式码率时改走 `-b:a`）
        args.extend(["-q:a".to_owned(), "4".to_owned()]);
    }
    if let Some(height) = options.resolution.target_height() {
        // 按高缩放、宽取偶自适应（保比例；多路真实视频流会全被缩放，属已知边角）
        args.extend(["-vf".to_owned(), format!("scale=-2:{height}")]);
    }
    if target.uses_faststart() {
        args.extend(["-movflags".to_owned(), "+faststart".to_owned()]);
    }
    args.push(if overwrite { "-y" } else { "-n" }.to_owned());
    args.push(output.to_owned());
    Ok(args)
}

/// 软件编码器→NVENC 名（`None` 即无对应硬编码器，如 `mpeg4/mpeg2video/vp9/av1`）
const fn nvenc_encoder_name(encoder: VideoEncoder) -> Option<&'static str> {
    match encoder {
        VideoEncoder::Libx264 => Some("h264_nvenc"),
        VideoEncoder::Libx265 => Some("hevc_nvenc"),
        _ => None,
    }
}

/// NVENC `-preset` 整数档（ffmpeg 9.0：`p1=3 … p7=9`，默认 `p4=6`；`hevc` 同刻度）
const fn nvenc_preset(preset: VideoPreset) -> u32 {
    match preset {
        VideoPreset::Ultrafast => 3,
        VideoPreset::Veryfast => 5,
        VideoPreset::Medium => 7,
        VideoPreset::Slow => 9,
    }
}

/// NVENC `-cq`（与同系 CPU 的 CRF 档对齐；单测锁定，调参只改此处）
const fn nvenc_cq(encoder: VideoEncoder, quality: VideoQuality) -> u32 {
    use VideoQuality::{Compact, High, Standard};
    // 仅 264/265 系进此分支（其余为防御性回落，取值与 264 一致）
    let hevc = matches!(encoder, VideoEncoder::Libx265);
    match quality {
        High => {
            if hevc {
                20
            } else {
                18
            }
        }
        Standard => {
            if hevc {
                24
            } else {
                23
            }
        }
        Compact => {
            if hevc {
                30
            } else {
                28
            }
        }
    }
}

/// 组装 NVENC 硬编参数：解码侧不动（CPU 解码帧由硬编内部上传，无需 `hwupload` 链），
/// 只换视频编码段为 `-c:v h264/hevc_nvenc -preset <int> -rc vbr -cq <v>`；
/// 音频/分辨率/faststart/覆写与 CPU 分支同规则复用；
/// 内嵌封面随映射丢弃，文本元数据与章节全清（见 `reencode_map_args`）
fn build_hw_args(
    input: &str,
    output: &str,
    overwrite: bool,
    options: &VideoConvertOptions,
) -> anyhow::Result<Vec<String>> {
    if input.is_empty() || output.is_empty() {
        anyhow::bail!("input and output must not be empty");
    }
    let target = options.target;
    let Some(nvenc) = nvenc_encoder_name(options.video_encoder) else {
        anyhow::bail!("no nvenc encoder for {:?}", options.video_encoder);
    };
    let mut args = vec!["-i".to_owned(), input.to_owned()];
    args.extend(reencode_map_args());
    args.extend([
        "-c:v".to_owned(),
        nvenc.to_owned(),
        "-preset".to_owned(),
        nvenc_preset(options.preset).to_string(),
        // `-rc vbr`（=1，显式指定，`-cq` 生效的前提；不依赖驱动默认）
        "-rc".to_owned(),
        "1".to_owned(),
        "-cq".to_owned(),
        nvenc_cq(options.video_encoder, options.quality).to_string(),
        // 钳制 8bit 平面格式（防 10bit/422 输入进硬编失败；8bit 源等价无损）
        "-pix_fmt".to_owned(),
        "yuv420p".to_owned(),
    ]);
    args.extend(["-c:a".to_owned(), audio_codec_for(target).to_owned()]);
    if let Some(bitrate) = options.audio_bitrate.ffmpeg_value() {
        args.extend(["-b:a".to_owned(), bitrate.to_owned()]);
    }
    if let Some(height) = options.resolution.target_height() {
        args.extend(["-vf".to_owned(), format!("scale=-2:{height}")]);
    }
    if target.uses_faststart() {
        args.extend(["-movflags".to_owned(), "+faststart".to_owned()]);
    }
    args.push(if overwrite { "-y" } else { "-n" }.to_owned());
    args.push(output.to_owned());
    Ok(args)
}

/// 画质取值的两种形状：`CRF` 系（264/265/vp9/av1）与固定量化系（mpeg4/mpeg2）
enum QualityArgs {
    Crf(u32),
    Quantizer(u32),
}

/// 画质档映射（纯函数，单测锁定；`Standard` 列即各编码器的旧默认值）
const fn quality_args(encoder: VideoEncoder, quality: VideoQuality) -> QualityArgs {
    use VideoEncoder::{Av1, Libx264, Libx265, Mpeg2Video, Mpeg4, Vp9};
    use VideoQuality::{Compact, High, Standard};
    match encoder {
        Libx264 => QualityArgs::Crf(match quality {
            High => 18,
            Standard => 23,
            Compact => 28,
        }),
        Libx265 => QualityArgs::Crf(match quality {
            High => 20,
            Standard => 24,
            Compact => 30,
        }),
        Vp9 => QualityArgs::Crf(match quality {
            High => 25,
            Standard => 30,
            Compact => 38,
        }),
        Av1 => QualityArgs::Crf(match quality {
            High => 20,
            Standard => 28,
            Compact => 36,
        }),
        Mpeg4 => QualityArgs::Quantizer(match quality {
            High => 2,
            Standard => 3,
            Compact => 5,
        }),
        Mpeg2Video => QualityArgs::Quantizer(match quality {
            High => 2,
            Standard => 4,
            Compact => 6,
        }),
    }
}

/// 目标容器的音频编码器（与旧行为一致：mp4 系/ts 走 `aac`，webm 走 `opus`，avi 走 `mp3`）
const fn audio_codec_for(target: VideoTarget) -> &'static str {
    match target {
        VideoTarget::Mp4
        | VideoTarget::M4v
        | VideoTarget::Mov
        | VideoTarget::Mkv
        | VideoTarget::Ts => "aac",
        VideoTarget::Webm => "libopus",
        VideoTarget::Avi => "libmp3lame",
    }
}

/// 预留输出路径：输出目录自动建，茎名 + 目标后缀；`Overwrite` 直用首候选，
/// `Skip` 遇存在即 `None`，`Increment` 按 `名-2.后缀` 递增（与元数据侧同规则）
fn reserve_convert_output(
    input: &str,
    output_dir: &str,
    target: VideoTarget,
    overwrite: OverwritePolicy,
) -> anyhow::Result<Option<String>> {
    let out_dir = Path::new(output_dir);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("cannot create output dir: {output_dir}"))?;
    let stem = Path::new(input)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .with_context(|| format!("invalid file name: {input}"))?;
    let extension = target.extension();
    let first = out_dir.join(format!("{stem}.{extension}"));
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

/// 跳过诊断用的首候选路径：与 `reserve_convert_output` 同规则组装（仅展示，不预留）
fn skip_candidate(input: &str, output_dir: &str, target: VideoTarget) -> String {
    Path::new(input)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map_or_else(
            || output_dir.to_owned(),
            |stem| {
                Path::new(output_dir)
                    .join(format!("{stem}.{}", target.extension()))
                    .to_string_lossy()
                    .into_owned()
            },
        )
}

/// 同路径覆写的工作路径：ffmpeg 禁止原地编辑，命中时返回同目录临时名
/// （保留目标后缀供容器判定）；不命中即原文（与元数据侧同语义）
fn work_output_for(input: &str, output: &str) -> String {
    if Path::new(output) != Path::new(input) {
        return output.to_owned();
    }
    let extension = Path::new(output)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp4");
    PathBuf::from(format!("{output}.tmp.tool-dock.{extension}"))
        .to_string_lossy()
        .into_owned()
}

/// 执行单程：拉起进程，成功即 `Ok`，失败带 `stderr` 尾部（调用方决定是否回落）
fn run_ffmpeg(ffmpeg: &Path, args: &[String]) -> Result<(), String> {
    match std::process::Command::new(ffmpeg).args(args).output() {
        Ok(result) if result.status.success() => Ok(()),
        Ok(result) => Err(format!("ffmpeg failed: {}", stderr_tail(&result.stderr))),
        Err(err) => Err(format!("failed to run ffmpeg: {err:#}")),
    }
}

/// 回落编排（可注入执行器，单测覆盖）：先跑首程，成功即 `(true, true, None)`；
/// 失败则调 `on_retry`（调用方删残缺输出）后再跑第二程，成功即 `(true, false, None)`；
/// 双败时合并两程诊断（阶段名由调用方给，如 `remux/hwencode/reencode`）
fn run_with_fallback(
    run: &dyn Fn(&[String]) -> Result<(), String>,
    first: &[String],
    second: &[String],
    first_name: &str,
    second_name: &str,
    on_retry: &dyn Fn(),
) -> (bool, bool, Option<String>) {
    match run(first) {
        Ok(()) => (true, true, None),
        Err(first_err) => {
            on_retry();
            match run(second) {
                Ok(()) => (true, false, None),
                Err(second_err) => (
                    false,
                    false,
                    Some(format!(
                        "{first_name} failed: {first_err}; {second_name} failed: {second_err}"
                    )),
                ),
            }
        }
    }
}

/// 重编码程编排（可注入执行器，单测覆盖）：`hw` 为 `Some` 即先硬编、失败回落 CPU，
/// 返回 `(成功, 是否实际走硬加速, 诊断)`；`None` 即纯 CPU 单程
fn run_reencode_stage(
    run: &dyn Fn(&[String]) -> Result<(), String>,
    hw: Option<&[String]>,
    cpu: &[String],
    on_retry: &dyn Fn(),
) -> (bool, bool, Option<String>) {
    hw.map_or_else(
        || match run(cpu) {
            Ok(()) => (true, false, None),
            Err(err) => (false, false, Some(err)),
        },
        |hw_args| {
            let (ok, tried_hw, diagnosis) =
                run_with_fallback(run, hw_args, cpu, "hwencode", "reencode", on_retry);
            (ok, ok && tried_hw, diagnosis)
        },
    )
}

/// 前置门禁：输入白名单 + 编码器×容器兼容性 + 加速组合校验；
/// 失败直接装好 `Outcome`（不拉起进程）
fn check_convert_request(
    input: &str,
    options: &VideoConvertOptions,
) -> Result<(), VideoConvertOutcome> {
    use VideoConvertErrorCode::{InvalidOptions, UnsupportedFormat};

    let target = options.target;
    if !is_supported_video(input) {
        return Err(VideoConvertOutcome::failure(
            input,
            UnsupportedFormat,
            format!("unsupported video format: {input}"),
            target,
        ));
    }
    // 正常经前端重置走不到，裸调契约时拦截
    if !target.allowed_encoders().contains(&options.video_encoder) {
        return Err(VideoConvertOutcome::failure(
            input,
            InvalidOptions,
            format!(
                "encoder {:?} not supported for target {:?}",
                options.video_encoder, target
            ),
            target,
        ));
    }
    // 加速只影响重编码程：容器无硬编码器、或编码器无硬编对应即拒绝
    if options.accelerator == Accelerator::Nvenc
        && (!target.supports_hw() || nvenc_encoder_name(options.video_encoder).is_none())
    {
        return Err(VideoConvertOutcome::failure(
            input,
            InvalidOptions,
            format!(
                "nvenc not supported for target {:?} with encoder {:?}",
                target, options.video_encoder
            ),
            target,
        ));
    }
    Ok(())
}

/// 转换单文件：输出路径按输出目录 + 原名茎 + 目标后缀计算；
/// 业务失败全部装进 `VideoConvertOutcome` 返回，不抛错（单文件失败跳过继续是常态 UI 状态）
pub fn convert_video_format(
    ffmpeg: &Path,
    input: &str,
    options: &VideoConvertOptions,
) -> VideoConvertOutcome {
    use VideoConvertErrorCode::{FfmpegFailed, OutputNotWritable, Skipped};

    if let Err(outcome) = check_convert_request(input, options) {
        return outcome;
    }
    let target = options.target;
    let output = match reserve_convert_output(input, &options.output_dir, target, options.overwrite)
    {
        Ok(Some(path)) => path,
        Ok(None) => {
            return VideoConvertOutcome::failure(
                input,
                Skipped,
                format!(
                    "output already exists: {}",
                    skip_candidate(input, &options.output_dir, target)
                ),
                target,
            );
        }
        Err(err) => {
            return VideoConvertOutcome::failure(
                input,
                OutputNotWritable,
                format!("{err:#}"),
                target,
            );
        }
    };
    // 同路径覆写（输出即输入）：先写临时文件再原子替换
    let work_output = work_output_for(input, &output);
    let plain_overwrite = options.overwrite == OverwritePolicy::Overwrite;
    let remux_args = match build_remux_args(input, &work_output, plain_overwrite, target) {
        Ok(args) => args,
        Err(err) => {
            return VideoConvertOutcome::failure(input, FfmpegFailed, format!("{err:#}"), target);
        }
    };
    let reencode_args = match build_reencode_args(input, &work_output, plain_overwrite, options) {
        Ok(args) => args,
        Err(err) => {
            return VideoConvertOutcome::failure(input, FfmpegFailed, format!("{err:#}"), target);
        }
    };
    // 加速只影响重编码程：`CopyOnly` 下忽略（前端不展示加速入口，走不到）；
    // 其它模式按需组装硬编 argv（门禁已保证合法，组装失败即 `FfmpegFailed`）
    let hw_args = if options.accelerator == Accelerator::Nvenc {
        match build_hw_args(input, &work_output, plain_overwrite, options) {
            Ok(args) => Some(args),
            Err(err) => {
                return VideoConvertOutcome::failure(
                    input,
                    FfmpegFailed,
                    format!("{err:#}"),
                    target,
                );
            }
        }
    } else {
        None
    };
    let run = |args: &[String]| run_ffmpeg(ffmpeg, args);
    let clear_partial = || {
        let _ = std::fs::remove_file(&work_output);
    };
    // 成功时 `used_hw` 仅硬编首程命中才为真（回落 CPU 成功即为假，前端如实展示）
    let (ok, tried_copy, used_hw, diagnosis) = run_convert_stages(
        &run,
        &remux_args,
        hw_args.as_deref(),
        &reencode_args,
        options.mode,
        &clear_partial,
    );
    if work_output != output {
        if ok {
            if let Err(err) = std::fs::rename(&work_output, &output) {
                return VideoConvertOutcome::failure(
                    input,
                    FfmpegFailed,
                    format!("failed to replace output: {err:#}"),
                    target,
                );
            }
        } else {
            let _ = std::fs::remove_file(&work_output);
        }
    }
    if ok {
        VideoConvertOutcome::success(input.to_owned(), output, target, tried_copy, used_hw)
    } else {
        VideoConvertOutcome::failure(
            input,
            FfmpegFailed,
            diagnosis.unwrap_or_else(|| "ffmpeg failed".to_owned()),
            target,
        )
    }
}

/// 执行转码多程（`run` 可注入；重编码程细节经 `run_reencode_stage` 单测覆盖）：
/// 返回 `(成功, 流复制, 硬加速, 诊断)`；`Auto` 下首程（流复制）已败则 `tried_copy` 恒为假
fn run_convert_stages(
    run: &dyn Fn(&[String]) -> Result<(), String>,
    remux: &[String],
    hw: Option<&[String]>,
    cpu: &[String],
    mode: ConvertMode,
    on_retry: &dyn Fn(),
) -> (bool, bool, bool, Option<String>) {
    match mode {
        ConvertMode::CopyOnly => match run(remux) {
            Ok(()) => (true, true, false, None),
            Err(err) => (false, false, false, Some(err)),
        },
        ConvertMode::Reencode => {
            let (ok, used_hw, diagnosis) = run_reencode_stage(run, hw, cpu, on_retry);
            (ok, false, used_hw, diagnosis)
        }
        ConvertMode::Auto => match run(remux) {
            Ok(()) => (true, true, false, None),
            Err(remux_err) => {
                on_retry();
                let (ok, used_hw, diagnosis) = run_reencode_stage(run, hw, cpu, on_retry);
                let diagnosis =
                    diagnosis.map(|inner| format!("remux failed: {remux_err}; {inner}"));
                (ok, false, used_hw, diagnosis)
            }
        },
    }
}

/// 取 `stderr` 末 2KB：既保留诊断信息，又防损坏文件刷出海量日志（与元数据侧同口径）
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
    fn target_extensions_align_with_input_whitelist() {
        // 目标七容器与输入白名单对齐：前后端下拉与对话框过滤器同源
        let mut extensions: Vec<&str> = [
            VideoTarget::Mp4,
            VideoTarget::M4v,
            VideoTarget::Mov,
            VideoTarget::Mkv,
            VideoTarget::Webm,
            VideoTarget::Avi,
            VideoTarget::Ts,
        ]
        .iter()
        .map(|target| target.extension())
        .collect();
        extensions.sort_unstable();
        let mut supported: Vec<&str> = SUPPORTED_VIDEO_EXTENSIONS.to_vec();
        supported.sort_unstable();
        assert_eq!(extensions, supported);
    }

    #[test]
    fn target_serde_is_lowercase_for_frontend_dropdown() {
        assert_eq!(
            serde_json::to_value(VideoTarget::Mp4).unwrap(),
            serde_json::Value::String("mp4".to_owned())
        );
        assert_eq!(
            serde_json::to_value(ConvertMode::CopyOnly).unwrap(),
            serde_json::Value::String("copyonly".to_owned())
        );
        assert_eq!(
            serde_json::to_value(VideoTarget::Ts).unwrap(),
            serde_json::Value::String("ts".to_owned())
        );
    }

    #[test]
    fn remux_args_carry_all_streams_with_faststart_only_for_mp4_family() {
        let mp4 = build_remux_args("in.mkv", "out.mp4", true, VideoTarget::Mp4).unwrap();
        assert_eq!(
            mp4,
            vec![
                "-i",
                "in.mkv",
                "-map",
                "0",
                "-c",
                "copy",
                "-movflags",
                "+faststart",
                "-y",
                "out.mp4",
            ]
        );
        for target in [
            VideoTarget::Mkv,
            VideoTarget::Webm,
            VideoTarget::Avi,
            VideoTarget::Ts,
        ] {
            let args = build_remux_args("in.mov", "out", false, target).unwrap();
            assert!(args.contains(&"-c".to_owned()));
            assert!(args.contains(&"copy".to_owned()));
            assert!(!args.iter().any(|arg| arg == "+faststart"), "{target:?}");
            assert!(args.contains(&"-n".to_owned()));
        }
    }

    #[test]
    fn reencode_args_pick_container_codecs() {
        let mp4 =
            build_reencode_args("in.mkv", "out.mp4", true, &options_for(VideoTarget::Mp4)).unwrap();
        assert!(mp4.contains(&"libx264".to_owned()));
        assert!(mp4.contains(&"aac".to_owned()));
        assert!(mp4.contains(&"+faststart".to_owned()));
        let webm = build_reencode_args("in.mp4", "out.webm", true, &options_for(VideoTarget::Webm))
            .unwrap();
        assert!(webm.contains(&"libvpx-vp9".to_owned()));
        assert!(webm.contains(&"libopus".to_owned()));
        assert!(!webm.contains(&"libx264".to_owned()));
        let avi =
            build_reencode_args("in.mp4", "out.avi", true, &options_for(VideoTarget::Avi)).unwrap();
        assert!(avi.contains(&"mpeg4".to_owned()));
        assert!(avi.contains(&"libmp3lame".to_owned()));
        let mkv = build_reencode_args("in.avi", "out.mkv", false, &options_for(VideoTarget::Mkv))
            .unwrap();
        assert!(mkv.contains(&"libx264".to_owned()));
        assert!(mkv.contains(&"-n".to_owned()));
        // ts 与 mp4 系同编码（`mpegts` 无 `faststart` 概念）
        let ts =
            build_reencode_args("in.mp4", "out.ts", true, &options_for(VideoTarget::Ts)).unwrap();
        assert!(ts.contains(&"libx264".to_owned()));
        assert!(ts.contains(&"aac".to_owned()));
        assert!(!ts.iter().any(|arg| arg == "+faststart"));
    }

    /// 默认选项组装器（单测用）：各目标取允许表首项 + 全默认重编码参数
    fn options_for(target: VideoTarget) -> VideoConvertOptions {
        VideoConvertOptions {
            target,
            mode: ConvertMode::Reencode,
            output_dir: String::new(),
            overwrite: OverwritePolicy::Overwrite,
            quality: VideoQuality::Standard,
            preset: VideoPreset::Veryfast,
            video_encoder: target.allowed_encoders()[0],
            audio_bitrate: AudioBitrate::Default,
            resolution: OutputResolution::Source,
            accelerator: Accelerator::Cpu,
        }
    }

    #[test]
    fn default_options_use_cover_free_maps() {
        // 默认值组合的映射与剥离段：除封面外全轨携带 + 元数据/章节全清（`remux` 仍全量复制）
        let maps = vec![
            "-map",
            "0:V?",
            "-map",
            "0:a?",
            "-map",
            "0:s?",
            "-map",
            "0:d?",
            "-map",
            "0:t?",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
        ];
        assert_eq!(
            build_reencode_args("in.mkv", "out.mp4", true, &options_for(VideoTarget::Mp4)).unwrap(),
            [
                vec!["-i", "in.mkv"],
                maps.clone(),
                vec![
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "23",
                    "-c:a",
                    "aac",
                    "-movflags",
                    "+faststart",
                    "-y",
                    "out.mp4",
                ],
            ]
            .concat()
        );
        assert_eq!(
            build_reencode_args("in.mp4", "out.webm", true, &options_for(VideoTarget::Webm))
                .unwrap(),
            [
                vec!["-i", "in.mp4"],
                maps.clone(),
                vec![
                    "-c:v",
                    "libvpx-vp9",
                    "-crf",
                    "30",
                    "-b:v",
                    "0",
                    "-c:a",
                    "libopus",
                    "-y",
                    "out.webm",
                ],
            ]
            .concat()
        );
        assert_eq!(
            build_reencode_args("in.mp4", "out.avi", true, &options_for(VideoTarget::Avi)).unwrap(),
            [
                vec!["-i", "in.mp4"],
                maps.clone(),
                vec![
                    "-c:v",
                    "mpeg4",
                    "-q:v",
                    "3",
                    "-c:a",
                    "libmp3lame",
                    "-q:a",
                    "4",
                    "-y",
                    "out.avi",
                ],
            ]
            .concat()
        );
        assert_eq!(
            build_reencode_args("in.mp4", "out.ts", true, &options_for(VideoTarget::Ts)).unwrap(),
            [
                vec!["-i", "in.mp4"],
                maps,
                vec![
                    "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-c:a", "aac", "-y",
                    "out.ts",
                ],
            ]
            .concat()
        );
    }

    #[test]
    fn reencode_drops_covers_and_strips_metadata() {
        // 裸 `-map 0` 相邻对不得出现（出现即封面会被卷入编码）；剥离旗齐全
        for target in [
            VideoTarget::Mp4,
            VideoTarget::Webm,
            VideoTarget::Avi,
            VideoTarget::Ts,
        ] {
            let args = build_reencode_args("in.mp4", "out", true, &options_for(target)).unwrap();
            assert!(
                !args.windows(2).any(|pair| pair == ["-map", "0"]),
                "{target:?}"
            );
            for flag in ["0:V?", "-map_metadata", "-map_chapters"] {
                assert!(args.contains(&flag.to_owned()), "{target:?}");
            }
        }
        // 硬编分支同规则
        let nvenc = VideoConvertOptions {
            accelerator: Accelerator::Nvenc,
            ..options_for(VideoTarget::Mp4)
        };
        let args = build_hw_args("in.mp4", "out.mp4", true, &nvenc).unwrap();
        assert!(!args.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(args.contains(&"-map_metadata".to_owned()));
        // 流复制路径保持全量复制（无损语义，封面/元数据全留）
        let remux = build_remux_args("in.mp4", "out.mp4", true, VideoTarget::Mp4).unwrap();
        assert!(remux.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(!remux.contains(&"-map_metadata".to_owned()));
    }

    #[test]
    fn quality_maps_per_encoder() {
        // 高/省档按编码器映射（`Standard` 列即旧默认值，上式已锁定）
        let high = VideoConvertOptions {
            quality: VideoQuality::High,
            ..options_for(VideoTarget::Mp4)
        };
        let args = build_reencode_args("in.mp4", "out.mp4", true, &high).unwrap();
        assert!(args.contains(&"18".to_owned()));
        let compact = VideoConvertOptions {
            quality: VideoQuality::Compact,
            ..options_for(VideoTarget::Webm)
        };
        let args = build_reencode_args("in.mp4", "out.webm", true, &compact).unwrap();
        assert!(args.contains(&"38".to_owned()));
        let mpeg = VideoConvertOptions {
            quality: VideoQuality::High,
            ..options_for(VideoTarget::Avi)
        };
        let args = build_reencode_args("in.mp4", "out.avi", true, &mpeg).unwrap();
        assert!(args.contains(&"-q:v".to_owned()));
        assert!(args.contains(&"2".to_owned()));
    }

    #[test]
    fn preset_only_for_h264_family() {
        let slow = VideoConvertOptions {
            preset: VideoPreset::Slow,
            ..options_for(VideoTarget::Mp4)
        };
        let args = build_reencode_args("in.mp4", "out.mp4", true, &slow).unwrap();
        assert!(args.contains(&"slow".to_owned()));
        // vp9/av1/mpeg 系忽略 `-preset`（前端按编码器隐藏，后端同样不拼）
        let webm = VideoConvertOptions {
            preset: VideoPreset::Slow,
            ..options_for(VideoTarget::Webm)
        };
        let args = build_reencode_args("in.mp4", "out.webm", true, &webm).unwrap();
        assert!(!args.iter().any(|arg| arg == "slow"));
        assert!(!args.iter().any(|arg| arg == "-preset"));
    }

    #[test]
    fn audio_bitrate_and_resolution_flags() {
        // 默认挡不拼 `-b:a`/`-vf`
        let plain =
            build_reencode_args("in.mp4", "out.mp4", true, &options_for(VideoTarget::Mp4)).unwrap();
        assert!(!plain.contains(&"-b:a".to_owned()));
        assert!(!plain.contains(&"-vf".to_owned()));
        // 显式挡拼旗
        let explicit = VideoConvertOptions {
            audio_bitrate: AudioBitrate::Kb192,
            resolution: OutputResolution::P720,
            ..options_for(VideoTarget::Mp4)
        };
        let args = build_reencode_args("in.mp4", "out.mp4", true, &explicit).unwrap();
        assert!(args.contains(&"-b:a".to_owned()));
        assert!(args.contains(&"192k".to_owned()));
        assert!(args.contains(&"scale=-2:720".to_owned()));
    }

    #[test]
    fn encoder_target_matrix_and_defaults() {
        assert_eq!(
            VideoTarget::Mp4.allowed_encoders(),
            &[VideoEncoder::Libx264, VideoEncoder::Libx265]
        );
        assert_eq!(
            VideoTarget::Ts.allowed_encoders(),
            &[
                VideoEncoder::Libx264,
                VideoEncoder::Libx265,
                VideoEncoder::Mpeg2Video
            ]
        );
        assert_eq!(
            VideoTarget::Webm.allowed_encoders(),
            &[VideoEncoder::Vp9, VideoEncoder::Av1]
        );
        assert_eq!(VideoTarget::Avi.allowed_encoders(), &[VideoEncoder::Mpeg4]);
        // 允许表首项即默认（复刻各容器的旧默认编码器）
        assert_eq!(
            VideoTarget::Mp4.allowed_encoders()[0],
            VideoEncoder::Libx264
        );
        assert_eq!(VideoTarget::Avi.allowed_encoders()[0], VideoEncoder::Mpeg4);
        assert_eq!(VideoTarget::Webm.allowed_encoders()[0], VideoEncoder::Vp9);
        assert!(VideoEncoder::Libx265.uses_preset());
        assert!(!VideoEncoder::Vp9.uses_preset());
    }

    #[test]
    fn option_enums_serde_lowercase_with_defaults() {
        assert_eq!(
            serde_json::to_value(VideoQuality::Compact).unwrap(),
            serde_json::Value::String("compact".to_owned())
        );
        assert_eq!(
            serde_json::to_value(VideoEncoder::Mpeg2Video).unwrap(),
            serde_json::Value::String("mpeg2video".to_owned())
        );
        assert_eq!(
            serde_json::to_value(AudioBitrate::Kb192).unwrap(),
            serde_json::Value::String("kb192".to_owned())
        );
        assert_eq!(
            serde_json::to_value(OutputResolution::P720).unwrap(),
            serde_json::Value::String("p720".to_owned())
        );
        // 旧载荷缺新字段仍可反序列化（前向兼容）
        let legacy = serde_json::json!({
            "target": "mp4",
            "mode": "auto",
            "output_dir": "/tmp",
            "overwrite": "increment",
        });
        let options: VideoConvertOptions = serde_json::from_value(legacy).unwrap();
        assert_eq!(options.quality, VideoQuality::Standard);
        assert_eq!(options.video_encoder, VideoEncoder::Libx264);
        assert_eq!(options.resolution, OutputResolution::Source);
    }

    #[test]
    fn rejects_empty_paths() {
        assert!(build_remux_args("", "out.mp4", true, VideoTarget::Mp4).is_err());
        assert!(build_reencode_args("in.mp4", "", true, &options_for(VideoTarget::Mp4)).is_err());
    }

    #[test]
    fn reserve_renames_stem_with_target_extension() {
        let dir =
            std::env::temp_dir().join(format!("tool-dock-convert-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dir_text = dir.to_string_lossy().into_owned();

        // 异后缀输入按茎名换后缀
        let reserved = reserve_convert_output(
            "src/clip.mov",
            &dir_text,
            VideoTarget::Mp4,
            OverwritePolicy::Overwrite,
        )
        .unwrap();
        assert!(reserved.is_some_and(|path| path.ends_with("clip.mp4")));

        std::fs::write(dir.join("clip.mp4"), b"x").unwrap();
        // 跳过策略：已存在即 `None`
        assert!(
            reserve_convert_output(
                "src/clip.mov",
                &dir_text,
                VideoTarget::Mp4,
                OverwritePolicy::Skip
            )
            .unwrap()
            .is_none()
        );
        // 递增策略：`clip-2.mp4`
        let reserved = reserve_convert_output(
            "src/clip.mov",
            &dir_text,
            VideoTarget::Mp4,
            OverwritePolicy::Increment,
        )
        .unwrap();
        assert!(reserved.is_some_and(|path| path.ends_with("clip-2.mp4")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn convert_reports_business_outcomes_without_spawning() {
        let fake = Path::new("/nonexistent/ffmpeg");
        let options = VideoConvertOptions {
            target: VideoTarget::Mp4,
            mode: ConvertMode::Auto,
            output_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            overwrite: OverwritePolicy::Overwrite,
            quality: VideoQuality::Standard,
            preset: VideoPreset::Veryfast,
            video_encoder: VideoEncoder::Libx264,
            audio_bitrate: AudioBitrate::Default,
            resolution: OutputResolution::Source,
            accelerator: Accelerator::Cpu,
        };
        // 非白名单扩展名直接拒收（不拉起进程）
        let outcome = convert_video_format(fake, "clip.txt", &options);
        assert!(!outcome.ok);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoConvertErrorCode::UnsupportedFormat)
        );
        assert!(!outcome.tried_copy);
        // 编码器×容器不兼容同样不拉起进程
        let mismatched = VideoConvertOptions {
            video_encoder: VideoEncoder::Vp9,
            ..options
        };
        let outcome = convert_video_format(fake, "clip.mp4", &mismatched);
        assert!(!outcome.ok);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoConvertErrorCode::InvalidOptions)
        );
    }

    #[test]
    fn fallback_runs_second_only_after_first_failure() {
        use std::cell::RefCell;

        let calls = RefCell::new(Vec::new());
        let run = |args: &[String]| -> Result<(), String> {
            calls.borrow_mut().push(args.join(" "));
            if args.iter().any(|arg| arg == "copy") {
                return Err("copy failed".to_owned());
            }
            Ok(())
        };
        let (ok, tried_copy, diagnosis) = run_with_fallback(
            &run,
            &["-c".to_owned(), "copy".to_owned()],
            &["-c:v".to_owned(), "libx264".to_owned()],
            "remux",
            "reencode",
            &|| {},
        );
        assert!(ok);
        assert!(!tried_copy);
        assert!(diagnosis.is_none());
        assert_eq!(calls.borrow().len(), 2);

        // 首程成功即流复制产物，不跑第二程
        let calls = RefCell::new(Vec::new());
        let run = |args: &[String]| -> Result<(), String> {
            calls.borrow_mut().push(args.join(" "));
            Ok(())
        };
        let (ok, tried_copy, diagnosis) = run_with_fallback(
            &run,
            &["-c".to_owned(), "copy".to_owned()],
            &["-c:v".to_owned(), "libx264".to_owned()],
            "remux",
            "reencode",
            &|| {},
        );
        assert!(ok);
        assert!(tried_copy);
        assert!(diagnosis.is_none());
        assert_eq!(calls.borrow().len(), 1);

        // 双败合并两程诊断（阶段名由调用方给）
        let run = |_: &[String]| -> Result<(), String> { Err("boom".to_owned()) };
        let (ok, tried_copy, diagnosis) = run_with_fallback(
            &run,
            &["first".to_owned()],
            &["second".to_owned()],
            "hwencode",
            "reencode",
            &|| {},
        );
        assert!(!ok);
        assert!(!tried_copy);
        let diagnosis = diagnosis.unwrap();
        assert!(diagnosis.contains("hwencode failed"));
        assert!(diagnosis.contains("reencode failed"));
    }

    #[test]
    fn reencode_stage_tracks_hw_usage() {
        use std::cell::RefCell;

        let hw = ["-c:v".to_owned(), "h264_nvenc".to_owned()];
        let cpu = ["-c:v".to_owned(), "libx264".to_owned()];
        // 硬编成功即 `used_hw`
        let run = |_: &[String]| -> Result<(), String> { Ok(()) };
        let (ok, used_hw, diagnosis) = run_reencode_stage(&run, Some(&hw), &cpu, &|| {});
        assert!(ok);
        assert!(used_hw);
        assert!(diagnosis.is_none());
        // 硬编失败回落 CPU 成功：成功但非硬加速
        let calls = RefCell::new(Vec::new());
        let run = |args: &[String]| -> Result<(), String> {
            calls.borrow_mut().push(args.join(" "));
            if args.iter().any(|arg| arg == "h264_nvenc") {
                return Err("no nvidia device".to_owned());
            }
            Ok(())
        };
        let (ok, used_hw, diagnosis) = run_reencode_stage(&run, Some(&hw), &cpu, &|| {});
        assert!(ok);
        assert!(!used_hw);
        assert!(diagnosis.is_none());
        assert_eq!(calls.borrow().len(), 2);
        // 双败合并诊断
        let run = |_: &[String]| -> Result<(), String> { Err("boom".to_owned()) };
        let (ok, used_hw, diagnosis) = run_reencode_stage(&run, Some(&hw), &cpu, &|| {});
        assert!(!ok);
        assert!(!used_hw);
        let diagnosis = diagnosis.unwrap();
        assert!(diagnosis.contains("hwencode failed"));
        assert!(diagnosis.contains("reencode failed"));
        // 纯 CPU 单程：失败即原错
        let (ok, used_hw, diagnosis) = run_reencode_stage(&run, None, &cpu, &|| {});
        assert!(!ok);
        assert!(!used_hw);
        assert_eq!(diagnosis, Some("boom".to_owned()));
    }

    #[test]
    fn nvenc_args_use_hw_encoder_with_cq_and_pix_fmt() {
        // 默认挡字面量锁：`-c:v h264_nvenc -preset 5(p3) -rc vbr(1) -cq 23 -pix_fmt yuv420p`
        let nvenc = VideoConvertOptions {
            accelerator: Accelerator::Nvenc,
            ..options_for(VideoTarget::Mp4)
        };
        assert_eq!(
            build_hw_args("in.mkv", "out.mp4", true, &nvenc).unwrap(),
            vec![
                "-i",
                "in.mkv",
                "-map",
                "0:V?",
                "-map",
                "0:a?",
                "-map",
                "0:s?",
                "-map",
                "0:d?",
                "-map",
                "0:t?",
                "-map_metadata",
                "-1",
                "-map_chapters",
                "-1",
                "-c:v",
                "h264_nvenc",
                "-preset",
                "5",
                "-rc",
                "1",
                "-cq",
                "23",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-movflags",
                "+faststart",
                "-y",
                "out.mp4",
            ]
        );
        // hevc 走 `hevc_nvenc`，ts 无 `faststart`
        let hevc = VideoConvertOptions {
            accelerator: Accelerator::Nvenc,
            video_encoder: VideoEncoder::Libx265,
            ..options_for(VideoTarget::Ts)
        };
        let args = build_hw_args("in.mp4", "out.ts", true, &hevc).unwrap();
        assert!(args.contains(&"hevc_nvenc".to_owned()));
        assert!(!args.iter().any(|arg| arg == "+faststart"));
        // 无对应硬编码器即组装失败（门禁已拦截，此处为防御）
        let mpeg = VideoConvertOptions {
            accelerator: Accelerator::Nvenc,
            video_encoder: VideoEncoder::Mpeg4,
            ..options_for(VideoTarget::Avi)
        };
        assert!(build_hw_args("in.mp4", "out.avi", true, &mpeg).is_err());
    }

    #[test]
    fn nvenc_preset_and_cq_maps() {
        // 速度四档 → p1/p3/p5/p7（3/5/7/9）
        assert_eq!(nvenc_preset(VideoPreset::Ultrafast), 3);
        assert_eq!(nvenc_preset(VideoPreset::Veryfast), 5);
        assert_eq!(nvenc_preset(VideoPreset::Medium), 7);
        assert_eq!(nvenc_preset(VideoPreset::Slow), 9);
        // 画质三档与同系 CPU 的 CRF 档对齐
        assert_eq!(nvenc_cq(VideoEncoder::Libx264, VideoQuality::Compact), 28);
        assert_eq!(nvenc_cq(VideoEncoder::Libx265, VideoQuality::High), 20);
    }

    #[test]
    fn hw_validation_matrix() {
        let fake = Path::new("/nonexistent/ffmpeg");
        let base = options_for(VideoTarget::Mp4);
        // mp4 + 264 + nvenc 放行（走到拉起进程才失败，此处只断不断言拒绝）
        let ok = VideoConvertOptions {
            accelerator: Accelerator::Nvenc,
            ..base.clone()
        };
        let outcome = convert_video_format(fake, "clip.mp4", &ok);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoConvertErrorCode::FfmpegFailed)
        );
        // webm 无硬编码器直接拒绝
        let webm = VideoConvertOptions {
            target: VideoTarget::Webm,
            video_encoder: VideoEncoder::Vp9,
            accelerator: Accelerator::Nvenc,
            ..base.clone()
        };
        let outcome = convert_video_format(fake, "clip.webm", &webm);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoConvertErrorCode::InvalidOptions)
        );
        // ts + mpeg2video 无硬编对应直接拒绝
        let ts = VideoConvertOptions {
            target: VideoTarget::Ts,
            video_encoder: VideoEncoder::Mpeg2Video,
            accelerator: Accelerator::Nvenc,
            ..base.clone()
        };
        let outcome = convert_video_format(fake, "clip.ts", &ts);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoConvertErrorCode::InvalidOptions)
        );
        // ts + 264 + nvenc 放行
        let ts_hw = VideoConvertOptions {
            target: VideoTarget::Ts,
            video_encoder: VideoEncoder::Libx264,
            accelerator: Accelerator::Nvenc,
            ..base
        };
        let outcome = convert_video_format(fake, "clip.ts", &ts_hw);
        assert_eq!(
            outcome.error.map(|error| error.code),
            Some(VideoConvertErrorCode::FfmpegFailed)
        );
    }

    #[test]
    fn hw_target_matrix_and_serde() {
        assert!(VideoTarget::Mp4.supports_hw());
        assert!(VideoTarget::Ts.supports_hw());
        assert!(!VideoTarget::Webm.supports_hw());
        assert!(!VideoTarget::Avi.supports_hw());
        assert_eq!(
            nvenc_encoder_name(VideoEncoder::Libx264),
            Some("h264_nvenc")
        );
        assert_eq!(nvenc_encoder_name(VideoEncoder::Mpeg4), None);
        assert_eq!(
            serde_json::to_value(Accelerator::Nvenc).unwrap(),
            serde_json::Value::String("nvenc".to_owned())
        );
        // 旧载荷缺 `accelerator` 仍可反序列化（前向兼容，默认 CPU）
        let legacy = serde_json::json!({
            "target": "mp4",
            "mode": "auto",
            "output_dir": "/tmp",
            "overwrite": "increment",
        });
        let options: VideoConvertOptions = serde_json::from_value(legacy).unwrap();
        assert_eq!(options.accelerator, Accelerator::Cpu);
        // 旧结果缺 `used_hw` 仍可反序列化（默认非硬加速）
        let legacy_outcome = serde_json::json!({
            "ok": true,
            "input": "a.mp4",
            "output": "b.mp4",
            "error": null,
            "target": "mp4",
            "tried_copy": false,
        });
        let outcome: VideoConvertOutcome = serde_json::from_value(legacy_outcome).unwrap();
        assert!(!outcome.used_hw);
    }

    #[test]
    fn stderr_tail_keeps_last_bytes() {
        assert_eq!(stderr_tail(b"abc"), "abc");
        let long = vec![b'x'; 3000];
        assert_eq!(stderr_tail(&long).len(), 2048);
    }
}
