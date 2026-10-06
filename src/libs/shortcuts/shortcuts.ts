// 快捷键前后端胶水：前端经插件 JS API 直调注册 / 注销 / 查询，薄封装只做异常归一。
// 插件包动态导入：浏览器预览与 node 单测下 import 本体不抛错，非桌面端按不可用归一。
import type { ShortcutHandler } from "@tauri-apps/plugin-global-shortcut";

/** 关闭窗口演示预设键：`X` 呼应标题栏关闭图标，与后端固定演示键同族好记 */
export const CLOSE_WINDOW_SHORTCUT = "Ctrl+Alt+X";

/** 注册全局快捷键；`Released` 在薄层直接过滤，回调只收 `Pressed` */
export async function registerShortcut(
  accelerator: string,
  handler: (event: { state: "Pressed" }) => void,
): Promise<void> {
  const { register } = await import("@tauri-apps/plugin-global-shortcut");
  const wrapped: ShortcutHandler = (event) => {
    if (event.state === "Pressed") handler({ state: event.state });
  };
  await register(accelerator, wrapped);
}

/** 注销全局快捷键；未注册时后端幂等成功，调用方无需预查 */
export async function unregisterShortcut(accelerator: string): Promise<void> {
  const { unregister } = await import("@tauri-apps/plugin-global-shortcut");
  await unregister(accelerator);
}

/** 查询是否已注册；非桌面端（插件缺失）回 `false`，调用方标不可用 */
export async function isShortcutRegistered(accelerator: string): Promise<boolean> {
  try {
    const { isRegistered } = await import("@tauri-apps/plugin-global-shortcut");
    return await isRegistered(accelerator);
  } catch {
    return false;
  }
}
