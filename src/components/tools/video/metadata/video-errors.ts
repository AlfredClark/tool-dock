import type { CoverResult, VideoMetadataErrorCode } from "$libs/commands/bindings";
import { m } from "$libs/i18n/paraglide/messages";

// 视频元数据错误呈现：后端错误码→前端文案的纯映射。
// 业务错误是常态 UI 状态（单文件失败跳过继续），经 Outcome 数据返回，不走 CommandError。

/** 错误码对应的用户文案。switch 不设 default：后端增减变体时此处编译期拦截。 */
export function videoErrorText(code: VideoMetadataErrorCode): string {
  switch (code) {
    case "UnsupportedFormat":
      return m.tool_video_error_unsupported();
    case "InvalidTags":
      return m.tool_video_error_invalid_tags();
    case "OutputNotWritable":
      return m.tool_video_error_unwritable();
    case "FfmpegFailed":
      return m.tool_video_error_ffmpeg_failed();
    case "Skipped":
      return m.tool_video_error_skipped();
  }
}

/** 封面结果备注：`Kept` 无备注（`null`），其余按值展示。switch 不设 default：同上。 */
export function coverNoteText(result: CoverResult): string | null {
  switch (result) {
    case "Kept":
      return null;
    case "Embedded":
      return m.tool_video_cover_embedded();
    case "Cleared":
      return m.tool_video_cover_cleared();
    case "SkippedNoFile":
      return m.tool_video_cover_skipped_no_file();
    case "SkippedUnsupported":
      return m.tool_video_cover_skipped_unsupported();
    case "SkippedInvalid":
      return m.tool_video_cover_skipped_invalid();
  }
}
