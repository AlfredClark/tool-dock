import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ToolGrid from "../../../../../components/widget/tools/tool-grid.svelte";
import { TOOLS } from "$libs/tools/registry";

const gotoMock = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({
  goto: gotoMock,
}));

vi.mock("$app/paths", () => ({
  resolve: (path: string): string => path,
}));

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
});

describe("工具网格", () => {
  it("按分类渲染全部工具卡片", () => {
    render(ToolGrid);

    // 4 分类 × 每类 3 个演示工具，共 12 张卡片
    const cards = screen.getAllByRole("button");
    expect(cards).toHaveLength(TOOLS.length);
    for (const tool of TOOLS) {
      // 节点环境无 window，Paraglide 回落 baseLocale，取到英文工具名
      expect(screen.getByRole("button", { name: tool.name() })).not.toBeNull();
    }
  });

  it("点击卡片跳转对应工具路由", async () => {
    const user = userEvent.setup();
    render(ToolGrid);

    const card = screen.getByRole("button", { name: TOOLS[0].name() });
    // 卡片简介单行截断，悬浮经原生 tooltip 显示全文
    const description = within(card).getByText(TOOLS[0].description());
    expect(description.getAttribute("title")).toBe(TOOLS[0].description());

    await user.click(card);
    expect(gotoMock).toHaveBeenCalledWith(TOOLS[0].path);
  });
});
