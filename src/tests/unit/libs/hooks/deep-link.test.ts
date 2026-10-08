import { beforeEach, describe, expect, it, vi } from "vitest";
import { handleDeepLinkUrl, isDeepLink, routeForUrl } from "$hooks/deep-link.svelte";

const gotoMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({
  info: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("$app/navigation", () => ({ goto: gotoMock }));
vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("isDeepLink", () => {
  it("仅接受本应用协议", () => {
    expect(isDeepLink("tool-dock://settings")).toBe(true);
    expect(isDeepLink("tool-dock://")).toBe(true);
    expect(isDeepLink("https://example.com")).toBe(false);
    expect(isDeepLink("other://settings")).toBe(false);
    expect(isDeepLink("")).toBe(false);
  });
});

describe("routeForUrl", () => {
  it("映射已知路径到应用内路由", () => {
    expect(routeForUrl("tool-dock://settings")).toBe("/settings");
    expect(routeForUrl("tool-dock:///settings")).toBe("/settings");
    expect(routeForUrl("tool-dock://about")).toBe("/about");
    expect(routeForUrl("tool-dock://about?tab=1")).toBe("/about");
    expect(routeForUrl("tool-dock://")).toBe("/");
    expect(routeForUrl("tool-dock:///")).toBe("/");
  });

  it("映射工具列表与已登记的工具详情页", () => {
    expect(routeForUrl("tool-dock://tools")).toBe("/tools");
    expect(routeForUrl("tool-dock:///tools")).toBe("/tools");
    expect(routeForUrl("tool-dock://text/convert")).toBe("/text/convert");
    expect(routeForUrl("tool-dock:///text/convert?from=home")).toBe("/text/convert");
    expect(routeForUrl("tool-dock://image/resize")).toBe("/image/resize");
    expect(routeForUrl("tool-dock:///image/resize?from=home")).toBe("/image/resize");
    expect(routeForUrl("tool-dock://video/metadata")).toBe("/video/metadata");
    expect(routeForUrl("tool-dock:///video/metadata?from=home")).toBe("/video/metadata");
    expect(routeForUrl("tool-dock://video/convert")).toBe("/video/convert");
    expect(routeForUrl("tool-dock:///video/convert?from=home")).toBe("/video/convert");
  });

  it("未知路径与非本协议回落空", () => {
    expect(routeForUrl("tool-dock://unknown")).toBeNull();
    expect(routeForUrl("tool-dock://tools/unknown")).toBeNull();
    expect(routeForUrl("tool-dock://text/demo")).toBeNull();
    expect(routeForUrl("tool-dock://image/unknown")).toBeNull();
    expect(routeForUrl("tool-dock://video/unknown")).toBeNull();
    expect(routeForUrl("https://example.com")).toBeNull();
  });
});

describe("handleDeepLinkUrl", () => {
  it("命中跳转并提示", () => {
    handleDeepLinkUrl("tool-dock://settings");

    expect(gotoMock).toHaveBeenCalledWith("/settings");
    expect(toastMocks.info).toHaveBeenCalled();
  });

  it("未知路径停留并警告", () => {
    handleDeepLinkUrl("tool-dock://nope");

    expect(gotoMock).not.toHaveBeenCalled();
    expect(toastMocks.warning).toHaveBeenCalled();
  });

  it("短窗内重复到达只处理一次", () => {
    handleDeepLinkUrl("tool-dock://about");
    handleDeepLinkUrl("tool-dock://about");

    expect(gotoMock).toHaveBeenCalledTimes(1);
  });

  it("去重表有界：同窗突发超限淘汰最旧", () => {
    // query 参与去重键、剥离后同路由，保证走跳转分支而非未知分支
    vi.useFakeTimers();
    try {
      vi.setSystemTime(1_000_000);
      handleDeepLinkUrl("tool-dock://about?victim=1");
      for (let index = 0; index < 150; index += 1) {
        handleDeepLinkUrl(`tool-dock://about?burst=${index}`);
      }
      // 受害者早被挤出，同窗重到会再次处理
      const before = gotoMock.mock.calls.length;
      handleDeepLinkUrl("tool-dock://about?victim=1");
      expect(gotoMock.mock.calls.length).toBe(before + 1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("淘汰只丢最旧：窗内新条目去重语义不变", () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(1_000_000);
      for (let index = 0; index < 110; index += 1) {
        handleDeepLinkUrl(`tool-dock://about?fill=${index}`);
      }
      handleDeepLinkUrl("tool-dock://about?fresh=1");
      const before = gotoMock.mock.calls.length;
      // fresh 未被淘汰（淘汰的是最旧的 filler），重复到达仍忽略
      handleDeepLinkUrl("tool-dock://about?fresh=1");
      expect(gotoMock.mock.calls.length).toBe(before);
      // 已淘汰的最旧 filler 重到会再次处理
      handleDeepLinkUrl("tool-dock://about?fill=0");
      expect(gotoMock.mock.calls.length).toBe(before + 1);
    } finally {
      vi.useRealTimers();
    }
  });
});
