import { describe, expect, it } from "vitest";
import { CONVERT_FORMATS, formatLabel } from "$components/tools/text/convert/formats";
import {
  convertErrorText,
  errorLineText,
  toBindingFormat,
} from "$components/tools/text/convert/convert-errors";
import type { ErrorCode } from "$libs/commands/bindings";

const ALL_CODES: ErrorCode[] = [
  "TooLarge",
  "UnknownFormat",
  "ParseFailed",
  "UnsupportedInput",
  "UnsupportedOutput",
  "NonTableRoot",
  "UnsupportedValue",
];

describe("数据互转错误映射", () => {
  it("本地格式直转契约格式（取值一致，仅收窄类型）", () => {
    for (const format of CONVERT_FORMATS) {
      expect(toBindingFormat(format)).toBe(format);
    }
  });

  it("全部错误码都有非空文案", () => {
    for (const code of ALL_CODES) {
      expect(convertErrorText(code, null).length).toBeGreaterThan(0);
    }
  });

  it("输入输出不支持时文案带格式名", () => {
    expect(convertErrorText("UnsupportedInput", "json")).toContain(formatLabel("json"));
    expect(convertErrorText("UnsupportedOutput", "xml")).toContain(formatLabel("xml"));
  });

  it("未知格式回落原文不白屏", () => {
    expect(convertErrorText("UnsupportedInput", null)).toContain("unknown");
  });

  it("错误位置文案：缺行号返回 null，缺列只显示行", () => {
    expect(errorLineText(null, null)).toBeNull();
    expect(errorLineText(null, 5)).toBeNull();
    expect(errorLineText(3, null)).toContain("3");
    const full = errorLineText(3, 5);
    expect(full).toContain("3");
    expect(full).toContain("5");
  });
});
