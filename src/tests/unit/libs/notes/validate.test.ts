import { describe, expect, it } from "vitest";
import { MAX_NOTE_BODY_LEN, MAX_NOTE_TITLE_LEN, validateNote } from "$libs/notes/validate";

describe("validateNote", () => {
  it("合法输入返回空", () => {
    expect(validateNote("标题", "正文")).toBeNull();
    expect(validateNote("a".repeat(MAX_NOTE_TITLE_LEN), "b".repeat(MAX_NOTE_BODY_LEN))).toBeNull();
  });

  it("空标题与纯空白被拒绝", () => {
    expect(validateNote("", "正文")).toBe("notes_title_required");
    expect(validateNote("   ", "正文")).toBe("notes_title_required");
  });

  it("超长标题被拒绝", () => {
    expect(validateNote("a".repeat(MAX_NOTE_TITLE_LEN + 1), "")).toBe("notes_title_too_long");
  });

  it("超长正文被拒绝", () => {
    expect(validateNote("标题", "b".repeat(MAX_NOTE_BODY_LEN + 1))).toBe("notes_body_too_long");
  });

  it("多错并存时报首个", () => {
    expect(validateNote("", "b".repeat(MAX_NOTE_BODY_LEN + 1))).toBe("notes_title_required");
  });
});
