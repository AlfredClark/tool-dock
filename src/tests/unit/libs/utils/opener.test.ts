import { afterEach, describe, expect, it, vi } from "vitest";
import { openExternal } from "$libs/utils/opener";

describe("openExternal", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("非 Tauri 环境返回假，调用方据此提示", async () => {
    await expect(openExternal("https://example.com")).resolves.toBe(false);
  });

  it("git+ 前缀剥离后按 https 处理", async () => {
    // 非 Tauri 环境动态导入失败，断言点在于前缀剥离不触发 scheme 拒绝上报
    vi.spyOn(console, "error").mockImplementation(() => {});
    await expect(openExternal("git+https://github.com/example/repo")).resolves.toBe(false);
    expect(console.error).not.toHaveBeenCalledWith(
      expect.stringContaining("[opener] unsupported url scheme"),
    );
  });

  it("非 http(s) scheme 直接拒绝并上报", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    await expect(openExternal("mailto:someone@example.com")).resolves.toBe(false);
    await expect(openExternal("file:///etc/passwd")).resolves.toBe(false);
    expect(console.error).toHaveBeenCalledWith(
      expect.stringContaining("[opener] unsupported url scheme"),
    );
  });
});
