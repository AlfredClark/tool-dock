import { describe, expect, it } from "vitest";
import { resizeErrorText } from "$components/tools/image/resize/resize-errors";

describe("resizeErrorText", () => {
  it("各错误码映射非空文案", () => {
    const codes = [
      "TooLarge",
      "UnsupportedFormat",
      "DecodeFailed",
      "InvalidSize",
      "EncodeFailed",
      "OutputNotWritable",
      "Skipped",
    ] as const;
    for (const code of codes) {
      expect(resizeErrorText(code).length).toBeGreaterThan(0);
    }
  });

  it("跳过与失败文案不同", () => {
    expect(resizeErrorText("Skipped")).not.toBe(resizeErrorText("DecodeFailed"));
  });
});
