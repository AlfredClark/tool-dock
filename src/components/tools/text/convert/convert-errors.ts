// 数据互转错误呈现：后端错误码→前端文案的纯映射 + 本地格式→契约格式的薄转换。
// 业务错误是常态 UI 状态（用户纠错中），经 Outcome 数据返回，不走 CommandError。
import type { ConvertFormat as BindingFormat, ErrorCode } from "$libs/commands/bindings";
import { m } from "$libs/i18n/paraglide/messages";
import { formatLabel, type ConvertFormat } from "./formats";

/** 本地 UI 格式转契约格式：两者取值一致，此处只做类型收窄（禁止类型断言） */
export function toBindingFormat(format: ConvertFormat): BindingFormat {
  return format;
}

/**
 * 错误码对应的用户文案：`format` 有值时带上格式名；`message`（解析器原文）由调用方另起一行展示。 switch 不设
 * default：后端增减 ErrorCode 变体时此处编译期拦截。
 */
export function convertErrorText(code: ErrorCode, format: BindingFormat | null): string {
  switch (code) {
    case "TooLarge":
      return m.tool_convert_error_too_large();
    case "UnknownFormat":
      return m.tool_convert_error_unknown_format();
    case "ParseFailed":
      return m.tool_convert_error_parse_failed();
    case "UnsupportedInput":
      return m.tool_convert_error_unsupported_input({ format: formatName(format) });
    case "UnsupportedOutput":
      return m.tool_convert_error_unsupported_output({ format: formatName(format) });
    case "NonTableRoot":
      return m.tool_convert_error_non_table_root({ format: formatName(format) });
    case "UnsupportedValue":
      return m.tool_convert_error_unsupported_value();
  }
}

/** 错误位置文案：行号缺失返回 `null`（调用方不渲染位置行）；列缺失只显示行 */
export function errorLineText(line: number | null, column: number | null): string | null {
  if (line == null) return null;
  if (column == null) return m.tool_convert_error_line({ line });
  return m.tool_convert_error_line_column({ line, column });
}

/** 错误文案里的格式名：未知格式回落原文，不白屏 */
function formatName(format: BindingFormat | null): string {
  return format === null ? "unknown" : formatLabel(format);
}
