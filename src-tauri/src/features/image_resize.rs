//! 图片批量调整尺寸业务逻辑（工具路由 `image/resize`）：元信息读取 + 单张缩放的纯函数实现。
//!
//! 输入六格式：JPEG/PNG/WebP/BMP/TIFF/GIF（动图只取首帧静态处理，`Original` 输出时转 PNG，
//! 是已知取舍）；输出四格式：保持原样/JPEG/PNG/WebP（`image 0.25` 的 WebP 编码仅无损，
//! 质量滑块只对 JPEG 生效，其余格式忽略该值）。
//! EXIF 方向仅处理 JPEG（最常见来源），读取后摆正再算尺寸，保证预览与输出一致。
//! 业务失败全部装进 [`ResizeSingleOutcome`] 返回，不抛错（单张失败跳过继续是常态 UI 状态）；
//! 文件打不开这类偶发异常才经 `anyhow` 走 `CommandError`（仅 `read_image_info`）。

use std::io::Cursor;

use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};
use serde::{Deserialize, Serialize};
use specta::Type;

/// 单个输入文件上限：50 MiB，超限直接拒收（错误码 `TooLarge`），不进解码器
const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// 单边上限：16384px，超限即 `InvalidSize`（防误填与内存爆炸）
const MAX_SIDE: u32 = 16384;

/// 总像素上限：一亿像素，超限即 `InvalidSize`（RGBA 约 400MB，不可接受）
const MAX_PIXELS: u64 = 100_000_000;

/// 百分比上限定为 1000%（更大纯属误填）；下限 1%（0% 无意义）
const MAX_PERCENT: u32 = 1000;

/// 支持的输入图片格式：与文件对话框过滤器、扩展名白名单三处同源（见 `SUPPORTED_EXTENSIONS`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Bmp,
    Tiff,
    Gif,
}

/// 精确尺寸的填充策略：拉伸/适应/裁剪/留白（`Contain` 输出即 fitted 尺寸，无画布；
/// `Pad` 输出精确画布，留白透明或白色，见 `render`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    Stretch,
    Contain,
    Cover,
    Pad,
}

/// 尺寸调整模式：宽高锁定/百分比/精确尺寸（线上传 `lowercase`，与前端参数面板取值一致）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum ResizeMode {
    WidthHeight {
        width: Option<u32>,
        height: Option<u32>,
        lock_ratio: bool,
    },
    Percent {
        percent: u32,
    },
    Exact {
        width: u32,
        height: u32,
        fit: FitMode,
    },
}

/// 输出格式：`Original` 保持输入格式（GIF 输入转 PNG，动图只取首帧）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Original,
    Jpeg,
    Png,
    Webp,
}

/// 手动旋转/翻转：在 EXIF 自动摆正之后、尺寸规划之前应用（90/270 系自动交换宽高）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ImageRotation {
    None,
    Cw90,
    Cw180,
    Ccw90,
    FlipHorizontal,
}

/// 缩放插值算法：自建映射枚举（`image::FilterType` 是外部类型，`specta` 导出不了）；
/// 默认 `Lanczos3` 高质量，`Nearest` 最快但锯齿明显
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

/// 重名文件处理策略：递增重命名（现状）/ 直接覆盖 / 跳过（错误码 `Skipped`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum OverwritePolicy {
    Increment,
    Overwrite,
    Skip,
}

/// 单张处理选项（前端参数面板全量下发；`quality` 仅 JPEG 生效，1-100，越界钳制；
/// `target_size_kb` 仅 JPEG 生效，置位时二分质量并隐藏质量滑块）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ResizeOptions {
    pub mode: ResizeMode,
    pub format: OutputFormat,
    pub quality: u8,
    pub output_dir: String,
    pub no_upscale: bool,
    pub filename_suffix: bool,
    pub rotation: ImageRotation,
    pub filter: ResizeFilter,
    pub overwrite: OverwritePolicy,
    pub target_size_kb: Option<u32>,
}

/// 图片元信息：列表展示与预计输出尺寸计算用（`file_size` 为字节数，输入有 50MB 上限，转 `u32` 必成功）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub file_size: u32,
}

/// 业务错误码：前端按码映射 i18n 文案，`message` 只做诊断补充（英文技术文本）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum ResizeErrorCode {
    TooLarge,
    UnsupportedFormat,
    DecodeFailed,
    InvalidSize,
    EncodeFailed,
    OutputNotWritable,
    Skipped,
}

/// 业务错误体：扁平结构便于 `specta` 导出，前端按 `code` 分支
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ResizeError {
    pub code: ResizeErrorCode,
    pub message: Option<String>,
}

/// 单张处理结果：`ok` 为真时读 `output`/输出尺寸，为假时读 `error`（失败跳过继续）；
/// `target_met` 仅设目标大小时有值（`None` 即无目标；非 JPEG 时为 `Some(false)`，前端再分“未达成/不适用”）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ResizeSingleOutcome {
    pub ok: bool,
    pub input: String,
    pub output: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub error: Option<ResizeError>,
    pub target_met: Option<bool>,
}

impl ResizeSingleOutcome {
    const fn success(
        input: String,
        output: String,
        width: u32,
        height: u32,
        target_met: Option<bool>,
    ) -> Self {
        Self {
            ok: true,
            input,
            output: Some(output),
            width: Some(width),
            height: Some(height),
            error: None,
            target_met,
        }
    }

    fn failure(input: String, code: ResizeErrorCode, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            input,
            output: None,
            width: None,
            height: None,
            error: Some(ResizeError {
                code,
                message: Some(message.into()),
            }),
            target_met: None,
        }
    }
}

/// 扩展名白名单：与对话框过滤器同源，未知扩展名直接拒收（内容嗅探只做第二道校验）
const SUPPORTED_EXTENSIONS: &[(&str, ImageFormat)] = &[
    ("jpg", ImageFormat::Jpeg),
    ("jpeg", ImageFormat::Jpeg),
    ("png", ImageFormat::Png),
    ("webp", ImageFormat::Webp),
    ("bmp", ImageFormat::Bmp),
    ("tif", ImageFormat::Tiff),
    ("tiff", ImageFormat::Tiff),
    ("gif", ImageFormat::Gif),
];

/// 拖放路径展开结果：文件原样透传，文件夹展平为其下图片；
/// `output_dir` 为首个文件夹下的 `resized/`（前端仅在输出目录为空时采用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ExpandDropOutcome {
    pub files: Vec<String>,
    pub output_dir: Option<String>,
}

/// 展开拖放路径：不存在/非图片文件原样透传（前端按单张标红），
/// 文件夹展开为顶层图片文件（按文件名排序，不递归，避免误扫深层目录）；
/// 首个文件夹给出 `resized/` 建议输出目录。目录枚举失败整体抛错，前端回落原路径。
pub fn expand_dropped_paths(paths: &[String]) -> anyhow::Result<ExpandDropOutcome> {
    use anyhow::Context;

    let mut files = Vec::new();
    let mut output_dir = None;
    for path in paths {
        let candidate = std::path::Path::new(path);
        if !candidate.is_dir() {
            files.push(path.clone());
            continue;
        }
        let mut inner: Vec<String> = std::fs::read_dir(candidate)
            .with_context(|| format!("cannot list directory: {path}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|child| child.is_file())
            .filter_map(|child| {
                let text = child.to_str()?;
                supported_format_from_path(text)?;
                Some(text.to_string())
            })
            .collect();
        inner.sort();
        files.extend(inner);
        if output_dir.is_none() {
            output_dir = candidate.join("resized").to_str().map(str::to_string);
        }
    }
    Ok(ExpandDropOutcome { files, output_dir })
}

/// 按扩展名判定格式：大小写不敏感，无扩展名或不在白名单即 `None`
pub fn supported_format_from_path(path: &str) -> Option<ImageFormat> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .and_then(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .find(|(known, _)| *known == ext)
                .map(|(_, format)| *format)
        })
}

