// 重命名规则纯函数：拆扩展/单条应用/按序应用/预览拼装/工厂/数组变换，供 workspace 调用与单测覆盖。
// 无 Svelte 依赖，可在 node 单测直接导入；非法参数一律整条跳过（passthrough），预览永不抛错。
import type {
  RenamerCharsClass,
  RenamerNormalizePreset,
  RenamerRule,
  RenamerRuleKind,
} from "./renamer-types";

/** 规则候选顺序：即添加下拉框的展示顺序 */
export const RULE_KINDS: RenamerRuleKind[] = [
  "affix",
  "case",
  "normalize",
  "replace",
  "number",
  "slice",
  "insert",
  "chars",
];

/** 拆分主名与扩展：扩展取最后一个 `.` 后缀；点文件（如 `.gitignore`）视为无扩展 */
export function splitName(filename: string): { stem: string; ext: string } {
  const dot = filename.lastIndexOf(".");
  if (dot <= 0 || dot === filename.length - 1) return { stem: filename, ext: "" };
  return { stem: filename.slice(0, dot), ext: filename.slice(dot) };
}

/** 解析非负整数文本：非法返回 `null`（调用方整条跳过） */
function parseCount(text: string): number | null {
  if (!/^\d+$/.test(text.trim())) return null;
  return Number.parseInt(text.trim(), 10);
}

/** 解析可负整数文本（切片锚点/范围、插入位置用）：非法返回 `null`（调用方整条跳过） */
function parseSliceInt(text: string): number | null {
  if (!/^-?\d+$/.test(text.trim())) return null;
  return Number.parseInt(text.trim(), 10);
}

/** 字符取舍的匹配源：类别映射为字符类，`custom` 转义后入 `[]`；空自定义返回 `null`（调用方整条跳过） */
function charsClassSource(cls: RenamerCharsClass, custom: string): string | null {
  switch (cls) {
    case "digits":
      return "\\p{Nd}";
    case "letters":
      return "\\p{Script=Latin}";
    case "chinese":
      return "\\p{Script=Han}";
    case "spaces":
      return "\\s";
    case "custom":
      return custom === "" ? null : `[${escapeRegExp(custom)}]`;
  }
}

/** 转义正则特殊字符（`replace` 大小写不敏感分支用） */
function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** 渲染编号格式：`$n` 为编号占位，`$$` 转义为字面 `$`（无占位即静态文本） */
function renderNumberFormat(format: string, padded: string): string {
  return format.replace(/\$\$|\$n/g, (match) => (match === "$$" ? "$" : padded));
}

/** 应用 `title` 模式：按空白/下划线/连字符分词，每词首字母大写 */
function toTitleCase(stem: string): string {
  return stem
    .toLowerCase()
    .split(/([\s_-]+)/)
    .map((part) =>
      /^[\s_-]+$/.test(part) || part === "" ? part : part.charAt(0).toUpperCase() + part.slice(1),
    )
    .join("");
}

/**
 * 规范化：按预设做单一变换（`trim` 去首尾空白，`remove-spaces` 去全部空白，`remove-illegal`
 * 删系统非法字符，`collapse-spaces` 连续空白收束为单空格，`spaces-to-underscore/hyphen` 空白段转单分隔符）
 */
