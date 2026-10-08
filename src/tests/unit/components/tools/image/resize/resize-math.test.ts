import { describe, expect, it } from "vitest";
import {
  RATIO_ITEMS,
  applyExactRatio,
  basenameOf,
  extensionOf,
  formatBytes,
  isImageExtension,
  isJpegOutput,
  parseRatio,
  planDimensions,
  ratioPlaceholder,
  rotateSize,
} from "$components/tools/image/resize/resize-math";
import type { ResizeMode } from "$libs/commands/bindings";
import {
  DEFAULT_RESIZE_PARAMS,
  type ResizeParamsState,
} from "$components/tools/image/resize/resize-types";

// 纯前端镜像：与后端 `plan_dimensions` 同语义，断言向量与后端单测对齐，
// 改一处必须同步另一处（见 `features/image_resize.rs` 的同名用例）。
function boxMode(width: number | null, height: number | null, lock: boolean): ResizeMode {
  return { kind: "widthheight", width, height, lock_ratio: lock };
}

describe("planDimensions", () => {
  it("单边按比例换算", () => {
    expect(planDimensions(200, 100, boxMode(100, null, true), false)).toEqual({
      width: 100,
      height: 50,
    });
    expect(planDimensions(200, 100, boxMode(null, 50, true), false)).toEqual({
      width: 100,
      height: 50,
    });
  });

  it("锁定双填内适应，不锁定拉伸", () => {
    expect(planDimensions(200, 100, boxMode(100, 100, true), false)).toEqual({
      width: 100,
      height: 50,
    });
    expect(planDimensions(200, 100, boxMode(100, 100, false), false)).toEqual({
      width: 100,
      height: 100,
    });
  });

  it("拒绝空与零值", () => {
    expect(planDimensions(200, 100, boxMode(null, null, true), false)).toBeNull();
    expect(planDimensions(200, 100, boxMode(0, 10, false), false)).toBeNull();
    expect(planDimensions(0, 100, boxMode(10, null, true), false)).toBeNull();
  });

  it("百分比换算与边界", () => {
    expect(planDimensions(200, 100, { kind: "percent", percent: 50 }, false)).toEqual({
      width: 100,
      height: 50,
    });
    expect(planDimensions(200, 100, { kind: "percent", percent: 0 }, false)).toBeNull();
    expect(planDimensions(200, 100, { kind: "percent", percent: 1001 }, false)).toBeNull();
  });

  it("精确模式画布规则", () => {
    const contain: ResizeMode = { kind: "exact", width: 100, height: 100, fit: "contain" };
    expect(planDimensions(200, 100, contain, false)).toEqual({ width: 100, height: 50 });
    for (const fit of ["stretch", "cover", "pad"] as const) {
      const exact: ResizeMode = { kind: "exact", width: 100, height: 100, fit };
      expect(planDimensions(200, 100, exact, false)).toEqual({ width: 100, height: 100 });
    }
  });

  it("不放大时整体回落原尺寸", () => {
    expect(planDimensions(200, 100, boxMode(400, null, true), true)).toEqual({
      width: 200,
      height: 100,
    });
    expect(planDimensions(200, 100, { kind: "percent", percent: 200 }, true)).toEqual({
      width: 200,
      height: 100,
    });
    expect(planDimensions(200, 100, { kind: "percent", percent: 200 }, false)).toEqual({
      width: 400,
      height: 200,
    });
  });

  it("拦截超限尺寸", () => {
    const huge: ResizeMode = { kind: "exact", width: 16385, height: 10, fit: "stretch" };
    expect(planDimensions(10, 10, huge, false)).toBeNull();
    expect(planDimensions(10, 10, boxMode(16384, 16384, false), false)).toBeNull();
  });
});

describe("formatBytes", () => {
  it("按 B/KB/MB 分档", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.00 MB");
    expect(formatBytes(-1)).toBe("0 B");
  });
});

describe("文件名工具", () => {
  it("取扩展名与 basename", () => {
    expect(extensionOf("PHOTO.JPG")).toBe("jpg");
    expect(extensionOf("noext")).toBe("");
    expect(basenameOf("/a/b/c.png")).toBe("c.png");
    expect(basenameOf("C:\\a\\b.bmp")).toBe("b.bmp");
  });

  it("白名单判定", () => {
    expect(isImageExtension("a.webp")).toBe(true);
    expect(isImageExtension("a.txt")).toBe(false);
  });
});

