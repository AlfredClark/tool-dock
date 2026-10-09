import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ImageList from "../../../../../../components/tools/image/resize/image-list.svelte";
import type { ResizeImageItem } from "../../../../../../components/tools/image/resize/resize-types";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

function makeItem(partial: Partial<ResizeImageItem> & { id: string }): ResizeImageItem {
  return {
    path: `/tmp/${partial.id}.png`,
    name: `${partial.id}.png`,
    previewUrl: "data:,",
    detailUrl: "",
    detailLoading: false,
    detailFailed: false,
    revokePreview: false,
    info: { width: 64, height: 48, format: "png", file_size: 1024 },
    errorDetail: null,
    status: "ready",
    output: null,
    outputSize: null,
    targetMet: null,
    ...partial,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("图片列表", () => {
  it("空态提示添加", () => {
    render(ImageList, {
      props: {
        items: [],
        selectedId: null,
        onSelect: vi.fn(),
        onRemove: vi.fn(),
        onAdd: vi.fn(),
        onClear: vi.fn(),
        summary: null,
        processing: false,
        onRetry: vi.fn(),
      },
    });

    expect(screen.getByText("Drop images here, or add them with the button below")).not.toBeNull();
  });

  it("点击行选中，点击移除按钮只移除不选中", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const onRemove = vi.fn();
    const items = [makeItem({ id: "a" }), makeItem({ id: "b", status: "invalid", info: null })];
    render(ImageList, {
      props: {
        items,
        selectedId: "a",
        onSelect,
        onRemove,
        onAdd: vi.fn(),
        onClear: vi.fn(),
        summary: null,
        processing: false,
        onRetry: vi.fn(),
      },
    });

    // 选中行高亮：不可读行显示徽章
    expect(screen.getByText("Unreadable")).not.toBeNull();
    expect(screen.getByText("64×48 · PNG · 1.0 KB")).not.toBeNull();

    await user.click(screen.getByText("b.png"));
    expect(onSelect).toHaveBeenCalledWith("b");

    await user.click(screen.getAllByRole("button", { name: "Remove" })[0]);
    expect(onRemove).toHaveBeenCalledWith("a");
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("添加与清空回调", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    const onClear = vi.fn();
    render(ImageList, {
      props: {
        items: [makeItem({ id: "a" })],
        selectedId: null,
        onSelect: vi.fn(),
        onRemove: vi.fn(),
        onAdd,
        onClear,
        summary: null,
        processing: false,
        onRetry: vi.fn(),
      },
    });

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(onAdd).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Images · 1 images")).not.toBeNull();
  });

  it("汇总条常驻结果，失败时可重试", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    render(ImageList, {
      props: {
        items: [makeItem({ id: "a", status: "failed", errorDetail: "DecodeFailed" })],
        selectedId: null,
        onSelect: vi.fn(),
        onRemove: vi.fn(),
        onAdd: vi.fn(),
        onClear: vi.fn(),
        summary: { ok: 0, failed: 1, skipped: 0 },
        processing: false,
        onRetry,
      },
    });

    expect(screen.getByText("Done: 0 succeeded, 1 failed, 0 skipped")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Retry failed" }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("处理中禁用重试按钮", () => {
    render(ImageList, {
      props: {
        items: [makeItem({ id: "a" })],
        selectedId: null,
        onSelect: vi.fn(),
        onRemove: vi.fn(),
        onAdd: vi.fn(),
        onClear: vi.fn(),
        summary: { ok: 1, failed: 1, skipped: 0 },
        processing: true,
        onRetry: vi.fn(),
      },
    });

    expect(screen.getByRole("button", { name: "Retry failed" }).hasAttribute("disabled")).toBe(
      true,
    );
  });

  it("处理中锁定添加/清空/单项移除，选中不受影响", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const onRemove = vi.fn();
    const onAdd = vi.fn();
    const onClear = vi.fn();
    render(ImageList, {
      props: {
        items: [makeItem({ id: "a" })],
        selectedId: null,
        onSelect,
        onRemove,
        onAdd,
        onClear,
        summary: null,
        processing: true,
        onRetry: vi.fn(),
      },
    });

    expect(screen.getByRole("button", { name: "Add images" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Clear" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Remove" }).hasAttribute("disabled")).toBe(true);

    // 禁用态下点击无回调；选中浏览仍可用
    await user.click(screen.getByText("a.png"));
    expect(onSelect).toHaveBeenCalledWith("a");
    expect(onAdd).not.toHaveBeenCalled();
    expect(onClear).not.toHaveBeenCalled();
    expect(onRemove).not.toHaveBeenCalled();
  });
});
