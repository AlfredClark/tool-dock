import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import ConvertWorkspace from "$components/tools/text/convert/convert-workspace.svelte";
import { m } from "$libs/i18n/paraglide/messages";

// 编辑器替身：与流程测试同理，用原生 textarea 承接接线。
const convertDataMock = vi.hoisted(() => vi.fn());
const readTextFileMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn(), info: vi.fn() }));
const onDragDropEventMock = vi.hoisted(() => vi.fn());
vi.mock("$libs/commands", () => ({
  default: { convertData: convertDataMock, readTextFile: readTextFileMock },
}));
vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));
vi.mock("$components/tools/text/convert/code-editor.svelte", async () => {
  const stub = await import("../../../../stubs/code-editor-stub.svelte");
  return { default: stub.default };
});
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: onDragDropEventMock }),
}));

beforeEach(() => {
  vi.clearAllMocks();
  convertDataMock.mockImplementation(() => ({
    result: () =>
      Promise.resolve({
        status: "ok",
        data: { ok: true, output: "", detected: null, error: null },
      }),
  }));
  readTextFileMock.mockImplementation(() => ({
    result: () => Promise.resolve({ status: "ok", data: "" }),
  }));
  // 默认无 Tauri 运行时，走 DOM 降级路径；桌面路径用例自行桩 `__TAURI_INTERNALS__`
  onDragDropEventMock.mockRejectedValue(new Error("no tauri runtime"));
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("数据互转拖放载入", () => {
  /** 输入输出两框：替身按 label 区分，顺序即左右栏顺序 */
  function editorBoxes(): HTMLTextAreaElement[] {
    return [
      screen.getByRole("textbox", { name: m.tool_convert_input_label() }),
      screen.getByRole("textbox", { name: m.tool_convert_output_label() }),
    ] as HTMLTextAreaElement[];
  }

  function dragFiles(types: string[] = ["Files"]): { types: string[] } {
    return { types };
  }

  it("拖入文件显示 overlay，离开后消失", async () => {
    render(ConvertWorkspace);

    expect(screen.queryByText(m.tool_convert_drop_hint())).toBeNull();
    await fireEvent.dragEnter(document.body, { dataTransfer: dragFiles() });
    expect(screen.getByText(m.tool_convert_drop_hint())).not.toBeNull();
    await fireEvent.dragLeave(document.body, { dataTransfer: dragFiles() });
    expect(screen.queryByText(m.tool_convert_drop_hint())).toBeNull();
  });

  it("DOM 放置读取文件内容并按扩展名预选格式", async () => {
    render(ConvertWorkspace);

    const file = new File(['{"a": 1}'], "data.json", { type: "application/json" });
    await fireEvent.drop(document.body, { dataTransfer: { types: ["Files"], files: [file] } });

    await waitFor(
      () => {
        expect(editorBoxes()[0]?.value).toBe('{"a": 1}');
      },
      { timeout: 3000 },
    );
    // 扩展名 json → 输入格式切到 JSON（触发器文案同步）
    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    expect(formatSelects[0]?.textContent).toContain("JSON");
  });

  it("超大文件直接拒绝且不调转换", async () => {
    render(ConvertWorkspace);

    const file = new File(["x".repeat(1024 * 1024 + 1)], "big.json", { type: "application/json" });
    await fireEvent.drop(document.body, { dataTransfer: { types: ["Files"], files: [file] } });

    expect(toastMocks.error).toHaveBeenCalledWith(m.tool_convert_file_too_large());
    expect(editorBoxes()[0]?.value).toBe("");
    expect(convertDataMock).not.toHaveBeenCalled();
  });

  it("多文件仅读取第一个并提示", async () => {
    render(ConvertWorkspace);

    const first = new File(["a: 1"], "a.yaml", { type: "text/yaml" });
    const second = new File(["b: 2"], "b.yaml", { type: "text/yaml" });
    await fireEvent.drop(document.body, {
      dataTransfer: { types: ["Files"], files: [first, second] },
    });

    await waitFor(
      () => {
        expect(editorBoxes()[0]?.value).toBe("a: 1");
      },
      { timeout: 3000 },
    );
    expect(toastMocks.info).toHaveBeenCalledWith(m.tool_convert_drop_multiple());
  });

  it("Tauri 放置经后端命令读取并预选格式", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    let captured: ((event: unknown) => void) | undefined;
    onDragDropEventMock.mockImplementationOnce((handler: (event: unknown) => void) => {
      captured = handler;
      return Promise.resolve(vi.fn());
    });
    readTextFileMock.mockImplementationOnce(() => ({
      result: () => Promise.resolve({ status: "ok", data: "a: 1" }),
    }));
    render(ConvertWorkspace);
    await waitFor(() => {
      expect(captured).toBeDefined();
    });

    captured?.({ payload: { type: "enter", paths: ["/tmp/a.yaml"], position: { x: 0, y: 0 } } });
    expect(await screen.findByText(m.tool_convert_drop_hint())).not.toBeNull();

    captured?.({ payload: { type: "drop", paths: ["/tmp/a.yaml"], position: { x: 0, y: 0 } } });
    await waitFor(() => {
      expect(readTextFileMock).toHaveBeenCalledWith("/tmp/a.yaml");
    });
    await waitFor(
      () => {
        expect(editorBoxes()[0]?.value).toBe("a: 1");
      },
      { timeout: 3000 },
    );
    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    expect(formatSelects[0]?.textContent).toContain("YAML");
  });

  it("Tauri 后端读取失败时 toast", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    let captured: ((event: unknown) => void) | undefined;
    onDragDropEventMock.mockImplementationOnce((handler: (event: unknown) => void) => {
      captured = handler;
      return Promise.resolve(vi.fn());
    });
    readTextFileMock.mockImplementationOnce(() => ({
      result: () => Promise.resolve({ status: "error", error: { kind: "Internal", message: "x" } }),
    }));
    render(ConvertWorkspace);
    await waitFor(() => {
      expect(captured).toBeDefined();
    });

    captured?.({
      payload: { type: "drop", paths: ["/tmp/missing.json"], position: { x: 0, y: 0 } },
    });
    await waitFor(() => {
      expect(toastMocks.error).toHaveBeenCalledWith(m.tool_convert_file_read_failed());
    });
    expect(editorBoxes()[0]?.value).toBe("");
  });
});
