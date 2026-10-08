import { describe, expect, it } from "vitest";
import { formatDownloadSize } from "$components/tools/video/metadata/metadata-types";

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
