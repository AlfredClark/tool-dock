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
  it("前后缀添加拼接", () => {
    expect(
      applyRule(
        "pic",
        { ...createRule("affix"), mode: "add", position: "prefix", text: "IMG_" },
        0,
      ),
    ).toBe("IMG_pic");
    expect(
      applyRule("pic", { ...createRule("affix"), mode: "add", position: "suffix", text: "_v2" }, 0),
    ).toBe("pic_v2");
  });

  it("前后缀删除按具体文本匹配，不匹配与空文本跳过", () => {
    expect(
      applyRule(
        "IMG_pic",
        { ...createRule("affix"), mode: "remove", position: "prefix", text: "IMG_" },
        0,
      ),
    ).toBe("pic");
    expect(
      applyRule(
        "pic_v2",
        { ...createRule("affix"), mode: "remove", position: "suffix", text: "_v2" },
        0,
      ),
    ).toBe("pic");
    // 位置不匹配整条跳过
    expect(
      applyRule(
        "IMG_pic",
        { ...createRule("affix"), mode: "remove", position: "suffix", text: "IMG_" },
        0,
      ),
    ).toBe("IMG_pic");
    expect(
      applyRule(
        "pic_v2",
        { ...createRule("affix"), mode: "remove", position: "prefix", text: "_v2" },
        0,
      ),
    ).toBe("pic_v2");
    expect(
      applyRule("pic", { ...createRule("affix"), mode: "remove", position: "prefix", text: "" }, 0),
    ).toBe("pic");
  });

  it("大小写四种模式", () => {
    expect(applyRule("hELLo", { ...createRule("case"), mode: "upper" }, 0)).toBe("HELLO");
    expect(applyRule("hELLo", { ...createRule("case"), mode: "lower" }, 0)).toBe("hello");
    expect(applyRule("hELLo WoRLD", { ...createRule("case"), mode: "sentence" }, 0)).toBe(
      "Hello world",
    );
    expect(applyRule("", { ...createRule("case"), mode: "sentence" }, 0)).toBe("");
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

  it("自动编号格式：$n 为编号占位，$$ 转义，无占位即静态文本", () => {
    expect(
      applyRule(
        "pic",
        { ...createRule("number"), position: "prefix", separator: "_", format: "No.$n" },
        1,
      ),
    ).toBe("No.002_pic");
    expect(
      applyRule(
        "pic",
        { ...createRule("number"), position: "suffix", separator: "-", format: "($n)" },
        0,
      ),
    ).toBe("pic-(001)");
    // 无占位即静态文本
    expect(
      applyRule(
        "pic",
        { ...createRule("number"), position: "prefix", separator: "_", format: "v" },
        0,
      ),
    ).toBe("v_pic");
    // $$ 转义为字面 $，$$$n 即 $ + 编号
    expect(
      applyRule(
        "pic",
        { ...createRule("number"), position: "prefix", separator: "", format: "$$$n" },
        4,
      ),
    ).toBe("$005pic");
  });

  it("范围切片：锚点定位 + 范围向左右截取，超界截断", () => {
    // 需求示例：锚点 1 取右 2 个 → bc；锚点 -1（末字符）取左 2 个 → fg
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "1", length: "2" }, 0)).toBe(
      "bc",
    );
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "-1", length: "-2" }, 0)).toBe(
      "fg",
    );
    // 锚点 0 即开头
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "0", length: "3" }, 0)).toBe(
      "abc",
    );
    // 负锚点超界钳到开头
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "-99", length: "2" }, 0)).toBe(
      "ab",
    );
    // 正向超界截断
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "5", length: "9" }, 0)).toBe(
      "fg",
    );
    // 锚点落在末尾之后 → 空串
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "9", length: "2" }, 0)).toBe("");
    // 锚点 0 向左：仅锚点位在范围内
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "0", length: "-2" }, 0)).toBe(
      "a",
    );
    // 范围 0 取空
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "2", length: "0" }, 0)).toBe("");
    // 空参静默跳过
    expect(applyRule("abcdefg", createRule("slice"), 0)).toBe("abcdefg");
    // 非法参数整条跳过
    expect(applyRule("abcdefg", { ...createRule("slice"), anchor: "x", length: "2" }, 0)).toBe(
      "abcdefg",
    );
  });

  it("规范化六种预设各做一件事", () => {
    expect(applyRule("  pic  ", { ...createRule("normalize"), preset: "trim" }, 0)).toBe("pic");
    expect(applyRule("a b\tc", { ...createRule("normalize"), preset: "remove-spaces" }, 0)).toBe(
      "abc",
    );
    expect(
      applyRule('a/b:c*d?e"f<g>h|i', { ...createRule("normalize"), preset: "remove-illegal" }, 0),
    ).toBe("abcdefghi");
    expect(applyRule("a  b\tc", { ...createRule("normalize"), preset: "collapse-spaces" }, 0)).toBe(
      "a b c",
    );
    expect(
      applyRule("a  b\tc", { ...createRule("normalize"), preset: "spaces-to-underscore" }, 0),
    ).toBe("a_b_c");
    expect(
      applyRule("a  b\tc", { ...createRule("normalize"), preset: "spaces-to-hyphen" }, 0),
    ).toBe("a-b-c");
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

  it("前后缀默认操作为添加", () => {
    expect(createRule("affix")).toMatchObject({ mode: "add" });
  });

  it("候选顺序：规范化紧随大小写之后，范围切片居末", () => {
    expect(RULE_KINDS).toEqual([
      "affix",
      "case",
      "normalize",
      "replace",
      "regex",
      "number",
      "slice",
    ]);
  });

  it("范围切片默认锚点为 0 且范围留空（新建静默无操作）", () => {
    expect(createRule("slice")).toMatchObject({ anchor: "0", length: "" });
  });

  it("规范化默认预设为去首尾空格", () => {
    expect(createRule("normalize")).toMatchObject({ preset: "trim" });
  });

  it("自动编号默认格式为 $n（与旧行为一致）", () => {
    expect(createRule("number")).toMatchObject({ format: "$n" });
  });
});

