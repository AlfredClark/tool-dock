import { describe, expect, it } from "vitest";
import { coverNoteText, videoErrorText } from "$components/tools/video/metadata/video-errors";

describe("videoErrorText", () => {
  it("全部错误码均有文案", () => {
    for (const code of [
      "UnsupportedFormat",
      "InvalidTags",
      "OutputNotWritable",
      "FfmpegFailed",
      "Skipped",
    ] as const) {
      expect(videoErrorText(code).length).toBeGreaterThan(0);
    }
  });
});

describe("coverNoteText", () => {
  it("保持无备注，其余均有文案", () => {
    expect(coverNoteText("Kept")).toBeNull();
    for (const result of [
      "Embedded",
      "Cleared",
      "SkippedNoFile",
      "SkippedUnsupported",
      "SkippedInvalid",
    ] as const) {
      expect(coverNoteText(result)?.length).toBeGreaterThan(0);
    }
  });
});
