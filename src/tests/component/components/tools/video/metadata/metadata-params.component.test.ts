import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import MetadataParams from "../../../../../../components/tools/video/metadata/metadata-params.svelte";
import {
  DEFAULT_METADATA_PARAMS,
  type MetadataParamsState,
} from "../../../../../../components/tools/video/metadata/metadata-types";
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
    params: DEFAULT_METADATA_PARAMS,
    onParamsChange: vi.fn(),
    onChooseOutputDir: vi.fn(),
    canStart: false,
    processing: false,
    progress: null,
    startLabel: m.tool_video_start(),
    onStart: vi.fn(),
    templateErrors: [],
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

function renderParams(partial: Partial<MetadataParamsState> = {}) {
  let current: MetadataParamsState = {
    ...DEFAULT_METADATA_PARAMS,
    templates: { ...DEFAULT_METADATA_PARAMS.templates },
    cleared: { ...DEFAULT_METADATA_PARAMS.cleared },
    ...partial,
  };
  const onParamsChange = vi.fn((next: MetadataParamsState) => {
    current = next;
  });
  render(MetadataParams, { props: baseProps({ params: current, onParamsChange }) });
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

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("元数据参数", () => {
  it("默认编辑页签展示六字段与占位提示", () => {
    renderParams();

    expect(screen.getByLabelText(m.tool_video_tag_title())).not.toBeNull();
    expect(screen.getByText(m.tool_video_template_hint())).not.toBeNull();
  });

  it("模板输入原样回写", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams();

    await user.type(screen.getByLabelText(m.tool_video_tag_title()), "片");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ templates: expect.objectContaining({ title: "片" }) }),
    );
  });

  it("清空按钮切换清空标记", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams();

    await user.click(
      screen.getByRole("button", {
        name: `${m.tool_video_clear_tag()} ${m.tool_video_tag_title()}`,
      }),
    );
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ cleared: expect.objectContaining({ title: true }) }),
    );
  });

  it("未知占位符展示错误行", () => {
    render(MetadataParams, { props: baseProps({ templateErrors: ["bad"] }) });

    expect(screen.getByText(m.tool_video_unknown_placeholder({ names: "%bad%" }))).not.toBeNull();
  });

  it("封面模板输入原样回写，清除按钮切换标记", async () => {
    const user = userEvent.setup();
    const { onParamsChange } = renderParams();

    await user.type(screen.getByLabelText(m.tool_video_cover_label()), "%filename%.jpg");
    expect(onParamsChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ coverFile: "%filename%.jpg" }),
    );
    await user.click(screen.getByRole("button", { name: m.tool_video_clear_cover() }));
    expect(onParamsChange).toHaveBeenLastCalledWith(expect.objectContaining({ coverClear: true }));
  });

  it("页签可切到 FFmpeg 管理页", async () => {
    const user = userEvent.setup();
    renderParams();

    await user.click(screen.getByRole("tab", { name: m.tool_video_tab_ffmpeg() }));
    expect(screen.getByRole("button", { name: m.tool_ffmpeg_refresh() })).not.toBeNull();
  });

  it("引擎缺失时编辑页展示引导横幅", async () => {
    const user = userEvent.setup();
    render(MetadataParams, { props: baseProps({ ffmpegMissing: true }) });

    await user.click(screen.getByText(`${m.tool_ffmpeg_missing()} →`));
    expect(screen.getByRole("button", { name: m.tool_ffmpeg_refresh() })).not.toBeNull();
  });
});
