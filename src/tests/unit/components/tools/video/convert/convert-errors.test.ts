import { describe, expect, it } from "vitest";
import { convertErrorText, convertNoteText } from "$components/tools/video/convert/convert-errors";

describe("convertErrorText", () => {
  it("全部错误码均有文案", () => {
    for (const code of [
      "UnsupportedFormat",
      "OutputNotWritable",
      "FfmpegFailed",
      "Skipped",
      "InvalidOptions",
    ] as const) {
      expect(convertErrorText(code).length).toBeGreaterThan(0);
    }
  });
});

describe("convertNoteText", () => {
  it("流复制、硬加速与 CPU 重编码各有文案", () => {
    expect(convertNoteText(true, false).length).toBeGreaterThan(0);
    expect(convertNoteText(false, true).length).toBeGreaterThan(0);
    expect(convertNoteText(false, false).length).toBeGreaterThan(0);
    expect(convertNoteText(true, false)).not.toBe(convertNoteText(false, false));
    expect(convertNoteText(false, true)).not.toBe(convertNoteText(false, false));
  });
});
