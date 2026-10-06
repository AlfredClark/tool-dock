import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import type { TimeInfo } from "$libs/http/types";
import Component from "$components/widget/demo/demo-http-section.svelte";

// 纯替身：时间接口、提示全部 mock，只验证取数接线——点击取数、成功展示、失败提示。
const fetchCurrentTimeMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));

vi.mock("$libs/http/time", () => ({
  fetchCurrentTime: fetchCurrentTimeMock,
  resolveTimeZone: () => "Asia/Shanghai",
}));

vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));

const current: TimeInfo = {
  dateTime: "2026-09-26T13:06:48",
  date: "09/26/2026",
  time: "13:06",
  timeZone: "Asia/Shanghai",
  dayOfWeek: "Saturday",
};

beforeEach(() => {
  vi.clearAllMocks();
  fetchCurrentTimeMock.mockResolvedValue({ ...current });
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("网络请求分组", () => {
  it("渲染标题与获取按钮，取数前无结果行", () => {
    render(Component);

    expect(screen.getByText("HTTP request")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Fetch time" })).not.toBeNull();
    expect(screen.queryByText("09/26/2026 13:06")).toBeNull();
  });

  it("点击后取数并展示时间", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Fetch time" }));

    expect(fetchCurrentTimeMock).toHaveBeenCalledOnce();
    expect(await screen.findByText("09/26/2026 13:06")).not.toBeNull();
    expect(screen.getByText("Saturday")).not.toBeNull();
  });

  it("失败时提示且保留旧结果", async () => {
    fetchCurrentTimeMock.mockRejectedValueOnce(new Error("offline"));
    const user = userEvent.setup();
    render(Component);

    await user.click(screen.getByRole("button", { name: "Fetch time" }));

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalledOnce();
    });
    expect(screen.queryByText("09/26/2026 13:06")).toBeNull();
  });
});
