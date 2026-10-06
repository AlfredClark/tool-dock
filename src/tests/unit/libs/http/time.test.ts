import { beforeEach, describe, expect, it, vi } from "vitest";
import { HttpError } from "$libs/http/types";
import { fetchCurrentTime, resolveTimeZone } from "$libs/http/time";

vi.mock("$libs/http/client", () => ({
  httpFetch: vi.fn(),
}));

import { httpFetch as httpFetchMock } from "$libs/http/client";

const payload = {
  dateTime: "2026-09-26T13:06:48",
  date: "09/26/2026",
  time: "13:06",
  timeZone: "Asia/Shanghai",
  dayOfWeek: "Saturday",
  dstActive: false,
};

function jsonResponse(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("fetchCurrentTime", () => {
  it("合法返回体透出展示字段", async () => {
    vi.mocked(httpFetchMock).mockResolvedValue(jsonResponse(payload));

    const info = await fetchCurrentTime("Asia/Shanghai");

    expect(info.dateTime).toBe(payload.dateTime);
    expect(info.timeZone).toBe("Asia/Shanghai");
    expect(vi.mocked(httpFetchMock).mock.calls[0][0]).toContain("timeZone=Asia%2FShanghai");
  });

  it("缺省时区走设备时区", async () => {
    vi.mocked(httpFetchMock).mockResolvedValue(jsonResponse(payload));

    await fetchCurrentTime();

    const url = vi.mocked(httpFetchMock).mock.calls[0][0] as string;
    expect(url).toContain("timeZone=");
    expect(url.split("timeZone=")[1].length).toBeGreaterThan(0);
  });

  it("响应体非法抛 decode 错误", async () => {
    vi.mocked(httpFetchMock).mockResolvedValue(jsonResponse({ dateTime: 123 }));

    await expect(fetchCurrentTime("UTC")).rejects.toMatchObject({
      kind: "decode",
    } as Partial<HttpError>);
  });

  it("json 解析失败抛 decode 错误", async () => {
    vi.mocked(httpFetchMock).mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new Error("bad json");
      },
    } as unknown as Response);

    await expect(fetchCurrentTime("UTC")).rejects.toMatchObject({
      kind: "decode",
    } as Partial<HttpError>);
  });

  it("底层网络错误直接透传", async () => {
    vi.mocked(httpFetchMock).mockRejectedValue(new HttpError("timeout", "timed out"));

    await expect(fetchCurrentTime("UTC")).rejects.toMatchObject({
      kind: "timeout",
    } as Partial<HttpError>);
  });
});

describe("resolveTimeZone", () => {
  it("始终返回非空字符串", () => {
    expect(resolveTimeZone().length).toBeGreaterThan(0);
  });
});
