import type { ResizeErrorCode } from "$libs/commands/bindings";
import { m } from "$libs/i18n/paraglide/messages";

// 图片尺寸错误呈现：后端错误码→前端文案的纯映射。
// 业务错误是常态 UI 状态（单张失败跳过继续），经 Outcome 数据返回，不走 CommandError。

/** 错误码对应的用户文案。switch 不设 default：后端增减 `ResizeErrorCode` 变体时此处编译期拦截。 */
export function resizeErrorText(code: ResizeErrorCode): string {
  switch (code) {
    case "TooLarge":
      return m.tool_resize_error_too_large();
    case "UnsupportedFormat":
      return m.tool_resize_error_unsupported();
    case "DecodeFailed":
      return m.tool_resize_error_decode_failed();
    case "InvalidSize":
      return m.tool_resize_error_invalid_size();
    case "EncodeFailed":
      return m.tool_resize_error_encode_failed();
    case "OutputNotWritable":
      return m.tool_resize_error_unwritable();
    case "Skipped":
      return m.tool_resize_error_skipped();
  }
}
