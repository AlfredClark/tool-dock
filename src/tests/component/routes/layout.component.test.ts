import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Harness from "../stubs/root-layout-harness.svelte";

// 根布局回归：外观初始化只跑一次，未落盘的预览不得被重初始化覆盖。
// 背景：initAppearance 读写同一批外观状态，若被包进跟踪型 effect，
// 每次改动都会触发“从磁盘全量重读”，字重拖拽预览会被存量打回导致滑块冻住。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}
if (typeof window.matchMedia === "undefined") {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

vi.mock("mode-watcher", async (importOriginal) => {
  const Empty = (await import("../stubs/empty.svelte")).default;
  return {
    ...(await importOriginal<typeof import("mode-watcher")>()),
    ModeWatcher: Empty,
    userPrefersMode: { current: "system" },
    setMode: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ onCloseRequested: async () => async () => {} }),
}));

vi.mock("$hooks/config.svelte", () => ({
  configState: { value: null },
  hydrateConfig: vi.fn(async () => {}),
  hydrateAndAlignLocale: vi.fn(async () => {}),
}));

vi.mock("$hooks/deep-link.svelte", () => ({
  initDeepLinks: vi.fn(async () => async () => {}),
}));

vi.mock("$hooks/updater.svelte", () => ({
  maybeAutoCheckForUpdate: vi.fn(),
}));

vi.mock("$libs/commands", () => ({
  default: { quitApp: () => ({ failed: () => {} }) },
}));

vi.mock("$libs/commands/cores", () => ({
  reportCommandFailure: vi.fn(),
}));

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
});

describe("根布局外观对齐", () => {
  it("首帧按存量对齐字重", async () => {
    localStorage.setItem("font-weight", "700");
    render(Harness);

    expect(screen.getByTestId("weight").textContent).toBe("700");
    expect(document.documentElement.style.getPropertyValue("--app-font-weight")).toBe("700");
  });

  it("未落盘的字重预览不被重初始化覆盖", async () => {
    const user = userEvent.setup();
    render(Harness);
    expect(screen.getByTestId("weight").textContent).toBe("400");

    await user.click(screen.getByRole("button", { name: "preview-500" }));
    expect(screen.getByTestId("weight").textContent).toBe("500");
    expect(document.documentElement.style.getPropertyValue("--app-font-weight")).toBe("500");
    // 预览语义：内存与样式即时生效，磁盘保持不动
    expect(localStorage.getItem("font-weight")).toBeNull();
    // 等 effect 全部 flush：若初始化被重复订阅，此处会被存量打回 400
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.getByTestId("weight").textContent).toBe("500");
    expect(document.documentElement.style.getPropertyValue("--app-font-weight")).toBe("500");
  });
});