function normalizeStem(stem: string, preset: RenamerNormalizePreset): string {
  switch (preset) {
    case "trim":
      return stem.trim();
    case "remove-spaces":
      return stem.replace(/\s+/g, "");
    case "remove-illegal":
      return stem.replace(/[/\\:*?"<>|\p{Cc}]/gu, "");
    case "collapse-spaces":
      return stem.replace(/\s+/g, " ");
    case "spaces-to-underscore":
      return stem.replace(/\s+/g, "_");
    case "spaces-to-hyphen":
      return stem.replace(/\s+/g, "-");
  }
}

/** 应用单条规则：返回新主名。 `index` 为文件在全量列表中的加入序号（自动编号基准，筛选/排序不改变编号）。 */
export function applyRule(stem: string, rule: RenamerRule, index: number): string {
  switch (rule.kind) {
    case "affix": {
      if (rule.mode === "remove") {
        if (rule.text === "") return stem;
        if (rule.position === "prefix")
          return stem.startsWith(rule.text) ? stem.slice(rule.text.length) : stem;
        return stem.endsWith(rule.text) ? stem.slice(0, stem.length - rule.text.length) : stem;
      }
      return rule.position === "prefix" ? `${rule.text}${stem}` : `${stem}${rule.text}`;
    }
    case "case": {
      if (rule.mode === "upper") return stem.toUpperCase();
      if (rule.mode === "lower") return stem.toLowerCase();
      if (rule.mode === "sentence")
        return stem.charAt(0).toUpperCase() + stem.slice(1).toLowerCase();
      return toTitleCase(stem);
    }
    case "replace": {
      if (rule.find === "") return stem;
      if (rule.mode === "regex") {
        try {
          return stem.replace(new RegExp(rule.find, rule.matchCase ? "g" : "gi"), rule.replacement);
        } catch {
          return stem;
        }
      }
      if (rule.matchCase) return stem.split(rule.find).join(rule.replacement);
      return stem.replace(new RegExp(escapeRegExp(rule.find), "gi"), rule.replacement);
    }
    case "number": {
      const start = parseCount(rule.start);
      const step = parseCount(rule.step);
      const digits = parseCount(rule.digits);
      if (start === null || step === null || digits === null) return stem;
      const padded = String(start + index * step).padStart(digits, "0");
      const block = renderNumberFormat(rule.format, padded);
      return rule.position === "prefix"
        ? `${block}${rule.separator}${stem}`
        : `${stem}${rule.separator}${block}`;
    }
    case "normalize":
      return normalizeStem(stem, rule.preset);
    case "slice": {
      if (rule.anchor === "" || rule.length === "") return stem;
      const anchor = parseSliceInt(rule.anchor);
      const length = parseSliceInt(rule.length);
      if (anchor === null || length === null) return stem;
      const size = stem.length;
      const start = anchor >= 0 ? Math.min(anchor, size) : Math.max(size + anchor, 0);
      if (length >= 0) return stem.slice(start, Math.min(start + length, size));
      return stem.slice(Math.max(start + length + 1, 0), start + 1);
    }
    case "insert": {
      if (rule.text === "" || rule.position === "") return stem;
      const position = parseSliceInt(rule.position);
      if (position === null) return stem;
      const size = stem.length;
      const at = position >= 0 ? Math.min(position, size) : Math.max(size + position, 0);
      return stem.slice(0, at) + rule.text + stem.slice(at);
    }
    case "chars": {
      const source = charsClassSource(rule.class, rule.custom);
      if (source === null) return stem;
      if (rule.action === "delete") return stem.replace(new RegExp(source, "gu"), "");
      return (stem.match(new RegExp(source, "gu")) ?? []).join("");
    }
  }
}

/** 按序应用启用的规则：数组顺序即应用顺序 */
export function applyRules(stem: string, rules: RenamerRule[], index: number): string {
  let current = stem;
  for (const rule of rules) {
    if (rule.enabled) current = applyRule(current, rule, index);
  }
  return current;
}

/** 预览完整文件名：拆扩展 → 主名按序应用 → 拼回（零规则直接返回原名） */
export function previewName(filename: string, rules: RenamerRule[], index: number): string {
  if (rules.length === 0) return filename;
  const { stem, ext } = splitName(filename);
  return `${applyRules(stem, rules, index)}${ext}`;
}

/** 规则工厂：默认值 + 启用 + 随机 id（泛型收窄返回成员类型，调用方 spread 改写可过检） */
export function createRule<K extends RenamerRuleKind>(kind: K): Extract<RenamerRule, { kind: K }> {
  const id = crypto.randomUUID();
  switch (kind) {
    case "affix":
      return { id, kind, enabled: true, mode: "add", position: "suffix", text: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
    case "case":
      return { id, kind, enabled: true, mode: "lower" } as Extract<RenamerRule, { kind: K }>;
    case "replace":
      return {
        id,
        kind,
        enabled: true,
        mode: "plain",
        find: "",
        replacement: "",
        matchCase: true,
      } as Extract<RenamerRule, { kind: K }>;
    case "number":
      return {
        id,
        kind,
        enabled: true,
        start: "1",
        step: "1",
        digits: "3",
        position: "prefix",
        separator: "_",
        format: "$n",
      } as Extract<RenamerRule, { kind: K }>;
    case "normalize":
      return { id, kind, enabled: true, preset: "trim" } as Extract<RenamerRule, { kind: K }>;
    case "slice":
      return { id, kind, enabled: true, anchor: "0", length: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
    case "insert":
      return { id, kind, enabled: true, position: "0", text: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
    case "chars":
      return { id, kind, enabled: true, action: "delete", class: "digits", custom: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
  }
}

/** 校验规则参数：合法返回 `null`，非法返回错误原因（卡片红 Badge 展示，预览层照常跳过） */
export function validateRule(
  rule: RenamerRule,
): "bad-regex" | "bad-number" | "bad-slice" | "bad-insert" | null {
  switch (rule.kind) {
    case "slice": {
      if (rule.anchor === "" || rule.length === "") return null;
      return parseSliceInt(rule.anchor) === null || parseSliceInt(rule.length) === null
        ? "bad-slice"
        : null;
    }
    case "insert": {
      if (rule.text === "" || rule.position === "") return null;
      return parseSliceInt(rule.position) === null ? "bad-insert" : null;
    }
    case "replace": {
      if (rule.mode !== "regex" || rule.find === "") return null;
      try {
        new RegExp(rule.find);
        return null;
      } catch {
        return "bad-regex";
      }
    }
    case "number":
      return parseCount(rule.start) === null ||
        parseCount(rule.step) === null ||
        parseCount(rule.digits) === null
        ? "bad-number"
        : null;
    default:
      return null;
  }
}

/** 移动规则到指定下标：越界钳制，同位/未知 id 返回原数组（引用不变） */
export function moveRuleTo(rules: RenamerRule[], id: string, toIndex: number): RenamerRule[] {
  const from = rules.findIndex((rule) => rule.id === id);
  const to = Math.max(0, Math.min(toIndex, rules.length - 1));
  if (from < 0 || from === to) return rules;
  const copy = [...rules];
  const [picked] = copy.splice(from, 1);
  if (picked !== undefined) copy.splice(to, 0, picked);
  return copy;
}

/** 切换启用开关 */
export function toggleRule(rules: RenamerRule[], id: string, enabled: boolean): RenamerRule[] {
  return rules.map((rule) => (rule.id === id ? { ...rule, enabled } : rule));
}

/** 替换单条规则（参数卡编辑整体回写） */
export function updateRule(rules: RenamerRule[], next: RenamerRule): RenamerRule[] {
  return rules.map((rule) => (rule.id === next.id ? next : rule));
}

/** 删除单条规则 */
export function removeRule(rules: RenamerRule[], id: string): RenamerRule[] {
  return rules.filter((rule) => rule.id !== id);
}
