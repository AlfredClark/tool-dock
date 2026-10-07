import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ConvertWorkspace from "$components/tools/text/convert/convert-workspace.svelte";
import { m } from "$libs/i18n/paraglide/messages";

// jsdom 的 ResizeObserver 不可构造，bits-ui Tooltip 的 floating layer 用得到，此处无条件换成空实现。
globalThis.ResizeObserver = class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
} as unknown as typeof ResizeObserver;

// 编辑器替身：CodeMirror 在 jsdom 下无 Range 测量能力，键盘输入不可靠，
// 转换流程测试改用原生 textarea 替身（见 tests/component/stubs），只验证调用与分支接线。
const convertDataMock = vi.hoisted(() => vi.fn());
const copyTextMock = vi.hoisted(() => vi.fn());
const toastMocks = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }));
vi.mock("$libs/commands", () => ({
  default: { convertData: convertDataMock, copyText: copyTextMock },
}));
vi.mock("$libs/utils/toast", () => ({ toast: toastMocks }));
vi.mock("$components/tools/text/convert/code-editor.svelte", async () => {
  const stub = await import("../../../../stubs/code-editor-stub.svelte");
  return { default: stub.default };
});

beforeEach(() => {
  vi.clearAllMocks();
  // 链式 API 形状：`.failed(cb).result()`，与 EnhancedCommand 的调用面一致
  copyTextMock.mockImplementation(() => ({
    failed: () => ({ result: () => Promise.resolve({ status: "ok", data: null }) }),
  }));
});

afterEach(() => {
  cleanup();
  document.body.removeAttribute("style");
});

describe("数据互转实时转换", () => {
  /** 输入输出两框：替身按 label 区分，顺序即左右栏顺序 */
  function editorBoxes(): HTMLTextAreaElement[] {
    return [
      screen.getByRole("textbox", { name: m.tool_convert_input_label() }),
      screen.getByRole("textbox", { name: m.tool_convert_output_label() }),
    ] as HTMLTextAreaElement[];
  }

  it("输入停止后调命令，输出写入输出端", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: { ok: true, output: '{\n  "a": 1\n}', detected: "json", error: null },
        }),
    }));
    render(ConvertWorkspace);

    // user-event 把 `{` 解析为键盘指令，双写才打出字面花括号
    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');

    await waitFor(
      () => {
        expect(convertDataMock).toHaveBeenCalledWith('{"a": 1}', null, "json", {
          json_indent: "two",
          xml_root_name: "root",
          xml_declaration: false,
          xml_indent: "two",
          ini_kv_separator: "compact",
          properties_escape_unicode: true,
          xml_trailing_newline: true,
        });
      },
      { timeout: 3000 },
    );
    await waitFor(
      () => {
        expect(editorBoxes()[1]?.value).toContain('"a"');
      },
      { timeout: 3000 },
    );
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("转换失败时错误条展示且保留上次成功输出", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation((input: string) => ({
      result: () =>
        Promise.resolve(
          input.endsWith("___")
            ? {
                status: "ok",
                data: {
                  ok: false,
                  output: "",
                  detected: null,
                  error: {
                    code: "UnknownFormat",
                    format: null,
                    message: null,
                    line: null,
                    column: null,
                  },
                },
              }
            : {
                status: "ok",
                data: { ok: true, output: '{\n  "a": 1\n}', detected: "json", error: null },
              },
        ),
    }));
    render(ConvertWorkspace);

    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');
    await waitFor(
      () => {
        expect(editorBoxes()[1]?.value).toContain('"a"');
      },
      { timeout: 3000 },
    );

    // 追加非法字符破坏输入，进入失败分支
    await user.type(editorBoxes()[0] as HTMLElement, "___");
    await waitFor(
      () => {
        expect(screen.getByRole("alert")).not.toBeNull();
      },
      { timeout: 3000 },
    );
    expect(screen.getByRole("alert").textContent).toContain(m.tool_convert_error_unknown_format());
    // 上次成功输出保留
    expect(editorBoxes()[1]?.value).toContain('"a"');
  });

  it("识别徽章展示并点击落盘为显式格式", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: { ok: true, output: "{}", detected: "json", error: null },
        }),
    }));
    render(ConvertWorkspace);

    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');
    const badgeLabel = m.tool_convert_detected_as({ format: "JSON" });
    await waitFor(
      () => {
        expect(screen.getByRole("button", { name: badgeLabel })).not.toBeNull();
      },
      { timeout: 3000 },
    );

    // 点击徽章：输入格式由自动识别落盘为 JSON，徽章随之消失
    await user.click(screen.getByRole("button", { name: badgeLabel }));
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: badgeLabel })).toBeNull();
    });
    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    expect(formatSelects[0]?.textContent).toContain("JSON");
  });

  it("格式化按钮按自身格式回写输入", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: { ok: true, output: "FORMATTED", detected: "json", error: null },
        }),
    }));
    render(ConvertWorkspace);

    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');
    await waitFor(
      () => {
        expect(convertDataMock).toHaveBeenCalled();
      },
      { timeout: 3000 },
    );
    await user.click(screen.getByRole("button", { name: m.tool_convert_format_button() }));
    await waitFor(
      () => {
        expect(editorBoxes()[0]?.value).toBe("FORMATTED");
      },
      { timeout: 3000 },
    );
    expect(toastMocks.success).toHaveBeenCalledWith(m.tool_convert_formatted());
  });

  it("清空按钮清空输入并联动清空输出", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: { ok: true, output: "OUT", detected: "json", error: null },
        }),
    }));
    render(ConvertWorkspace);

    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');
    await waitFor(
      () => {
        expect(editorBoxes()[1]?.value).toBe("OUT");
      },
      { timeout: 3000 },
    );
    await user.click(screen.getByRole("button", { name: m.tool_convert_clear_button() }));
    await waitFor(() => {
      expect(editorBoxes()[0]?.value).toBe("");
    });
    expect(editorBoxes()[1]?.value).toBe("");
  });

  it("复制按钮经命令写剪贴板并提示", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: { ok: true, output: "OUT", detected: "json", error: null },
        }),
    }));
    render(ConvertWorkspace);

    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');
    await waitFor(
      () => {
        expect(editorBoxes()[1]?.value).toBe("OUT");
      },
      { timeout: 3000 },
    );
    await user.click(screen.getByRole("button", { name: m.tool_convert_copy_button() }));
    await waitFor(() => {
      expect(copyTextMock).toHaveBeenCalledWith("OUT");
    });
    expect(toastMocks.success).toHaveBeenCalledWith(m.tool_convert_copied());
  });

  it("错误行号点击下发输入端滚动请求", async () => {
    const user = userEvent.setup();
    convertDataMock.mockImplementation(() => ({
      result: () =>
        Promise.resolve({
          status: "ok",
          data: {
            ok: false,
            output: "",
            detected: null,
            error: {
              code: "ParseFailed",
              format: "json",
              message: "boom",
              line: 2,
              column: 5,
            },
          },
        }),
    }));
    render(ConvertWorkspace);

    await user.type(editorBoxes()[0] as HTMLElement, '{{"a": 1}');
    const jumpLabel = m.tool_convert_error_line_column({ line: 2, column: 5 });
    await waitFor(
      () => {
        expect(screen.getByRole("button", { name: jumpLabel })).not.toBeNull();
      },
      { timeout: 3000 },
    );
    await user.click(screen.getByRole("button", { name: jumpLabel }));
    // 替身经 data 属性透出滚动请求：输入框行号为 2
    expect(editorBoxes()[0]?.dataset.scrollLine).toBe("2");
  });
});
