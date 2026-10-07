import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { createRawSnippet } from "svelte";
import ToolsLayout from "../../../../routes/(tools)/+layout.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同布局容器测试）。
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

// 标题栏窗口操作在单测无 Tauri 运行时，全部按不可用回落
vi.mock("$libs/utils/window-controls", () => ({
  isWindowControlsAvailable: vi.fn().mockResolvedValue(false),
  isWindowAlwaysOnTop: vi.fn().mockResolvedValue(false),
  toggleAlwaysOnTopWindow: vi.fn().mockResolvedValue(false),
  minimizeWindow: vi.fn().mockResolvedValue(false),
  toggleMaximizeWindow: vi.fn().mockResolvedValue(false),
  closeWindow: vi.fn().mockResolvedValue(false),
  isWindowMaximized: vi.fn().mockResolvedValue(false),
  onWindowResized: vi.fn().mockResolvedValue(null),
}));

// 工具详情路由状态就地提供最小桩：路径命中注册表的数据互转工具
vi.mock("$app/state", () => ({
  page: { url: new URL("http://localhost/text/convert") },
}));

const gotoMock = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({
  goto: gotoMock,
}));

vi.mock("$app/paths", () => ({
  resolve: (path: string): string => path,
}));

function renderWithProbe(): void {
  const children = createRawSnippet(() => ({
    render: () => "<span>probe-content</span>",
  }));
  render(ToolsLayout, { props: { children } });
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("工具分组布局", () => {
  it("标题栏居中显示注册表工具名并渲染子内容", () => {
    renderWithProbe();

    // 节点环境无 window，Paraglide 回落 baseLocale，取到英文工具名
    expect(screen.getByText("Data converter")).not.toBeNull();
    expect(screen.getByText("probe-content")).not.toBeNull();
  });

  it("返回按钮跳工具列表而非 history.back", async () => {
    const user = userEvent.setup();
    renderWithProbe();

    // 深链冷启动无历史栈，返回必须走确定性路由跳转
    await user.click(screen.getByRole("button", { name: "Back to tools" }));
    expect(gotoMock).toHaveBeenCalledWith("/tools");
  });
});
