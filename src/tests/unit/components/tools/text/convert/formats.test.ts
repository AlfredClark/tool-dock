import { describe, expect, it } from "vitest";
import {
  CONVERT_FORMATS,
  INPUT_FORMATS,
  OUTPUT_FORMATS,
  formatLabel,
  inputFormatItems,
  isConvertFormat,
  isInputFormat,
  outputFormatItems,
} from "$components/tools/text/convert/formats";

describe("数据互转格式定义", () => {
  it("支持六种转换格式", () => {
    expect(CONVERT_FORMATS).toEqual(["json", "yaml", "xml", "toml", "ini", "properties"]);
    expect(OUTPUT_FORMATS).toEqual(CONVERT_FORMATS);
  });

  it("输入端比输出端多一个自动识别", () => {
    expect(INPUT_FORMATS).toEqual(["auto", ...CONVERT_FORMATS]);
    expect(INPUT_FORMATS).toHaveLength(OUTPUT_FORMATS.length + 1);
  });

  it("格式展示名为大写字面量", () => {
    expect(formatLabel("json")).toBe("JSON");
    expect(formatLabel("yaml")).toBe("YAML");
    expect(formatLabel("xml")).toBe("XML");
    expect(formatLabel("toml")).toBe("TOML");
    expect(formatLabel("ini")).toBe("INI");
    expect(formatLabel("properties")).toBe("Properties");
  });

  it("收窄谓词拒绝非法取值", () => {
    expect(isConvertFormat("json")).toBe(true);
    expect(isConvertFormat("auto")).toBe(false);
    expect(isConvertFormat("csv")).toBe(false);
    expect(isConvertFormat(undefined)).toBe(false);
    expect(isInputFormat("auto")).toBe(true);
    expect(isInputFormat("yaml")).toBe(true);
    expect(isInputFormat("csv")).toBe(false);
  });

  it("输入端候选项含自动识别，输出端不含", () => {
    const inputValues = inputFormatItems().map((item) => item.value);
    expect(inputValues).toContain("auto");
    const outputValues = outputFormatItems().map((item) => item.value);
    expect(outputValues).not.toContain("auto");
    expect(outputValues).toHaveLength(CONVERT_FORMATS.length);
  });
});
