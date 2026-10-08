import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ConvertWorkspace from "../../../../../../components/tools/video/convert/convert-workspace.svelte";
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
const readVideoMetadataMock = vi.hoisted(() => vi.fn());
const getVideoThumbnailMock = vi.hoisted(() => vi.fn());
const expandDroppedVideoPathsMock = vi.hoisted(() => vi.fn());
const convertVideoFormatMock = vi.hoisted(() => vi.fn());
const ensureFfmpegMock = vi.hoisted(() => vi.fn());
const reinstallManagedFfmpegMock = vi.hoisted(() => vi.fn());
const openDialogMock = vi.hoisted(() => vi.fn());
const onDragDropEventMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn(), info: vi.fn() }));
vi.mock("$libs/commands", () => ({
  default: {
    getFfmpegStatus: getFfmpegStatusMock,
    getVideoDir: getVideoDirMock,
    readVideoMetadata: readVideoMetadataMock,
    getVideoThumbnail: getVideoThumbnailMock,
    expandDroppedVideoPaths: expandDroppedVideoPathsMock,
    convertVideoFormat: convertVideoFormatMock,
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
  format_name: "matroska,webm",
  duration_seconds: 60,
  file_size: 1000,
  stream: { width: 1920, height: 1080, codec_name: "h264" },
  has_cover: false,
  tags: { title: null, artist: null, album: null, genre: null, date: null, comment: null },
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
  readVideoMetadataMock.mockImplementation(() => ({
    result: () => Promise.resolve({ status: "ok", data: VIDEO_INFO }),
    failed: () => ({ result: () => Promise.resolve({ status: "ok", data: VIDEO_INFO }) }),
  }));
  getVideoThumbnailMock.mockImplementation(() => ({
    result: () => Promise.resolve({ status: "ok", data: "" }),
  }));
  expandDroppedVideoPathsMock.mockImplementation(() => ({
    result: () =>
      Promise.resolve({ status: "ok", data: { files: ["/v/a.mkv"], output_dir: "/v/edited" } }),
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

describe("视频格式转换三栏", () => {
  it("空态渲染三栏，开始按钮禁用", async () => {
    render(ConvertWorkspace);

    expect(
      await screen.findByText(m.tool_video_list_count({ count: 0 }), { exact: false }),
    ).not.toBeNull();
    expect(screen.getByText(m.tool_video_preview_empty())).not.toBeNull();
    expect(
      (screen.getByRole("button", { name: m.tool_video_start() }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it("引擎缺失时格式页引导去 FFmpeg 页下载", async () => {
    const user = userEvent.setup();
    stubDesktopBasics(FFMPEG_MISSING);
    render(ConvertWorkspace);

    const banner = await screen.findByText(`${m.tool_ffmpeg_missing()} →`);
    await user.click(banner);
    expect(screen.getByRole("button", { name: m.tool_ffmpeg_download() })).not.toBeNull();
  });

  it("完整链路：添加→读元信息→转换完成并标注流复制", async () => {
    const user = userEvent.setup();
    openDialogMock.mockResolvedValue(["/v/a.mkv"]);
    const outcome = {
      ok: true,
      input: "/v/a.mkv",
      output: "/v/edited/a.mp4",
      error: null,
      target: "mp4",
      tried_copy: true,
    };
    convertVideoFormatMock.mockImplementation(() => ({
      result: () => Promise.resolve({ status: "ok", data: outcome }),
      failed: () => ({ result: () => Promise.resolve({ status: "ok", data: outcome }) }),
    }));
    render(ConvertWorkspace);

    await user.click(await screen.findByRole("button", { name: m.tool_video_list_add() }));
    // 读元信息后行内展示时长与分辨率
    expect(await screen.findByText("01:00 · 1920×1080 · 1000 B")).not.toBeNull();

    // 默认目标 mp4 + 智能模式直接开始
    await user.click(screen.getByRole("button", { name: m.tool_video_start() }));

    expect(convertVideoFormatMock).toHaveBeenCalledWith(
      "/v/a.mkv",
      expect.objectContaining({
        target: "mp4",
        mode: "auto",
        output_dir: "/v/edited",
        overwrite: "increment",
        quality: "standard",
        preset: "veryfast",
        video_encoder: "libx264",
        audio_bitrate: "default",
        resolution: "source",
        accelerator: "cpu",
      }),
    );
    expect(
      await screen.findByText(m.tool_video_done({ ok: 1, failed: 0, skipped: 0 })),
    ).not.toBeNull();
    // 成功项展示流复制备注
    expect(await screen.findByText(m.tool_video_convert_note_copy())).not.toBeNull();
  });

  it("浏览器文件仅展示且标记不可处理", async () => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    render(ConvertWorkspace);

    const file = new File(["x"], "clip.mkv", { type: "video/x-matroska" });
    await fireEvent.drop(document.body, { dataTransfer: { types: ["Files"], files: [file] } });

    expect(await screen.findByText("clip.mkv")).not.toBeNull();
    expect(screen.getByText(m.tool_video_browser_unsupported())).not.toBeNull();
    expect(
      (screen.getByRole("button", { name: m.tool_video_start() }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });
});