/// 输出扩展名：`Original` 跟输入走（GIF 转 PNG，首帧静态化）；显式格式按目标扩展名
pub const fn output_extension(requested: OutputFormat, source: ImageFormat) -> &'static str {
    match requested {
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Png => "png",
        OutputFormat::Webp => "webp",
        OutputFormat::Original => original_extension(source),
    }
}

/// 原格式扩展名：GIF 静态化落 PNG，其余跟输入格式（与内层 `match` 拆分，消重复臂）
const fn original_extension(source: ImageFormat) -> &'static str {
    match source {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png | ImageFormat::Gif => "png",
        ImageFormat::Webp => "webp",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
    }
}

/// 目标尺寸规划（纯函数，前端 `resize-math.ts` 有同语义镜像，两侧单测用同一向量）：
/// 宽高双填 + 锁定即内适应；单填按比例；`no_upscale` 时任一边超出即整体回落原尺寸
/// （逐边钳制会破坏纵横比，回落整图更可预测）；`Cover`/`Pad` 画布即目标尺寸，
/// `Contain`/锁定模式输出 fitted 尺寸
pub fn plan_dimensions(
    src_width: u32,
    src_height: u32,
    mode: &ResizeMode,
    no_upscale: bool,
) -> Result<(u32, u32), ResizeErrorCode> {
    if src_width == 0 || src_height == 0 {
        return Err(ResizeErrorCode::InvalidSize);
    }
    let planned = match *mode {
        ResizeMode::WidthHeight {
            width,
            height,
            lock_ratio,
        } => plan_box(src_width, src_height, width, height, lock_ratio)?,
        ResizeMode::Percent { percent } => plan_percent(src_width, src_height, percent)?,
        ResizeMode::Exact { width, height, fit } => {
            plan_exact(src_width, src_height, width, height, fit)?
        }
    };
    let (mut out_w, mut out_h) = planned;
    if no_upscale && (out_w > src_width || out_h > src_height) {
        out_w = src_width;
        out_h = src_height;
    }
    guard_sides(out_w, out_h)?;
    Ok((out_w, out_h))
}

/// 宽高模式：双空/零值拒绝；双填 + 锁定走内适应，双填不锁定即拉伸；单填按比例换算
fn plan_box(
    src_width: u32,
    src_height: u32,
    width: Option<u32>,
    height: Option<u32>,
    lock_ratio: bool,
) -> Result<(u32, u32), ResizeErrorCode> {
    match (width, height) {
        (None, None) | (Some(0), _) | (_, Some(0)) => Err(ResizeErrorCode::InvalidSize),
        (Some(w), Some(h)) if !lock_ratio => Ok((w, h)),
        (Some(w), Some(h)) => Ok(fit_inside(src_width, src_height, w, h)),
        (Some(w), None) => {
            let h = scale_side(src_height, w, src_width)?;
            Ok((w, h))
        }
        (None, Some(h)) => {
            let w = scale_side(src_width, h, src_height)?;
            Ok((w, h))
        }
    }
}

/// 百分比模式：1%-1000%，换算后至少 1px（`u64` 中转防 `u32` 溢出）
fn plan_percent(
    src_width: u32,
    src_height: u32,
    percent: u32,
) -> Result<(u32, u32), ResizeErrorCode> {
    if !(1..=MAX_PERCENT).contains(&percent) {
        return Err(ResizeErrorCode::InvalidSize);
    }
    let width = ((u64::from(src_width) * u64::from(percent)) / 100).max(1);
    let height = ((u64::from(src_height) * u64::from(percent)) / 100).max(1);
    let (width, height) = (saturating_u32(width), saturating_u32(height));
    guard_sides(width, height)?;
    Ok((width, height))
}

/// 精确模式：零值/超边拒绝；`Contain` 输出 fitted 尺寸，其余输出精确画布
fn plan_exact(
    src_width: u32,
    src_height: u32,
    width: u32,
    height: u32,
    fit: FitMode,
) -> Result<(u32, u32), ResizeErrorCode> {
    if width == 0 || height == 0 {
        return Err(ResizeErrorCode::InvalidSize);
    }
    guard_sides(width, height)?;
    if fit == FitMode::Contain {
        return Ok(fit_inside(src_width, src_height, width, height));
    }
    Ok((width, height))
}

/// 内适应换算：取最小缩放比，两边都不超出目标框（至少 1px）
// `as` 安全：输入边长 ≥1（调用方已拒零），缩放比为正有限值，`round` 后非负
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn fit_inside(src_width: u32, src_height: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    let scale =
        (f64::from(box_w) / f64::from(src_width)).min(f64::from(box_h) / f64::from(src_height));
    let width = ((f64::from(src_width) * scale).round() as u64).max(1);
    let height = ((f64::from(src_height) * scale).round() as u64).max(1);
    (saturating_u32(width), saturating_u32(height))
}

/// 单边按比例换算：`target * src_side / src_base`（`u64` 中转，至少 1px）
fn scale_side(src_side: u32, target: u32, src_base: u32) -> Result<u32, ResizeErrorCode> {
    if src_base == 0 {
        return Err(ResizeErrorCode::InvalidSize);
    }
    let scaled = ((u64::from(src_side) * u64::from(target)) / u64::from(src_base)).max(1);
    Ok(saturating_u32(scaled))
}

/// 尺寸守卫：单边与总像素双上限，超限即 `InvalidSize`（内存爆炸拦在分配之前）
fn guard_sides(width: u32, height: u32) -> Result<(), ResizeErrorCode> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(ResizeErrorCode::InvalidSize);
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(ResizeErrorCode::InvalidSize);
    }
    Ok(())
}

/// `u64` 转契约 `u32`：输入有像素上限，转换必成功，保底防溢出
fn saturating_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// 读取图片元信息：路径来自拖放/对话框，后端二次校验（存在性/文件类型/大小/可解码）；
/// 只读文件头尺寸（`into_dimensions`），不做全图解码；JPEG 方向摆正后宽高可能互换。
/// 文件打不开是偶发异常，直接抛错走 `CommandError`；前端按单张标红处理。
pub fn read_image_info(path: &str) -> anyhow::Result<ImageInfo> {
    use anyhow::Context;

    let metadata =
        std::fs::metadata(path).with_context(|| format!("cannot access file: {path}"))?;
    if !metadata.is_file() {
        anyhow::bail!("not a regular file: {path}");
    }
    if metadata.len() > MAX_FILE_BYTES {
        anyhow::bail!("file exceeds the 50 MiB limit: {path}");
    }
    let hinted = supported_format_from_path(path);
    if hinted.is_none() {
        anyhow::bail!("unsupported image format: {path}");
    }
    let reader = ImageReader::open(path)
        .with_context(|| format!("cannot open image: {path}"))?
        .with_guessed_format()
        .with_context(|| format!("cannot detect image format: {path}"))?;
    // 内容嗅探优先：扩展名仅做白名单，真格式以解码器为准（大小写/别名在此收敛）
    let sniffed = reader.format().and_then(native_to_format);
    let format = sniffed.or(hinted).unwrap_or(ImageFormat::Png);
    let mut decoder = reader
        .into_decoder()
        .map_err(|err| anyhow::anyhow!("cannot create image decoder: {err}"))?;
    // 方向读取失败按无变换处理（非 JPEG 等格式本就没有 EXIF 方向）
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let (mut width, mut height) = decoder.dimensions();
    if orientation_swaps_axes(orientation) {
        std::mem::swap(&mut width, &mut height);
    }
    Ok(ImageInfo {
        width,
        height,
        format,
        file_size: u32::try_from(metadata.len()).unwrap_or(u32::MAX),
    })
}

