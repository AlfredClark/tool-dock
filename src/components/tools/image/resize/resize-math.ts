import type { ImageFormat, ImageRotation, OutputFormat, ResizeMode } from "$libs/commands/bindings";
import type { ResizeParamsState } from "./resize-types";

// 尺寸规划的纯前端镜像：与后端 `plan_dimensions` 同语义（四舍五入/钳制/回落规则一致），
// 供预览“预计输出”即时计算；两侧单测用同一向量，改一处必须同步另一处。

/** 单边上限：与后端 `MAX_SIDE` 同值 */
export const RESIZE_MAX_SIDE = 16384;

/** 总像素上限：与后端 `MAX_PIXELS` 同值 */
export const RESIZE_MAX_PIXELS = 100_000_000;

/** 百分比上下限：与后端 `1..=MAX_PERCENT` 同值 */
export const RESIZE_MIN_PERCENT = 1;
export const RESIZE_MAX_PERCENT = 1000;

/** 图片扩展名白名单：与后端 `SUPPORTED_EXTENSIONS` 同源（对话框过滤器共用） */
export const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff", "gif"];

/** 目标尺寸规划：非法返回 `null`（开始按钮禁用 + 提示），语义对齐后端 */
export function planDimensions(
  srcWidth: number,
  srcHeight: number,
  mode: ResizeMode,
  noUpscale: boolean,
): { width: number; height: number } | null {
  if (
    !Number.isInteger(srcWidth) ||
    !Number.isInteger(srcHeight) ||
    srcWidth <= 0 ||
    srcHeight <= 0
  ) {
    return null;
  }
  const planned = matchMode(srcWidth, srcHeight, mode);
  if (!planned) return null;
  let { width, height } = planned;
  if (noUpscale && (width > srcWidth || height > srcHeight)) {
    width = srcWidth;
    height = srcHeight;
  }
  if (!guardSides(width, height)) return null;
  return { width, height };
}

/** 按模式算画布：`contain` 输出 fitted，其余精确画布；非法输入返回 `null` */
function matchMode(
  srcWidth: number,
  srcHeight: number,
  mode: ResizeMode,
): { width: number; height: number } | null {
  switch (mode.kind) {
    case "widthheight":
      return planBox(srcWidth, srcHeight, mode.width, mode.height, mode.lock_ratio);
    case "percent":
      return planPercent(srcWidth, srcHeight, mode.percent);
    case "exact": {
      if (!guardSides(mode.width, mode.height)) return null;
      if (mode.fit === "contain") return fitInside(srcWidth, srcHeight, mode.width, mode.height);
      return { width: mode.width, height: mode.height };
    }
  }
}

/** 宽高模式：双空/零拒绝；双填不锁定即拉伸；单填按比例 */
function planBox(
  srcWidth: number,
  srcHeight: number,
  width: number | null,
  height: number | null,
  lockRatio: boolean,
): { width: number; height: number } | null {
  if (width == null && height == null) return null;
  if ((width != null && width <= 0) || (height != null && height <= 0)) return null;
  if (width != null && height != null) {
    if (!lockRatio) return guardSides(width, height) ? { width, height } : null;
    return fitInside(srcWidth, srcHeight, width, height);
  }
  if (width != null) {
    const scaled = Math.max(1, Math.floor((srcHeight * width) / srcWidth));
    return guardSides(width, scaled) ? { width, height: scaled } : null;
  }
  // `height != null`（上已排除双空）
  const target = height ?? 0;
  const scaled = Math.max(1, Math.floor((srcWidth * target) / srcHeight));
  return guardSides(scaled, target) ? { width: scaled, height: target } : null;
}

/** 百分比模式：1%-1000%，换算后至少 1px */
function planPercent(
  srcWidth: number,
  srcHeight: number,
  percent: number,
): { width: number; height: number } | null {
  if (!Number.isInteger(percent) || percent < RESIZE_MIN_PERCENT || percent > RESIZE_MAX_PERCENT) {
    return null;
  }
  const width = Math.max(1, Math.floor((srcWidth * percent) / 100));
  const height = Math.max(1, Math.floor((srcHeight * percent) / 100));
  return guardSides(width, height) ? { width, height } : null;
}

/** 内适应：取最小缩放比四舍五入（与后端 `round` 对齐），至少 1px */
function fitInside(
  srcWidth: number,
  srcHeight: number,
  boxWidth: number,
  boxHeight: number,
): { width: number; height: number } | null {
  const scale = Math.min(boxWidth / srcWidth, boxHeight / srcHeight);
  const width = Math.max(1, Math.round(srcWidth * scale));
  const height = Math.max(1, Math.round(srcHeight * scale));
  return guardSides(width, height) ? { width, height } : null;
}

