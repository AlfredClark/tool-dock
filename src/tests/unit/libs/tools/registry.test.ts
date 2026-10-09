import { describe, expect, it } from "vitest";
import {
  TOOLS,
  TOOL_CATEGORIES,
  categoryLabel,
  isToolCategory,
  resolveTool,
  searchTools,
} from "$libs/tools/registry";

describe("工具注册表", () => {
  it("路径不重复且落在所属分类目录之下", () => {
    const paths = TOOLS.map((tool) => tool.path);

    expect(new Set(paths).size).toBe(paths.length);
    for (const tool of TOOLS) {
      expect(tool.path.startsWith(`/${tool.category}/`)).toBe(true);
      expect(resolveTool(tool.path)).toBe(tool);
    }
  });

  it("每个工具都有文案与图标", () => {
    for (const tool of TOOLS) {
      // 节点环境无 window，Paraglide 回落 baseLocale，取到英文文案
      expect(tool.name().length).toBeGreaterThan(0);
      expect(tool.description().length).toBeGreaterThan(0);
      expect(tool.icon).toBeDefined();
    }
  });

  it("按路径命中工具，未登记回落空", () => {
    for (const tool of TOOLS) {
      expect(resolveTool(tool.path)).toBe(tool);
    }
    expect(resolveTool("/tools/unknown")).toBeUndefined();
    expect(resolveTool("/text/unknown")).toBeUndefined();
    expect(resolveTool("/about")).toBeUndefined();
  });

  it("分类收窄只放行已登记的取值", () => {
    for (const category of TOOL_CATEGORIES) {
      expect(isToolCategory(category)).toBe(true);
    }
    expect(isToolCategory("audio")).toBe(false);
    expect(isToolCategory(undefined)).toBe(false);
  });

  it("未知分类名回落原文不断言", () => {
    expect(categoryLabel("audio")).toBe("audio");
  });
});

describe("工具搜索", () => {
  it("空查询返回空数组", () => {
    expect(searchTools("")).toEqual([]);
    expect(searchTools("   ")).toEqual([]);
  });

  it("按名称命中且大小写不敏感", () => {
    const names = searchTools("CONVERT").map((tool) => tool.id);

    expect(names).toEqual(["data-convert", "video-convert"]);
  });

  it("按分类名命中", () => {
    // baseLocale 为英文，分类名为 Text / Image / Network / System
    expect(searchTools("text").map((tool) => tool.id)).toEqual(["data-convert"]);
    expect(searchTools("network")).toEqual([]);
  });

  it("无命中返回空数组", () => {
    expect(searchTools("不存在的工具zzz")).toEqual([]);
  });
});

describe("图片尺寸工具", () => {
  it("注册信息完整", () => {
    const tool = resolveTool("/image/resize");

    expect(tool?.id).toBe("image-resize");
    expect(tool?.category).toBe("image");
    expect(tool?.name()).toBe("Resize");
  });

  it("搜索命中", () => {
    const hits = searchTools("Resize").map((tool) => tool.id);

    expect(hits).toContain("image-resize");
  });
});

describe("视频元数据工具", () => {
  it("注册信息完整", () => {
    const tool = resolveTool("/video/metadata");

    expect(tool?.id).toBe("video-metadata");
    expect(tool?.category).toBe("video");
    expect(tool?.name()).toBe("Metadata editor");
  });

  it("搜索命中", () => {
    const hits = searchTools("Metadata").map((tool) => tool.id);

    expect(hits).toContain("video-metadata");
  });

  it("视频分类文案已登记", () => {
    expect(categoryLabel("video")).toBe("Video");
    expect(isToolCategory("video")).toBe(true);
  });
});

describe("视频格式转换工具", () => {
  it("注册信息完整", () => {
    const tool = resolveTool("/video/convert");

    expect(tool?.id).toBe("video-convert");
    expect(tool?.category).toBe("video");
    expect(tool?.name()).toBe("Format converter");
  });

  it("搜索命中", () => {
    const hits = searchTools("Format converter").map((tool) => tool.id);

    expect(hits).toContain("video-convert");
  });
});

describe("批量重命名工具", () => {
  it("注册信息完整", () => {
    const tool = resolveTool("/system/renamer");

    expect(tool?.id).toBe("renamer");
    expect(tool?.category).toBe("system");
    expect(tool?.name()).toBe("Batch renamer");
  });

  it("搜索命中", () => {
    const hits = searchTools("Renamer").map((tool) => tool.id);

    expect(hits).toContain("renamer");
  });

  it("系统分类文案已登记", () => {
    expect(categoryLabel("system")).toBe("System");
    expect(isToolCategory("system")).toBe(true);
  });
});
