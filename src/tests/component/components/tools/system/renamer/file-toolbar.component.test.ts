import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import FileToolbar from "../../../../../../components/tools/system/renamer/file-toolbar.svelte";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
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

function renderToolbar(
  overrides: Record<string, unknown> = {},
): Record<string, ReturnType<typeof vi.fn>> {
  const callbacks = {
    onToggleAll: vi.fn(),
    onQueryChange: vi.fn(),
    onSortChange: vi.fn(),
    onRemoveSelected: vi.fn(),
  };
  render(FileToolbar, {
    props: {
      headerChecked: false,
      headerDisabled: false,
      query: "",
      sortKey: "name-asc",
      selectedCount: 0,
      ...callbacks,
      ...overrides,
    },
  });
  return callbacks;
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("文件工具栏", () => {
  it("无勾选时不渲染删除按钮", () => {
    renderToolbar({ selectedCount: 0 });

    expect(screen.queryByRole("button", { name: "Remove selected" })).toBeNull();
    expect(screen.getByPlaceholderText("Filter by name…")).not.toBeNull();
  });

  it("有勾选时渲染删除按钮并透出计数", async () => {
    const user = userEvent.setup();
    const { onRemoveSelected } = renderToolbar({ selectedCount: 2 });

    await user.click(screen.getByRole("button", { name: "Remove selected" }));
    expect(onRemoveSelected).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Remove selected · 2")).not.toBeNull();
  });

  it("全选复选框转发切换", async () => {
    const user = userEvent.setup();
    const { onToggleAll } = renderToolbar({});

    await user.click(screen.getByRole("checkbox"));
    expect(onToggleAll).toHaveBeenCalledWith(true);
  });

  it("搜索输入转发文本变更", async () => {
    const user = userEvent.setup();
    const { onQueryChange } = renderToolbar({});

    await user.type(screen.getByPlaceholderText("Filter by name…"), "pic");
    expect(onQueryChange).toHaveBeenLastCalledWith("pic");
  });

  it("排序触发器显示当前选项文案", () => {
    // jsdom 下 floating 定位失效、portal 保持隐藏，bits-ui 下拉交互不可测（全仓一致）；
    // 此处只断言触发器按 value 解析出文案（需传 items，否则显示裸 key）。
    renderToolbar({ sortKey: "name-desc" });

    expect(screen.getByRole("button", { name: "Sort" }).textContent).toContain("Name Z→A");
  });
});