/// 旋转 90/270 系方向会互换宽高（`apply_orientation` 后亦如此，此处提前对齐）
const fn orientation_swaps_axes(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

/// 原生格式映射：内容嗅探结果转契约格式（未知变体回落 `None`，调用方再用扩展名兜底）
const fn native_to_format(format: image::ImageFormat) -> Option<ImageFormat> {
    use image::ImageFormat as Native;
    match format {
        Native::Jpeg => Some(ImageFormat::Jpeg),
        Native::Png => Some(ImageFormat::Png),
        Native::WebP => Some(ImageFormat::Webp),
        Native::Bmp => Some(ImageFormat::Bmp),
        Native::Tiff => Some(ImageFormat::Tiff),
        Native::Gif => Some(ImageFormat::Gif),
        _ => None,
    }
}

/// 执行单张缩放：全链路不抛错，失败一律装进 `ResizeSingleOutcome`（批量跳过继续）。
/// 输出目录不存在即创建；重名按 `overwrite` 策略处理（递增/覆盖/跳过）。
pub fn resize_image(input: &str, options: &ResizeOptions) -> ResizeSingleOutcome {
    match resize_inner(input, options) {
        Ok((output, width, height, target_met)) => {
            ResizeSingleOutcome::success(input.to_string(), output, width, height, target_met)
        }
        Err((code, message)) => ResizeSingleOutcome::failure(input.to_string(), code, message),
    }
}

/// 输入校验失败分类（内部类型）：`decode_oriented` 的结构化错误，调用方直接映射错误码。
/// 此前靠消息子串（`"50 MiB"` / `"unsupported image format"`）反推，改一句文案就错配。
#[derive(Debug)]
enum InputFault {
    TooLarge(String),
    Unsupported(String),
    Decode(anyhow::Error),
}

impl From<InputFault> for anyhow::Error {
    fn from(fault: InputFault) -> Self {
        match fault {
            InputFault::TooLarge(message) | InputFault::Unsupported(message) => {
                anyhow::anyhow!("{message}")
            }
            InputFault::Decode(err) => err,
        }
    }
}

/// 输入校验 + 解码 + 方向摆正（`resize_inner` 与缩略图共用）：
/// 存在性/文件类型/大小/扩展名白名单 + 全图解码；JPEG 按 EXIF 摆正，保证预览与输出一致。
/// 文件一次读全量：大小复检防竞态，解码与方向嗅探吃同一份字节，不再重复 `open`；
/// 解码走内容嗅探（不依赖扩展名选解码器），与扩展名白名单双保险。
fn decode_oriented(input: &str) -> Result<(DynamicImage, ImageFormat), InputFault> {
    use anyhow::Context;

    let metadata = std::fs::metadata(input)
        .with_context(|| format!("cannot access file: {input}"))
        .map_err(InputFault::Decode)?;
    if !metadata.is_file() {
        return Err(InputFault::Decode(anyhow::anyhow!(
            "not a regular file: {input}"
        )));
    }
    let bytes = std::fs::read(input)
        .with_context(|| format!("cannot read file: {input}"))
        .map_err(InputFault::Decode)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err(InputFault::TooLarge(format!(
            "file exceeds the 50 MiB limit: {input}"
        )));
    }
    let source_format = supported_format_from_path(input)
        .ok_or_else(|| InputFault::Unsupported(format!("unsupported image format: {input}")))?;
    let mut decoded = image::load_from_memory(&bytes)
        .with_context(|| format!("cannot decode image: {input}"))
        .map_err(InputFault::Decode)?;
    // JPEG 方向摆正：`load_from_memory` 不应用 EXIF，此处手动对齐预览与输出
    if source_format == ImageFormat::Jpeg
        && let Some(orientation) = jpeg_orientation_from_bytes(&bytes)
        && orientation != Orientation::NoTransforms
    {
        decoded.apply_orientation(orientation);
    }
    Ok((decoded, source_format))
}

/// JPEG EXIF 方向读取（字节版）：失败按无变换处理（无 EXIF/非 JPEG 头都不阻断主流程）
fn jpeg_orientation_from_bytes(bytes: &[u8]) -> Option<Orientation> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    decoder.orientation().ok()
}

/// 缩略图边长上限：2048（请求超限直接拒绝，不进解码器）
const MAX_THUMB_SIDE: u32 = 2048;

/// 缩略图 JPEG 质量：预览用途，72 兼顾体积与清晰度
const THUMB_QUALITY: u8 = 72;

/// 读取预览缩略图：等比压到 `max_side` 内（小图不放大），JPEG 编码后包成
/// `data:` URL 回前端。WebView 的 `asset` 协议需配 scope 才放行本地路径，
/// 直接回 data URL 可保持最小授权（CSP 的 `img-src` 已放行 `data:`）。
/// 失败抛错走 `CommandError`，前端按单张标红处理。
pub fn read_image_thumbnail(path: &str, max_side: u32) -> anyhow::Result<String> {
    if !(1..=MAX_THUMB_SIDE).contains(&max_side) {
        anyhow::bail!("invalid thumbnail size: {max_side}");
    }
    let (decoded, _) = decode_oriented(path)?;
    thumb_data_url(&decoded, max_side)
}

/// 批量并发度：解码 + `Lanczos3` 为 CPU 密集，保守取 4（单线程解码库下超配无收益）
pub const IMAGE_BATCH_WORKERS: usize = 4;

/// 单次批量上限：500（前端超限自行切块，`data:` URL 总量可控不爆 IPC）
pub const MAX_IMAGE_BATCH: usize = 500;

/// 列表缩略图边长上限：128（96px 列表行 + 高分屏余量；768 大预览走单文件命令按需取）
pub const MAX_LIST_SIDE: u32 = 128;

/// 批量单项结果：`ok` 为真读 `info`（`thumb` 缺失仅空预览，不降级条目），为假读 `error`；
/// 顺序与入参一一对应，前端按下标合并无需再对齐
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ImageBatchItem {
    pub path: String,
    pub ok: bool,
    pub info: Option<ImageInfo>,
    pub thumb: Option<String>,
    pub error: Option<String>,
}

impl ImageBatchItem {
    const fn success(path: String, info: ImageInfo, thumb: Option<String>) -> Self {
        Self {
            path,
            ok: true,
            info: Some(info),
            thumb,
            error: None,
        }
    }

    const fn failure(path: String, message: String) -> Self {
        Self {
            path,
            ok: false,
            info: None,
            thumb: None,
            error: Some(message),
        }
    }
}

/// 批量读取元信息 + 列表小图：单文件内一次全解码同时产出两者（旧双命令是两次读盘），
/// 文件间 4 线程并发；单项失败装进 `error` 跳过继续，不抛错中断整批。
/// 参数非法（边长越界/超量）才整批抛错，前端切块重试。
pub fn read_image_batch(paths: &[String], list_side: u32) -> anyhow::Result<Vec<ImageBatchItem>> {
    if !(1..=MAX_LIST_SIDE).contains(&list_side) {
        anyhow::bail!("invalid list thumbnail size: {list_side}");
    }
    if paths.len() > MAX_IMAGE_BATCH {
        anyhow::bail!("too many files in one batch: {}", paths.len());
    }
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    // 槽位与入参同下标：工作线程只写自己抢到的槽，互不重叠，顺序天然保留
    let slots: Vec<std::sync::Mutex<Option<ImageBatchItem>>> =
        paths.iter().map(|_| std::sync::Mutex::new(None)).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = IMAGE_BATCH_WORKERS.min(paths.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(path) = paths.get(index) else {
                        break;
                    };
                    let item = read_image_entry(path, list_side);
                    if let Ok(mut slot) = slots[index].lock() {
                        *slot = Some(item);
                    }
                }
            });
        }
    });
    let mut out = Vec::with_capacity(paths.len());
    for (index, slot) in slots.into_iter().enumerate() {
        match slot.into_inner() {
            Ok(Some(item)) => out.push(item),
            // 锁中毒仅理论可达：回落失败项保住顺序与长度，前端按单张标红
            _ => out.push(ImageBatchItem::failure(
                paths[index].clone(),
                "batch slot poisoned".to_string(),
            )),
        }
    }
    Ok(out)
}

