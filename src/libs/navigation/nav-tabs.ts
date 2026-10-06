// 顶部导航标签注册表：`path` 与 SvelteKit 路由一一对应，同时作为 tabs 的取值。
// 文案直接取 Paraglide 消息函数，切换语言走整页重载，故此处无需响应式包装。
import InfoIcon from "@lucide/svelte/icons/info";
import FlaskConicalIcon from "@lucide/svelte/icons/flask-conical";
import HouseIcon from "@lucide/svelte/icons/house";
import NotebookPenIcon from "@lucide/svelte/icons/notebook-pen";
import SettingsIcon from "@lucide/svelte/icons/settings";
import { m } from "$libs/i18n/paraglide/messages";

/** 导航标签清单，数组顺序即渲染顺序；`as const` 保留路径字面量类型，注册错路由会在编译期报错 */
export const NAV_TABS = [
  { path: "/", label: m.nav_home, icon: HouseIcon },
  // 演示页：模板能力一览，文案键同样 `demo_` 开头，删除演示时本行一并删除
  { path: "/demo", label: m.demo_nav_label, icon: FlaskConicalIcon },
  // 笔记页：最小端到端业务示例，删除业务示例时本行一并删除
  { path: "/notes", label: m.notes_nav_label, icon: NotebookPenIcon },
  { path: "/settings", label: m.nav_settings, icon: SettingsIcon },
  { path: "/about", label: m.nav_about, icon: InfoIcon },
] as const;

/** 单个导航标签 */
export type NavTab = (typeof NAV_TABS)[number];

/** 已登记的页面路径，`resolve()` 需要具体的路由字面量而非 `string` */
export type NavTabPath = NavTab["path"];

/** 校验并收窄为已登记的页面路径 */
export function isNavTabPath(value: unknown): value is NavTabPath {
  return NAV_TABS.some((tab) => tab.path === value);
}

/** 按路径匹配标签，未命中返回 `undefined`（子页面等不选中任何标签） */
export function resolveNavTab(pathname: string): NavTab | undefined {
  return NAV_TABS.find((tab) => tab.path === pathname);
}
