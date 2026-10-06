// 快捷键输入校验：纯函数，用户键入的任意组合先过格式与保留键两道门。
// 与迁移 SQL 无关（无落盘），但与 capability 的最小授权同源：能注册什么由这里先收敛。

/** 快捷键输入长度上限（字符）；超长一律拒绝，不做截断 */
export const MAX_SHORTCUT_LEN = 32;

/** 修饰键别名表（小写 → 规范形）；主键不在此表，原样保留大小写语义 */
const MODIFIER_ALIASES = new Map([
  ["ctrl", "Ctrl"],
  ["control", "Control"],
  ["alt", "Alt"],
  ["option", "Option"],
  ["shift", "Shift"],
  ["super", "Super"],
  ["meta", "Meta"],
  ["command", "Command"],
  ["commandorcontrol", "CommandOrControl"],
]);

/** 允许的修饰键（上表值域）；`CommandOrControl` 跨平台，其余按平台语义 */
const MODIFIERS = new Set(MODIFIER_ALIASES.values());

/** 允许的主键：单字母数字、F1–F24、常用功能键（大小写不敏感） */
const MAIN_KEY_PATTERN =
  /^([A-Za-z0-9]|F([1-9]|1[0-9]|2[0-4])|Space|Tab|Enter|Escape|Delete|Backspace|Up|Down|Left|Right|Home|End|PageUp|PageDown|Insert)$/i;

/** 系统保留组合（小写比对，主键大小写不敏感）；OS 层面再被抢占则 `register()` 抛错走 toast */
const BLOCKED_SHORTCUTS = new Set(["alt+f4", "ctrl+alt+delete"]);

/** 校验失败键：调用方按键取文案 */
export type ShortcutValidationError = "demo_shortcut_invalid" | "demo_shortcut_blocked";

/** 修饰键查别名表归一（`ctrl` → `Ctrl`），主键原样保留 */
function normalizeToken(token: string): string {
  return MODIFIER_ALIASES.get(token.toLowerCase()) ?? token;
}

/** 归一化用户输入：去首尾空白、`+` 两侧空格折叠、修饰键归一 */
// 空片段（如 `Ctrl++A`）保留为空，后续按非法主键拒绝，不静默吞掉
export function normalizeShortcut(raw: string): string {
  return raw
    .trim()
    .split("+")
    .map((part) => normalizeToken(part.trim()))
    .join("+");
}

/** 校验归一化后的组合：空 / 超长 / 无修饰键 / 非法主键 / 保留键逐项拒绝 */
export function validateShortcut(raw: string): ShortcutValidationError | null {
  const normalized = normalizeShortcut(raw);
  if (normalized.length === 0) return "demo_shortcut_invalid";
  if (normalized.length > MAX_SHORTCUT_LEN) return "demo_shortcut_invalid";
  const parts = normalized.split("+");
  const main = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);
  // 修饰键缺失或含未知词、主键不在白名单，一律按格式无效拒绝
  if (modifiers.length === 0) return "demo_shortcut_invalid";
  if (!modifiers.every((modifier) => MODIFIERS.has(modifier))) return "demo_shortcut_invalid";
  if (!MAIN_KEY_PATTERN.test(main)) return "demo_shortcut_invalid";
  if (BLOCKED_SHORTCUTS.has(normalized.toLowerCase())) return "demo_shortcut_blocked";
  return null;
}