/** 尺寸守卫：与后端 `guard_sides` 同规则（零/超边/超像素拒绝） */
function guardSides(width: number, height: number): boolean {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) {
    return false;
  }
  if (width > RESIZE_MAX_SIDE || height > RESIZE_MAX_SIDE) return false;
  return width * height <= RESIZE_MAX_PIXELS;
}

/** 文件大小格式化：B/KB/MB 两位小数（列表与预览共用） */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/** 取文件扩展名（小写，无扩展名返回空串） */
export function extensionOf(filename: string): string {
  const dot = filename.lastIndexOf(".");
  if (dot < 0) return "";
  return filename.slice(dot + 1).toLowerCase();
}

/** 扩展名是否在图片白名单内 */
export function isImageExtension(filename: string): boolean {
  return IMAGE_EXTENSIONS.includes(extensionOf(filename));
}

/** 取文件名（去目录部分，兼容 `/` 与 `\` 分隔符） */
export function basenameOf(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

/** 长宽比取值：`free` 为自由比例，其余为 `宽:高`（如下拉值 `16:9`） */
export const RATIO_FREE = "free";

/** 常见长宽比候选：顺序即下拉渲染顺序 */
export const RATIO_ITEMS = ["1:1", "4:3", "3:4", "3:2", "2:3", "16:9", "9:16", "16:10"] as const;

/** 长宽比示例尺寸：切换比例时输入框 placeholder 同步为符合比例的值（仅提示，不写回） */
const RATIO_PLACEHOLDERS: Record<string, { width: string; height: string }> = {
  "1:1": { width: "512", height: "512" },
  "4:3": { width: "800", height: "600" },
  "3:4": { width: "600", height: "800" },
  "3:2": { width: "900", height: "600" },
  "2:3": { width: "600", height: "900" },
  "16:9": { width: "1280", height: "720" },
  "9:16": { width: "720", height: "1280" },
  "16:10": { width: "1280", height: "800" },
};

/** 取长宽比示例：自由/未知返回 `null`（调用方回落默认提示） */
export function ratioPlaceholder(value: string): { width: string; height: string } | null {
  return RATIO_PLACEHOLDERS[value] ?? null;
}

/** 旋转后尺寸：90/270 系交换宽高（与后端 `apply_rotation` 同顺序，预览预计值用） */
export function rotateSize(
  size: { width: number; height: number },
  rotation: ImageRotation,
): { width: number; height: number } {
  switch (rotation) {
    case "cw90":
    case "ccw90":
      return { width: size.height, height: size.width };
    default:
      return { width: size.width, height: size.height };
  }
}

/**
 * 是否 JPEG 输出（含 `original` 跟随 JPEG 输入）：与后端 `is_jpeg_output` 同语义， 目标大小 UI
 * 与“不适用”提示用，两侧改动必须同步
 */
export function isJpegOutput(source: ImageFormat, requested: OutputFormat): boolean {
  return requested === "jpeg" || (requested === "original" && source === "jpeg");
}

/** 解析长宽比：`free`/非法返回 `null`（调用方即自由模式） */
export function parseRatio(value: string): { width: number; height: number } | null {
  const parts = value.split(":");
  if (parts.length !== 2) return null;
  const [widthText, heightText] = parts;
  if (!/^\d+$/.test(widthText.trim()) || !/^\d+$/.test(heightText.trim())) return null;
  const width = Number.parseInt(widthText.trim(), 10);
  const height = Number.parseInt(heightText.trim(), 10);
  if (width <= 0 || height <= 0) return null;
  return { width, height };
}

/** 解析正整数文本：空/非法返回 `null`（与 workspace 的 `parsePositiveInt` 同语义，此处仅内部用） */
function parseDimensionText(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed === "" || !/^\d+$/.test(trimmed)) return null;
  const value = Number.parseInt(trimmed, 10);
  return value > 0 ? value : null;
}

/**
 * 精确尺寸联动：按 `editedKey` 边换算另一边（四舍五入，至少 1px）；
 * 自由比例/被编辑边非法时原样返回；两边以最后编辑边为准（选择比例时调用方按宽优先传入）
 */
export function applyExactRatio(
  state: ResizeParamsState,
  editedKey: "exactWidth" | "exactHeight",
): ResizeParamsState {
  if (state.modeKind !== "exact") return state;
  const ratio = parseRatio(state.exactRatio);
  if (!ratio) return state;
  if (editedKey === "exactWidth") {
    const width = parseDimensionText(state.exactWidth);
    if (width == null) return state;
    const height = Math.max(1, Math.round((width * ratio.height) / ratio.width));
    if (String(height) === state.exactHeight) return state;
    return { ...state, exactHeight: String(height) };
  }
  const height = parseDimensionText(state.exactHeight);
  if (height == null) return state;
  const width = Math.max(1, Math.round((height * ratio.width) / ratio.height));
  if (String(width) === state.exactWidth) return state;
  return { ...state, exactWidth: String(width) };
}
