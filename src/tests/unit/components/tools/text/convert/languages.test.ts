import { describe, expect, it } from "vitest";
import { CONVERT_FORMATS } from "$components/tools/text/convert/formats";
import { extensionForFormat } from "$components/tools/text/convert/languages";

describe("数据互转编辑器语言映射", () => {
  it("六种格式均有高亮扩展", () => {
    for (const format of CONVERT_FORMATS) {
      expect(extensionForFormat(format)).toBeDefined();
    }
  });

  it("同格式每次返回新实例（供 Compartment 重配）", () => {
    expect(extensionForFormat("json")).not.toBe(extensionForFormat("json"));
  });
});
