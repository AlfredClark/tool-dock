// 通用请求封装：超时熔断 + 有限重试 + 错误归一，前端网络出口唯一收敛点。
// 插件包动态导入：浏览器预览与 node 单测下 import 本体不抛错，失败按网络错误归一。
import type { HttpErrorKind } from "$libs/http/types";
import { HttpError } from "$libs/http/types";

/** 默认总超时；外部接口 RTT 毫秒级，10s 足够区分真故障与慢网络 */
export const DEFAULT_TIMEOUT_MS = 10_000;
/** 默认重试次数（不含首次）；仅超时与网络错误值得重试 */
export const DEFAULT_RETRIES = 1;

export interface HttpFetchOptions {
  timeoutMs?: number;
  retries?: number;
}

/** 经 `plugin-http` 发请求；失败抛 `HttpError`，调用方 `try/catch` 后 toast 即可 */
export async function httpFetch(input: string, options?: HttpFetchOptions): Promise<Response> {
  const timeoutMs = options?.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const retries = options?.retries ?? DEFAULT_RETRIES;

  let fetchImpl: (input: string, init?: RequestInit) => Promise<Response>;
  try {
    ({ fetch: fetchImpl } = await import("@tauri-apps/plugin-http"));
  } catch {
    // 非 Tauri 环境（浏览器预览）无插件运行时，直接归一不上报噪音
    throw new HttpError("network", "http is only available in the desktop app");
  }

  let lastError: HttpError = new HttpError("network", `request failed: ${input}`);
  for (let attempt = 0; attempt <= retries; attempt += 1) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetchImpl(input, { signal: controller.signal });
      if (!response.ok) {
        // 非 2xx 不重试：服务端已明确表态，再发只是重复失败
        throw new HttpError(
          "http",
          `request failed with status ${response.status}`,
          response.status,
        );
      }
      return response;
    } catch (error) {
      lastError = normalizeFetchError(error, input);
      // 仅超时与网络错误进下一次重试，`http` 分支直接抛出
      if (lastError.kind !== "timeout" && lastError.kind !== "network") throw lastError;
    } finally {
      clearTimeout(timer);
    }
  }
  throw lastError;
}

/** 把 `fetch` 的抛错二分为超时与网络错误，其余 HttpError 原样透传 */
function normalizeFetchError(error: unknown, input: string): HttpError {
  if (error instanceof HttpError) return error;
  const kind: HttpErrorKind = isAbortError(error) ? "timeout" : "network";
  const detail = error instanceof Error ? error.message : String(error);
  return new HttpError(kind, `request failed (${kind}): ${input} — ${detail}`);
}

/** 超时熔断唯一信号：`AbortController.abort()` 触发的 `AbortError` */
function isAbortError(error: unknown): boolean {
  return error instanceof DOMException
    ? error.name === "AbortError"
    : (error as { name?: unknown })?.name === "AbortError";
}
