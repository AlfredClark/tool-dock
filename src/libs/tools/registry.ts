// 工具注册表：首页搜索、工具页网格、`(tools)` 标题栏三处同源，新增工具只需加条目。
// 文案存消息函数引用（切换语言走整页重载，此处无需响应式包装），调用方使用时再求值。
import ArrowLeftRightIcon from "@lucide/svelte/icons/arrow-left-right";
import { m } from "$libs/i18n/paraglide/messages";

/** 工具分类：新增分类时扩展该元组并补 `tool_category_*` 文案，网格自动多出一组 */
export const TOOL_CATEGORIES = ["text", "image", "network", "system"] as const;

/** 工具分类取值 */
export type ToolCategory = (typeof TOOL_CATEGORIES)[number];

/** 工具清单，数组顺序即工具页网格内渲染顺序；`as const` 保留路径字面量类型 */
export const TOOLS = [
  {
    id: "data-convert",
    path: "/text/convert",
    category: "text",
    name: m.tool_data_convert_name,
    description: m.tool_data_convert_description,
    icon: ArrowLeftRightIcon,
  },
] as const;

/** 单个工具条目 */
export type ToolEntry = (typeof TOOLS)[number];

/** 已登记的工具路径，`resolve()` 需要具体的路由字面量而非 `string` */
export type ToolPath = ToolEntry["path"];

/** 校验并收窄为已登记的工具分类 */
export function isToolCategory(value: unknown): value is ToolCategory {
  return (TOOL_CATEGORIES as readonly string[]).includes(value as string);
}

/** 分类名文案：未知分类回落原文（注册表缺 key 时不白屏） */
export function categoryLabel(category: string): string {
  switch (category) {
    case "text":
      return m.tool_category_text();
    case "image":
      return m.tool_category_image();
    case "network":
      return m.tool_category_network();
    case "system":
      return m.tool_category_system();
    default:
      return category;
  }
}

/** 按路径匹配工具，未命中返回 `undefined`（标题栏回落应用名） */
export function resolveTool(pathname: string): ToolEntry | undefined {
  return TOOLS.find((tool) => tool.path === pathname);
}

/** 按名称 + 简介 + 分类名过滤工具；空查询返回空数组（首页不弹下拉框） */
export function searchTools(query: string): ToolEntry[] {
  const keyword = query.trim().toLowerCase();
  if (keyword === "") {
    return [];
  }
  return TOOLS.filter((tool) => {
    const haystack =
      `${tool.name()} ${tool.description()} ${categoryLabel(tool.category)}`.toLowerCase();
    return haystack.includes(keyword);
  });
}
