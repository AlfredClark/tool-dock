import type { VideoConvertErrorCode } from "$libs/commands/bindings";
import { m } from "$libs/i18n/paraglide/messages";

// 视频格式转换错误呈现：后端错误码→前端文案的纯映射。
// 业务错误是常态 UI 状态（单文件失败跳过继续），经 Outcome 数据返回，不走 CommandError。

/** 错误码对应的用户文案。switch 不设 default：后端增减变体时此处编译期拦截。 */
export function convertErrorText(code: VideoConvertErrorCode): string {
  switch (code) {
    case "UnsupportedFormat":
      return m.tool_video_error_unsupported();
    case "OutputNotWritable":
      return m.tool_video_error_unwritable();
    case "FfmpegFailed":
      return m.tool_video_error_ffmpeg_failed();
    case "Skipped":
      return m.tool_video_error_skipped();
    case "InvalidOptions":
      return m.tool_video_error_invalid_options();
  }
}

/** 转换备注：流复制 / 硬加速重编码 / CPU 重编码三选一，成功项展示 */
export function convertNoteText(triedCopy: boolean, usedHw: boolean): string {
  if (triedCopy) return m.tool_video_convert_note_copy();
  if (usedHw) return m.tool_video_convert_note_hw();
  return m.tool_video_convert_note_reencode();
}
