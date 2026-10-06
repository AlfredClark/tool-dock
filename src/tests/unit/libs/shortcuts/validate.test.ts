import { describe, expect, it } from "vitest";
import { MAX_SHORTCUT_LEN, normalizeShortcut, validateShortcut } from "$libs/shortcuts/validate";

describe("normalizeShortcut", () => {
  it("去空白并归一修饰键大小写", () => {
    expect(normalizeShortcut("  ctrl+alt+k ")).toBe("Ctrl+Alt+k");
    expect(normalizeShortcut("Ctrl + Shift + D")).toBe("Ctrl+Shift+D");
  });

  it("空片段不吞掉", () => {
    expect(normalizeShortcut("Ctrl++A")).toBe("Ctrl++A");
  });
});

describe("validateShortcut", () => {
  it("合法组合通过", () => {
    expect(validateShortcut("Ctrl+Shift+D")).toBeNull();
    expect(validateShortcut("ctrl+alt+k")).toBeNull();
    expect(validateShortcut("CommandOrControl+Shift+X")).toBeNull();
    expect(validateShortcut("Alt+F1")).toBeNull();
  });

  it("空输入拒绝", () => {
    expect(validateShortcut("")).toBe("demo_shortcut_invalid");
    expect(validateShortcut("   ")).toBe("demo_shortcut_invalid");
  });

  it("超长拒绝", () => {
    expect(validateShortcut(`Ctrl+${"A".repeat(MAX_SHORTCUT_LEN)}`)).toBe("demo_shortcut_invalid");
  });

  it("无修饰键拒绝", () => {
    expect(validateShortcut("A")).toBe("demo_shortcut_invalid");
  });

  it("未知修饰键与非法主键拒绝", () => {
    expect(validateShortcut("Win+A")).toBe("demo_shortcut_invalid");
    expect(validateShortcut("Ctrl++A")).toBe("demo_shortcut_invalid");
    expect(validateShortcut("Ctrl+F25")).toBe("demo_shortcut_invalid");
  });

  it("系统保留组合拒绝", () => {
    expect(validateShortcut("Alt+F4")).toBe("demo_shortcut_blocked");
    expect(validateShortcut("ctrl+alt+delete")).toBe("demo_shortcut_blocked");
  });
});
