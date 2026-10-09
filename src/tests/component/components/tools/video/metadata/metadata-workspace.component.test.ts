import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import MetadataWorkspace from "../../../../../../components/tools/video/metadata/metadata-workspace.svelte";
import { m } from "$libs/i18n/paraglide/messages";

// bits-ui 组件在 jsdom 下缺失的浏览器 API，就地补齐（同其它工具测试）。
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

const getFfmpegStatusMock = vi.hoisted(() => vi.fn());
const getVideoDirMock = vi.hoisted(() => vi.fn());
const readVideoBatchMock = vi.hoisted(() => vi.fn());
const getVideoThumbnailMock = vi.hoisted(() => vi.fn());
const expandDroppedVideoPathsMock = vi.hoisted(() => vi.fn());
const applyVideoMetadataMock = vi.hoisted(() => vi.fn());
const ensureFfmpegMock = vi.hoisted(() => vi.fn());
const reinstallManagedFfmpegMock = vi.hoisted(() => vi.fn());
const openDialogMock = vi.hoisted(() => vi.fn());
const onDragDropEventMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn(), info: vi.fn() }));
vi.mock("$libs/commands", () => ({
  default: {
    getFfmpegStatus: getFfmpegStatusMock,
    getVideoDir: getVideoDirMock,
    readVideoBatch: readVideoBatchMock,
    getVideoThumbnail: getVideoThumbnailMock,
    expandDroppedVideoPaths: expandDroppedVideoPathsMock,
    applyVideoMetadata: applyVideoMetadataMock,
    ensureFfmpeg: ensureFfmpegMock,
    reinstallManagedFfmpeg: reinstallManagedFfmpegMock,
  },
}));
vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialogMock }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: onDragDropEventMock }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

const FFMPEG_READY = {
  available: true,
  origin: "managed",
  ffmpeg_version: "9.0",
  ffprobe_version: "9.0",
  pinned_version: "9.0",
  ffmpeg_path: "/data/bins/video/9.0/ffmpeg",
  ffprobe_path: "/data/bins/video/9.0/ffprobe",
};
const FFMPEG_MISSING = {
  available: false,
  origin: null,
  ffmpeg_version: null,
  ffprobe_version: null,
  pinned_version: "9.0",
  ffmpeg_path: null,
  ffprobe_path: null,
};
const VIDEO_INFO = {
  format_name: "mov,mp4,m4a,3gp,3g2,mj2",
  duration_seconds: 60,
  file_size: 1000,
  stream: { width: 1920, height: 1080, codec_name: "h264" },
  has_cover: false,
  tags: { title: "旧标题", artist: null, album: null, genre: null, date: null, comment: null },
};

/** 命令链默认桩：引擎就绪 + 输出目录 + 读元信息成功 + 海报空 + 透传展开 */
function stubDesktopBasics(
  status: typeof FFMPEG_READY | typeof FFMPEG_MISSING = FFMPEG_READY,
): void {
  getFfmpegStatusMock.mockImplementation(() => ({
    result: () => Promise.resolve({ status: "ok", data: status }),
    success: (onOk: (value: unknown) => unknown) => {
      onOk(status);
      return { failed: () => {} };
    },
  }));
  getVideoDirMock.mockImplementation(() => ({
    result: () => Promise.resolve({ status: "ok", data: "/vids" }),
    failed: () => ({ result: () => Promise.resolve({ status: "ok", data: "/vids" }) }),
  }));
  readVideoBatchMock.mockImplementation((paths: string[]) => ({
    result: () =>
      Promise.resolve({
        status: "ok",
        data: paths.map((path) => ({ path, ok: true, info: VIDEO_INFO, thumb: "", error: null })),
      }),
    failed: () => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: paths.map((path) => ({
            path,
            ok: true,
            info: VIDEO_INFO,
            thumb: "",
            error: null,
          })),
        }),
    }),
  }));
  getVideoThumbnailMock.mockImplementation(() => ({
    result: () => Promise.resolve({ status: "ok", data: "" }),
  }));
  expandDroppedVideoPathsMock.mockImplementation(() => ({
    result: () =>
      Promise.resolve({ status: "ok", data: { files: ["/v/a.mp4"], output_dir: "/v/edited" } }),
  }));
  onDragDropEventMock.mockReturnValue(Promise.resolve(() => {}));
}

beforeEach(() => {
  vi.clearAllMocks();
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  stubDesktopBasics();
});

