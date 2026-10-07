import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ConvertPane from "$components/tools/text/convert/convert-pane.svelte";
import ConvertWorkspace from "$components/tools/text/convert/convert-workspace.svelte";
import type { ConvertOptions } from "$libs/commands/bindings";
import { m } from "$libs/i18n/paraglide/messages";

// jsdom 缺少指针捕获与滚动 API，bits-ui 下拉用得到，仅在本文件内就地补齐（同通用设置测试）。
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
}
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}
// jsdom 的 ResizeObserver 不可构造，bits-ui Tooltip 的 floating layer 用得到，此处无条件换成空实现。
globalThis.ResizeObserver = class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
} as unknown as typeof ResizeObserver;

const defaultOptions: ConvertOptions = {
  json_indent: "two",
  xml_root_name: "root",
  xml_declaration: false,
  xml_indent: "two",
  ini_kv_separator: "compact",
  properties_escape_unicode: true,
  xml_trailing_newline: true,
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  cleanup();
  // bits-ui 下拉打开时给 body 加滚动锁定样式，卸载后手动清理，避免泄漏到后续用例。
  document.body.removeAttribute("style");
});

describe("数据互转双栏布局", () => {
  it("左右各一个工具栏：格式下拉 + 高级选项按钮", () => {
    render(ConvertWorkspace);

    // bits-ui 的 SelectTrigger 渲染为 button，经 aria-label 命名；节点环境文案回落 baseLocale（en）
    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    expect(formatSelects).toHaveLength(2);
    const advancedButtons = screen.getAllByRole("button", {
      name: m.tool_convert_advanced_options(),
    });
    expect(advancedButtons).toHaveLength(2);
  });

  it("默认输入端自动识别、输出端 JSON", () => {
    render(ConvertWorkspace);

    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    expect(within(formatSelects[0] as HTMLElement).getByText(m.tool_convert_format_auto()));
    expect(within(formatSelects[1] as HTMLElement).getByText("JSON"));
  });

  it("左右各挂载一个编辑器：输入端可编辑、输出端只读", () => {
    const { container } = render(ConvertWorkspace);

    const editors = container.querySelectorAll(".cm-editor");
    expect(editors).toHaveLength(2);
    // 只读态下 CodeMirror 保留 contenteditable，改以 aria-readonly 区分
    const contents = container.querySelectorAll(".cm-content");
    expect(contents[0]?.getAttribute("aria-readonly")).toBeNull();
    expect(contents[1]?.getAttribute("aria-readonly")).toBe("true");
  });

  it("高级选项栏默认收起，点击后输入端无选项、输出端按格式分支", async () => {
    const user = userEvent.setup();
    render(ConvertWorkspace);

    expect(screen.queryByText(m.tool_convert_no_input_options())).toBeNull();

    const advancedButtons = screen.getAllByRole("button", {
      name: m.tool_convert_advanced_options(),
    });
    await user.click(advancedButtons[0] as HTMLElement);
    expect(screen.getByText(m.tool_convert_no_input_options())).not.toBeNull();

    // 输出端默认 JSON：展示缩进选项
    await user.click(advancedButtons[1] as HTMLElement);
    expect(screen.getByText(m.tool_convert_indent_label())).not.toBeNull();

    // 再次点击收起输入端高级选项栏
    await user.click(advancedButtons[0] as HTMLElement);
    expect(screen.queryByText(m.tool_convert_no_input_options())).toBeNull();
  });

  it("输入端下拉含自动识别共七项", async () => {
    // jsdom 无布局，选项始终不可见，只能按 `data-value` 定位（同通用设置测试）
    const user = userEvent.setup();
    render(ConvertWorkspace);

    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    await user.click(formatSelects[0] as HTMLElement);
    await waitFor(() => {
      expect(document.querySelector('[data-value="auto"]')).not.toBeNull();
    });
    expect(document.querySelectorAll("[data-value]")).toHaveLength(7);
  });

  it("输出端下拉仅六种格式、无自动识别", async () => {
    const user = userEvent.setup();
    render(ConvertWorkspace);

    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    await user.click(formatSelects[1] as HTMLElement);
    await waitFor(() => {
      expect(document.querySelector('[data-value="json"]')).not.toBeNull();
    });
    expect(document.querySelector('[data-value="auto"]')).toBeNull();
    expect(document.querySelectorAll("[data-value]")).toHaveLength(6);
  });

  it("输出端切到 YAML 后高级选项展示暂无选项说明", async () => {
    const user = userEvent.setup();
    render(ConvertWorkspace);

    const advancedButtons = screen.getAllByRole("button", {
      name: m.tool_convert_advanced_options(),
    });
    await user.click(advancedButtons[1] as HTMLElement);

    const formatSelects = screen.getAllByRole("button", { name: m.tool_convert_format_label() });
    await user.click(formatSelects[1] as HTMLElement);
    await waitFor(() => {
      expect(document.querySelector('[data-value="yaml"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="yaml"]') as HTMLElement);
    expect(screen.getByText(m.tool_convert_no_output_options({ format: "YAML" }))).not.toBeNull();
  });
});

describe("数据互转单栏", () => {
  it("错误条展示错误码文案与解析器原文", () => {
    render(ConvertPane, {
      props: {
        side: "output",
        format: "json",
        advancedOpen: false,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: {
          code: "ParseFailed",
          format: "json",
          message: "boom at line 1",
          line: 1,
          column: 5,
        },
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
      },
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain(m.tool_convert_error_parse_failed());
    expect(alert.textContent).toContain("boom at line 1");
  });

  it("缩进切换回写选项", async () => {
    const user = userEvent.setup();
    const onOptionsChange = vi.fn();
    render(ConvertPane, {
      props: {
        side: "output",
        format: "json",
        advancedOpen: true,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange,
        onEditorInput: vi.fn(),
      },
    });

    await user.click(screen.getByRole("button", { name: m.tool_convert_indent_label() }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="four"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="four"]') as HTMLElement);
    expect(onOptionsChange).toHaveBeenCalledWith({ ...defaultOptions, json_indent: "four" });
  });

  it("XML 根名与声明头回写选项", async () => {
    const user = userEvent.setup();
    const onOptionsChange = vi.fn();
    render(ConvertPane, {
      props: {
        side: "output",
        format: "xml",
        advancedOpen: true,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange,
        onEditorInput: vi.fn(),
      },
    });

    const rootInput = screen.getByRole("textbox", { name: m.tool_convert_xml_root_label() });
    await user.clear(rootInput);
    await user.type(rootInput, "data");
    expect(onOptionsChange).toHaveBeenCalledWith({ ...defaultOptions, xml_root_name: "data" });

    await user.click(screen.getByRole("switch", { name: m.tool_convert_xml_declaration_label() }));
    expect(onOptionsChange).toHaveBeenCalledWith({ ...defaultOptions, xml_declaration: true });
  });

  it("XML 缩进切换回写选项", async () => {
    const user = userEvent.setup();
    const onOptionsChange = vi.fn();
    render(ConvertPane, {
      props: {
        side: "output",
        format: "xml",
        advancedOpen: true,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange,
        onEditorInput: vi.fn(),
      },
    });

    await user.click(screen.getByRole("button", { name: m.tool_convert_indent_label() }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="tab"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="tab"]') as HTMLElement);
    expect(onOptionsChange).toHaveBeenCalledWith({ ...defaultOptions, xml_indent: "tab" });
  });

  it("识别徽章仅 auto 有结果时展示，点击落盘显式格式", async () => {
    const user = userEvent.setup();
    const onFormatChange = vi.fn();
    render(ConvertPane, {
      props: {
        side: "input",
        format: "auto",
        advancedOpen: false,
        editorValue: "x",
        detectedFormat: "json",
        options: defaultOptions,
        error: null,
        onFormatChange,
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
        onFormat: vi.fn(),
        onClear: vi.fn(),
      },
    });

    const badge = screen.getByRole("button", {
      name: m.tool_convert_detected_as({ format: "JSON" }),
    });
    await user.click(badge);
    expect(onFormatChange).toHaveBeenCalledWith("json");
  });

  it("输入端操作按钮回调与禁用态", async () => {
    const user = userEvent.setup();
    const onFormat = vi.fn();
    const onClear = vi.fn();
    render(ConvertPane, {
      props: {
        side: "input",
        format: "json",
        advancedOpen: false,
        editorValue: "x",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
        onFormat,
        onClear,
      },
    });

    await user.click(screen.getByRole("button", { name: m.tool_convert_format_button() }));
    expect(onFormat).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: m.tool_convert_clear_button() }));
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("空内容时操作按钮禁用", () => {
    render(ConvertPane, {
      props: {
        side: "input",
        format: "json",
        advancedOpen: false,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
        onFormat: vi.fn(),
        onClear: vi.fn(),
      },
    });

    expect(
      screen.getByRole("button", { name: m.tool_convert_format_button() }).hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.getByRole("button", { name: m.tool_convert_clear_button() }).hasAttribute("disabled"),
    ).toBe(true);
  });

  it("输出端复制按钮回调", async () => {
    const user = userEvent.setup();
    const onCopy = vi.fn();
    render(ConvertPane, {
      props: {
        side: "output",
        format: "json",
        advancedOpen: false,
        editorValue: "x",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
        onCopy,
      },
    });

    await user.click(screen.getByRole("button", { name: m.tool_convert_copy_button() }));
    expect(onCopy).toHaveBeenCalledTimes(1);
  });

  it("有行号的错误条可点击跳转，无行号时为纯文本", async () => {
    const user = userEvent.setup();
    const onErrorJump = vi.fn();
    const { unmount } = render(ConvertPane, {
      props: {
        side: "output",
        format: "json",
        advancedOpen: false,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: { code: "ParseFailed", format: "json", message: null, line: 3, column: null },
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
        onErrorJump,
      },
    });

    const jump = screen.getByRole("button", { name: m.tool_convert_error_line({ line: 3 }) });
    await user.click(jump);
    expect(onErrorJump).toHaveBeenCalledTimes(1);
    unmount();

    // 无行号：纯文本展示，无可点击按钮
    render(ConvertPane, {
      props: {
        side: "output",
        format: "json",
        advancedOpen: false,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: { code: "ParseFailed", format: "json", message: null, line: null, column: null },
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange: vi.fn(),
        onEditorInput: vi.fn(),
        onErrorJump,
      },
    });
    expect(
      screen.queryByRole("button", { name: m.tool_convert_error_line({ line: 3 }) }),
    ).toBeNull();
    expect(screen.getByRole("alert").textContent).toContain(m.tool_convert_error_parse_failed());
  });

  it("INI 分隔符切换回写选项", async () => {
    const user = userEvent.setup();
    const onOptionsChange = vi.fn();
    render(ConvertPane, {
      props: {
        side: "output",
        format: "ini",
        advancedOpen: true,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange,
        onEditorInput: vi.fn(),
      },
    });

    await user.click(screen.getByRole("button", { name: m.tool_convert_ini_separator_label() }));
    await waitFor(() => {
      expect(document.querySelector('[data-value="spaced"]')).not.toBeNull();
    });
    await user.click(document.querySelector('[data-value="spaced"]') as HTMLElement);
    expect(onOptionsChange).toHaveBeenCalledWith({
      ...defaultOptions,
      ini_kv_separator: "spaced",
    });
  });

  it("Properties 转义与 XML 尾换行开关回写选项", async () => {
    const user = userEvent.setup();
    const onOptionsChange = vi.fn();
    const { unmount } = render(ConvertPane, {
      props: {
        side: "output",
        format: "properties",
        advancedOpen: true,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange,
        onEditorInput: vi.fn(),
      },
    });

    await user.click(
      screen.getByRole("switch", { name: m.tool_convert_properties_escape_label() }),
    );
    expect(onOptionsChange).toHaveBeenCalledWith({
      ...defaultOptions,
      properties_escape_unicode: false,
    });
    unmount();

    render(ConvertPane, {
      props: {
        side: "output",
        format: "xml",
        advancedOpen: true,
        editorValue: "",
        detectedFormat: null,
        options: defaultOptions,
        error: null,
        onFormatChange: vi.fn(),
        onToggleAdvanced: vi.fn(),
        onOptionsChange,
        onEditorInput: vi.fn(),
      },
    });
    await user.click(
      screen.getByRole("switch", { name: m.tool_convert_xml_trailing_newline_label() }),
    );
    expect(onOptionsChange).toHaveBeenCalledWith({
      ...defaultOptions,
      xml_trailing_newline: false,
    });
  });
});
