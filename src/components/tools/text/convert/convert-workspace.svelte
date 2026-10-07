<script lang="ts">
  // 数据互转双栏容器：持有页面级状态（格式/选项/文本/识别结果/错误），输入停止 300ms 后经命令链转后端。
  // 空输入直接清空不调命令；过期响应按序号丢弃；IPC 真异常 toast 提示并保留旧输出（plugin-log 自动上报）。
  import commands from "$libs/commands";
  import type {
    ConvertError,
    ConvertFormat as BindingFormat,
    ConvertOptions,
  } from "$libs/commands/bindings";
  import ConvertPane from "./convert-pane.svelte";
  import { toBindingFormat } from "./convert-errors";
  import { isConvertFormat, type ConvertFormat, type InputFormat } from "./formats";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";
  import { cn } from "$libs/utils/shadcn-svelte";

  /** 输入停止后等待转换的防抖时长 */
  const CONVERT_DEBOUNCE_MS = 300;

  /** 输入端格式：默认自动识别 */
  let inputFormat = $state<InputFormat>("auto");

  /** 输出端格式：默认 JSON，必须显式指定（无 auto） */
  let outputFormat = $state<ConvertFormat>("json");

  /** 两侧高级选项栏显隐（各自独立） */
  let inputAdvancedOpen = $state(false);
  let outputAdvancedOpen = $state(false);

  /** 两侧编辑器文本 */
  let inputText = $state("");
  let outputText = $state("");

  /** 转换选项：输出端高级选项控件读写，输入端忽略 */
  let convertOptions = $state<ConvertOptions>({
    json_indent: "two",
    xml_root_name: "root",
    xml_declaration: false,
    xml_indent: "two",
    ini_kv_separator: "compact",
    properties_escape_unicode: true,
    xml_trailing_newline: true,
  });

  /** 自动识别结果：驱动输入端编辑器高亮 */
  let detectedFormat = $state<ConvertFormat | null>(null);

  /** 当前转换错误：输出端错误条展示，成功即清空（失败不清空输出） */
  let convertError = $state<ConvertError | null>(null);

  /** 输入端行跳转请求：错误条点击后下发（消费语义，`seq` 区分连续点击） */
  let inputScrollTick = $state<{ line: number; seq: number } | null>(null);

  // 请求序号：防抖 + IPC 竞态下只采纳最新一次响应；普通变量即可，不进响应式跟踪
  let requestSeq = 0;

  // 滚动序号：同行连续点击也要触发跳转，普通变量即可
  let scrollSeq = 0;

  /** 输入端文本回写：值相同不重复赋值，避免编辑器同步回环 */
  function handleInputText(next: string): void {
    if (next !== inputText) inputText = next;
  }

  /** 输出端只读，输入回调仅占位（与单栏 props 对齐） */
  function handleOutputText(next: string): void {
    if (next !== outputText) outputText = next;
  }

  /** 输出端格式切换：拒绝 auto（候选里本就没有，双保险） */
  function handleOutputFormatChange(format: InputFormat): void {
    if (isConvertFormat(format)) outputFormat = format;
  }

  /** 转换选项变更：整体替换，触发防抖重转 */
  function handleOptionsChange(options: ConvertOptions): void {
    convertOptions = options;
  }

  /** 输入端清空：置空即触发空短路，输出/错误/识别一并清空 */
  function handleClearInput(): void {
    inputText = "";
  }

  /** 输入端格式化：按自身格式原地美化（auto 用识别结果）；失败 toast，不动原文 */
  async function handleFormatInput(): Promise<void> {
    if (inputText.trim() === "") return;
    const from = inputFormat === "auto" ? null : toBindingFormat(inputFormat);
    const resolved = inputFormat === "auto" ? detectedFormat : inputFormat;
    if (!resolved) {
      toast.error(m.tool_convert_format_no_detected());
      return;
    }
    const to = toBindingFormat(resolved);
    const result = await commands.convertData(inputText, from, to, snapshotOptions()).result();
    if (result.status === "error") {
      toast.error(m.tool_convert_request_failed());
      return;
    }
    const outcome = result.data;
    if (outcome.ok) {
      inputText = outcome.output;
      toast.success(m.tool_convert_formatted());
    } else {
      toast.error(m.tool_convert_format_failed());
    }
  }

  /** 输出端复制：经后端剪贴板命令（遵 Rust 侧剪贴板惯例）；成功与失败均 toast（复制无可见落点） */
  async function handleCopyOutput(): Promise<void> {
    if (outputText === "") return;
    const result = await commands
      .copyText(outputText)
      .failed(() => {
        toast.error(m.tool_convert_copy_failed());
      })
      .result();
    if (result.status === "ok") toast.success(m.tool_convert_copied());
  }

  /** 错误条跳转：有行号才下发滚动请求（按钮仅行号存在时可点击，双保险） */
  function handleErrorJump(): void {
    const line = convertError?.line;
    if (line == null) return;
    inputScrollTick = { line, seq: (scrollSeq += 1) };
  }

  /** 转换选项快照：逐字段读取进依赖跟踪（直接传代理对象读不到字段变更） */
  function snapshotOptions(): ConvertOptions {
    return {
      json_indent: convertOptions.json_indent,
      xml_root_name: convertOptions.xml_root_name,
      xml_declaration: convertOptions.xml_declaration,
      xml_indent: convertOptions.xml_indent,
      ini_kv_separator: convertOptions.ini_kv_separator,
      properties_escape_unicode: convertOptions.properties_escape_unicode,
      xml_trailing_newline: convertOptions.xml_trailing_newline,
    };
  }

  /** 执行一次转换：关键状态变更在 await 结算后分支处理，不在回调里做（遵链式 API 约定） */
  async function runConvert(
    text: string,
    from: BindingFormat | null,
    to: BindingFormat,
    options: ConvertOptions,
  ): Promise<void> {
    const ticket = (requestSeq += 1);
    const result = await commands.convertData(text, from, to, options).result();
    if (ticket !== requestSeq) return;
    if (result.status === "error") {
      toast.error(m.tool_convert_request_failed());
      return;
    }
    const outcome = result.data;
    detectedFormat = outcome.detected;
    if (outcome.ok) {
      outputText = outcome.output;
      convertError = null;
    } else {
      convertError = outcome.error;
    }
  }

  // 输入/格式/选项任一变化即重排转换：空输入短路清空，非空防抖
  $effect(() => {
    const text = inputText;
    const from = inputFormat === "auto" ? null : toBindingFormat(inputFormat);
    const to = toBindingFormat(outputFormat);
    const options = snapshotOptions();
    if (text.trim() === "") {
      outputText = "";
      detectedFormat = null;
      convertError = null;
      return;
    }
    const timer = setTimeout(() => {
      void runConvert(text, from, to, options);
    }, CONVERT_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });
</script>

<!-- 桌面端固定左右双栏：gap-px + bg-border 形成 1px 分隔线 -->
<div class={cn("flex h-full w-full gap-px bg-border")}>
  <ConvertPane
    side="input"
    format={inputFormat}
    advancedOpen={inputAdvancedOpen}
    editorValue={inputText}
    {detectedFormat}
    options={convertOptions}
    error={null}
    scrollTick={inputScrollTick}
    onFormatChange={(format) => (inputFormat = format)}
    onToggleAdvanced={() => (inputAdvancedOpen = !inputAdvancedOpen)}
    onOptionsChange={handleOptionsChange}
    onEditorInput={handleInputText}
    onFormat={() => void handleFormatInput()}
    onClear={handleClearInput}
  />
  <ConvertPane
    side="output"
    format={outputFormat}
    advancedOpen={outputAdvancedOpen}
    editorValue={outputText}
    {detectedFormat}
    options={convertOptions}
    error={convertError}
    scrollTick={null}
    onFormatChange={handleOutputFormatChange}
    onToggleAdvanced={() => (outputAdvancedOpen = !outputAdvancedOpen)}
    onOptionsChange={handleOptionsChange}
    onEditorInput={handleOutputText}
    onCopy={() => void handleCopyOutput()}
    onErrorJump={handleErrorJump}
  />
</div>
