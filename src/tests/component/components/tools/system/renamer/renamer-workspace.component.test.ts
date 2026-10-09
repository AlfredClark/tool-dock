import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import RenamerWorkspace from "../../../../../../components/tools/system/renamer/renamer-workspace.svelte";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

// bits-ui 与 paneforge 在 jsdom 下缺失的浏览器 API，就地补齐（同工具布局测试）。
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

// 对话框 mock：节点环境无 Tauri 运行时，添加文件走该桩返回路径
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

// 命令链桩：返回可链式调用的结算体（`.failed().result()`），数据走内存直给
function okChain<T>(data: T) {
  return {
    failed() {
      return this;
    },
    async result() {
      return { status: "ok", data } as const;
    },
  };
}

const commandsMock = vi.hoisted(() => ({
  // 注意：必须同步返回链式对象，不能包 `async`（原生 Promise 没有 `.failed()`）
  renameFiles: vi.fn(),
}));

vi.mock("$libs/commands", () => ({ default: commandsMock }));

const toastMocks = vi.hoisted(() => ({
  info: vi.fn(),
  error: vi.fn(),
  success: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));

// webview mock：桌面标记伪造后 onMount 会进 Tauri 拖放分支，此处直接拒收走 DOM 降级
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn().mockRejectedValue(new Error("no runtime")),
  }),
}));

const openMock = vi.mocked(openDialog);

beforeEach(() => {
  vi.clearAllMocks();
  openMock.mockResolvedValue(["/tmp/b.png", "/tmp/a.png"]);
});

afterEach(() => {
  cleanup();
  // bits-ui 下拉打开时给 body 加滚动锁定样式，卸载后手动清理，避免泄漏到后续用例
  document.body.removeAttribute("style");
  // 桌面标记按用例设置，此处统一清理，避免泄漏到后续用例
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

/** 伪造桌面端：`isDesktop` 在组件初始化时读取该标记，执行流用例需要 */
function mockDesktop(): void {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    value: {},
    configurable: true,
    writable: true,
  });
}

