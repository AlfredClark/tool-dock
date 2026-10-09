import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import FileList from "../../../../../../components/tools/system/renamer/file-list.svelte";
import type { RenamerFileItem } from "../../../../../../components/tools/system/renamer/renamer-types";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

function makeItem(id: string, partial: Partial<RenamerFileItem> = {}): RenamerFileItem {
  return { id, path: `/tmp/${id}`, name: id, newName: id, checked: false, error: null, ...partial };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("重命名文件列表", () => {
  it("空态提示添加", () => {
    render(FileList, {
      props: { items: [], onToggleOne: vi.fn(), onRemoveOne: vi.fn(), onAdd: vi.fn() },
    });

    expect(screen.getByText("Drop files here, or add them below")).not.toBeNull();
  });

  it("行展示原文件名与新文件名", () => {
    render(FileList, {
      props: {
        items: [makeItem("a.png", { newName: "a.png" })],
        onToggleOne: vi.fn(),
        onRemoveOne: vi.fn(),
        onAdd: vi.fn(),
      },
    });

    // 原名与新名各渲染一次，共两处同名文本
    expect(screen.getAllByText("a.png")).toHaveLength(2);
  });

  it("行勾选与移除各自转发", async () => {
    const user = userEvent.setup();
    const onToggleOne = vi.fn();
    const onRemoveOne = vi.fn();
    render(FileList, {
      props: {
        items: [makeItem("a.png"), makeItem("b.png")],
        onToggleOne,
        onRemoveOne,
        onAdd: vi.fn(),
      },
    });

    await user.click(screen.getByRole("checkbox", { name: "b.png" }));
    expect(onToggleOne).toHaveBeenCalledWith("b.png", true);

    await user.click(screen.getAllByRole("button", { name: "Remove" })[0]);
    expect(onRemoveOne).toHaveBeenCalledWith("a.png");
  });

  it("空态添加按钮触发导入", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(FileList, {
      props: { items: [], onToggleOne: vi.fn(), onRemoveOne: vi.fn(), onAdd },
    });

    await user.click(screen.getByRole("button", { name: "Add files" }));
    expect(onAdd).toHaveBeenCalledTimes(1);
  });
});
