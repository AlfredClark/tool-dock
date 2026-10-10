import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import RuleToolbar from "../../../../../../components/tools/system/renamer/rule-toolbar.svelte";
import { RULE_KINDS } from "../../../../../../components/tools/system/renamer/renamer-rules";

// bits-ui 在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
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

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
  // bits-ui 下拉打开时给 body 加滚动锁定样式，卸载后手动清理，避免泄漏到后续用例
  document.body.removeAttribute("style");
});

describe("规则工具栏", () => {
  it("无规则时隐藏清空按钮，保留添加按钮", () => {
    render(RuleToolbar, { props: { ruleCount: 0, onClear: vi.fn(), onAdd: vi.fn() } });

    expect(screen.queryByRole("button", { name: "Clear rules" })).toBeNull();
    expect(screen.getByRole("button", { name: "Add rule" })).not.toBeNull();
  });

  it("有规则时清空按钮触发回调", async () => {
    const user = userEvent.setup();
    const onClear = vi.fn();
    render(RuleToolbar, { props: { ruleCount: 2, onClear, onAdd: vi.fn() } });

    await user.click(screen.getByRole("button", { name: "Clear rules" }));
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("添加下拉框列出全部六种规则类型", async () => {
    const user = userEvent.setup();
    render(RuleToolbar, {
      props: { ruleCount: 0, onClear: vi.fn(), onAdd: vi.fn() },
    });

    await user.click(screen.getByRole("button", { name: "Add rule" }));
    // jsdom 下 floating 定位失效、portal 保持隐藏，角色查询不可见；
    // 菜单挂载于 document.body，直接数 DOM 节点
    expect(document.querySelectorAll('[data-slot="dropdown-menu-item"]')).toHaveLength(
      RULE_KINDS.length,
    );
  });

  it("菜单项点击透出对应类型", async () => {
    const user = userEvent.setup();
    const onAdd = vi.fn();
    render(RuleToolbar, {
      props: { ruleCount: 0, onClear: vi.fn(), onAdd },
    });

    await user.click(screen.getByRole("button", { name: "Add rule" }));
    const target = [...document.querySelectorAll('[data-slot="dropdown-menu-item"]')].find((item) =>
      item.textContent?.includes("Slice"),
    );
    if (!target) throw new Error("slice menu item missing");
    await fireEvent.click(target);
    expect(onAdd).toHaveBeenCalledWith("slice");
  });
});