/// 批量单项装配：一次 `decode_oriented` 同时拿尺寸与像素，缩略图失败仅空图不降级条目
fn read_image_entry(path: &str, list_side: u32) -> ImageBatchItem {
    let (decoded, format) = match decode_oriented(path) {
        Ok(pair) => pair,
        Err(fault) => return ImageBatchItem::failure(path.to_string(), fault_message(&fault)),
    };
    let file_size = std::fs::metadata(path).map_or(u32::MAX, |meta| {
        u32::try_from(meta.len()).unwrap_or(u32::MAX)
    });
    let info = ImageInfo {
        width: decoded.width(),
        height: decoded.height(),
        format,
        file_size,
    };
    // 缩略图失败仅空图（`None`），不降级条目
    let thumb = thumb_data_url(&decoded, list_side).ok();
    ImageBatchItem::success(path.to_string(), info, thumb)
}

/// 输入校验失败转单项错误文本：与 `resize_inner` 的码映射同口径，前端只做展示
fn fault_message(fault: &InputFault) -> String {
    match fault {
        InputFault::TooLarge(message) | InputFault::Unsupported(message) => message.clone(),
        InputFault::Decode(err) => format!("{err:#}"),
    }
}

// `as` 安全：`max_side` 已钳制 1-2048，缩放比为 `(0, 1]` 有限值，`round` 后非负
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn thumb_data_url(decoded: &DynamicImage, max_side: u32) -> anyhow::Result<String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let longest = decoded.width().max(decoded.height());
    let (thumb_w, thumb_h) = if longest <= max_side {
        (decoded.width(), decoded.height())
    } else {
        let scale = f64::from(max_side) / f64::from(longest);
        let width = ((f64::from(decoded.width()) * scale).round() as u64).max(1);
        let height = ((f64::from(decoded.height()) * scale).round() as u64).max(1);
        (saturating_u32(width), saturating_u32(height))
    };
    // 缩略图固定最高质量插值（预览用途，不跟随用户选择的批量插值）
    let thumb = DynamicImage::ImageRgba8(imageops_resize(
        decoded,
        thumb_w,
        thumb_h,
        FilterType::Lanczos3,
    ));
    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, THUMB_QUALITY);
    flatten_white(&thumb)
        .write_with_encoder(encoder)
        .map_err(|err| anyhow::anyhow!("cannot encode thumbnail: {err}"))?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        STANDARD.encode(buf.into_inner())
    ))
}

/// 单张缩放产物：输出路径/输出宽/输出高/目标大小是否达成（无目标时为 `None`）
type ResizeProduct = (String, u32, u32, Option<bool>);

fn resize_inner(
    input: &str,
    options: &ResizeOptions,
) -> Result<ResizeProduct, (ResizeErrorCode, String)> {
    use anyhow::Context;

    let fail =
        |code: ResizeErrorCode, message: String| -> (ResizeErrorCode, String) { (code, message) };
    let (decoded, source_format) = decode_oriented(input).map_err(|fault| match fault {
        InputFault::TooLarge(message) => fail(ResizeErrorCode::TooLarge, message),
        InputFault::Unsupported(message) => fail(ResizeErrorCode::UnsupportedFormat, message),
        InputFault::Decode(err) => fail(ResizeErrorCode::DecodeFailed, format!("{err:#}")),
    })?;
    // 手动旋转：EXIF 摆正之后、尺寸规划之前（90/270 系交换宽高，后续规划自动对齐）
    let decoded = apply_rotation(decoded, options.rotation);
    let (src_w, src_h) = (decoded.width(), decoded.height());
    let (out_w, out_h) = plan_dimensions(src_w, src_h, &options.mode, options.no_upscale)
        .map_err(|code| fail(code, "invalid target size for this image".to_string()))?;
    let rendered = render(
        &decoded,
        out_w,
        out_h,
        exact_fit(&options.mode),
        filter_type(options.filter),
    );
    let (encoded, target_met) = encode_rendered(&rendered, source_format, options)
        .map_err(|message| fail(ResizeErrorCode::EncodeFailed, message))?;
    let output_path = reserve_output_path(input, options, source_format, out_w, out_h)
        .map_err(|err| fail(ResizeErrorCode::OutputNotWritable, format!("{err:#}")))?;
    let Some(output_path) = output_path else {
        // 重名跳过：文件已存在且策略为 `Skip`，不写盘（前端计入跳过数，不算失败）
        return Err(fail(
            ResizeErrorCode::Skipped,
            format!(
                "output exists, skipped: {}",
                output_name(input, options, source_format, out_w, out_h)
            ),
        ));
    };
    std::fs::write(&output_path, encoded)
        .with_context(|| format!("cannot write output file: {output_path}"))
        .map_err(|err| fail(ResizeErrorCode::OutputNotWritable, format!("{err:#}")))?;
    Ok((output_path, out_w, out_h, target_met))
}

/// 精确模式的填充策略提取：非精确模式一律按直接缩放渲染（`plan` 已算好 fitted 尺寸）
const fn exact_fit(mode: &ResizeMode) -> Option<FitMode> {
    match *mode {
        ResizeMode::Exact { fit, .. } => Some(fit),
        _ => None,
    }
}

/// 手动旋转/翻转：`DynamicImage` 原生方法保色深（不经过 RGBA 中转）；
/// 90/270 系交换宽高，调用方在规划前应用即可对齐
fn apply_rotation(image: DynamicImage, rotation: ImageRotation) -> DynamicImage {
    match rotation {
        ImageRotation::None => image,
        ImageRotation::Cw90 => image.rotate90(),
        ImageRotation::Cw180 => image.rotate180(),
        ImageRotation::Ccw90 => image.rotate270(),
        ImageRotation::FlipHorizontal => image.fliph(),
    }
}

/// 插值映射：契约枚举转 `image` 原生类型（透传给全部缩放点）
const fn filter_type(filter: ResizeFilter) -> FilterType {
    match filter {
        ResizeFilter::Nearest => FilterType::Nearest,
        ResizeFilter::Triangle => FilterType::Triangle,
        ResizeFilter::CatmullRom => FilterType::CatmullRom,
        ResizeFilter::Gaussian => FilterType::Gaussian,
        ResizeFilter::Lanczos3 => FilterType::Lanczos3,
    }
}

/// 是否 JPEG 输出（含 `Original` 跟随 JPEG 输入）：质量滑块与目标大小只在此生效
const fn is_jpeg_output(source: ImageFormat, requested: OutputFormat) -> bool {
    matches!(requested, OutputFormat::Jpeg)
        || (matches!(requested, OutputFormat::Original) && matches!(source, ImageFormat::Jpeg))
}

/// 目标文件大小上限（KB）：0 视为未设（前端钳制 10-51200，此处双保险）
fn target_bytes(options: &ResizeOptions) -> Option<u64> {
    match options.target_size_kb {
        Some(kb) if kb > 0 => Some(u64::from(kb) * 1024),
        _ => None,
    }
}

/// 渲染：按 `filter` 插值缩放；`Cover` 按覆盖比例缩放后居中裁剪；
/// `Pad` 按适应比例缩放后居中贴到画布（PNG/WebP 透明底，其余白色底）
fn render(
    image: &DynamicImage,
    width: u32,
    height: u32,
    fit: Option<FitMode>,
    filter: FilterType,
) -> DynamicImage {
    match fit {
        Some(FitMode::Cover) => render_cover(image, width, height, filter),
        Some(FitMode::Pad) => render_pad(image, width, height, filter),
        _ => DynamicImage::ImageRgba8(imageops_resize(image, width, height, filter)),
    }
}

