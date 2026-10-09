// 视频格式转换工具的界面态类型：后端契约走 `bindings`，此处只放前端自有状态。

import type {
  Accelerator,
  AudioBitrate,
  ConvertMode,
  OutputResolution,
  OverwritePolicy,
  VideoEncoder,
  VideoFileMetadata,
  VideoPreset,
  VideoQuality,
  VideoTarget,
} from "$libs/commands/bindings";

/** 目标容器候选：与后端 `VideoTarget` 七变体一致，下拉渲染顺序 */
export const VIDEO_TARGETS: readonly VideoTarget[] = [
  "mp4",
  "m4v",
  "mov",
  "mkv",
  "webm",
  "avi",
  "ts",
];

/** 转码模式候选：与后端 `ConvertMode` 一致（线上传小写无分隔，`copyonly` 注意拼写） */
export const CONVERT_MODES: readonly ConvertMode[] = ["auto", "copyonly", "reencode"];

/** 目标→编码器允许表：与后端 `VideoTarget::allowed_encoders` 同源（首项为默认） */
export const ENCODERS_BY_TARGET: Record<VideoTarget, readonly VideoEncoder[]> = {
  mp4: ["libx264", "libx265"],
  m4v: ["libx264", "libx265"],
  mov: ["libx264", "libx265"],
  mkv: ["libx264", "libx265"],
  webm: ["vp9", "av1"],
  avi: ["mpeg4"],
  ts: ["libx264", "libx265", "mpeg2video"],
};

/** 画质档候选：三档通用语义，后端按编码器映射具体值 */
export const VIDEO_QUALITIES: readonly VideoQuality[] = ["high", "standard", "compact"];

/** 编码速度候选：仅 264/265 系有效（`supportsPreset` 为假时隐藏） */
export const VIDEO_PRESETS: readonly VideoPreset[] = ["ultrafast", "veryfast", "medium", "slow"];

/** 音频码率候选：`default` 即不传 `-b:a`，复刻旧行为 */
export const AUDIO_BITRATES: readonly AudioBitrate[] = ["default", "kb128", "kb192", "kb320"];

/** 分辨率候选：`source` 即不缩放，其余按高缩放、宽自适应 */
export const OUTPUT_RESOLUTIONS: readonly OutputResolution[] = ["source", "p1080", "p720", "p480"];

/** 目标的默认编码器：允许表首项（复刻各容器的旧默认行为） */
export function defaultEncoderFor(target: VideoTarget): VideoEncoder {
  return ENCODERS_BY_TARGET[target][0];
}

/** 编码器是否支持速度档：仅 264/265 系拼 `-preset` */
export function supportsPreset(encoder: VideoEncoder): boolean {
  return encoder === "libx264" || encoder === "libx265";
}

/** 目标→加速器允许表：与后端 `supports_hw` 同源（第一批仅 NVENC） */
export const ACCELERATORS_BY_TARGET: Record<VideoTarget, readonly Accelerator[]> = {
  mp4: ["cpu", "nvenc"],
  m4v: ["cpu", "nvenc"],
  mov: ["cpu", "nvenc"],
  mkv: ["cpu", "nvenc"],
  webm: ["cpu"],
  avi: ["cpu"],
  ts: ["cpu", "nvenc"],
};

/** 目标的默认加速器：CPU（旧行为，加速一律显式选择） */
export function defaultAcceleratorFor(target: VideoTarget): Accelerator {
  return ACCELERATORS_BY_TARGET[target][0];
}

/** 编码器是否有硬编对应（`mpeg2video` 等传统编码器选它时加速归 CPU） */
export function supportsHwEncoding(encoder: VideoEncoder): boolean {
  return encoder === "libx264" || encoder === "libx265";
}

/** 切目标后的状态重置：编码器与加速归新目标默认、速度归默认，其余通用档保留 */
export function resetForTarget(params: ConvertParamsState, next: VideoTarget): ConvertParamsState {
  return {
    ...params,
    target: next,
    videoEncoder: defaultEncoderFor(next),
    preset: "veryfast",
    accelerator: defaultAcceleratorFor(next),
  };
}

/** 切编码器后的加速重置：无硬编对应的编码器即回 CPU */
export function resetForEncoder(
  params: ConvertParamsState,
  next: VideoEncoder,
): ConvertParamsState {
  return {
    ...params,
    videoEncoder: next,
    preset: supportsPreset(next) ? params.preset : "veryfast",
    accelerator: supportsHwEncoding(next) ? params.accelerator : "cpu",
  };
}

