// 数据互转工具的格式定义：工具私有的应用知识，放在工具目录内，不进 libs/。
// 格式名展示用大写字面量（JSON 等缩写不翻译），只有“自动识别”走 i18n。
import { m } from "$libs/i18n/paraglide/messages";

/** 可转换的数据格式：六种，数组顺序即下拉框展示顺序 */
export const CONVERT_FORMATS = ["json", "yaml", "xml", "toml", "ini", "properties"] as const;

/** 可转换的数据格式取值 */
export type ConvertFormat = (typeof CONVERT_FORMATS)[number];

/** 输入端候选项：比输出端多一个自动识别 */
export const INPUT_FORMATS = ["auto", ...CONVERT_FORMATS] as const;

/** 输入端格式取值（含自动识别） */
export type InputFormat = (typeof INPUT_FORMATS)[number];

/** 输出端候选项：必须显式指定格式，不支持自动识别 */
export const OUTPUT_FORMATS = CONVERT_FORMATS;

/** 是否为已登记的转换格式（下拉回调用值收窄，不用类型断言） */
export function isConvertFormat(value: unknown): value is ConvertFormat {
  return (CONVERT_FORMATS as readonly string[]).includes(value as string);
}

/** 是否为合法的输入端取值（含自动识别） */
export function isInputFormat(value: unknown): value is InputFormat {
  return (INPUT_FORMATS as readonly string[]).includes(value as string);
}

/** 格式展示名：缩写保持大写，直出字面量 */
export function formatLabel(format: ConvertFormat): string {
  switch (format) {
    case "json":
      return "JSON";
    case "yaml":
      return "YAML";
    case "xml":
      return "XML";
    case "toml":
      return "TOML";
    case "ini":
      return "INI";
    case "properties":
      return "Properties";
  }
}

/** 输入端下拉候选项（含自动识别，展示文案在调用时求值） */
export function inputFormatItems(): { value: InputFormat; label: string }[] {
  return INPUT_FORMATS.map((value) => ({
    value,
    label: value === "auto" ? m.tool_convert_format_auto() : formatLabel(value),
  }));
}

/** 输出端下拉候选项（无自动识别） */
export function outputFormatItems(): { value: ConvertFormat; label: string }[] {
  return OUTPUT_FORMATS.map((value) => ({ value, label: formatLabel(value) }));
}

/** 浏览器降级路径的文件大小上限：与后端 `MAX_INPUT_LEN` 对齐（1 MiB），改动时两边同步 */
export const DROP_MAX_BYTES = 1024 * 1024;

/** 文件扩展名→格式映射：拖拽载入时预选输入格式；`Record` 键缺失即未知格式 */
const EXTENSION_FORMATS: Record<string, ConvertFormat> = {
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  xml: "xml",
  toml: "toml",
  ini: "ini",
  properties: "properties",
};

/** 按文件名取格式：未知扩展名返回 `null`（保持当前选择，走自动识别） */
export function formatFromExtension(filename: string): ConvertFormat | null {
  const ext = filename.split(".").pop()?.toLowerCase();
  if (!ext) return null;
  return EXTENSION_FORMATS[ext] ?? null;
}

/** 取路径末段文件名（兼容 `/` 与 `\` 分隔符） */
export function basenameOf(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