/// 直接缩放：统一转 RGBA 再缩（调用方按输出格式展平，渲染层不关心目标编码）
fn imageops_resize(
    image: &DynamicImage,
    width: u32,
    height: u32,
    filter: FilterType,
) -> image::RgbaImage {
    image::imageops::resize(&image.to_rgba8(), width, height, filter)
}

/// 覆盖裁剪：按最大比缩放使画布全覆盖，再居中裁出精确画布
// `as` 安全：源图与目标边长 ≥1（解码真图与守卫保证），缩放比为正有限值
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn render_cover(image: &DynamicImage, width: u32, height: u32, filter: FilterType) -> DynamicImage {
    let (src_w, src_h) = (f64::from(image.width()), f64::from(image.height()));
    let scale = (f64::from(width) / src_w).max(f64::from(height) / src_h);
    let scaled_w = ((src_w * scale).round() as u64).max(1);
    let scaled_h = ((src_h * scale).round() as u64).max(1);
    let scaled = imageops_resize(
        image,
        saturating_u32(scaled_w),
        saturating_u32(scaled_h),
        filter,
    );
    let (off_x, off_y) = (
        scaled.width().saturating_sub(width) / 2,
        scaled.height().saturating_sub(height) / 2,
    );
    let cropped = image::imageops::crop_imm(&scaled, off_x, off_y, width, height).to_image();
    DynamicImage::ImageRgba8(cropped)
}

/// 留白画布：适应缩放后居中粘贴；透明底供 PNG/WebP，白色底供其余格式
/// （调用方 `encode_image` 同规则展平，两处分支必须同步改）
fn render_pad(image: &DynamicImage, width: u32, height: u32, filter: FilterType) -> DynamicImage {
    let (fitted_w, fitted_h) = fit_inside(image.width(), image.height(), width, height);
    let fitted = imageops_resize(image, fitted_w, fitted_h, filter);
    let (off_x, off_y) = (
        width.saturating_sub(fitted_w) / 2,
        height.saturating_sub(fitted_h) / 2,
    );
    let mut canvas = image::RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));
    image::imageops::overlay(&mut canvas, &fitted, i64::from(off_x), i64::from(off_y));
    DynamicImage::ImageRgba8(canvas)
}

/// 编码：JPEG 按质量有损（透明先压白底），PNG/WebP 无损，BMP/TIFF 仅原格式直通
fn encode_image(
    image: &DynamicImage,
    source: ImageFormat,
    options: &ResizeOptions,
) -> Result<Vec<u8>, String> {
    use image::codecs::png::PngEncoder;

    let effective = match options.format {
        OutputFormat::Original => match source {
            ImageFormat::Gif | ImageFormat::Png => OutputFormat::Png,
            ImageFormat::Jpeg => OutputFormat::Jpeg,
            ImageFormat::Webp => OutputFormat::Webp,
            ImageFormat::Bmp | ImageFormat::Tiff => return encode_other(image, source),
        },
        explicit => explicit,
    };
    let mut buf = Cursor::new(Vec::new());
    match effective {
        // 常规质量路径走共享单次编码（目标大小走二分，不进这里）
        OutputFormat::Jpeg => {
            return encode_jpeg_with_quality(image, options.quality.clamp(1, 100));
        }
        OutputFormat::Png => {
            let encoder = PngEncoder::new(&mut buf);
            image
                .write_with_encoder(encoder)
                .map_err(|err| err.to_string())?;
        }
        OutputFormat::Webp => {
            // `image 0.25` 的 WebP 编码仅无损（VP8L），`quality` 在此不生效是已知局限
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
            image
                .write_with_encoder(encoder)
                .map_err(|err| err.to_string())?;
        }
        OutputFormat::Original => unreachable!("GIF 已在上游映射为 PNG，其余原格式走直通"),
    }
    Ok(buf.into_inner())
}

/// JPEG 单次编码（透明先压白底）：目标大小二分与常规质量路径共用
fn encode_jpeg_with_quality(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    use image::codecs::jpeg::JpegEncoder;

    let mut buf = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100));
    flatten_white(image)
        .write_with_encoder(encoder)
        .map_err(|err| err.to_string())?;
    Ok(buf.into_inner())
}

/// 渲染结果编码（含目标大小）：`target_size_kb` 置位且 JPEG 输出时二分质量；
/// 置位但非 JPEG 时常规编码 + 未达成（前端区分“未达成/不适用”，不静默）；
/// 未置位时是否达成恒为 `None`
fn encode_rendered(
    image: &DynamicImage,
    source: ImageFormat,
    options: &ResizeOptions,
) -> Result<(Vec<u8>, Option<bool>), String> {
    match (
        target_bytes(options),
        is_jpeg_output(source, options.format),
    ) {
        (Some(target), true) => {
            let (bytes, met) = encode_jpeg_target(image, target)?;
            Ok((bytes, Some(met)))
        }
        (Some(_), false) => Ok((encode_image(image, source, options)?, Some(false))),
        (None, _) => Ok((encode_image(image, source, options)?, None)),
    }
}

/// 目标文件大小（仅 JPEG）：质量在 `[5, 95]` 二分，取达标的最高质量；
/// 连最低质量都超则输出最小文件 + 未达成（调用方照常落盘，前端附注提示）
// `as` 安全：`mid` 恒 ∈[5, 95]（循环不变式），转 `u8` 不截断；文件字节数转 `u64` 在 64 位下无损
#[allow(clippy::cast_possible_truncation)]
fn encode_jpeg_target(image: &DynamicImage, target: u64) -> Result<(Vec<u8>, bool), String> {
    const MIN_QUALITY: u32 = 5;
    const MAX_QUALITY: u32 = 95;

    // `u32` 域二分防 `u8` 溢出；达标即收敛，约 7 次编码（JPEG 编码快，可接受）。
    // `mid` 恒 ≥5（`low` 只增不减），`mid - 1` 永不下溢
    let mut low = MIN_QUALITY;
    let mut high = MAX_QUALITY;
    let mut best = (encode_jpeg_with_quality(image, MIN_QUALITY as u8)?, false);
    while low <= high {
        let mid = low + (high - low) / 2;
        let bytes = encode_jpeg_with_quality(image, mid as u8)?;
        if (bytes.len() as u64) <= target {
            best = (bytes, true);
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    Ok(best)
}

/// BMP/TIFF 原格式直通：只有 `Original` 能走到这里（显式转 BMP/TIFF 不开放）
fn encode_other(image: &DynamicImage, source: ImageFormat) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    match source {
        ImageFormat::Bmp => {
            let encoder = image::codecs::bmp::BmpEncoder::new(&mut buf);
            image
                .write_with_encoder(encoder)
                .map_err(|err| err.to_string())?;
        }
        ImageFormat::Tiff => {
            let encoder = image::codecs::tiff::TiffEncoder::new(&mut buf);
            image
                .write_with_encoder(encoder)
                .map_err(|err| err.to_string())?;
        }
        _ => return Err("only BMP/TIFF pass through here".to_string()),
    }
    Ok(buf.into_inner())
}

/// 透明压白底：JPEG/BMP 无透明通道，RGBA 先与白底合成再编码（避免透明变黑）。
/// 整块 buffer 按 4 字节切分混合（`+127` 四舍五入，结果恒 ≤255），避开逐像素 `put_pixel` 的边界检查；
/// 大图（上限 1 亿像素）下与此前逐像素版本输出逐字节一致
fn flatten_white(image: &DynamicImage) -> DynamicImage {
    let rgba = image.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    // RGBA 缓冲恒为 4 字节对齐；容量按原长预留（多 1/4，免除法）
    let mut flat = Vec::with_capacity(rgba.len());
    // RGBA 缓冲恒为 4 字节对齐；余数恒空（长度必整除 4），用 `_` 显式忽略
    let (chunks, _) = rgba.as_raw().as_chunks::<4>();
    for pixel in chunks {
        let [red, green, blue, alpha] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        let opacity = u32::from(alpha);
        // `u32` 域：分子恒 ≤65152，分母 255，结果恒 ≤255，`try_from` 永不失败
        let blend = |value: u8| {
            u8::try_from((u32::from(value) * opacity + 255 * (255 - opacity) + 127) / 255)
                .unwrap_or(u8::MAX)
        };
        flat.extend_from_slice(&[blend(red), blend(green), blend(blue)]);
    }
    DynamicImage::ImageRgb8(
        image::RgbImage::from_raw(width, height, flat).unwrap_or_else(|| {
            image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]))
        }),
    )
}

