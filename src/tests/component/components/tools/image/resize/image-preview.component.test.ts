import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/svelte";
import ImagePreview from "../../../../../../components/tools/image/resize/image-preview.svelte";
import type { ResizeImageItem } from "../../../../../../components/tools/image/resize/resize-types";

afterEach(() => {
  cleanup();
});

describe("图片预览", () => {
  it("未选中显示空态", () => {
    render(ImagePreview, { props: { item: null, estimated: null, targetIgnored: false } });

    expect(screen.getByText("Select an image on the left to preview")).not.toBeNull();
  });

  it("选中项展示原图与元信息及预计尺寸", () => {
    const item: ResizeImageItem = {
      id: "/tmp/a.png",
      path: "/tmp/a.png",
      name: "a.png",
      previewUrl: "data:,",
      detailUrl: "",
      detailLoading: false,
      detailFailed: false,
      revokePreview: false,
      info: { width: 200, height: 100, format: "jpeg", file_size: 2048 },
      errorDetail: null,
      status: "ready",
      output: null,
      outputSize: null,
      targetMet: null,
    };
    render(ImagePreview, {
      props: { item, estimated: { width: 100, height: 50 }, targetIgnored: false },
    });

    expect(screen.getByAltText("a.png")).not.toBeNull();
    expect(screen.getByText("200×100")).not.toBeNull();
    expect(screen.getByText("Estimated 100×50")).not.toBeNull();
  });

  it("载入中显示骨架与加载文案，不误显示不可读", () => {
    const item: ResizeImageItem = {
      id: "/tmp/a.png",
      path: "/tmp/a.png",
      name: "a.png",
      previewUrl: "",
      detailUrl: "",
      detailLoading: false,
      detailFailed: false,
      revokePreview: false,
      info: null,
      errorDetail: null,
      status: "loading",
      output: null,
      outputSize: null,
      targetMet: null,
    };
    render(ImagePreview, { props: { item, estimated: null, targetIgnored: false } });

    // 预览骨架 + 信息行各一处加载文案
    expect(screen.getAllByText("Loading preview…")).toHaveLength(2);
    expect(screen.queryByText("Unreadable")).toBeNull();
  });

  it("目标未达成显示附注", () => {
    const item: ResizeImageItem = {
      id: "/tmp/a.png",
      path: "/tmp/a.png",
      name: "a.png",
      previewUrl: "data:,",
      detailUrl: "",
      detailLoading: false,
      detailFailed: false,
      revokePreview: false,
      info: { width: 200, height: 100, format: "jpeg", file_size: 2048 },
      errorDetail: null,
      status: "done",
      output: "/tmp/out/a.png",
      outputSize: { width: 100, height: 50 },
      targetMet: false,
    };
    render(ImagePreview, { props: { item, estimated: null, targetIgnored: false } });

    expect(screen.getByText("Target missed, smallest file written")).not.toBeNull();
    expect(screen.getByText("→ 100×50")).not.toBeNull();
  });

  it("目标不适用与未达成区分展示", () => {
    const item: ResizeImageItem = {
      id: "/tmp/a.png",
      path: "/tmp/a.png",
      name: "a.png",
      previewUrl: "data:,",
      detailUrl: "",
      detailLoading: false,
      detailFailed: false,
      revokePreview: false,
      info: { width: 200, height: 100, format: "png", file_size: 2048 },
      errorDetail: null,
      status: "done",
      output: "/tmp/out/a.png",
      outputSize: { width: 100, height: 50 },
      targetMet: false,
    };
    render(ImagePreview, { props: { item, estimated: null, targetIgnored: true } });

    expect(screen.getByText("Target size applies to JPEG output only")).not.toBeNull();
    expect(screen.queryByText("Target missed, smallest file written")).toBeNull();
  });
});
