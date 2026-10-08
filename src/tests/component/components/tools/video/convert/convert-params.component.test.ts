import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ConvertParams from "../../../../../../components/tools/video/convert/convert-params.svelte";
import {
  DEFAULT_CONVERT_PARAMS,
  type ConvertParamsState,
} from "../../../../../../components/tools/video/convert/convert-types";
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

function baseProps(overrides: Record<string, unknown> = {}) {
  return {
    params: DEFAULT_CONVERT_PARAMS,
    onParamsChange: vi.fn(),
    onChooseOutputDir: vi.fn(),
    canStart: false,
    processing: false,
    progress: null,
    startLabel: m.tool_video_start(),
    onStart: vi.fn(),
    ffmpegMissing: false,
    ffmpegStatus: null,
    ffmpegBusy: false,
    ffmpegDownloading: false,
    ffmpegProgressText: null,
    ffmpegProgressValue: 0,
    ffmpegError: null,
    onRefreshFfmpeg: vi.fn(),
    onEnsureFfmpeg: vi.fn(),
    onRepairFfmpeg: vi.fn(),
    ...overrides,
  };
}

function renderParams(
  partial: Partial<ConvertParamsState> = {},
  props: Record<string, unknown> = {},
) {
  let current: ConvertParamsState = { ...DEFAULT_CONVERT_PARAMS, ...partial };
  const onParamsChange = vi.fn((next: ConvertParamsState) => {
    current = next;
  });
  render(ConvertParams, { props: baseProps({ params: current, onParamsChange, ...props }) });
  return {
    onParamsChange,
    get current() {
      return current;
    },
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

/** 在下拉中按值点选：jsdom 无布局，选项始终不可见，只能按 `data-value` 定位。 */
async function chooseOption(
  user: ReturnType<typeof userEvent.setup>,
  triggerName: string,
  value: string,
): Promise<void> {
  await user.click(screen.getByRole("button", { name: triggerName }));
  await waitFor(() => {
    expect(document.querySelector(`[data-value="${value}"]`)).not.toBeNull();
  });
  await user.click(document.querySelector(`[data-value="${value}"]`) as HTMLElement);
}

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("格式转换参数", () => {
  it("默认格式页签展示目标格式与转码模式下拉", () => {
    renderParams();

    // 页签与分组标题同文案（英文均为 Format），断言两者皆在
    expect(screen.getByRole("tab", { name: m.tool_video_convert_group_format() })).not.toBeNull();
    expect(screen.getAllByText(m.tool_video_convert_group_format()).length).toBe(2);
    expect(screen.getByText(m.tool_video_convert_target_label())).not.toBeNull();
    expect(screen.getByText(m.tool_video_convert_mode_label())).not.toBeNull();
  });

  it("输出目录输入原样回写", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams();

    await user.type(screen.getByLabelText(m.tool_video_output_label()), "/vids");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ outputDir: "/vids" }),
    );
  });

  it("未选输出目录时开始按钮禁用", () => {
    render(ConvertParams, { props: baseProps({ canStart: false }) });

    expect(
      (screen.getByRole("button", { name: m.tool_video_start() }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it("页签可切到 FFmpeg 管理页", async () => {
    const user = userEvent.setup();
    renderParams();

    await user.click(screen.getByRole("tab", { name: m.tool_video_tab_ffmpeg() }));
    expect(screen.getByRole("button", { name: m.tool_ffmpeg_refresh() })).not.toBeNull();
  });

  it("引擎缺失时格式页展示引导横幅", async () => {
    const user = userEvent.setup();
    render(ConvertParams, { props: baseProps({ ffmpegMissing: true }) });

    await user.click(screen.getByText(`${m.tool_ffmpeg_missing()} →`));
    expect(screen.getByRole("button", { name: m.tool_ffmpeg_refresh() })).not.toBeNull();
  });
});

describe("重编码选项组", () => {
  it("非重编码模式不展示选项组", () => {
    renderParams({ mode: "auto" });

    expect(screen.queryByText(m.tool_video_convert_group_reencode())).toBeNull();
    expect(screen.queryByText(m.tool_video_convert_quality_label())).toBeNull();
  });

  it("重编码模式展示全行（mp4 双编码器 + 速度档）", () => {
    renderParams({ mode: "reencode", target: "mp4" });

    expect(screen.getByText(m.tool_video_convert_group_reencode())).not.toBeNull();
    expect(screen.getByText(m.tool_video_convert_encoder_label())).not.toBeNull();
    expect(screen.getByText(m.tool_video_convert_quality_label())).not.toBeNull();
    expect(screen.getByText(m.tool_video_convert_preset_label())).not.toBeNull();
    expect(screen.getByText(m.tool_video_convert_audio_label())).not.toBeNull();
    expect(screen.getByText(m.tool_video_convert_resolution_label())).not.toBeNull();
  });

  it("avi 单编码器隐藏编码器行与速度行", () => {
    // 经 `resetForTarget` 后的真实状态：编码器已归 `mpeg4`
    renderParams({ mode: "reencode", target: "avi", videoEncoder: "mpeg4" });

    expect(screen.queryByText(m.tool_video_convert_encoder_label())).toBeNull();
    expect(screen.queryByText(m.tool_video_convert_preset_label())).toBeNull();
    expect(screen.getByText(m.tool_video_convert_quality_label())).not.toBeNull();
  });

  it("av1 编码器隐藏速度行", () => {
    renderParams({ mode: "reencode", target: "webm", videoEncoder: "av1" });

    expect(screen.getByText(m.tool_video_convert_encoder_label())).not.toBeNull();
    expect(screen.queryByText(m.tool_video_convert_preset_label())).toBeNull();
  });

  it("切目标重置编码器与速度档、保留通用档", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams({
      mode: "reencode",
      target: "mp4",
      quality: "compact",
      preset: "slow",
      videoEncoder: "libx265",
    });

    await chooseOption(user, m.tool_video_convert_target_label(), "webm");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        target: "webm",
        videoEncoder: "vp9",
        preset: "veryfast",
        quality: "compact",
      }),
    );
  });

  it("切编码器回写新值（同系保留速度档）", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams({
      mode: "reencode",
      target: "mp4",
      preset: "slow",
      videoEncoder: "libx264",
    });

    await chooseOption(user, m.tool_video_convert_encoder_label(), "libx265");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ videoEncoder: "libx265", preset: "slow" }),
    );
  });

  it("画质切换回写新值", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams({ mode: "reencode" });

    await chooseOption(user, m.tool_video_convert_quality_label(), "compact");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ quality: "compact" }),
    );
  });
});

