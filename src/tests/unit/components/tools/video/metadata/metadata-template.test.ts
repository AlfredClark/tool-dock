import { describe, expect, it } from "vitest";
import {
  expandTemplate,
  findUnknownNames,
  formatClock,
  formatDate,
  isPlaceholderName,
  videoFileExt,
  videoFileStem,
  type PlaceholderContext,
} from "$components/tools/video/metadata/metadata-template";

const CTX: PlaceholderContext = {
  filename: "clip",
  ext: "mp4",
  width: 1920,
  height: 1080,
  durationSeconds: 65.7,
};
const TODAY = new Date(2026, 9, 8);

describe("expandTemplate", () => {
  it("全量占位符一次展开", () => {
    const { text, unknown } = expandTemplate(
      "%filename%.%ext% %width%x%height% %duration%s %date%",
      CTX,
      TODAY,
    );

    expect(text).toBe("clip.mp4 1920x1080 65s 2026-10-08");
    expect(unknown).toEqual([]);
  });

  it("转义与孤百分号按字面保留", () => {
    expect(expandTemplate("100%% 覆盖率", CTX, TODAY)).toEqual({
      text: "100% 覆盖率",
      unknown: [],
    });
    expect(expandTemplate("未闭合 100%", CTX, TODAY)).toEqual({
      text: "未闭合 100%",
      unknown: [],
    });
  });

  it("未知占位符原样保留并上报", () => {
    const { text, unknown } = expandTemplate("标题 %filname% %filname%", CTX, TODAY);

    expect(text).toBe("标题 %filname% %filname%");
    expect(unknown).toEqual(["filname"]);
  });

  it("上下文缺值按未知处理", () => {
    const { text, unknown } = expandTemplate("%width%x%height% %duration%s", {
      ...CTX,
      width: null,
      height: null,
      durationSeconds: null,
    });

    expect(text).toBe("%width%x%height% %duration%s");
    expect(unknown).toEqual(["width", "height", "duration"]);
  });

  it("空模板与纯文本直通", () => {
    expect(expandTemplate("", CTX, TODAY)).toEqual({ text: "", unknown: [] });
    expect(expandTemplate("纯文本", CTX, TODAY)).toEqual({ text: "纯文本", unknown: [] });
  });
});

describe("findUnknownNames", () => {
  it("只认已登记名，转义与孤百分号跳过", () => {
    expect(findUnknownNames("%filename% 100%% %bad%")).toEqual(["bad"]);
    expect(findUnknownNames("%filename% %ext%")).toEqual([]);
    expect(findUnknownNames("100%")).toEqual([]);
  });
});

describe("isPlaceholderName", () => {
  it("六占位符全认", () => {
    for (const name of ["filename", "ext", "width", "height", "duration", "date"]) {
      expect(isPlaceholderName(name)).toBe(true);
    }
    expect(isPlaceholderName("filname")).toBe(false);
  });
});

describe("formatDate", () => {
  it("本地日期补零", () => {
    expect(formatDate(TODAY)).toBe("2026-10-08");
    expect(formatDate(new Date(2026, 0, 5))).toBe("2026-01-05");
  });
});

describe("formatClock", () => {
  it("分秒补零并向下取整", () => {
    expect(formatClock(65.7)).toBe("01:05");
    expect(formatClock(5)).toBe("00:05");
    expect(formatClock(-3)).toBe("00:00");
  });
});

describe("videoFileStem", () => {
  it("兼容双分隔符与多点文件名", () => {
    expect(videoFileStem("C:\\videos\\my.clip.mp4")).toBe("my.clip");
    expect(videoFileStem("/home/u/clip.mp4")).toBe("clip");
    expect(videoFileStem("clip")).toBe("clip");
  });
});

describe("videoFileExt", () => {
  it("小写无点后缀，无后缀回落空串", () => {
    expect(videoFileExt("/home/u/CLIP.MP4")).toBe("mp4");
    expect(videoFileExt("clip")).toBe("");
  });
});
