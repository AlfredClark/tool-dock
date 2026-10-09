import { describe, expect, it } from "vitest";
import {
  RULE_KINDS,
  applyRule,
  applyRules,
  createRule,
  moveRuleTo,
  previewName,
  removeRule,
  splitName,
  toggleRule,
  updateRule,
  validateRule,
} from "$components/tools/system/renamer/renamer-rules";
import type { RenamerRule } from "$components/tools/system/renamer/renamer-types";

describe("splitName", () => {
  it("多点文件名取最后一段为扩展", () => {
    expect(splitName("archive.tar.gz")).toEqual({ stem: "archive.tar", ext: ".gz" });
  });

  it("点文件与无扩展名视为无扩展", () => {
    expect(splitName(".gitignore")).toEqual({ stem: ".gitignore", ext: "" });
    expect(splitName("README")).toEqual({ stem: "README", ext: "" });
    expect(splitName("a.")).toEqual({ stem: "a.", ext: "" });
  });
});

describe("applyRule", () => {
  it("前后缀拼接", () => {
    expect(applyRule("pic", { ...createRule("affix"), position: "prefix", text: "IMG_" }, 0)).toBe(
      "IMG_pic",
    );
    expect(applyRule("pic", { ...createRule("affix"), position: "suffix", text: "_v2" }, 0)).toBe(
      "pic_v2",
    );
  });

  it("前后缀按字符数删除，非法与超长处理", () => {
    expect(
      applyRule("IMG_pic", { ...createRule("strip"), position: "prefix", count: "4" }, 0),
    ).toBe("pic");
    expect(applyRule("pic_v2", { ...createRule("strip"), position: "suffix", count: "3" }, 0)).toBe(
      "pic",
    );
    // 非法字符数整条跳过
    expect(applyRule("pic", { ...createRule("strip"), position: "prefix", count: "abc" }, 0)).toBe(
      "pic",
    );
    expect(applyRule("pic", { ...createRule("strip"), position: "prefix", count: "" }, 0)).toBe(
      "pic",
    );
    // 删超长得空串
    expect(applyRule("pic", { ...createRule("strip"), position: "prefix", count: "99" }, 0)).toBe(
      "",
    );
  });

  it("大小写三种模式", () => {
    expect(applyRule("hELLo", { ...createRule("case"), mode: "upper" }, 0)).toBe("HELLO");
    expect(applyRule("hELLo", { ...createRule("case"), mode: "lower" }, 0)).toBe("hello");
    expect(applyRule("hello_world-foo bar", { ...createRule("case"), mode: "title" }, 0)).toBe(
      "Hello_World-Foo Bar",
    );
  });

  it("查找替换：空查找跳过，大小写开关", () => {
    expect(
      applyRule(
        "aaa",
        { ...createRule("replace"), find: "a", replacement: "b", matchCase: true },
        0,
      ),
    ).toBe("bbb");
    expect(
      applyRule(
        "aAa",
        { ...createRule("replace"), find: "a", replacement: "b", matchCase: false },
        0,
      ),
    ).toBe("bbb");
    expect(
      applyRule(
        "pic",
        { ...createRule("replace"), find: "", replacement: "x", matchCase: true },
        0,
      ),
    ).toBe("pic");
  });

  it("正则替换：分组引用与非法 pattern 跳过", () => {
    expect(
      applyRule(
        "IMG_2024",
        { ...createRule("regex"), pattern: "IMG_(\\d+)", replacement: "photo-$1" },
        0,
      ),
    ).toBe("photo-2024");
    expect(
      applyRule("pic", { ...createRule("regex"), pattern: "([a-z", replacement: "x" }, 0),
    ).toBe("pic");
    expect(applyRule("pic", { ...createRule("regex"), pattern: "", replacement: "x" }, 0)).toBe(
      "pic",
    );
  });

  it("自动编号：序号按 index 补零拼接，非法参数跳过", () => {
    const rule: RenamerRule = {
      ...createRule("number"),
      start: "1",
      step: "5",
      digits: "3",
      position: "prefix",
      separator: "_",
    };
    expect(applyRule("pic", rule, 0)).toBe("001_pic");
    expect(applyRule("pic", rule, 2)).toBe("011_pic");
    expect(
      applyRule("pic", { ...createRule("number"), start: "x", step: "1", digits: "3" }, 0),
    ).toBe("pic");
  });

  it("规范化：去空白转下划线去非法字符", () => {
    expect(applyRule("  my  photo: 01? ", createRule("normalize"), 0)).toBe("my_photo_01");
  });
});

