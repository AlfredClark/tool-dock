// 占位符模板引擎：`%name%` 展开 + `%%` 转义的纯逻辑。
// 展开只在前端做（逐项用该项自己的上下文），后端收到的已是最终值，
// 因此前后端无需同步此处的注册表；加占位符时同步补文案提示与单测即可。
import type { VideoFileMetadata } from "$libs/commands/bindings";

/** 已登记的占位符：键名即 `%键名%` 写法 */
export const PLACEHOLDER_NAMES = [
  "filename",
  "ext",
  "width",
  "height",
  "duration",
  "date",
] as const;

/** 占位符取值 */
export type PlaceholderName = (typeof PLACEHOLDER_NAMES)[number];

/** 展开上下文：调用方按单项元信息组装（含空值回落语义，见 `expandTemplate`） */
export interface PlaceholderContext {
  /** 无后缀文件名 */
  filename: string;
  /** 无点后缀名 */
  ext: string;
  /** 视频宽度（无视频流即 `null`） */
  width: number | null;
  /** 视频高度（无视频流即 `null`） */
  height: number | null;
  /** 时长秒数（缺失即 `null`） */
  durationSeconds: number | null;
}

/** 展开结果：未知占位符原样保留并记入 `unknown`（调用方拦截开始） */
export interface ExpandResult {
  text: string;
  unknown: string[];
}

/** 是否为已登记的占位符名 */
export function isPlaceholderName(value: string): value is PlaceholderName {
  return (PLACEHOLDER_NAMES as readonly string[]).includes(value);
}

/** 模板中的占位名是否全部已登记（与具体文件无关，开始按钮门禁只看此项） */
export function findUnknownNames(template: string): string[] {
  const unknown: string[] = [];
  for (const name of scanPlaceholderNames(template)) {
    if (!isPlaceholderName(name) && !unknown.includes(name)) {
      unknown.push(name);
    }
  }
  return unknown;
}

/**
 * 按上下文展开：`%%` 即字面 `%`；未登记名原样保留并记入 `unknown`； 已登记但上下文无值（无视频流/无时长）同样原样保留并记入
 * `unknown` （调用方按单项失败处理，不静默写错值）
 */
export function expandTemplate(
  template: string,
  ctx: PlaceholderContext,
  today: Date = new Date(),
): ExpandResult {
  const unknown: string[] = [];
  let text = "";
  let index = 0;
  while (index < template.length) {
    const char = template[index];
    if (char !== "%") {
      text += char;
      index += 1;
      continue;
    }
    // `%%` 即字面 `%`
    if (template[index + 1] === "%") {
      text += "%";
      index += 2;
      continue;
    }
    const close = template.indexOf("%", index + 1);
    // 无闭合即字面 `%` 起的原文（孤 `%` 不算占位符，不拦截开始）
    if (close < 0) {
      text += template.slice(index);
      break;
    }
    const name = template.slice(index + 1, close);
    const value = resolvePlaceholder(name, ctx, today);
    if (value == null) {
      text += template.slice(index, close + 1);
      if (!unknown.includes(name)) {
        unknown.push(name);
      }
    } else {
      text += value;
    }
    index = close + 1;
  }
  return { text, unknown };
}

/** 取值：未登记返回 `null`（由调用方记入 `unknown`） */
function resolvePlaceholder(name: string, ctx: PlaceholderContext, today: Date): string | null {
  switch (name) {
    case "filename":
      return ctx.filename;
    case "ext":
      return ctx.ext;
    case "width":
      return ctx.width == null ? null : String(ctx.width);
    case "height":
      return ctx.height == null ? null : String(ctx.height);
    case "duration":
      return ctx.durationSeconds == null ? null : String(Math.floor(ctx.durationSeconds));
    case "date":
      return formatDate(today);
    default:
      return null;
  }
}

/** 扫描模板中的全部占位名（含重复与未登记；`%%` 与孤 `%` 跳过） */
function scanPlaceholderNames(template: string): string[] {
  const names: string[] = [];
  let index = 0;
  while (index < template.length) {
    if (template[index] !== "%") {
      index += 1;
      continue;
    }
    if (template[index + 1] === "%") {
      index += 2;
      continue;
    }
    const close = template.indexOf("%", index + 1);
    if (close < 0) break;
    names.push(template.slice(index + 1, close));
    index = close + 1;
  }
  return names;
}

/** 日期格式化：本地 `YYYY-MM-DD`（`%date%` 用） */
export function formatDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

/** 时长格式化：`mm:ss`（中栏技术行用；缺失回落 `—` 由调用方处理） */
export function formatClock(totalSeconds: number): string {
  const total = Math.max(0, Math.floor(totalSeconds));
  const minutes = String(Math.floor(total / 60)).padStart(2, "0");
  const seconds = String(total % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
}

/** 参数模板中的未知占位名（静态检查，与具体文件无关，开始按钮门禁） */
export function unknownNamesInTemplates(templates: Record<string, string>): string[] {
  const unknown: string[] = [];
  for (const template of Object.values(templates)) {
    for (const name of findUnknownNames(template)) {
      if (!unknown.includes(name)) {
        unknown.push(name);
      }
    }
  }
  return unknown;
}

/** 按单项组装展开上下文：文件名/后缀取路径，宽高/时长取元信息（缺失即 `null`） */
export function contextForItem(
  displayName: string,
  path: string,
  info: VideoFileMetadata | null,
): PlaceholderContext {
  const source = path !== "" ? path : displayName;
  return {
    filename: videoFileStem(source),
    ext: videoFileExt(source),
    width: info?.stream?.width ?? null,
    height: info?.stream?.height ?? null,
    durationSeconds: info?.duration_seconds ?? null,
  };
}

/** 取无后缀文件名：兼容 `/` 与 `\` 分隔符 */
export function videoFileStem(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot > 0 ? base.slice(0, dot) : base;
}

/** 取无点后缀名（小写）：无后缀返回空串 */
export function videoFileExt(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot > 0 ? base.slice(dot + 1).toLowerCase() : "";
}