describe("parseRatio", () => {
  it("解析宽高并拒绝非法", () => {
    expect(parseRatio("16:9")).toEqual({ width: 16, height: 9 });
    expect(parseRatio("1:1")).toEqual({ width: 1, height: 1 });
    expect(parseRatio("free")).toBeNull();
    expect(parseRatio("")).toBeNull();
    expect(parseRatio("16")).toBeNull();
    expect(parseRatio("0:9")).toBeNull();
    expect(parseRatio("x:9")).toBeNull();
  });
});

describe("ratioPlaceholder", () => {
  it("示例符合对应比例，自由与未知回落空", () => {
    expect(ratioPlaceholder("16:9")).toEqual({ width: "1280", height: "720" });
    expect(ratioPlaceholder("1:1")).toEqual({ width: "512", height: "512" });
    expect(ratioPlaceholder("free")).toBeNull();
    expect(ratioPlaceholder("bogus")).toBeNull();
  });

  it("全部候选都有示例", () => {
    for (const ratio of RATIO_ITEMS) {
      expect(ratioPlaceholder(ratio)).not.toBeNull();
    }
  });
});

describe("rotateSize", () => {
  it("90 系交换宽高，其余原样", () => {
    expect(rotateSize({ width: 200, height: 100 }, "cw90")).toEqual({
      width: 100,
      height: 200,
    });
    expect(rotateSize({ width: 200, height: 100 }, "ccw90")).toEqual({
      width: 100,
      height: 200,
    });
    expect(rotateSize({ width: 200, height: 100 }, "cw180")).toEqual({
      width: 200,
      height: 100,
    });
    expect(rotateSize({ width: 200, height: 100 }, "fliphorizontal")).toEqual({
      width: 200,
      height: 100,
    });
    expect(rotateSize({ width: 200, height: 100 }, "none")).toEqual({
      width: 200,
      height: 100,
    });
  });
});

describe("isJpegOutput", () => {
  it("显式 JPEG 恒为真，原样跟随 JPEG 输入", () => {
    expect(isJpegOutput("jpeg", "jpeg")).toBe(true);
    expect(isJpegOutput("png", "jpeg")).toBe(true);
    expect(isJpegOutput("jpeg", "original")).toBe(true);
    expect(isJpegOutput("png", "original")).toBe(false);
    expect(isJpegOutput("png", "png")).toBe(false);
    expect(isJpegOutput("gif", "original")).toBe(false);
  });
});

describe("applyExactRatio", () => {
  function exactState(partial: Partial<ResizeParamsState> = {}): ResizeParamsState {
    return { ...DEFAULT_RESIZE_PARAMS, modeKind: "exact", exactRatio: "16:9", ...partial };
  }

  it("自由比例原样返回", () => {
    const state = exactState({ exactRatio: "free", exactWidth: "160" });
    expect(applyExactRatio(state, "exactWidth").exactHeight).toBe("");
  });

  it("按编辑边换算另一边", () => {
    expect(applyExactRatio(exactState({ exactWidth: "160" }), "exactWidth").exactHeight).toBe("90");
    expect(applyExactRatio(exactState({ exactHeight: "90" }), "exactHeight").exactWidth).toBe(
      "160",
    );
  });

  it("四舍五入且至少 1px", () => {
    // 100 按 16:9 得 56.25 → 56
    expect(applyExactRatio(exactState({ exactWidth: "100" }), "exactWidth").exactHeight).toBe("56");
  });

  it("被编辑边非法不联动", () => {
    const state = exactState({ exactWidth: "abc", exactHeight: "50" });
    expect(applyExactRatio(state, "exactWidth").exactHeight).toBe("50");
  });

  it("非精确模式不联动", () => {
    const state: ResizeParamsState = {
      ...DEFAULT_RESIZE_PARAMS,
      modeKind: "box",
      exactRatio: "16:9",
    };
    expect(applyExactRatio(state, "exactWidth").exactHeight).toBe("");
  });
});
