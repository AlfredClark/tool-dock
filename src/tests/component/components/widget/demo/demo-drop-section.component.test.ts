import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import DemoDropSection from "../../../../../components/widget/demo/demo-drop-section.svelte";

// jsdom 无 matchMedia，移动端断点就地给假，默认桌面宽度不断点（卡片正常渲染）
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

const inspectDropMock = vi.hoisted(() => vi.fn());
const importDropMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));
const dragHandlers = vi.hoisted(() => [] as Array<(event: { payload: unknown }) => void>);

vi.mock("$libs/commands", () => ({
  default: {
    demoInspectDrop: inspectDropMock,
    demoImportDrop: importDropMock,
  },
}));

vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));

// 拖放订阅桩：收下 handler 供用例直接投递 `drop` 事件
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onDragDropEvent: (handler: (event: { payload: unknown }) => void) => {
      dragHandlers.push(handler);
      return () => {};
    },
  }),
}));

type FakeResult =
  { status: "ok"; data: unknown } | { status: "error"; error: { kind: string; message: string } };

/** 与 `EnhancedCommand` 行为一致的替身：按分支跑回调后结算结果 */
function fakeCommand(result: FakeResult): {
  success: (handler: (data: never) => unknown) => unknown;
  failed: (handler: (failure: never) => unknown) => unknown;
  result: () => Promise<FakeResult>;
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
    result() {
      return Promise.resolve(result);
    },
  };
  return chain;
}

/** 向订阅投递一次 `drop` 事件（等待异步订阅就绪） */
async function emitDrop(paths: string[]): Promise<void> {
  await vi.waitFor(() => {
    expect(dragHandlers.length).toBeGreaterThan(0);
  });
  dragHandlers[dragHandlers.length - 1]({ payload: { type: "drop", paths } });
}

beforeEach(() => {
  vi.clearAllMocks();
  dragHandlers.length = 0;
  inspectDropMock.mockReturnValue(fakeCommand({ status: "ok", data: [] }) as never);
  importDropMock.mockReturnValue(fakeCommand({ status: "ok", data: [] }) as never);
});

afterEach(() => {
  cleanup();
});

describe("文件拖放分组", () => {
  it("渲染标题与空闲放置区", async () => {
    render(DemoDropSection);

    expect(screen.getByText("File drop")).not.toBeNull();
    expect(await screen.findByText("Drag files here")).not.toBeNull();
  });

  it("放下文件后回显清单并可存入沙盒", async () => {
    inspectDropMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: [{ name: "a.txt", size: 12, is_dir: false }],
      }) as never,
    );
    const user = userEvent.setup();
    render(DemoDropSection);
    await screen.findByText("Drag files here");

    await emitDrop(["/tmp/a.txt"]);

    expect(await screen.findByText("a.txt")).not.toBeNull();
    expect(screen.getByText("12 bytes")).not.toBeNull();
    expect(inspectDropMock).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: "Save to sandbox" }));

    expect(importDropMock).toHaveBeenCalledOnce();
    expect(toastMocks.success).toHaveBeenCalledOnce();
    expect(toastMocks.error).not.toHaveBeenCalled();
  });

  it("鉴别失败时提示且不回显", async () => {
    inspectDropMock.mockReturnValue(
      fakeCommand({ status: "error", error: { kind: "Internal", message: "boom" } }) as never,
    );
    render(DemoDropSection);
    await screen.findByText("Drag files here");

    await emitDrop(["/tmp/a.txt"]);

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalledOnce();
    });
    expect(screen.queryByText("a.txt")).toBeNull();
  });

  it("存入沙盒失败时提示", async () => {
    inspectDropMock.mockReturnValue(
      fakeCommand({
        status: "ok",
        data: [{ name: "a.txt", size: 12, is_dir: false }],
      }) as never,
    );
    importDropMock.mockReturnValue(
      fakeCommand({ status: "error", error: { kind: "Internal", message: "boom" } }) as never,
    );
    const user = userEvent.setup();
    render(DemoDropSection);
    await screen.findByText("Drag files here");

    await emitDrop(["/tmp/a.txt"]);
    await screen.findByText("a.txt");
    await user.click(screen.getByRole("button", { name: "Save to sandbox" }));

    await vi.waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalledOnce();
    });
    expect(toastMocks.success).not.toHaveBeenCalled();
  });
});
