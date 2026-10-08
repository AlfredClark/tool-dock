import type {
  FitMode,
  ImageInfo,
  ImageRotation,
  OutputFormat,
  OverwritePolicy,
  ResizeFilter,
} from "$libs/commands/bindings";

// 图片尺寸工具的界面态类型：后端契约走 `bindings`，此处只放前端自有状态。

/** 参数面板的模式页签：与后端 `ResizeMode` 三变体一一对应 */
export type ResizeModeKind = "box" | "percent" | "exact";

/** 列表项状态：读取中/就绪/不可读/已完成/失败/已跳过（批量处理进度由 workspace 进度态表达） */
export type ResizeItemStatus = "loading" | "ready" | "invalid" | "done" | "failed" | "skipped";

/** 列表项：`previewUrl` 桌面端为后端缩略图 `data:` URL，浏览器为 `objectURL`（卸载时释放） */
export interface ResizeImageItem {
  /** 稳定键：桌面端用路径，浏览器用 `blob:<name>:<size>` 合成 */
  id: string;
  /** 本地路径（浏览器降级为空串，预览走 `previewUrl`） */
  path: string;
  /** 显示名：路径取 basename，浏览器取 File 名 */
  name: string;
  /** 预览地址：桌面端缩略图 `data:` URL，浏览器 `objectURL`；缩略图失败时为空串 */
  previewUrl: string;
  /** 是否需要释放 `previewUrl`（仅 `objectURL` 为真） */
  revokePreview: boolean;
  /** 元信息：`invalid` 时为 `null` */
  info: ImageInfo | null;
  /** 不可读/失败原因（英文诊断原文，前端展示通用文案 + 原文补充） */
  errorDetail: string | null;
  status: ResizeItemStatus;
  /** 输出路径：`done` 后展示 */
  output: string | null;
  /** 输出尺寸：`done` 后展示 */
  outputSize: { width: number; height: number } | null;
  /** 目标达成：仅设目标大小且成功时有值，`false` 即未达成（已输出最小文件） */
  targetMet: boolean | null;
}

/** 参数面板状态：数字输入保持字符串（空即未填），提交时解析 */
export interface ResizeParamsState {
  modeKind: ResizeModeKind;
  boxWidth: string;
  boxHeight: string;
  lockRatio: boolean;
  noUpscale: boolean;
  percent: number;
  exactWidth: string;
  exactHeight: string;
  fit: FitMode;
  format: OutputFormat;
  quality: number;
  outputDir: string;
  filenameSuffix: boolean;
  rotation: ImageRotation;
  filter: ResizeFilter;
  overwrite: OverwritePolicy;
  /** 目标大小 KB 文本：空即不启用，提交时解析 */
  targetSizeKb: string;
  /** 精确尺寸长宽比：`free` 自由，其余 `宽:高`（如 `16:9`），见 `resize-math` 的 `RATIO_*` */
  exactRatio: string;
}

/** 批量汇总：成功/失败/跳过三计数（toast 与列表汇总条同源） */
export interface ResizeSummary {
  ok: number;
  failed: number;
  skipped: number;
}

/** 参数初始值：宽高锁定 + 不放大 + 后缀，与规划默认值一致 */
export const DEFAULT_RESIZE_PARAMS: ResizeParamsState = {
  modeKind: "box",
  boxWidth: "",
  boxHeight: "",
  lockRatio: true,
  noUpscale: true,
  percent: 100,
  exactWidth: "",
  exactHeight: "",
  fit: "contain",
  format: "original",
  quality: 85,
  outputDir: "",
  filenameSuffix: true,
  rotation: "none",
  filter: "lanczos3",
  overwrite: "increment",
  targetSizeKb: "",
  exactRatio: "free",
};
