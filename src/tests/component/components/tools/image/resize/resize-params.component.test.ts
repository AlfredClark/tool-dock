import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ResizeParams from "../../../../../../components/tools/image/resize/resize-params.svelte";
import { DEFAULT_RESIZE_PARAMS } from "../../../../../../components/tools/image/resize/resize-types";
import type { ResizeParamsState } from "../../../../../../components/tools/image/resize/resize-types";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

function renderParams(
  partial: Partial<ResizeParamsState> = {},
  extra: Record<string, unknown> = {},
) {
  let current: ResizeParamsState = { ...DEFAULT_RESIZE_PARAMS, ...partial };
  const onParamsChange = vi.fn((next: ResizeParamsState) => {
    current = next;
  });
  const result = render(ResizeParams, {
    props: {
      params: current,
      onParamsChange,
      onChooseOutputDir: vi.fn(),
      canStart: false,
      processing: false,
      progress: null,
      startLabel: "Start",
      onStart: vi.fn(),
      hasGif: false,
      referenceSize: null,
      ...extra,
    },
  });
  return {
    onParamsChange,
    get current() {
      return current;
    },
    ...result,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("调整参数", () => {
  it("默认宽高模式 + 质量行可见（原格式附注）", () => {
    renderParams();

    expect(screen.getByLabelText("Width")).not.toBeNull();
    expect(screen.getByLabelText("Height")).not.toBeNull();
    expect(screen.getByText("85 · JPEG only")).not.toBeNull();
  });

  it("切换百分比页签回调用整体替换", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams();

    await user.click(screen.getByRole("tab", { name: "Percent" }));
    expect(onParamsChange).toHaveBeenCalledWith(expect.objectContaining({ modeKind: "percent" }));
  });

  it("处理中禁用开始按钮并展示进度", () => {
    renderParams(
      {},
      {
        processing: true,
        canStart: false,
        progress: { done: 1, total: 4 },
        startLabel: "Processing 1/4",
      },
    );

    expect(screen.getByRole("button", { name: "Processing 1/4" }).hasAttribute("disabled")).toBe(
      true,
    );
  });

  it("GIF 提示仅含 GIF 时展示", () => {
    const { unmount } = renderParams({}, { hasGif: true });
    expect(screen.getByText("GIF uses the first frame and outputs PNG")).not.toBeNull();
    unmount();
    renderParams({}, { hasGif: false });
    expect(screen.queryByText("GIF uses the first frame and outputs PNG")).toBeNull();
  });

  it("目标大小与质量滑块互斥切换", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams();

    // 默认质量模式：点附注链接切到目标模式（回调整体替换）
    await user.click(screen.getByRole("button", { name: "Target size →" }));
    expect(onParamsChange).toHaveBeenCalledWith(expect.objectContaining({ targetSizeKb: "200" }));
  });

  it("目标模式显示输入框并隐藏质量滑块", () => {
    renderParams({ targetSizeKb: "200" });

    expect(screen.getByLabelText("Target size")).not.toBeNull();
    expect(screen.queryByText("85 · JPEG only")).toBeNull();
  });

  it("锁定比例时参考尺寸联动另一边提示", () => {
    renderParams(
      { boxWidth: "50", boxHeight: "", lockRatio: true },
      { referenceSize: { width: 200, height: 100 } },
    );

    // 200x100 按宽 50 换算高 25，进 placeholder
    expect((screen.getByLabelText("Height") as HTMLInputElement).placeholder).toBe("25");
  });

  it("精确模式 placeholder 跟随长宽比", () => {
    renderParams({ modeKind: "exact", exactRatio: "16:9" });

    expect((screen.getByLabelText("Width") as HTMLInputElement).placeholder).toBe("1280");
    expect((screen.getByLabelText("Height") as HTMLInputElement).placeholder).toBe("720");
  });

  it("自由比例 placeholder 回落默认", () => {
    renderParams({ modeKind: "exact", exactRatio: "free" });

    expect((screen.getByLabelText("Width") as HTMLInputElement).placeholder).toBe("800");
    expect((screen.getByLabelText("Height") as HTMLInputElement).placeholder).toBe("600");
  });

  it("精确模式输宽按长宽比联动高", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams({ modeKind: "exact", exactRatio: "16:9" });

    // 逐字输入 160：最后一次回调带换算高 90
    await user.type(screen.getByLabelText("Width"), "160");
    const last = onParamsChange.mock.calls.at(-1)?.[0];
    expect(last).toEqual(expect.objectContaining({ exactWidth: "160", exactHeight: "90" }));
  });

  it("分组标题与新下拉渲染", () => {
    renderParams();

    expect(screen.getByText("Size")).not.toBeNull();
    expect(screen.getByText("Output")).not.toBeNull();
    expect(screen.getByText("Files")).not.toBeNull();
  });
});
