import { describe, expect, it } from "vitest";
import type { RenamerFileItem } from "$components/tools/system/renamer/renamer-types";
import {
  basenameOf,
  filterByQuery,
  removeChecked,
  setCheckedForIds,
  sortItems,
} from "$components/tools/system/renamer/renamer-utils";

function makeItem(id: string, partial: Partial<RenamerFileItem> = {}): RenamerFileItem {
  return { id, path: `/tmp/${id}`, name: id, newName: id, checked: false, error: null, ...partial };
}

describe("filterByQuery", () => {
  it("空查询返回原数组引用", () => {
    const items = [makeItem("a.png"), makeItem("b.png")];

    expect(filterByQuery(items, "")).toBe(items);
    expect(filterByQuery(items, "   ")).toBe(items);
  });

  it("按文件名子串命中且大小写不敏感", () => {
    const items = [makeItem("Photo.PNG"), makeItem("doc.txt")];

    expect(filterByQuery(items, "photo").map((item) => item.id)).toEqual(["Photo.PNG"]);
    expect(filterByQuery(items, "不存在zzz")).toEqual([]);
  });
});

describe("sortItems", () => {
  it("升/降序排列且不改原数组", () => {
    const items = [makeItem("b.png"), makeItem("a.png")];

    expect(sortItems(items, "name-asc").map((item) => item.id)).toEqual(["a.png", "b.png"]);
    expect(sortItems(items, "name-desc").map((item) => item.id)).toEqual(["b.png", "a.png"]);
    expect(items.map((item) => item.id)).toEqual(["b.png", "a.png"]);
  });
});

describe("setCheckedForIds", () => {
  it("只改指定 id 的勾选态", () => {
    const items = [makeItem("a"), makeItem("b")];

    const next = setCheckedForIds(items, new Set(["a"]), true);

    expect(next.find((item) => item.id === "a")?.checked).toBe(true);
    expect(next.find((item) => item.id === "b")?.checked).toBe(false);
  });
});

describe("removeChecked", () => {
  it("只删除已勾选项", () => {
    const items = [makeItem("a", { checked: true }), makeItem("b")];

    expect(removeChecked(items).map((item) => item.id)).toEqual(["b"]);
  });
});

describe("basenameOf", () => {
  it("同时处理正反斜杠分隔符", () => {
    expect(basenameOf("/tmp/dir/a.png")).toBe("a.png");
    expect(basenameOf("C:\\Users\\me\\b.png")).toBe("b.png");
    expect(basenameOf("plain.txt")).toBe("plain.txt");
  });
});
