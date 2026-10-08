// 视频元数据工具的界面态类型：后端契约走 `bindings`，此处只放前端自有状态。

import type { VideoFileMetadata } from "$libs/commands/bindings";

/** 标签键：六个通用标题系字段，与后端 `VideoTags` 一一对应 */
export const TAG_KEYS = ["title", "artist", "album", "genre", "date", "comment"] as const;

/** 标签键取值 */
export type TagKey = (typeof TAG_KEYS)[number];

/** 列表项状态：读取中/就绪/不可读/已完成/失败/已跳过（批量进度由 workspace 进度态表达） */
export type VideoItemStatus = "loading" | "ready" | "invalid" | "done" | "failed" | "skipped";

/** 列表项：`previewUrl` 为后端海报帧 `data:` URL（卸载时无需释放，非 `objectURL`） */
export interface VideoItem {
  /** 稳定键：桌面端用路径，浏览器用 `blob:<name>:<size>` 合成 */
  id: string;
  /** 本地路径（浏览器降级为空串，仅展示文件名） */
  path: string;
  /** 显示名：路径取 basename，浏览器取 File 名 */
  name: string;
  /** 海报地址：后端抽帧 `data:` URL；抽帧失败时为空串 */
  previewUrl: string;
  /** 元信息：`invalid` 时为 `null` */
  info: VideoFileMetadata | null;
  /** 不可读/失败原因（英文诊断原文，前端展示通用文案 + 原文补充） */
  errorDetail: string | null;
  status: VideoItemStatus;
  /** 输出路径：`done` 后展示 */
  output: string | null;
  /** 封面备注：`done` 后展示嵌入/跳过原因 */
  coverNote: string | null;
}

/** 参数面板状态：模板文本保持字符串（空即保持原值），提交时逐项展开 */
export interface MetadataParamsState {
  /** 六字段模板：空即保持原值，非空按占位符展开后写入 */
  templates: Record<TagKey, string>;
  /** 清空标记：置位即该字段写空串（清空标签），与模板互斥（置位即忽略模板） */
  cleared: Record<TagKey, boolean>;
  /** 封面模板：空即保持原封面，非空按占位符展开后解析文件 */
  coverFile: string;
  /** 封面清除：置位即清除封面（与封面模板互斥，置位优先） */
  coverClear: boolean;
  /** 输出目录：空即未选，开始按钮禁用 */
  outputDir: string;
  /** 重名策略：与图片侧同语义 */
  overwrite: "increment" | "overwrite" | "skip";
}

/** 参数初始值：模板全空（保持原值）+ 输出目录空 + 自动重命名 */
export const DEFAULT_METADATA_PARAMS: MetadataParamsState = {
  templates: { title: "", artist: "", album: "", genre: "", date: "", comment: "" },
  cleared: { title: false, artist: false, album: false, genre: false, date: false, comment: false },
  coverFile: "",
  coverClear: false,
  outputDir: "",
  overwrite: "increment",
};

/** 批量汇总：成功/失败/跳过三计数（toast 与列表汇总条同源） */
export interface VideoSummary {
  ok: number;
  failed: number;
  skipped: number;
}

/** 下载量格式化：B/KB/MB 一位小数（FFmpeg 页进度行用） */
export function formatDownloadSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
