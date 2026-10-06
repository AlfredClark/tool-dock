import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  isShortcutRegistered,
  registerShortcut,
  unregisterShortcut,
} from "$libs/shortcuts/shortcuts";

// 薄封装只验证转交：参数原样透传，handler 只收 Pressed 事件。
const registerMock = vi.fn();
const unregisterMock = vi.fn();
const isRegisteredMock = vi.fn();

vi.mock("@tauri-apps/plugin-global-shortcut", () => ({
  register: registerMock,
  unregister: unregisterMock,
  isRegistered: isRegisteredMock,
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("快捷键胶水", () => {
  it("注册透传组合键与处理器", async () => {
    await registerShortcut("Ctrl+Alt+K", () => {});

    expect(registerMock).toHaveBeenCalledOnce();
    expect(registerMock.mock.calls[0][0]).toBe("Ctrl+Alt+K");
    expect(typeof registerMock.mock.calls[0][1]).toBe("function");
  });

  it("处理器只收 Pressed 事件", async () => {
    const seen: string[] = [];
    await registerShortcut("Ctrl+Alt+K", (event) => {
      seen.push(event.state);
    });
    const handler = registerMock.mock.calls[0][1] as (event: {
      state: "Pressed" | "Released";
      shortcut: string;
      id: number;
    }) => void;

    handler({ state: "Pressed", shortcut: "Ctrl+Alt+K", id: 1 });
    handler({ state: "Released", shortcut: "Ctrl+Alt+K", id: 1 });

    // Released 在薄层过滤，回调只见 Pressed
    expect(seen).toEqual(["Pressed"]);
  });

  it("注销透传组合键", async () => {
    await unregisterShortcut("Ctrl+Alt+K");

    expect(unregisterMock).toHaveBeenCalledWith("Ctrl+Alt+K");
  });

  it("查询返回后端结果", async () => {
    isRegisteredMock.mockResolvedValue(true);

    expect(await isShortcutRegistered("Ctrl+Alt+K")).toBe(true);
  });
});
