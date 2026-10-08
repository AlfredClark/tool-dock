import { describe, expect, it } from "vitest";
import {
  ACCELERATORS_BY_TARGET,
  AUDIO_BITRATES,
  CONVERT_MODES,
  DEFAULT_CONVERT_PARAMS,
  ENCODERS_BY_TARGET,
  OUTPUT_RESOLUTIONS,
  VIDEO_PRESETS,
  VIDEO_QUALITIES,
  VIDEO_TARGETS,
  defaultAcceleratorFor,
  defaultEncoderFor,
  expectedOutputName,
  formatClock,
  formatDownloadSize,
  resetForEncoder,
  resetForTarget,
  supportsHwEncoding,
  supportsPreset,
  videoFileExt,
  videoFileStem,
} from "$components/tools/video/convert/convert-types";

describe("formatDownloadSize", () => {
  it("按 B/KB/MB 一位小数格式化", () => {
    expect(formatDownloadSize(512)).toBe("512 B");
    expect(formatDownloadSize(2048)).toBe("2.0 KB");
    expect(formatDownloadSize(80 * 1024 * 1024)).toBe("80.0 MB");
  });

  it("非法输入回落零字节", () => {
    expect(formatDownloadSize(Number.NaN)).toBe("0 B");
    expect(formatDownloadSize(-1)).toBe("0 B");
  });
});

describe("formatClock", () => {
  it("按 mm:ss 格式化并钳制负值", () => {
    expect(formatClock(65)).toBe("01:05");
    expect(formatClock(3600)).toBe("60:00");
    expect(formatClock(-5)).toBe("00:00");
  });
});

describe("videoFileStem/videoFileExt", () => {
  it("兼容正反斜杠并取小写后缀", () => {
    expect(videoFileStem("C:\\vids\\Clip.MOV")).toBe("Clip");
    expect(videoFileExt("C:\\vids\\Clip.MOV")).toBe("mov");
    expect(videoFileStem("/v/a")).toBe("a");
    expect(videoFileExt("/v/a")).toBe("");
  });
});

describe("expectedOutputName", () => {
  it("茎名换目标后缀", () => {
    expect(expectedOutputName("/v/clip.mov", "mp4")).toBe("clip.mp4");
    expect(expectedOutputName("/v/clip.mp4", "webm")).toBe("clip.webm");
  });

  it("空茎名返回空串", () => {
    expect(expectedOutputName("", "mp4")).toBe("");
  });
});

describe("转换常量与默认值", () => {
  it("目标七容器与后端白名单对齐", () => {
    expect([...VIDEO_TARGETS].sort()).toEqual(["avi", "m4v", "mkv", "mov", "mp4", "ts", "webm"]);
  });

  it("三档模式齐全", () => {
    expect([...CONVERT_MODES].sort()).toEqual(["auto", "copyonly", "reencode"]);
  });

  it("默认转 mp4 + 智能模式", () => {
    expect(DEFAULT_CONVERT_PARAMS.target).toBe("mp4");
    expect(DEFAULT_CONVERT_PARAMS.mode).toBe("auto");
    expect(DEFAULT_CONVERT_PARAMS.outputDir).toBe("");
    expect(DEFAULT_CONVERT_PARAMS.overwrite).toBe("increment");
  });

  it("重编码默认挡复刻旧 argv（标准画质 + 很快 + 默认编码器 + 默认音频 + 原分辨率）", () => {
    expect(DEFAULT_CONVERT_PARAMS.quality).toBe("standard");
    expect(DEFAULT_CONVERT_PARAMS.preset).toBe("veryfast");
    expect(DEFAULT_CONVERT_PARAMS.videoEncoder).toBe("libx264");
    expect(DEFAULT_CONVERT_PARAMS.audioBitrate).toBe("default");
    expect(DEFAULT_CONVERT_PARAMS.resolution).toBe("source");
    expect(DEFAULT_CONVERT_PARAMS.accelerator).toBe("cpu");
  });
});