describe("validateRule", () => {
  it("三类非法参数检出", () => {
    expect(validateRule({ ...createRule("regex"), pattern: "([a-z" })).toBe("bad-regex");
    expect(validateRule({ ...createRule("number"), start: "1", step: "x", digits: "3" })).toBe(
      "bad-number",
    );
    expect(validateRule({ ...createRule("slice"), anchor: "1", length: "x" })).toBe("bad-slice");
  });

  it("合法与无参规则返回空", () => {
    expect(validateRule(createRule("affix"))).toBeNull();
    expect(validateRule(createRule("normalize"))).toBeNull();
    expect(validateRule(createRule("slice"))).toBeNull();
    expect(validateRule({ ...createRule("slice"), anchor: "-1", length: "-2" })).toBeNull();
  });
});

describe("规则数组变换", () => {
  it("moveRuleTo 按下标移动，越界钳制、同位返回原引用", () => {
    const rules = [createRule("affix"), createRule("replace"), createRule("case")];
    const first = rules[0]?.id ?? "";
    expect(moveRuleTo(rules, first, 2).map((rule) => rule.kind)).toEqual([
      "replace",
      "case",
      "affix",
    ]);
    expect(moveRuleTo(rules, first, 99).map((rule) => rule.kind)).toEqual([
      "replace",
      "case",
      "affix",
    ]);
    expect(moveRuleTo(rules, first, -5)).toBe(rules);
    expect(moveRuleTo(rules, first, 0)).toBe(rules);
    expect(moveRuleTo(rules, "missing", 1)).toBe(rules);
  });

  it("toggle/update/remove 按 id 生效", () => {
    const rules = [createRule("affix"), createRule("replace")];
    const id = rules[0]?.id ?? "";
    expect(toggleRule(rules, id, false)[0]).toMatchObject({ enabled: false });
    const next = { ...createRule("case"), id };
    expect(updateRule(rules, next)[0]).toMatchObject({ kind: "case", id });
    expect(removeRule(rules, id)).toHaveLength(1);
  });
});
