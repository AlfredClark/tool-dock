import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ResizeWorkspace from "../../../../../../components/tools/image/resize/resize-workspace.svelte";
import type { ExpandDropOutcome, ResizeSingleOutcome } from "$libs/commands/bindings";

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

// 命令链桩：返回可链式调用的结算体（`.failed().result()`），数据走内存直给。
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
  // 注意：必须同步返回链式对象（`.failed().result()`），不能包 `async`
  // （原生 Promise 没有 `.failed()`，会复现“失败不是函数”的假阳性）
  readImageBatch: vi.fn((paths: string[]) =>
    okChain(
      paths.map((path) => ({
        path,
        ok: true,
        info: { width: 100, height: 80, format: "png", file_size: 1024 },
        thumb: "data:image/jpeg;base64,AAA",
        error: null,
      })),
    ),
  ),
  getImageThumbnail: vi.fn(() => okChain("data:image/jpeg;base64,AAA")),
  resizeImage: vi.fn((input: string) =>
    okChain<ResizeSingleOutcome>({
      ok: true,
      input,
      output: `${input}.out.png`,
      width: 50,
      height: 40,
      error: null,
      target_met: null,
    }),
  ),
  // 默认直通原路径（文件夹用例按需覆盖实现）
  expandDroppedPaths: vi.fn((paths: string[]) =>
    okChain<ExpandDropOutcome>({ files: paths, output_dir: null }),
  ),
  // 默认取不到系统图片目录（默认目录用例按需覆盖实现）
  getPictureDir: vi.fn(() => okChain<string | null>(null)),
}));

vi.mock("$libs/commands", () => ({ default: commandsMock }));

const dialogMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: dialogMock }));

const toastMocks = vi.hoisted(() => ({ info: vi.fn(), error: vi.fn(), success: vi.fn() }));
vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));

beforeEach(() => {
  vi.clearAllMocks();
  // 添加图片返回两条路径，选输出目录返回目录（按调用参数区分）
  dialogMock.mockImplementation(async (options?: { directory?: boolean }) => {
    if (options?.directory) return "/tmp/out";
    return ["/tmp/a.png", "/tmp/b.png"];
  });
  // 路径展开默认直通（文件夹用例在用例内覆盖）
  commandsMock.expandDroppedPaths.mockImplementation((paths: string[]) =>
    okChain<ExpandDropOutcome>({ files: paths, output_dir: null }),
  );
  // 系统图片目录默认取不到（默认目录用例在用例内覆盖）
  commandsMock.getPictureDir.mockImplementation(() => okChain<string | null>(null));
});

afterEach(() => {
  cleanup();
});