describe("硬件加速行", () => {
  const nvencStatus = {
    available: true,
    origin: "managed",
    ffmpeg_version: "9.0",
    ffprobe_version: "9.0",
    pinned_version: "9.0",
    ffmpeg_path: "/bin/ffmpeg",
    ffprobe_path: "/bin/ffprobe",
    accelerators: ["nvenc"],
  } as const;

  it("目标支持且本机有加速才展示", () => {
    renderParams({ mode: "reencode", target: "mp4" }, { ffmpegStatus: nvencStatus });

    expect(screen.getByText(m.tool_video_convert_accel_label())).not.toBeNull();
  });

  it("引擎未知或目标不支持即隐藏", () => {
    // 状态未知
    renderParams({ mode: "reencode", target: "mp4" }, { ffmpegStatus: null });
    expect(screen.queryByText(m.tool_video_convert_accel_label())).toBeNull();
    cleanup();

    // webm 无硬编码器
    renderParams({ mode: "reencode", target: "webm" }, { ffmpegStatus: nvencStatus });
    expect(screen.queryByText(m.tool_video_convert_accel_label())).toBeNull();
  });

  it("加速切换回写新值", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams(
      { mode: "reencode", target: "mp4" },
      { ffmpegStatus: nvencStatus },
    );

    await chooseOption(user, m.tool_video_convert_accel_label(), "nvenc");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ accelerator: "nvenc" }),
    );
  });

  it("切目标加速归 CPU", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams(
      { mode: "reencode", target: "mp4", accelerator: "nvenc" },
      { ffmpegStatus: nvencStatus },
    );

    await chooseOption(user, m.tool_video_convert_target_label(), "mkv");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ target: "mkv", accelerator: "cpu" }),
    );
  });

  it("切无硬编对应的编码器加速归 CPU", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams(
      { mode: "reencode", target: "ts", videoEncoder: "libx264", accelerator: "nvenc" },
      { ffmpegStatus: nvencStatus },
    );

    await chooseOption(user, m.tool_video_convert_encoder_label(), "mpeg2video");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ videoEncoder: "mpeg2video", accelerator: "cpu" }),
    );
  });
});
