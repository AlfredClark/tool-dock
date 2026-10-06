// 外部链接薄封装：调用失败一律返回假，浏览器预览与单测环境下调用方只需分支提示。
// 仅允许 `http(s)` 外链：`package.json` 的仓库地址是 `git+https://` 前缀，需先剥离再校验，
// 其它 scheme（如 `file:` / `javascript:` / `mailto:`）一律拒绝，避免 `opener:default`
// 能力被误用；后续需要邮箱/本地资源入口时再扩展白名单。
import { reportCommandFailure } from "$libs/commands/cores";

/**
 * 在系统默认应用中打开外部链接，成功返回真
 *
 * 网关契约：全仓唯一的 `openUrl` 出口，禁止绕过本函数直引 `@tauri-apps/plugin-opener`。
 */
export async function openExternal(url: string): Promise<boolean> {
  const normalized = url.startsWith("git+") ? url.slice("git+".length) : url;
  if (!/^https?:\/\//i.test(normalized)) {
    reportCommandFailure("[opener] unsupported url scheme", url);
    return false;
  }
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(normalized);
    return true;
  } catch {
    return false;
  }
}