describe("applyRules", () => {
  it("按数组顺序应用，禁用跳过", () => {
    const rules: RenamerRule[] = [
      { ...createRule("case"), mode: "lower" },
      { ...createRule("affix"), position: "suffix", text: "_v2", enabled: false },
      { ...createRule("affix"), position: "suffix", text: "_ok" },
    ];
    expect(applyRules("PIC", rules, 0)).toBe("pic_ok");
  });
});

describe("previewName", () => {
  it("零规则返回原名", () => {
    expect(previewName("a.png", [], 0)).toBe("a.png");
  });

  it("扩展名保留，规则只改主名", () => {
    const rules: RenamerRule[] = [{ ...createRule("case"), mode: "upper" }];
    expect(previewName("photo.png", rules, 0)).toBe("PHOTO.png");
    expect(previewName("archive.tar.gz", rules, 0)).toBe("ARCHIVE.TAR.gz");
  });
});

describe("createRule", () => {
  it("七种类型默认值完整", () => {
    for (const kind of RULE_KINDS) {
      const rule = createRule(kind);
      expect(rule.kind).toBe(kind);
      expect(rule.enabled).toBe(true);
      expect(rule.id.length).toBeGreaterThan(0);
    }
    expect(RULE_KINDS).toHaveLength(7);
  });
});

describe("validateRule", () => {
  it("三类非法参数检出", () => {
    expect(validateRule({ ...createRule("strip"), count: "x" })).toBe("bad-count");
    expect(validateRule({ ...createRule("regex"), pattern: "([a-z" })).toBe("bad-regex");
    expect(validateRule({ ...createRule("number"), start: "1", step: "x", digits: "3" })).toBe(
      "bad-number",
    );
  });

  it("合法与无参规则返回空", () => {
    expect(validateRule({ ...createRule("strip"), count: "2" })).toBeNull();
    expect(validateRule(createRule("normalize"))).toBeNull();
    expect(validateRule(createRule("affix"))).toBeNull();
  });
});

describe("规则数组变换", () => {
  it("moveRuleTo 按下标移动，越界钳制、同位返回原引用", () => {
    const rules = [createRule("affix"), createRule("strip"), createRule("case")];
    const first = rules[0]?.id ?? "";
    expect(moveRuleTo(rules, first, 2).map((rule) => rule.kind)).toEqual([
      "strip",
      "case",
      "affix",
    ]);
    expect(moveRuleTo(rules, first, 99).map((rule) => rule.kind)).toEqual([
      "strip",
      "case",
      "affix",
    ]);
    expect(moveRuleTo(rules, first, -5)).toBe(rules);
    expect(moveRuleTo(rules, first, 0)).toBe(rules);
    expect(moveRuleTo(rules, "missing", 1)).toBe(rules);
  });

  it("toggle/update/remove 按 id 生效", () => {
    const rules = [createRule("affix"), createRule("strip")];
    const id = rules[0]?.id ?? "";
    expect(toggleRule(rules, id, false)[0]).toMatchObject({ enabled: false });
    const next = { ...createRule("case"), id };
    expect(updateRule(rules, next)[0]).toMatchObject({ kind: "case", id });
    expect(removeRule(rules, id)).toHaveLength(1);
  });
});