/// 输出路径预留：目录不存在即创建；`filename_suffix` 开启追加 `_{宽}x{高}`。
/// 重名按策略处理：`Increment` 追加 `-2`/`-3`；`Overwrite` 直接覆盖（返回首候选）；
/// `Skip` 且已存在返回 `None`（调用方记 `Skipped`，不写盘）
fn reserve_output_path(
    input: &str,
    options: &ResizeOptions,
    source: ImageFormat,
    width: u32,
    height: u32,
) -> anyhow::Result<Option<String>> {
    use anyhow::Context;

    let out_dir = std::path::Path::new(&options.output_dir);
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("cannot create output dir: {}", options.output_dir))?;
    let stem = std::path::Path::new(input)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .with_context(|| format!("invalid file name: {input}"))?;
    let ext = output_extension(options.format, source);
    let base = if options.filename_suffix {
        format!("{stem}_{width}x{height}")
    } else {
        stem.to_string()
    };
    let first = out_dir.join(format!("{base}.{ext}"));
    if options.overwrite == OverwritePolicy::Overwrite {
        return first
            .to_str()
            .map(str::to_string)
            .with_context(|| "output path is not valid UTF-8".to_string())
            .map(Some);
    }
    if options.overwrite == OverwritePolicy::Skip && first.exists() {
        return Ok(None);
    }
    let mut candidate = first;
    let mut index = 1;
    while candidate.exists() {
        index += 1;
        candidate = out_dir.join(format!("{base}-{index}.{ext}"));
    }
    candidate
        .to_str()
        .map(str::to_string)
        .with_context(|| "output path is not valid UTF-8".to_string())
        .map(Some)
}