describe("重编码选项表", () => {
  it("七目标编码器允许表与后端同源（首项为默认）", () => {
    expect(ENCODERS_BY_TARGET.mp4).toEqual(["libx264", "libx265"]);
    expect(ENCODERS_BY_TARGET.webm).toEqual(["vp9", "av1"]);
    expect(ENCODERS_BY_TARGET.avi).toEqual(["mpeg4"]);
    expect(ENCODERS_BY_TARGET.ts).toEqual(["libx264", "libx265", "mpeg2video"]);
  });

  it("默认编码器即允许表首项", () => {
    for (const target of VIDEO_TARGETS) {
      expect(defaultEncoderFor(target)).toBe(ENCODERS_BY_TARGET[target][0]);
    }
  });

  it("仅 264 系支持速度档", () => {
    expect(supportsPreset("libx264")).toBe(true);
    expect(supportsPreset("libx265")).toBe(true);
    expect(supportsPreset("vp9")).toBe(false);
    expect(supportsPreset("av1")).toBe(false);
    expect(supportsPreset("mpeg4")).toBe(false);
    expect(supportsPreset("mpeg2video")).toBe(false);
  });

  it("候选集齐全", () => {
    expect([...VIDEO_QUALITIES].sort()).toEqual(["compact", "high", "standard"]);
    expect([...VIDEO_PRESETS].sort()).toEqual(["medium", "slow", "ultrafast", "veryfast"]);
    expect([...AUDIO_BITRATES].sort()).toEqual(["default", "kb128", "kb192", "kb320"]);
    expect([...OUTPUT_RESOLUTIONS].sort()).toEqual(["p1080", "p480", "p720", "source"]);
  });

  it("切目标重置编码器与速度档、保留通用档", () => {
    const next = resetForTarget(
      {
        ...DEFAULT_CONVERT_PARAMS,
        mode: "reencode",
        quality: "compact",
        preset: "slow",
        videoEncoder: "libx265",
        audioBitrate: "kb192",
        resolution: "p720",
        accelerator: "nvenc",
      },
      "webm",
    );
    expect(next.target).toBe("webm");
    expect(next.videoEncoder).toBe("vp9");
    expect(next.preset).toBe("veryfast");
    expect(next.accelerator).toBe("cpu");
    expect(next.quality).toBe("compact");
    expect(next.audioBitrate).toBe("kb192");
    expect(next.resolution).toBe("p720");
  });
});

describe("硬件加速选项表", () => {
  it("目标加速允许表与后端同源（第一批仅 NVENC）", () => {
    expect(ACCELERATORS_BY_TARGET.mp4).toEqual(["cpu", "nvenc"]);
    expect(ACCELERATORS_BY_TARGET.ts).toEqual(["cpu", "nvenc"]);
    expect(ACCELERATORS_BY_TARGET.webm).toEqual(["cpu"]);
    expect(ACCELERATORS_BY_TARGET.avi).toEqual(["cpu"]);
  });

  it("默认加速一律 CPU", () => {
    for (const target of VIDEO_TARGETS) {
      expect(defaultAcceleratorFor(target)).toBe("cpu");
    }
  });

  it("仅 264 系有硬编对应", () => {
    expect(supportsHwEncoding("libx264")).toBe(true);
    expect(supportsHwEncoding("libx265")).toBe(true);
    expect(supportsHwEncoding("mpeg2video")).toBe(false);
    expect(supportsHwEncoding("vp9")).toBe(false);
  });

  it("切无硬编对应的编码器即加速回 CPU，同系保留加速选择", () => {
    const back = resetForEncoder({ ...DEFAULT_CONVERT_PARAMS, accelerator: "nvenc" }, "mpeg2video");
    expect(back.videoEncoder).toBe("mpeg2video");
    expect(back.accelerator).toBe("cpu");
    const keep = resetForEncoder(
      { ...DEFAULT_CONVERT_PARAMS, accelerator: "nvenc", preset: "slow" },
      "libx265",
    );
    expect(keep.accelerator).toBe("nvenc");
    expect(keep.preset).toBe("slow");
  });
});
