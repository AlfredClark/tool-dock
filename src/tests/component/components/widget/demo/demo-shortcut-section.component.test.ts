import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Component from "$components/widget/demo/demo-shortcut-section.svelte";

// 纯替身：后端固定键命令、前端直调胶水、窗口关闭、提示全部 mock，
// 只验证快捷键卡片的接线——自定义槽位校验拦截、注册透传、关闭键切换。
const demoShortcutKeyMock = vi.hoisted(() => vi.fn());
const demoShortcutIsRegisteredMock = vi.hoisted(() => vi.fn());
const demoShortcutRegisterMock = vi.hoisted(() => vi.fn());
const demoShortcutUnregisterMock = vi.hoisted(() => vi.fn());
const registerShortcutMock = vi.hoisted(() => vi.fn());
const unregisterShortcutMock = vi.hoisted(() => vi.fn());
const isShortcutRegisteredMock = vi.hoisted(() => vi.fn());
const closeWindowMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：按分支跑回调后返回自身 */
function fakeCommand(result: FakeResult): {
  success: (handler: (data: never) => unknown) => unknown;
  failed: (handler: (failure: never) => unknown) => unknown;
} {
  const chain = {
    success(handler: (data: never) => unknown) {
      if (result.status === "ok") handler(result.data as never);
      return chain;
    },
    failed(handler: (failure: never) => unknown) {
      if (result.status === "error") handler(result.error as never);
      return chain;
    },
  };
  return chain;
}

vi.mock("$libs/commands", () => ({
  default: {
    demoShortcutKey: demoShortcutKeyMock,
    demoShortcutIsRegistered: demoShortcutIsRegisteredMock,
    demoShortcutRegister: demoShortcutRegisterMock,
    demoShortcutUnregister: demoShortcutUnregisterMock,
  },
}));

vi.mock("$libs/shortcuts/shortcuts", () => ({
  CLOSE_WINDOW_SHORTCUT: "Ctrl+Alt+X",
  registerShortcut: registerShortcutMock,
  unregisterShortcut: unregisterShortcutMock,
  isShortcutRegistered: isShortcutRegisteredMock,
}));

vi.mock("$libs/utils/window-controls", () => ({
  closeWindow: closeWindowMock,
}));

vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));

beforeEach(() => {
  vi.clearAllMocks();
  demoShortcutKeyMock.mockReturnValue(fakeCommand({ status: "ok", data: "Ctrl+Shift+D" }) as never);
  demoShortcutIsRegisteredMock.mockReturnValue(fakeCommand({ status: "ok", data: false }) as never);
  isShortcutRegisteredMock.mockResolvedValue(false);
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("全局快捷键分组", () => {
  it("渲染固定键、自定义槽位与关闭窗口三行", async () => {
    render(Component);

    expect(await screen.findByText("Ctrl+Shift+D")).not.toBeNull();
    expect(screen.getByPlaceholderText("e.g. Ctrl+Alt+K")).not.toBeNull();
    expect(screen.getByText("Ctrl+Alt+X")).not.toBeNull();
  });

  it("非法输入不调插件只提示", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.type(screen.getByPlaceholderText("e.g. Ctrl+Alt+K"), "A");
    // 注册按钮有两个（固定键行与自定义行），取第二个即自定义槽位
    await user.click(screen.getAllByRole("button", { name: "Register" })[1]);

    expect(registerShortcutMock).not.toHaveBeenCalled();
    expect(toastMocks.error).toHaveBeenCalledOnce();
  });

  it("合法输入归一后注册插件", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.type(screen.getByPlaceholderText("e.g. Ctrl+Alt+K"), "ctrl+alt+k");
    await user.click(screen.getAllByRole("button", { name: "Register" })[1]);

    expect(registerShortcutMock).toHaveBeenCalledOnce();
    expect(registerShortcutMock.mock.calls[0][0]).toBe("Ctrl+Alt+k");
    expect(toastMocks.success).toHaveBeenCalled();
  });

  it("保留组合被拒绝", async () => {
    const user = userEvent.setup();
    render(Component);

    await user.type(screen.getByPlaceholderText("e.g. Ctrl+Alt+K"), "Alt+F4");
    await user.click(screen.getAllByRole("button", { name: "Register" })[1]);

    expect(registerShortcutMock).not.toHaveBeenCalled();
    expect(toastMocks.error).toHaveBeenCalledOnce();
  });

  it("关闭窗口键切换注册与注销", async () => {
    const user = userEvent.setup();
    render(Component);

    // 三个 Register 按钮按 DOM 序：固定键行、自定义行、关闭行，取第三个
    await user.click(screen.getAllByRole("button", { name: "Register" })[2]);
    expect(registerShortcutMock).toHaveBeenCalledWith("Ctrl+Alt+X", expect.any(Function));
  });
});