/** 列表项状态：读取中/就绪/不可读/已完成/失败/已跳过（批量进度由 workspace 进度态表达） */
export type VideoConvertItemStatus =
  "loading" | "ready" | "invalid" | "done" | "failed" | "skipped";

/**
 * 列表项：`previewUrl` 为批量 96px 小海报（JPEG `data:` URL），`detailUrl` 为选中后懒加载的 768px
 * 大海报
 */
export interface VideoConvertItem {
  /** 稳定键：桌面端用路径，浏览器用 `browser:<name>:<size>` 合成 */
  id: string;
  /** 本地路径（浏览器降级为空串，仅展示文件名） */
  path: string;
  /** 显示名：路径取 basename，浏览器取 File 名 */
  name: string;
  /** 小海报：后端批量抽帧 `data:` URL；抽帧失败时为空串 */
  previewUrl: string;
  /** 大海报：选中项按需取 768px，空即未载（`detailLoading` 为真即在途） */
  detailUrl: string;
  /** 大海报在途：中栏先显小图，不阻塞开始按钮 */
  detailLoading: boolean;
  /** 大海报已失败：取回空或传输失败即终态，不再重试（防 `$effect` 空转刷命令） */
  detailFailed: boolean;
  /** 元信息：`invalid` 时为 `null` */
  info: VideoFileMetadata | null;
  /** 不可读/失败原因（英文诊断原文，前端展示通用文案 + 原文补充） */
  errorDetail: string | null;
  status: VideoConvertItemStatus;
  /** 输出路径：`done` 后展示 */
  output: string | null;
  /** 转换备注：`done` 后展示流复制/重编码（失败/跳过为 `null`） */
  note: string | null;
}

/** 参数面板状态：目标格式 + 转码模式 + 重编码五选项 + 加速 + 输出目录 + 重名策略（无模板展开） */
export interface ConvertParamsState {
  /** 目标容器：下拉选择，输出后缀据此计算 */
  target: VideoTarget;
  /** 转码模式：智能/仅换容器/强制重编码 */
  mode: ConvertMode;
  /** 画质档：三档通用语义，后端按编码器映射（仅重编码生效） */
  quality: VideoQuality;
  /** 编码速度：仅 264/265 系有效（仅重编码生效） */
  preset: VideoPreset;
  /** 视频编码器：候选随目标变化（仅重编码生效） */
  videoEncoder: VideoEncoder;
  /** 音频码率：`default` 即不指定（仅重编码生效） */
  audioBitrate: AudioBitrate;
  /** 输出分辨率：`source` 即不缩放（仅重编码生效） */
  resolution: OutputResolution;
  /** 硬件加速：`cpu` 为默认（旧行为）；`nvenc` 只影响重编码程 */
  accelerator: Accelerator;
  /** 输出目录：空即未选，开始按钮禁用 */
  outputDir: string;
  /** 重名策略：与图片/元数据侧同语义 */
  overwrite: OverwritePolicy;
}

/** 参数初始值：默认转 mp4 + 智能模式 + 重编码默认挡（复刻旧 argv）+ 输出目录空 + 自动重命名 */
export const DEFAULT_CONVERT_PARAMS: ConvertParamsState = {
  target: "mp4",
  mode: "auto",
  quality: "standard",
  preset: "veryfast",
  videoEncoder: "libx264",
  audioBitrate: "default",
  resolution: "source",
  accelerator: "cpu",
  outputDir: "",
  overwrite: "increment",
};

/** 批量汇总：成功/失败/跳过三计数（toast 与列表汇总条同源） */
export interface VideoConvertSummary {
  ok: number;
  failed: number;
  skipped: number;
}

/** 下载量格式化：B/KB/MB 一位小数（FFmpeg 页进度行用，与元数据侧同口径） */
export function formatDownloadSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** 时长格式化：`mm:ss`（技术行用） */
export function formatClock(totalSeconds: number): string {
  const total = Math.max(0, Math.floor(totalSeconds));
  const minutes = String(Math.floor(total / 60)).padStart(2, "0");
  const seconds = String(total % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
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

/** 预期输出名：原名茎 + 目标后缀（中栏目标行预览用，不预留路径） */
export function expectedOutputName(path: string, target: VideoTarget): string {
  const stem = videoFileStem(path);
  if (stem === "") return "";
  return `${stem}.${target}`;
}