describe("重命名 workspace", () => {
  it("渲染左右分栏骨架：左规则空态 + 右空列表", () => {
    render(RenamerWorkspace);

    expect(screen.getByRole("region", { name: "Rules" })).not.toBeNull();
    expect(screen.getByText("No rules yet, add one from the top right")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Add rule" })).not.toBeNull();
    // 无规则时隐藏清空按钮
    expect(screen.queryByRole("button", { name: "Clear rules" })).toBeNull();
    expect(screen.getByText("Drop files here, or add them below")).not.toBeNull();
    // 无勾选时不渲染删除按钮
    expect(screen.queryByRole("button", { name: "Remove selected" })).toBeNull();
  });

  it("添加文件后按名称升序展示，全选删除后回到空态", async () => {
    const user = userEvent.setup();
    render(RenamerWorkspace);

    await user.click(screen.getByRole("button", { name: "Add files" }));
    // 对话框按返回顺序入列表，展示按名称升序
    const rows = await screen.findAllByText("a.png");
    expect(rows.length).toBeGreaterThan(0);
    expect(screen.getAllByText("b.png").length).toBeGreaterThan(0);

    // 表头全选 → 删除按钮出现并带计数
    await user.click(screen.getByRole("checkbox", { name: "Files" }));
    expect(screen.getByText("Remove selected · 2")).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Remove selected" }));
    expect(screen.getByText("Drop files here, or add them below")).not.toBeNull();
    expect(screen.queryByRole("button", { name: "Remove selected" })).toBeNull();
  });

  it("搜索筛选过滤列表", async () => {
    const user = userEvent.setup();
    render(RenamerWorkspace);

    await user.click(screen.getByRole("button", { name: "Add files" }));
    await screen.findAllByText("a.png");

    await user.type(screen.getByPlaceholderText("Filter by name…"), "b.png");
    expect(screen.queryByText("a.png")).toBeNull();
    expect(screen.getAllByText("b.png").length).toBeGreaterThan(0);
  });

  it("添加前后缀规则后新文件名实时预览，扩展名保留", async () => {
    const user = userEvent.setup();
    const { container } = render(RenamerWorkspace);

    await user.click(screen.getByRole("button", { name: "Add files" }));
    await screen.findAllByText("a.png");

    await addRuleByLabel(user, "Add affix");
    // 后缀文本框是左栏唯一的文本输入
    const textInput = container.querySelector(
      'section[aria-label="Rules"] input',
    ) as HTMLInputElement | null;
    if (!textInput) throw new Error("affix text input missing");
    await user.type(textInput, "_v2");

    // 原名保留一处，新名实时派生且保留扩展名
    expect(screen.getByText("a.png")).not.toBeNull();
    expect(screen.getAllByText("a_v2.png").length).toBeGreaterThan(0);
    // 清空规则后预览恢复原名
    await user.click(screen.getByRole("button", { name: "Clear rules" }));
    expect(screen.getAllByText("a.png").length).toBeGreaterThan(0);
    expect(screen.queryByText("a_v2.png")).toBeNull();
  });

  it("禁用规则后跳过预览", async () => {
    const user = userEvent.setup();
    const { container } = render(RenamerWorkspace);

    await user.click(screen.getByRole("button", { name: "Add files" }));
    await screen.findAllByText("a.png");

    await addRuleByLabel(user, "Add affix");
    const textInput = container.querySelector(
      'section[aria-label="Rules"] input',
    ) as HTMLInputElement | null;
    if (!textInput) throw new Error("affix text input missing");
    await user.type(textInput, "_v2");
    expect(screen.getAllByText("a_v2.png").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Toggle enabled" }));
    expect(screen.queryByText("a_v2.png")).toBeNull();
    expect(screen.getAllByText("a.png").length).toBeGreaterThan(0);
  });

  it("拖放改变规则顺序", async () => {
    const user = userEvent.setup();
    render(RenamerWorkspace);

    await addRuleByLabel(user, "Add affix");
    await addRuleByLabel(user, "Strip affix");
    await addRuleByLabel(user, "Change case");
    // 左栏三张卡，顺序即添加顺序
    const region = screen.getByRole("region", { name: "Rules" });
    const cards = within(region).getAllByRole("listitem");
    expect(cards).toHaveLength(3);
    expect(cards[0]?.textContent).toContain("Add affix");

    // 拖末卡手柄，放到首卡上半区 → 末卡移到首位
    //（jsdom 无布局且合成事件带不上 clientY，只覆盖上半区路径；下半区靠真机验证）
    const grips = screen.getAllByRole("button", { name: "Drag to reorder" });
    const lastGrip = grips[grips.length - 1];
    const firstCard = cards[0];
    if (!lastGrip || !firstCard) throw new Error("drag nodes missing");
    await fireEvent.dragStart(lastGrip);
    await fireEvent.dragOver(firstCard);
    await fireEvent.drop(firstCard);

    const ordered = within(screen.getByRole("region", { name: "Rules" })).getAllByRole("listitem");
    expect(ordered[0]?.textContent).toContain("Change case");
    expect(ordered[2]?.textContent).toContain("Strip affix");
  });

  it("执行改名并结算落盘：成功更新路径，跳过常驻行徽章", async () => {
    const user = userEvent.setup();
    mockDesktop();
    const { container } = render(RenamerWorkspace);

    await user.click(screen.getByRole("button", { name: "Add files" }));
    await screen.findAllByText("a.png");
    await addRuleByLabel(user, "Add affix");
    const textInput = container.querySelector(
      'section[aria-label="Rules"] input',
    ) as HTMLInputElement | null;
    if (!textInput) throw new Error("affix text input missing");
    await user.type(textInput, "_v2");

    commandsMock.renameFiles.mockReturnValue(
      okChain([
        { ok: true, path: "/tmp/b.png", new_path: "/tmp/b_v2.png", skipped: null, error: null },
        { ok: false, path: "/tmp/a.png", new_path: null, skipped: "exists", error: null },
      ]),
    );
    // 对话框按 [b, a] 顺序入列表，payload 同序
    await user.click(screen.getByRole("button", { name: "Rename · 2" }));
    expect(commandsMock.renameFiles).toHaveBeenCalledWith([
      { path: "/tmp/b.png", new_name: "b_v2.png" },
      { path: "/tmp/a.png", new_name: "a_v2.png" },
    ]);

    // 成功项路径三件套更新，跳过项行徽章常驻原因
    expect(await screen.findByText("b_v2.png")).not.toBeNull();
    expect(screen.getByTitle("Target exists, skipped")).not.toBeNull();
    expect(screen.getByText("Done: 1 renamed, 1 skipped, 0 failed")).not.toBeNull();
    // 跳过项预览仍是脏的，可解决冲突后重跑，按钮保持可用
    expect(screen.getByRole("button", { name: "Rename · 2" }).hasAttribute("disabled")).toBe(false);
  });

  it("传输失败 toast 且不上汇总条", async () => {
    const user = userEvent.setup();
    mockDesktop();
    const { container } = render(RenamerWorkspace);

    await user.click(screen.getByRole("button", { name: "Add files" }));
    await screen.findAllByText("a.png");
    await addRuleByLabel(user, "Add affix");
    const textInput = container.querySelector(
      'section[aria-label="Rules"] input',
    ) as HTMLInputElement | null;
    if (!textInput) throw new Error("affix text input missing");
    await user.type(textInput, "_v2");

    commandsMock.renameFiles.mockReturnValue({
      failed(callback: (failure: unknown) => void) {
        callback(new Error("ipc down"));
        return this;
      },
      async result() {
        return { status: "error", error: new Error("ipc down") } as const;
      },
    });
    await user.click(screen.getByRole("button", { name: "Rename · 2" }));

    expect(toastMocks.error).toHaveBeenCalledWith("Rename request failed");
    expect(screen.queryByText(/Done:/)).toBeNull();
  });
});

/**
 * 经添加下拉框加入指定类型规则：jsdom 下 floating 定位失效、菜单 portal 保持隐藏， 角色查询不可见；菜单挂载于
 * document.body，直接对节点派发点击（与 Select 测试同因）。
 */
async function addRuleByLabel(
  user: ReturnType<typeof userEvent.setup>,
  label: string,
): Promise<void> {
  await user.click(screen.getByRole("button", { name: "Add rule" }));
  const target = [...document.querySelectorAll('[data-slot="dropdown-menu-item"]')].find((item) =>
    item.textContent?.includes(label),
  );
  if (!target) throw new Error(`rule menu item missing: ${label}`);
  await fireEvent.click(target);
  // 合成点击不走完整指针流程，菜单逻辑未关闭、body 滚动锁定残留；
  // Escape 尝试关闭，残留样式手动释放（纯 jsdom 产物，真机上选择即关闭）
  await user.keyboard("{Escape}");
  document.body.removeAttribute("style");
  expect(screen.getByText(label)).not.toBeNull();
}
