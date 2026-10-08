import { describe, expect, it } from "vitest";
import {
  CONVERT_FORMATS,
  DROP_MAX_BYTES,
  INPUT_FORMATS,
  OUTPUT_FORMATS,
  basenameOf,
  formatFromExtension,
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

  it("扩展名映射格式（大小写不敏感），未知返回 null", () => {
    expect(formatFromExtension("data.json")).toBe("json");
    expect(formatFromExtension("data.YAML")).toBe("yaml");
    expect(formatFromExtension("data.yml")).toBe("yaml");
    expect(formatFromExtension("data.xml")).toBe("xml");
    expect(formatFromExtension("data.toml")).toBe("toml");
    expect(formatFromExtension("data.ini")).toBe("ini");
    expect(formatFromExtension("data.properties")).toBe("properties");
    expect(formatFromExtension("data.txt")).toBeNull();
    expect(formatFromExtension("noext")).toBeNull();
    expect(formatFromExtension("")).toBeNull();
  });

  it("路径末段文件名兼容两种分隔符", () => {
    expect(basenameOf("/tmp/a.json")).toBe("a.json");
    expect(basenameOf("C:\\data\\b.yaml")).toBe("b.yaml");
    expect(basenameOf("plain.toml")).toBe("plain.toml");
  });

  it("拖放大小上限与后端转换上限对齐（1 MiB）", () => {
    expect(DROP_MAX_BYTES).toBe(1024 * 1024);
  });
});
