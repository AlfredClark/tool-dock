import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import RuleCard from "../../../../../../components/tools/system/renamer/rule-card.svelte";
import {
  RULE_KINDS,
  createRule,
} from "../../../../../../components/tools/system/renamer/renamer-rules";
import type { RenamerRule } from "../../../../../../components/tools/system/renamer/renamer-types";

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

const KIND_LABELS: Record<RenamerRule["kind"], string> = {
  affix: "Add affix",
  strip: "Strip affix",
  case: "Change case",
  replace: "Find & replace",
  regex: "Regex replace",
  number: "Auto number",
  normalize: "Normalize",
};

function renderCard(rule: RenamerRule, overrides: Record<string, unknown> = {}) {
  const callbacks = {
    onRuleChange: vi.fn(),
    onRemove: vi.fn(),
    onHandleDragStart: vi.fn(),
    onCardDragOver: vi.fn(),
    onCardDrop: vi.fn(),
    onDragEnd: vi.fn(),
  };
  render(RuleCard, {
    props: {
      rule,
      index: 1,
      dropBefore: false,
      dropAfter: false,
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
  document.body.removeAttribute("style");
});

describe("规则参数卡", () => {
  it.each(RULE_KINDS.map((kind) => [kind] as const))("渲染 %s 类型名与序号", (kind) => {
    renderCard(createRule(kind));

    expect(screen.getByText(KIND_LABELS[kind])).not.toBeNull();
    expect(screen.getByText("#2")).not.toBeNull();
  });

  it("前后缀文本输入整体回写", async () => {
    const user = userEvent.setup();
    const { onRuleChange } = renderCard(createRule("affix"));

    const [textInput] = screen.getAllByRole("textbox");
    if (!textInput) throw new Error("affix text input missing");
    await user.type(textInput, "_v2");
    expect(onRuleChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ kind: "affix", text: "_v2" }),
    );
  });

  it("启用按钮切换透出", async () => {
    const user = userEvent.setup();
    const { onRuleChange } = renderCard(createRule("replace"));

    await user.click(screen.getByRole("button", { name: "Toggle enabled" }));
    expect(onRuleChange).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "replace", enabled: false }),
    );
  });

  it("正则卡片提示进 placeholder，无独立提示行", () => {
    renderCard(createRule("regex"));

    expect(screen.getByPlaceholderText("e.g. IMG_(\\d+)")).not.toBeNull();
    expect(screen.getByPlaceholderText("e.g. photo-$1 ($1/$<name>/$&)")).not.toBeNull();
    // 旧 hint 行已删除（英文 hint 含 "for full match"）
    expect(screen.queryByText(/for full match/)).toBeNull();
  });

  it("非法正则显示错误徽章", () => {
    renderCard({ ...createRule("regex"), pattern: "([a-z" });

    // 错误徽章仅图标 + title tooltip（头部空间紧，不占文本行）
    expect(screen.getByTitle("Invalid regex, skipped")).not.toBeNull();
  });

  it("合法参数不显示错误徽章", () => {
    renderCard({ ...createRule("number"), start: "1", step: "1", digits: "3" });

    expect(screen.queryByTitle("Invalid numbering, skipped")).toBeNull();
  });

  it("头部从左到右：拖放手柄、启用、标题、删除", () => {
    renderCard(createRule("case"));

    // 拖放手柄最左、删除最右：比较同行按钮的文档顺序
    const header = screen.getByRole("button", { name: "Drag to reorder" }).parentElement;
    if (!header) throw new Error("card header missing");
    const names = [...header.querySelectorAll("button")].map((button) =>
      button.getAttribute("aria-label"),
    );
    expect(names).toEqual(["Drag to reorder", "Toggle enabled", "Remove rule"]);
  });

  it("拖放手柄可拖，卡片悬停与放置透出规则 id", async () => {
    const rule = createRule("case");
    const callbacks = renderCard(rule);

    const grip = screen.getByRole("button", { name: "Drag to reorder" });
    expect(grip.getAttribute("draggable")).toBe("true");
    await fireEvent.dragStart(grip);
    expect(callbacks.onHandleDragStart).toHaveBeenCalledWith(expect.anything(), rule.id);

    const item = screen.getByRole("listitem");
    await fireEvent.dragOver(item);
    expect(callbacks.onCardDragOver).toHaveBeenCalledWith(expect.anything(), rule.id);
    await fireEvent.drop(item);
    expect(callbacks.onCardDrop).toHaveBeenCalledWith(expect.anything(), rule.id);
    await fireEvent.dragEnd(grip);
    expect(callbacks.onDragEnd).toHaveBeenCalledTimes(1);
  });

  it("插入线按 props 显隐", () => {
    const { container } = render(RuleCard, {
      props: {
        rule: createRule("case"),
        index: 0,
        dropBefore: true,
        dropAfter: false,
        onRuleChange: vi.fn(),
        onRemove: vi.fn(),
        onHandleDragStart: vi.fn(),
        onCardDragOver: vi.fn(),
        onCardDrop: vi.fn(),
        onDragEnd: vi.fn(),
      },
    });

    expect(container.querySelectorAll(".bg-primary")).toHaveLength(1);
  });

  it("删除透出规则 id", async () => {
    const user = userEvent.setup();
    const rule = createRule("case");
    const { onRemove } = renderCard(rule);

    await user.click(screen.getByRole("button", { name: "Remove rule" }));
    expect(onRemove).toHaveBeenCalledWith(rule.id);
  });
});
