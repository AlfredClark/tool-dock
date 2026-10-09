// 重命名器纯函数：筛选/排序/勾选/删除选中，供 workspace 调用与单测覆盖。
// 无 Svelte 依赖，可在 node 单测直接导入。
import type { RenamerFileItem, RenamerSortKey } from "./renamer-types";

/** 按文件名子串筛选：空查询返回原数组（引用不变，避免多余重渲染） */
export function filterByQuery(items: RenamerFileItem[], query: string): RenamerFileItem[] {
  const keyword = query.trim().toLowerCase();
  if (keyword === "") return items;
  return items.filter((item) => item.name.toLowerCase().includes(keyword));
}

/** 按排序键排序：返回新数组，不改原数组 */
export function sortItems(items: RenamerFileItem[], sortKey: RenamerSortKey): RenamerFileItem[] {
  const copy = [...items];
  copy.sort((a, b) =>
    sortKey === "name-asc" ? a.name.localeCompare(b.name) : b.name.localeCompare(a.name),
  );
  return copy;
}

/** 批量设置勾选：仅改 `ids` 内的条目（表头全选只动可见行，不动被筛选隐藏的行） */
export function setCheckedForIds(
  items: RenamerFileItem[],
  ids: Set<string>,
  checked: boolean,
): RenamerFileItem[] {
  return items.map((item) => (ids.has(item.id) ? { ...item, checked } : item));
}

/** 删除已勾选项：返回新数组 */
export function removeChecked(items: RenamerFileItem[]): RenamerFileItem[] {
  return items.filter((item) => !item.checked);
}

/** 取路径 basename：同时处理 `/` 与 `\` 分隔符（桌面端跨平台路径） */
export function basenameOf(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/");
  return parts[parts.length - 1] ?? path;
}