/// 跳过诊断用的首候选路径：与 `reserve_output_path` 同规则组装（仅展示，不预留）
fn output_name(
    input: &str,
    options: &ResizeOptions,
    source: ImageFormat,
    width: u32,
    height: u32,
) -> String {
    let stem = std::path::Path::new(input)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output");
    let ext = output_extension(options.format, source);
    let base = if options.filename_suffix {
        format!("{stem}_{width}x{height}")
    } else {
        stem.to_string()
    };
    std::path::Path::new(&options.output_dir)
        .join(format!("{base}.{ext}"))
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    fn box_mode(width: Option<u32>, height: Option<u32>, lock: bool) -> ResizeMode {
        ResizeMode::WidthHeight {
            width,
            height,
            lock_ratio: lock,
        }
    }

    #[test]
    fn plan_single_side_scales_proportionally() {
        // 200x100 按宽 100 缩，高按比例得 50
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(Some(100), None, true), false),
            Ok((100, 50))
        );
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(None, Some(50), true), false),
            Ok((100, 50))
        );
    }

    #[test]
    fn plan_locked_box_fits_inside() {
        // 200x100 进 100x100 框：最小比 0.5，输出 100x50（不拉伸）
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(Some(100), Some(100), true), false),
            Ok((100, 50))
        );
        // 不锁定即精确拉伸
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(Some(100), Some(100), false), false),
            Ok((100, 100))
        );
    }

    #[test]
    fn plan_rejects_empty_and_zero() {
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(None, None, true), false),
            Err(ResizeErrorCode::InvalidSize)
        );
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(Some(0), Some(10), false), false),
            Err(ResizeErrorCode::InvalidSize)
        );
        assert_eq!(
            plan_dimensions(0, 100, &box_mode(Some(10), None, true), false),
            Err(ResizeErrorCode::InvalidSize)
        );
    }

    #[test]
    fn plan_percent_halves_and_rejects_bounds() {
        assert_eq!(
            plan_dimensions(200, 100, &ResizeMode::Percent { percent: 50 }, false),
            Ok((100, 50))
        );
        for bad in [0, MAX_PERCENT + 1] {
            assert_eq!(
                plan_dimensions(200, 100, &ResizeMode::Percent { percent: bad }, false),
                Err(ResizeErrorCode::InvalidSize)
            );
        }
    }

    #[test]
    fn plan_exact_modes_resolve_canvas() {
        // Contain 输出 fitted 尺寸，其余输出精确画布
        let contain = ResizeMode::Exact {
            width: 100,
            height: 100,
            fit: FitMode::Contain,
        };
        assert_eq!(plan_dimensions(200, 100, &contain, false), Ok((100, 50)));
        for fit in [FitMode::Stretch, FitMode::Cover, FitMode::Pad] {
            let exact = ResizeMode::Exact {
                width: 100,
                height: 100,
                fit,
            };
            assert_eq!(plan_dimensions(200, 100, &exact, false), Ok((100, 100)));
        }
    }

    #[test]
    fn plan_no_upscale_falls_back_to_source() {
        // 目标任一边超出即整体回落原尺寸（不逐边钳制，保纵横比）
        assert_eq!(
            plan_dimensions(200, 100, &box_mode(Some(400), None, true), true),
            Ok((200, 100))
        );
        assert_eq!(
            plan_dimensions(200, 100, &ResizeMode::Percent { percent: 200 }, true),
            Ok((200, 100))
        );
        // 未勾选时允许放大
        assert_eq!(
            plan_dimensions(200, 100, &ResizeMode::Percent { percent: 200 }, false),
            Ok((400, 200))
        );
    }

    #[test]
    fn plan_guards_absurd_sizes() {
        let huge = ResizeMode::Exact {
            width: MAX_SIDE + 1,
            height: 10,
            fit: FitMode::Stretch,
        };
        assert_eq!(
            plan_dimensions(10, 10, &huge, false),
            Err(ResizeErrorCode::InvalidSize)
        );
        // 单边合法但总像素超限（20000 边长被 MAX_SIDE 先拦，改用像素积用例）
        assert_eq!(
            plan_dimensions(10, 10, &box_mode(Some(16384), Some(16384), false), false),
            Err(ResizeErrorCode::InvalidSize)
        );
    }

    #[test]
    fn format_detection_is_case_insensitive() {
        assert_eq!(
            supported_format_from_path("/tmp/PHOTO.JPG"),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            supported_format_from_path("a/b/c.webp"),
            Some(ImageFormat::Webp)
        );
        assert_eq!(supported_format_from_path("noext"), None);
        assert_eq!(supported_format_from_path("doc.txt"), None);
    }

    #[test]
    fn output_extension_maps_gif_to_png() {
        assert_eq!(
            output_extension(OutputFormat::Original, ImageFormat::Gif),
            "png"
        );
        assert_eq!(
            output_extension(OutputFormat::Original, ImageFormat::Jpeg),
            "jpg"
        );
        assert_eq!(
            output_extension(OutputFormat::Webp, ImageFormat::Jpeg),
            "webp"
        );
    }

    /// 测试沙盒：进程级唯一子目录，用完尽力清理（断言失败也尝试删，不抛错）
    fn sandbox(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tool-dock-image-resize-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("test sandbox");
        dir
    }

    fn write_sample_png(path: &std::path::Path, width: u32, height: u32) {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            width,
            height,
            image::Rgb([200, 30, 30]),
        ));
        img.save(path).expect("write sample");
    }

    fn default_options(out_dir: &str) -> ResizeOptions {
        ResizeOptions {
            mode: ResizeMode::Percent { percent: 50 },
            format: OutputFormat::Original,
            quality: 85,
            output_dir: out_dir.to_string(),
            no_upscale: false,
            filename_suffix: true,
            rotation: ImageRotation::None,
            filter: ResizeFilter::Lanczos3,
            overwrite: OverwritePolicy::Increment,
            target_size_kb: None,
        }
    }

    #[test]
    fn roundtrip_info_and_resize_png() {
        let dir = sandbox("roundtrip");
        let input = dir.join("sample.png");
        write_sample_png(&input, 64, 48);
        let input_str = input.to_str().unwrap();

        let info = read_image_info(input_str).expect("info");
        assert_eq!(
            info,
            ImageInfo {
                width: 64,
                height: 48,
                format: ImageFormat::Png,
                file_size: u32::try_from(std::fs::metadata(&input).unwrap().len())
                    .unwrap_or(u32::MAX),
            }
        );

        let out_dir = dir.join("out");
        let outcome = resize_image(input_str, &default_options(out_dir.to_str().unwrap()));
        assert!(outcome.ok, "unexpected error: {:?}", outcome.error);
        assert_eq!((outcome.width, outcome.height), (Some(32), Some(24)));
        assert_eq!(outcome.target_met, None);
        let output = outcome.output.as_deref().unwrap();
        assert!(output.ends_with("sample_32x24.png"));
        let back = read_image_info(output).expect("output info");
        assert_eq!((back.width, back.height), (32, 24));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resize_reports_missing_and_unsupported() {
        let dir = sandbox("failures");
        let missing = dir.join("gone.png");
        let outcome = resize_image(
            missing.to_str().unwrap(),
            &default_options(dir.to_str().unwrap()),
        );
        assert!(!outcome.ok);
        assert_eq!(outcome.error.unwrap().code, ResizeErrorCode::DecodeFailed);

        let txt = dir.join("note.txt");
        std::fs::write(&txt, "hello").unwrap();
        let outcome = resize_image(
            txt.to_str().unwrap(),
            &default_options(dir.to_str().unwrap()),
        );
        assert_eq!(
            outcome.error.unwrap().code,
            ResizeErrorCode::UnsupportedFormat
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reserve_path_never_overwrites() {
        let dir = sandbox("reserve");
        let input = dir.join("pic.png");
        write_sample_png(&input, 8, 8);
        let options = default_options(dir.join("out").to_str().unwrap().to_string().as_str());
        let first = reserve_output_path(input.to_str().unwrap(), &options, ImageFormat::Png, 4, 4)
            .unwrap()
            .unwrap();
        assert!(first.ends_with("pic_4x4.png"));
        std::fs::write(&first, [0u8]).unwrap();
        let second = reserve_output_path(input.to_str().unwrap(), &options, ImageFormat::Png, 4, 4)
            .unwrap()
            .unwrap();
        assert!(second.ends_with("pic_4x4-2.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reserve_path_honors_overwrite_policy() {
        let dir = sandbox("reserve-policy");
        let input = dir.join("pic.png");
        write_sample_png(&input, 8, 8);
        let out_str = dir.join("out").to_str().unwrap().to_string();

        // Overwrite：已存在也返回首候选（调用方直接覆盖）
        let mut options = default_options(&out_str);
        options.overwrite = OverwritePolicy::Overwrite;
        let first = reserve_output_path(input.to_str().unwrap(), &options, ImageFormat::Png, 4, 4)
            .unwrap()
            .unwrap();
        assert!(first.ends_with("pic_4x4.png"));
        std::fs::write(&first, [0u8]).unwrap();
        let again = reserve_output_path(input.to_str().unwrap(), &options, ImageFormat::Png, 4, 4)
            .unwrap()
            .unwrap();
        assert_eq!(again, first);

        // Skip：已存在返回 None，不存在照常预留
        options.overwrite = OverwritePolicy::Skip;
        assert!(
            reserve_output_path(input.to_str().unwrap(), &options, ImageFormat::Png, 4, 4)
                .unwrap()
                .is_none()
        );
        std::fs::remove_file(&first).unwrap();
        assert!(
            reserve_output_path(input.to_str().unwrap(), &options, ImageFormat::Png, 4, 4)
                .unwrap()
                .is_some()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotation_swaps_axes_only_for_quarter_turns() {
        let img =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(20, 10, image::Rgb([1, 2, 3])));
        assert_eq!(
            apply_rotation(img.clone(), ImageRotation::None).dimensions(),
            (20, 10)
        );
        assert_eq!(
            apply_rotation(img.clone(), ImageRotation::Cw90).dimensions(),
            (10, 20)
        );
        assert_eq!(
            apply_rotation(img.clone(), ImageRotation::Ccw90).dimensions(),
            (10, 20)
        );
        assert_eq!(
            apply_rotation(img.clone(), ImageRotation::Cw180).dimensions(),
            (20, 10)
        );
        assert_eq!(
            apply_rotation(img.clone(), ImageRotation::FlipHorizontal).dimensions(),
            (20, 10)
        );
        // 色深保持：RGB 输入旋转后仍为 RGB（不经过 RGBA 中转）
        assert!(matches!(
            apply_rotation(img, ImageRotation::Cw90),
            DynamicImage::ImageRgb8(_)
        ));
    }

    #[test]
    fn rotation_applies_before_planning() {
        // 64x48 顺转 90° 后按 48x64 进规划：端到端验证管线顺序（解码→旋转→规划）
        let dir = sandbox("rotation-pipe");
        let input = dir.join("sample.png");
        write_sample_png(&input, 64, 48);
        let mut options = default_options(dir.join("out").to_str().unwrap());
        options.mode = ResizeMode::Percent { percent: 100 };
        options.rotation = ImageRotation::Cw90;
        let outcome = resize_image(input.to_str().unwrap(), &options);
        assert!(outcome.ok, "unexpected error: {:?}", outcome.error);
        assert_eq!((outcome.width, outcome.height), (Some(48), Some(64)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn filter_maps_to_native_variants() {
        assert_eq!(filter_type(ResizeFilter::Nearest), FilterType::Nearest);
        assert_eq!(filter_type(ResizeFilter::Triangle), FilterType::Triangle);
        assert_eq!(
            filter_type(ResizeFilter::CatmullRom),
            FilterType::CatmullRom
        );
        assert_eq!(filter_type(ResizeFilter::Gaussian), FilterType::Gaussian);
        assert_eq!(filter_type(ResizeFilter::Lanczos3), FilterType::Lanczos3);
    }

    /// 噪声图：纯色压得太小，二分行为看不出来；梯度噪声保证各质量档体积单调
    fn write_noisy_png(path: &std::path::Path, width: u32, height: u32) {
        let mut img = image::RgbImage::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let value = ((x * 37 + y * 91) % 251) as u8;
            *pixel = image::Rgb([value, 255 - value, (value / 2) + 64]);
        }
        DynamicImage::ImageRgb8(img)
            .save(path)
            .expect("write noisy");
    }

    #[test]
    fn jpeg_target_meets_loose_goal_and_reports_miss() {
        let dir = sandbox("target");
        let input = dir.join("noisy.png");
        write_noisy_png(&input, 320, 240);
        let input_str = input.to_str().unwrap();
        let (decoded, _) = decode_oriented(input_str).expect("decode");

        // 宽松目标：高达成（质量应接近上限区间）
        let (bytes, met) = encode_jpeg_target(&decoded, 500 * 1024).expect("encode");
        assert!(met);
        assert!((bytes.len() as u64) <= 500 * 1024);

        // 苛刻目标：输出最小文件 + 未达成（不抛错，调用方照常落盘；编码成功即非空）
        let (_, met) = encode_jpeg_target(&decoded, 100).expect("encode");
        assert!(!met);

        // 端到端：设目标走 JPEG 输出，`target_met` 有值
        let mut options = default_options(dir.join("out").to_str().unwrap());
        options.format = OutputFormat::Jpeg;
        options.target_size_kb = Some(500);
        let outcome = resize_image(input_str, &options);
        assert!(outcome.ok, "unexpected error: {:?}", outcome.error);
        assert_eq!(outcome.target_met, Some(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn target_ignored_for_non_jpeg_reports_unmet() {
        // 非 JPEG 设目标：常规编码 + `Some(false)`（前端再分“未达成/不适用”，不静默）
        let dir = sandbox("target-png");
        let input = dir.join("sample.png");
        write_sample_png(&input, 64, 48);
        let mut options = default_options(dir.join("out").to_str().unwrap());
        options.format = OutputFormat::Png;
        options.target_size_kb = Some(500);
        let outcome = resize_image(input.to_str().unwrap(), &options);
        assert!(outcome.ok, "unexpected error: {:?}", outcome.error);
        assert_eq!(outcome.target_met, Some(false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_policy_reports_skipped_not_failed() {
        let dir = sandbox("skip");
        let input = dir.join("sample.png");
        write_sample_png(&input, 32, 24);
        let input_str = input.to_str().unwrap();
        let out_str = dir.join("out").to_str().unwrap().to_string();

        // 先正常出一张，再同参数 Skip 重跑 → 跳过（非失败）
        let mut options = default_options(&out_str);
        let first = resize_image(input_str, &options);
        assert!(first.ok);
        options.overwrite = OverwritePolicy::Skip;
        let second = resize_image(input_str, &options);
        assert!(!second.ok);
        assert_eq!(second.error.clone().unwrap().code, ResizeErrorCode::Skipped);
        assert_eq!(second.target_met, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cover_and_pad_render_exact_canvas() {
        // Cover/Pad 输出精确画布（与 plan 一致），不抛错即过
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            20,
            10,
            image::Rgba([10, 20, 30, 255]),
        ));
        assert_eq!(
            render(&img, 10, 10, Some(FitMode::Cover), FilterType::Lanczos3).dimensions(),
            (10, 10)
        );
        assert_eq!(
            render(&img, 10, 10, Some(FitMode::Pad), FilterType::Lanczos3).dimensions(),
            (10, 10)
        );
        assert_eq!(
            render(&img, 5, 5, None, FilterType::Nearest).dimensions(),
            (5, 5)
        );
    }

    #[test]
    fn thumbnail_returns_data_url_within_bounds() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let dir = sandbox("thumbnail");
        let input = dir.join("big.png");
        write_sample_png(&input, 64, 48);
        let url = read_image_thumbnail(input.to_str().unwrap(), 32).expect("thumbnail");
        let payload = url
            .strip_prefix("data:image/jpeg;base64,")
            .expect("data URL prefix");
        let bytes = STANDARD.decode(payload).expect("base64");
        let decoded = image::load_from_memory(&bytes).expect("jpeg decode");
        assert_eq!((decoded.width(), decoded.height()), (32, 24));

        // 小图不放大：原样尺寸回包
        let url = read_image_thumbnail(input.to_str().unwrap(), 1024).expect("thumbnail");
        let payload = url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let bytes = STANDARD.decode(payload).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 48));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn thumbnail_rejects_bad_input() {
        let dir = sandbox("thumbnail-bad");
        assert!(read_image_thumbnail(dir.join("gone.png").to_str().unwrap(), 256).is_err());
        let input = dir.join("sample.png");
        write_sample_png(&input, 8, 8);
        assert!(read_image_thumbnail(input.to_str().unwrap(), 0).is_err());
        assert!(read_image_thumbnail(input.to_str().unwrap(), MAX_THUMB_SIDE + 1).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expand_flattens_dirs_and_suggests_resized() {
        let dir = sandbox("expand");
        write_sample_png(&dir.join("b.png"), 8, 8);
        // 故意扩展名与内容不符（PNG 内容配 `.jpg` 名）：只测扩展名过滤，不解码
        write_sample_png(&dir.join("a.jpg"), 8, 8);
        std::fs::write(dir.join("note.txt"), "hello").unwrap();
        let nested = dir.join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        write_sample_png(&nested.join("c.png"), 8, 8);

        // 文件夹：只收顶层图片（排序），非图片与子目录忽略，并给出 resized/ 建议
        let outcome = expand_dropped_paths(&[dir.to_str().unwrap().to_string()]).unwrap();
        let names: Vec<String> = outcome
            .files
            .iter()
            .map(|path| basename_of(path).to_string())
            .collect();
        assert_eq!(names, vec!["a.jpg".to_string(), "b.png".to_string()]);
        assert_eq!(
            outcome.output_dir,
            Some(dir.join("resized").to_str().unwrap().to_string())
        );

        // 文件与不存在路径原样透传，不给输出建议
        let missing = dir.join("gone.png");
        let outcome = expand_dropped_paths(&[
            dir.join("note.txt").to_str().unwrap().to_string(),
            missing.to_str().unwrap().to_string(),
        ])
        .unwrap();
        assert_eq!(outcome.files.len(), 2);
        assert_eq!(outcome.output_dir, None);

        // 空输入即空输出
        let outcome = expand_dropped_paths(&[]).unwrap();
        assert_eq!(outcome.files, Vec::<String>::new());
        assert_eq!(outcome.output_dir, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 测试用 basename：取文件名部分（与前端 `basenameOf` 同语义，此处仅断言用）
    fn basename_of(path: &str) -> &str {
        std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
    }

    #[test]
    fn batch_returns_ordered_items_and_skips_failures() {
        let dir = sandbox("batch");
        let first = dir.join("a.png");
        let second = dir.join("b.png");
        write_sample_png(&first, 64, 48);
        write_sample_png(&second, 32, 32);
        let missing = dir.join("gone.png");
        let txt = dir.join("note.txt");
        std::fs::write(&txt, "hello").unwrap();
        let paths = [
            first.to_str().unwrap().to_string(),
            missing.to_str().unwrap().to_string(),
            txt.to_str().unwrap().to_string(),
            second.to_str().unwrap().to_string(),
        ];
        let items = read_image_batch(&paths, 96).expect("batch");
        // 顺序与入参一致，失败项不中断其余
        assert_eq!(items.len(), 4);
        assert!(items[0].ok);
        assert_eq!(
            items[0].info.map(|info| (info.width, info.height)),
            Some((64, 48))
        );
        assert!(
            items[0]
                .thumb
                .as_deref()
                .is_some_and(|thumb| { thumb.starts_with("data:image/jpeg;base64,") })
        );
        assert!(!items[1].ok);
        assert!(items[1].error.is_some());
        assert!(!items[2].ok);
        assert!(items[3].ok);
        assert_eq!(
            items[3].info.map(|info| (info.width, info.height)),
            Some((32, 32))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batch_validates_params() {
        let dir = sandbox("batch-params");
        write_sample_png(&dir.join("a.png"), 8, 8);
        let paths = vec![dir.join("a.png").to_str().unwrap().to_string()];
        assert!(read_image_batch(&paths, 0).is_err());
        assert!(read_image_batch(&paths, MAX_LIST_SIDE + 1).is_err());
        assert_eq!(read_image_batch(&[], 96).unwrap(), Vec::new());
        let oversized = vec!["x".to_string(); MAX_IMAGE_BATCH + 1];
        assert!(read_image_batch(&oversized, 96).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