describe("图片尺寸批量流程", () => {
  it("处理完成后按钮回到空闲可用态", async () => {
    const user = userEvent.setup();
    render(ResizeWorkspace);

    // 添加两张图（元信息 + 缩略图走桩命令）
    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    expect(await screen.findByText("b.png")).not.toBeNull();

    // 选输出目录 + 填宽 50（锁定比例，50x40 合法）
    await user.click(screen.getByRole("button", { name: "Choose…" }));
    await user.type(screen.getByLabelText("Width"), "50");

    const start = screen.getByRole("button", { name: "Start" });
    expect(start.hasAttribute("disabled")).toBe(false);
    await user.click(start);

    // 处理中显示进度，完成后汇总 toast 且按钮复位（本次修复的回归点）
    await waitFor(() => {
      expect(toastMocks.info).toHaveBeenCalledWith("Done: 2 succeeded, 0 failed, 0 skipped");
    });
    const idle = await screen.findByRole("button", { name: "Start" });
    // 全部完成后按钮仍可用：已完成项可调参重新生成（需求回归点）
    expect(idle.hasAttribute("disabled")).toBe(false);
    expect(commandsMock.resizeImage).toHaveBeenCalledTimes(2);
    // 汇总条常驻列表底部（toast 消失后仍可查看）
    expect(screen.getByText("Done: 2 succeeded, 0 failed, 0 skipped")).not.toBeNull();
    expect(screen.queryByRole("button", { name: "Retry failed" })).toBeNull();

    // 再次点击重新生成（调参重跑），汇总更新
    await user.click(idle);
    await waitFor(() => {
      expect(commandsMock.resizeImage).toHaveBeenCalledTimes(4);
    });
    expect(screen.getByText("Done: 2 succeeded, 0 failed, 0 skipped")).not.toBeNull();
  });

  it("失败项经汇总条一键重试", async () => {
    const user = userEvent.setup();
    // 首张失败一次：业务失败装进 Outcome（非传输失败），重试后成功
    commandsMock.resizeImage.mockImplementationOnce((input: string) =>
      okChain<ResizeSingleOutcome>({
        ok: false,
        input,
        output: null,
        width: null,
        height: null,
        error: { code: "DecodeFailed", message: "boom" },
        target_met: null,
      }),
    );
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Choose…" }));
    await user.type(screen.getByLabelText("Width"), "50");
    await user.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() => {
      expect(toastMocks.info).toHaveBeenCalledWith("Done: 1 succeeded, 1 failed, 0 skipped");
    });
    expect(screen.getByText("Done: 1 succeeded, 1 failed, 0 skipped")).not.toBeNull();

    // 重试只重跑失败项，汇总更新
    await user.click(screen.getByRole("button", { name: "Retry failed" }));
    await waitFor(() => {
      expect(toastMocks.info).toHaveBeenCalledWith("Done: 1 succeeded, 0 failed, 0 skipped");
    });
    expect(screen.getByText("Done: 1 succeeded, 0 failed, 0 skipped")).not.toBeNull();
    expect(commandsMock.resizeImage).toHaveBeenCalledTimes(3);
  });

  it("跳过项单独计数并灰色徽章", async () => {
    const user = userEvent.setup();
    commandsMock.resizeImage.mockImplementationOnce((input: string) =>
      okChain<ResizeSingleOutcome>({
        ok: false,
        input,
        output: null,
        width: null,
        height: null,
        error: { code: "Skipped", message: "output exists, skipped: x" },
        target_met: null,
      }),
    );
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Choose…" }));
    await user.type(screen.getByLabelText("Width"), "50");
    await user.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() => {
      expect(toastMocks.info).toHaveBeenCalledWith("Done: 1 succeeded, 0 failed, 1 skipped");
    });
    expect(screen.getByText("Done: 1 succeeded, 0 failed, 1 skipped")).not.toBeNull();
    expect(screen.getByText("Skipped")).not.toBeNull();
  });

  it("目标大小透传到命令", async () => {
    const user = userEvent.setup();
    render(ResizeWorkspace);

    // 切到目标模式（链接预填 200KB，质量滑块隐藏）
    await user.click(screen.getByRole("button", { name: "Target size →" }));
    expect(screen.queryByText("85 · JPEG only")).toBeNull();
    expect((screen.getByLabelText("Target size") as HTMLInputElement).value).toBe("200");

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    // PNG 条目 + JPEG 目标：预览明示不适用
    expect(screen.getByText("Target size applies to JPEG output only")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Choose…" }));
    await user.type(screen.getByLabelText("Width"), "50");
    await user.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() => {
      expect(commandsMock.resizeImage).toHaveBeenCalledWith(
        "/tmp/a.png",
        expect.objectContaining({ target_size_kb: 200 }),
      );
    });
  });

  it("文件夹拖入自动展开并设置输出目录", async () => {
    const user = userEvent.setup();
    dialogMock.mockImplementation(async () => ["/tmp/photos"]);
    commandsMock.expandDroppedPaths.mockImplementation((paths: string[]) => {
      if (paths.includes("/tmp/photos")) {
        return okChain<ExpandDropOutcome>({
          files: ["/tmp/photos/a.png", "/tmp/photos/b.png"],
          output_dir: "/tmp/photos/resized",
        });
      }
      return okChain<ExpandDropOutcome>({
        files: ["/tmp/other/c.png"],
        output_dir: "/tmp/other/resized",
      });
    });
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    expect(await screen.findByText("b.png")).not.toBeNull();
    // 输出目录自动设为文件夹下的 resized/，并 toast 提示
    const outputDir = screen.getByLabelText("Output directory") as HTMLInputElement;
    expect(outputDir.value).toBe("/tmp/photos/resized");
    expect(toastMocks.info).toHaveBeenCalledWith(
      "Output directory auto-set to /tmp/photos/resized",
    );

    // 已选目录不再被第二次拖入覆盖
    dialogMock.mockImplementation(async () => ["/tmp/other"]);
    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("c.png")).not.toBeNull();
    expect((screen.getByLabelText("Output directory") as HTMLInputElement).value).toBe(
      "/tmp/photos/resized",
    );
  });

  it("读取中禁用开始按钮", async () => {
    const user = userEvent.setup();
    // 批量请求永不返回：条目卡在 loading
    commandsMock.readImageBatch.mockImplementationOnce((() => ({
      failed() {
        return this;
      },
      result() {
        return new Promise(() => {}) as Promise<never>;
      },
    })) as never);
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Choose…" }));
    await user.type(screen.getByLabelText("Width"), "50");

    // 模式目录齐备，但有条目仍在读取，开始保持禁用
    expect(screen.getByRole("button", { name: "Start" }).hasAttribute("disabled")).toBe(true);
  });

  it("批量一次取全元信息与小图", async () => {
    const user = userEvent.setup();
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    expect(await screen.findByText("b.png")).not.toBeNull();

    // 两张图一次批量调用：路径数组 + 96px 小图边长
    expect(commandsMock.readImageBatch).toHaveBeenCalledTimes(1);
    expect(commandsMock.readImageBatch).toHaveBeenCalledWith(["/tmp/a.png", "/tmp/b.png"], 96);
  });

  it("小图失败仍就绪，选中后大图懒加载", async () => {
    const user = userEvent.setup();
    // 首张小图缺失：条目照常就绪（仅空预览）
    commandsMock.readImageBatch.mockImplementationOnce(((paths: string[]) =>
      okChain(
        paths.map((path) => ({
          path,
          ok: true,
          info: { width: 100, height: 80, format: "png", file_size: 1024 },
          thumb: path.endsWith("a.png") ? null : "data:image/jpeg;base64,AAA",
          error: null,
        })),
      )) as never);
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    expect(await screen.findByText("b.png")).not.toBeNull();

    // 选中首张触发大图懒加载（768px 按需取）
    await waitFor(() => {
      expect(commandsMock.getImageThumbnail).toHaveBeenCalledWith("/tmp/a.png", 768);
    });
  });

  it("整块传输失败按块标红", async () => {
    const user = userEvent.setup();
    commandsMock.readImageBatch.mockImplementationOnce((() => ({
      failed(handler: (failure: unknown) => void) {
        handler(new Error("ipc down"));
        return this;
      },
      async result() {
        return { status: "error", error: new Error("ipc down") } as const;
      },
    })) as never);
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    // 两张都标不可读（列表 2 徽章 + 中栏空图兜底 1），开始按钮保持禁用
    expect(await screen.findAllByText("Unreadable")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Start" }).hasAttribute("disabled")).toBe(true);
  });

  it("载入中预览区与信息行显示加载态", async () => {
    const user = userEvent.setup();
    commandsMock.readImageBatch.mockImplementationOnce((() => ({
      failed() {
        return this;
      },
      result() {
        return new Promise(() => {}) as Promise<never>;
      },
    })) as never);
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    // 预览骨架 + 信息行各一处加载文案，不误显示“不可读”
    expect(screen.getAllByText("Loading preview…")).toHaveLength(2);
    expect(screen.queryByText("Unreadable")).toBeNull();
  });

  it("处理中锁定列表增删改", async () => {
    const user = userEvent.setup();
    // 单张处理永不返回：卡在处理中
    commandsMock.resizeImage.mockImplementation((() => ({
      failed() {
        return this;
      },
      result() {
        return new Promise(() => {}) as Promise<never>;
      },
    })) as never);
    render(ResizeWorkspace);

    await user.click(screen.getByRole("button", { name: "Add images" }));
    expect(await screen.findByText("a.png")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Choose…" }));
    await user.type(screen.getByLabelText("Width"), "50");
    await user.click(screen.getByRole("button", { name: "Start" }));

    // 处理中：添加/清空/单项移除全禁用，选中浏览仍可用
    expect(screen.getByRole("button", { name: "Add images" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "Clear" }).hasAttribute("disabled")).toBe(true);
    for (const remove of screen.getAllByRole("button", { name: "Remove" })) {
      expect(remove.hasAttribute("disabled")).toBe(true);
    }
    await user.click(screen.getByText("a.png"));
  });

  it("默认输出目录为系统图片目录", async () => {
    commandsMock.getPictureDir.mockImplementation(() => okChain<string | null>("/home/u/Pictures"));
    render(ResizeWorkspace);

    // 挂载后自动填入系统图片目录（不计入“动过”，文件夹仍可覆盖）
    await waitFor(() => {
      expect((screen.getByLabelText("Output directory") as HTMLInputElement).value).toBe(
        "/home/u/Pictures",
      );
    });
  });
});