afterEach(() => {
  cleanup();
  // bits-ui 弹窗会在 body 留 `pointer-events` 样式，跨用例清掉
  document.body.removeAttribute("style");
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("视频元数据三栏", () => {
  it("空态渲染三栏，开始按钮禁用", async () => {
    render(MetadataWorkspace);

    expect(
      await screen.findByText(m.tool_video_list_count({ count: 0 }), { exact: false }),
    ).not.toBeNull();
    expect(screen.getByText(m.tool_video_preview_empty())).not.toBeNull();
    expect(
      (screen.getByRole("button", { name: m.tool_video_start() }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it("未知占位符拦截开始并提示", async () => {
    const user = userEvent.setup();
    render(MetadataWorkspace);

    await user.type(screen.getByLabelText(m.tool_video_tag_title()), "%bad%");
    expect(
      (screen.getByRole("button", { name: m.tool_video_start() }) as HTMLButtonElement).disabled,
    ).toBe(true);
    expect(screen.getByText(m.tool_video_unknown_placeholder({ names: "%bad%" }))).not.toBeNull();
  });

  it("引擎缺失时编辑页引导去 FFmpeg 页下载", async () => {
    const user = userEvent.setup();
    stubDesktopBasics(FFMPEG_MISSING);
    render(MetadataWorkspace);

    const banner = await screen.findByText(`${m.tool_ffmpeg_missing()} →`);
    await user.click(banner);
    expect(screen.getByRole("button", { name: m.tool_ffmpeg_download() })).not.toBeNull();
  });

  it("载入中预览与信息区显示加载态，不误显示无标签", async () => {
    const user = userEvent.setup();
    openDialogMock.mockResolvedValue(["/v/a.mp4"]);
    readVideoBatchMock.mockImplementationOnce(() => ({
      failed() {
        return this;
      },
      result() {
        return new Promise(() => {}) as Promise<never>;
      },
    }));
    render(MetadataWorkspace);

    await user.click(await screen.findByRole("button", { name: m.tool_video_list_add() }));
    expect(await screen.findByText("a.mp4")).not.toBeNull();
    // 预览骨架 + 信息区各一处加载文案，不误显示“不可读”
    expect(screen.getAllByText("Loading preview…")).toHaveLength(2);
    expect(screen.queryByText("Unreadable")).toBeNull();
  });

  it("处理中锁定列表增删改", async () => {
    const user = userEvent.setup();
    openDialogMock.mockResolvedValue(["/v/a.mp4"]);
    // 写入永不返回：卡在处理中
    applyVideoMetadataMock.mockImplementation(() => ({
      failed() {
        return this;
      },
      result() {
        return new Promise(() => {}) as Promise<never>;
      },
    }));
    render(MetadataWorkspace);

    await user.click(await screen.findByRole("button", { name: m.tool_video_list_add() }));
    expect(await screen.findByText("01:00 · 1920×1080 · 1000 B")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: m.tool_video_start() }));

    // 处理中：添加/清空/单项移除全禁用
    expect(
      screen.getByRole("button", { name: m.tool_video_list_add() }).hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.getByRole("button", { name: m.tool_video_list_clear() }).hasAttribute("disabled"),
    ).toBe(true);
    for (const remove of screen.getAllByRole("button", { name: m.tool_video_list_remove() })) {
      expect(remove.hasAttribute("disabled")).toBe(true);
    }
  });

  it("完整链路：添加→读元信息→模板展开→处理完成", async () => {
    const user = userEvent.setup();
    openDialogMock.mockResolvedValue(["/v/a.mp4"]);
    applyVideoMetadataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: { ok: true, input: "/v/a.mp4", output: "/vids/a.mp4", error: null, cover: "Kept" },
        }),
      failed: () => ({
        result: () =>
          Promise.resolve({
            status: "ok",
            data: {
              ok: true,
              input: "/v/a.mp4",
              output: "/vids/a.mp4",
              error: null,
              cover: "Kept",
            },
          }),
      }),
    }));
    render(MetadataWorkspace);

    await user.click(await screen.findByRole("button", { name: m.tool_video_list_add() }));
    // 读元信息后行内展示时长与分辨率
    expect(await screen.findByText("01:00 · 1920×1080 · 1000 B")).not.toBeNull();

    // 模板 `%filename%` 按该项展开写入
    await user.type(screen.getByLabelText(m.tool_video_tag_title()), "%filename%_new");
    await user.click(screen.getByRole("button", { name: m.tool_video_start() }));

    expect(applyVideoMetadataMock).toHaveBeenCalledWith(
      "/v/a.mp4",
      expect.objectContaining({
        output_dir: "/v/edited",
        overwrite: "increment",
        tags: expect.objectContaining({ title: "a_new" }),
        cover: { file: null, clear: false },
      }),
    );
    expect(
      await screen.findByText(m.tool_video_done({ ok: 1, failed: 0, skipped: 0 })),
    ).not.toBeNull();
  });

  it("浏览器文件仅展示且标记不可处理", async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    render(MetadataWorkspace);

    const file = new File(["x"], "clip.mp4", { type: "video/mp4" });
    await fireEvent.drop(document.body, { dataTransfer: { types: ["Files"], files: [file] } });

    expect(await screen.findByText("clip.mp4")).not.toBeNull();
    expect(screen.getByText(m.tool_video_browser_unsupported())).not.toBeNull();
    expect(
      (screen.getByRole("button", { name: m.tool_video_start() }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });
});
