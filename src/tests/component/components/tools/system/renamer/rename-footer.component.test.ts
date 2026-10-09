import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import RenameFooter from "../../../../../../components/tools/system/renamer/rename-footer.svelte";

// bits-ui 在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
if (typeof ResizeObserver === "undefined") {
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
}

function renderFooter(overrides: Record<string, unknown> = {}) {
  const onStart = vi.fn();
  render(RenameFooter, {
    props: {
      executableCount: 2,
      processing: false,
      startDisabled: false,
      startTitle: null,
      summary: null,
      onStart,
      ...overrides,
    },
  });
  return { onStart };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("改名执行条", () => {
  it("开始按钮带可执行计数，点击透出", async () => {
    const user = userEvent.setup();
    const { onStart } = renderFooter({ executableCount: 3 });

    const button = screen.getByRole("button", { name: "Rename · 3" });
    expect(button.hasAttribute("disabled")).toBe(false);
    await user.click(button);
    expect(onStart).toHaveBeenCalledTimes(1);
  });

  it("处理中锁按钮并切换文案", () => {
    renderFooter({ processing: true, startDisabled: true });

    const button = screen.getByRole("button", { name: "Renaming…" });
    expect(button.hasAttribute("disabled")).toBe(true);
  });

  it("非桌面端禁用并悬浮说明", () => {
    renderFooter({ startDisabled: true, startTitle: "Desktop only" });

    const button = screen.getByRole("button", { name: "Rename · 2" });
    expect(button.hasAttribute("disabled")).toBe(true);
    expect(button.getAttribute("title")).toBe("Desktop only");
  });

  it("汇总文本常驻", () => {
    renderFooter({ summary: "Done: 1 renamed, 0 skipped, 0 failed" });

    expect(screen.getByText("Done: 1 renamed, 0 skipped, 0 failed")).not.toBeNull();
  });
});
