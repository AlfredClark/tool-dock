import { beforeEach, describe, expect, it, vi } from "vitest";
import { httpFetch } from "$libs/http/client";
import { HttpError } from "$libs/http/types";

vi.mock("@tauri-apps/plugin-http", () => ({
  fetch: vi.fn(),
}));

import { fetch as fetchMock } from "@tauri-apps/plugin-http";

function okResponse(): Response {
  return { ok: true, status: 200, json: async () => ({}) } as Response;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("httpFetch", () => {
  it("2xx 直接返回响应", async () => {
    vi.mocked(fetchMock).mockResolvedValue(okResponse());

    const response = await httpFetch("https://timeapi.io/api/time/current/zone?timeZone=UTC");

    expect(response.ok).toBe(true);
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it("非 2xx 抛 http 错误且不重试", async () => {
    vi.mocked(fetchMock).mockResolvedValue({ ok: false, status: 404 } as Response);

    const error = await httpFetch("https://timeapi.io/missing").catch((error: unknown) => error);

    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).kind).toBe("http");
    expect((error as HttpError).status).toBe(404);
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it("网络错误默认重试一次后抛出", async () => {
    vi.mocked(fetchMock).mockRejectedValue(new Error("connection refused"));

    const error = await httpFetch("https://timeapi.io/x").catch((error: unknown) => error);

    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).kind).toBe("network");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("retries 为 0 时失败即抛", async () => {
    vi.mocked(fetchMock).mockRejectedValue(new Error("connection refused"));

    await expect(httpFetch("https://timeapi.io/x", { retries: 0 })).rejects.toBeInstanceOf(
      HttpError,
    );
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it("超时熔断抛 timeout 错误", async () => {
    vi.mocked(fetchMock).mockImplementation(
      (_input: string | URL | Request, init?: RequestInit) =>
        new Promise<Response>((_resolve, reject) => {
          init?.signal?.addEventListener("abort", () => {
            // jsdom/node 下 DOMException 可用；兜底走普通 Error 的 AbortError 名
            try {
              reject(new DOMException("aborted", "AbortError"));
            } catch {
              reject(Object.assign(new Error("aborted"), { name: "AbortError" }));
            }
          });
        }),
    );

    const error = await httpFetch("https://timeapi.io/x", { timeoutMs: 20, retries: 0 }).catch(
      (error: unknown) => error,
    );

    expect(error).toBeInstanceOf(HttpError);
    expect((error as HttpError).kind).toBe("timeout");
  });

  it("非桌面环境 import 失败归一为网络错误", async () => {
    vi.mocked(fetchMock).mockRejectedValue(new Error("no runtime"));

    const error = await httpFetch("https://timeapi.io/x", { retries: 0 }).catch(
      (error: unknown) => error,
    );

    expect((error as HttpError).kind).toBe("network");
  });
});
