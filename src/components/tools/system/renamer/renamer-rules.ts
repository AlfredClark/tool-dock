// 重命名规则纯函数：拆扩展/单条应用/按序应用/预览拼装/工厂/数组变换，供 workspace 调用与单测覆盖。
// 无 Svelte 依赖，可在 node 单测直接导入；非法参数一律整条跳过（passthrough），预览永不抛错。
import type { RenamerRule, RenamerRuleKind } from "./renamer-types";

/** 规则候选顺序：即添加下拉框的展示顺序 */
export const RULE_KINDS: RenamerRuleKind[] = [
  "affix",
  "strip",
  "case",
  "replace",
  "regex",
  "number",
  "normalize",
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

/** 转义正则特殊字符（`replace` 大小写不敏感分支用） */
function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
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

/** 规范化：去首尾空白 → 连续空白转单下划线 → 移除系统非法字符 → 合并连续下划线 */
function normalizeStem(stem: string): string {
  return stem
    .trim()
    .replace(/\s+/g, "_")
    .replace(/[/\\:*?"<>|\p{Cc}]/gu, "")
    .replace(/_+/g, "_");
}

/** 应用单条规则：返回新主名。 `index` 为文件在全量列表中的加入序号（自动编号基准，筛选/排序不改变编号）。 */
export function applyRule(stem: string, rule: RenamerRule, index: number): string {
  switch (rule.kind) {
    case "affix":
      return rule.position === "prefix" ? `${rule.text}${stem}` : `${stem}${rule.text}`;
    case "strip": {
      const count = parseCount(rule.count);
      if (count === null || count <= 0) return stem;
      return rule.position === "prefix" ? stem.slice(count) : stem.slice(0, stem.length - count);
    }
    case "case": {
      if (rule.mode === "upper") return stem.toUpperCase();
      if (rule.mode === "lower") return stem.toLowerCase();
      return toTitleCase(stem);
    }
    case "replace": {
      if (rule.find === "") return stem;
      if (rule.matchCase) return stem.split(rule.find).join(rule.replacement);
      return stem.replace(new RegExp(escapeRegExp(rule.find), "gi"), rule.replacement);
    }
    case "regex": {
      if (rule.pattern === "") return stem;
      try {
        return stem.replace(new RegExp(rule.pattern, "g"), rule.replacement);
      } catch {
        return stem;
      }
    }
    case "number": {
      const start = parseCount(rule.start);
      const step = parseCount(rule.step);
      const digits = parseCount(rule.digits);
      if (start === null || step === null || digits === null) return stem;
      const padded = String(start + index * step).padStart(digits, "0");
      return rule.position === "prefix"
        ? `${padded}${rule.separator}${stem}`
        : `${stem}${rule.separator}${padded}`;
    }
    case "normalize":
      return normalizeStem(stem);
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
      return { id, kind, enabled: true, position: "suffix", text: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
    case "strip":
      return { id, kind, enabled: true, position: "prefix", count: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
    case "case":
      return { id, kind, enabled: true, mode: "lower" } as Extract<RenamerRule, { kind: K }>;
    case "replace":
      return { id, kind, enabled: true, find: "", replacement: "", matchCase: true } as Extract<
        RenamerRule,
        { kind: K }
      >;
    case "regex":
      return { id, kind, enabled: true, pattern: "", replacement: "" } as Extract<
        RenamerRule,
        { kind: K }
      >;
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
      } as Extract<RenamerRule, { kind: K }>;
    case "normalize":
      return { id, kind, enabled: true } as Extract<RenamerRule, { kind: K }>;
  }
}

/** 校验规则参数：合法返回 `null`，非法返回错误原因（卡片红 Badge 展示，预览层照常跳过） */
export function validateRule(rule: RenamerRule): "bad-count" | "bad-regex" | "bad-number" | null {
  switch (rule.kind) {
    case "strip":
      return parseCount(rule.count) === null ? "bad-count" : null;
    case "regex": {
      if (rule.pattern === "") return null;
      try {
        new RegExp(rule.pattern);
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
