<script lang="ts">
  // 数据互转单栏：工具栏（格式下拉 + 识别徽章 + 操作按钮 + 高级选项按钮）+ 可点击错误条 + 高级选项栏 + 编辑器。
  // 输入端下拉多一个自动识别；输出端高级选项按输出格式分支（JSON 缩进 / XML 三项 / INI 分隔符 / Properties 转义）。
  import type {
    ConvertError,
    ConvertOptions,
    IniKvSeparator,
    JsonIndent,
  } from "$libs/commands/bindings";
  import CopyIcon from "@lucide/svelte/icons/copy";
  import SlidersHorizontalIcon from "@lucide/svelte/icons/sliders-horizontal";
  import Trash2Icon from "@lucide/svelte/icons/trash-2";
  import WandSparklesIcon from "@lucide/svelte/icons/wand-sparkles";
  import { Button } from "$components/shadcn-svelte/button";
  import { Input } from "$components/shadcn-svelte/input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "$components/shadcn-svelte/select";
  import { Switch } from "$components/shadcn-svelte/switch";
  import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
  } from "$components/shadcn-svelte/tooltip";
  import CodeEditor from "./code-editor.svelte";
  import { convertErrorText, errorLineText } from "./convert-errors";
  import {
    formatLabel,
    inputFormatItems,
    isConvertFormat,
    isInputFormat,
    outputFormatItems,
    type ConvertFormat,
    type InputFormat,
  } from "./formats";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";

  interface Props {
    /** 左右栏标识：决定下拉候选项与高级选项内容 */
    side: "input" | "output";
    /** 当前格式：输出端恒为具体格式（类型上复用 InputFormat，运行时无 auto） */
    format: InputFormat;
    /** 高级选项栏是否展开 */
    advancedOpen: boolean;
    /** 编辑器内容 */
    editorValue: string;
    /** 自动识别结果：输入端 auto 时决定编辑器高亮，无结果即纯文本 */
    detectedFormat: ConvertFormat | null;
    /** 转换选项：输入端的值被忽略（无选项），仅输出端展示控件 */
    options: ConvertOptions;
    /** 当前转换错误：输出端展示错误条，输入端恒为 null */
    error: ConvertError | null;
    /** 行跳转请求：错误条点击后由 workspace 下发（仅输入端消费） */
    scrollTick?: { line: number; seq: number } | null;
    /** 格式切换回调 */
    onFormatChange: (format: InputFormat) => void;
    /** 高级选项显隐回调 */
    onToggleAdvanced: () => void;
    /** 转换选项变更回调（仅输出端控件触发） */
    onOptionsChange: (options: ConvertOptions) => void;
    /** 编辑器输入回调 */
    onEditorInput: (value: string) => void;
    /** 输入端格式化回调（输入端传入） */
    onFormat?: () => void;
    /** 输入端清空回调（输入端传入） */
    onClear?: () => void;
    /** 输出端复制回调（输出端传入） */
    onCopy?: () => void;
    /** 错误条跳转回调（输出端传入，有行号时可点击） */
    onErrorJump?: () => void;
  }

  let {
    side,
    format,
    advancedOpen,
    editorValue,
    detectedFormat,
    options,
    error,
    scrollTick = null,
    onFormatChange,
    onToggleAdvanced,
    onOptionsChange,
    onEditorInput,
    onFormat,
    onClear,
    onCopy,
    onErrorJump,
  }: Props = $props();

  // 输入端含自动识别，输出端仅六种格式；展示文案在调用时求值
  const formatItems = $derived<{ value: string; label: string }[]>(
    side === "input" ? inputFormatItems() : outputFormatItems(),
  );

  // bits-ui 给 string，经候选项收窄后才提交；输出端拒绝 auto（候选里本就没有，双保险）
  function handleFormatChange(next: string): void {
    if (!isInputFormat(next) || next === format) return;
    if (side === "output" && next === "auto") return;
    onFormatChange(next);
  }

  // 输入端 auto 且暂无识别结果时编辑器为纯文本；其余情况按具体格式高亮
  const editorFormat = $derived.by<ConvertFormat | null>(() => {
    if (format === "auto") return detectedFormat;
    return format;
  });

  const editorLabel = $derived(
    side === "input" ? m.tool_convert_input_label() : m.tool_convert_output_label(),
  );

  /** JSON 缩进候选项：`items` 让触发器在下拉关闭时也能解析出展示文案 */
  const indentItems: { value: JsonIndent; label: string }[] = [
    { value: "two", label: m.tool_convert_indent_two() },
    { value: "four", label: m.tool_convert_indent_four() },
    { value: "tab", label: m.tool_convert_indent_tab() },
  ];

  /** 缩进切换：经候选项收窄后才提交 */
  function handleIndentChange(next: string): void {
    const indent = indentItems.find((item) => item.value === next)?.value;
    if (!indent || indent === options.json_indent) return;
    onOptionsChange({ ...options, json_indent: indent });
  }

  /** XML 缩进切换：与 JSON 缩进复用同一候选项 */
  function handleXmlIndentChange(next: string): void {
    const indent = indentItems.find((item) => item.value === next)?.value;
    if (!indent || indent === options.xml_indent) return;
    onOptionsChange({ ...options, xml_indent: indent });
  }

  /** XML 根名输入：逐字回写（防抖转换在上层统一处理，此处不另设定时器） */
  function handleRootNameInput(event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.value === options.xml_root_name) return;
    onOptionsChange({ ...options, xml_root_name: target.value });
  }

  /** XML 声明头开关 */
  function handleDeclarationChange(checked: boolean): void {
    if (checked === options.xml_declaration) return;
    onOptionsChange({ ...options, xml_declaration: checked });
  }

  /** INI 分隔符候选项 */
  const separatorItems: { value: IniKvSeparator; label: string }[] = [
    { value: "compact", label: m.tool_convert_ini_separator_compact() },
    { value: "spaced", label: m.tool_convert_ini_separator_spaced() },
  ];

  /** INI 分隔符切换：经候选项收窄后才提交 */
  function handleSeparatorChange(next: string): void {
    const separator = separatorItems.find((item) => item.value === next)?.value;
    if (!separator || separator === options.ini_kv_separator) return;
    onOptionsChange({ ...options, ini_kv_separator: separator });
  }

  /** Properties Unicode 转义开关 */
  function handleEscapeChange(checked: boolean): void {
    if (checked === options.properties_escape_unicode) return;
    onOptionsChange({ ...options, properties_escape_unicode: checked });
  }

  /** XML 尾换行开关 */
  function handleTrailingNewlineChange(checked: boolean): void {
    if (checked === options.xml_trailing_newline) return;
    onOptionsChange({ ...options, xml_trailing_newline: checked });
  }

  // 输入端识别徽章：auto 且有识别结果时展示，点击落盘为显式格式
  const detectedBadge = $derived(
    side === "input" && format === "auto" && detectedFormat
      ? m.tool_convert_detected_as({ format: formatLabel(detectedFormat) })
      : null,
  );

  // 操作按钮可用态：空内容时禁用对应按钮
  const hasContent = $derived(editorValue.length > 0);
  const canFormat = $derived(hasContent && (format !== "auto" || detectedFormat !== null));

  // 错误位置文案：有行号才渲染"第 N 行"，点击跳输入端对应行
  const lineText = $derived(error ? errorLineText(error.line, error.column) : null);
  const canJump = $derived(lineText !== null && onErrorJump !== undefined);
</script>

<section aria-label={editorLabel} class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}>
  <!-- 工具栏：左格式下拉 + 识别徽章，右操作按钮 + 高级选项按钮 -->
  <div class={cn("flex h-10 shrink-0 items-center justify-between gap-2 border-b px-2")}>
    <div class={cn("flex min-w-0 items-center gap-1")}>
      <Select type="single" value={format} items={formatItems} onValueChange={handleFormatChange}>
        <SelectTrigger class="w-40" aria-label={m.tool_convert_format_label()}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {#each formatItems as item (item.value)}
            <SelectItem value={item.value}>{item.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
      {#if detectedBadge && detectedFormat}
        <Button
          variant="ghost"
          size="sm"
          onclick={() => onFormatChange(detectedFormat)}
          aria-label={detectedBadge}
          title={detectedBadge}
          class="max-w-36 shrink-0 truncate text-xs text-muted-foreground"
        >
          {detectedBadge}
        </Button>
      {/if}
    </div>
    <!-- bits-ui Tooltip 要求祖先挂 Provider：收敛在工具栏内，不污染共享布局 -->
    <TooltipProvider>
      <div class={cn("flex shrink-0 items-center gap-1")}>
        {#if side === "input"}
          {#if onFormat}
            <Tooltip>
              <TooltipTrigger>
                {#snippet child({ props })}
                  <Button
                    {...props}
                    variant="ghost"
                    size="icon-sm"
                    onclick={onFormat}
                    disabled={!canFormat}
                    aria-label={m.tool_convert_format_button()}
                  >
                    <WandSparklesIcon />
                  </Button>
                {/snippet}
              </TooltipTrigger>
              <TooltipContent>{m.tool_convert_format_button()}</TooltipContent>
            </Tooltip>
          {/if}
          {#if onClear}
            <Tooltip>
              <TooltipTrigger>
                {#snippet child({ props })}
                  <Button
                    {...props}
                    variant="ghost"
                    size="icon-sm"
                    onclick={onClear}
                    disabled={!hasContent}
                    aria-label={m.tool_convert_clear_button()}
                  >
                    <Trash2Icon />
                  </Button>
                {/snippet}
              </TooltipTrigger>
              <TooltipContent>{m.tool_convert_clear_button()}</TooltipContent>
            </Tooltip>
          {/if}
        {:else if onCopy}
          <Tooltip>
            <TooltipTrigger>
              {#snippet child({ props })}
                <Button
                  {...props}
                  variant="ghost"
                  size="icon-sm"
                  onclick={onCopy}
                  disabled={!hasContent}
                  aria-label={m.tool_convert_copy_button()}
                >
                  <CopyIcon />
                </Button>
              {/snippet}
            </TooltipTrigger>
            <TooltipContent>{m.tool_convert_copy_button()}</TooltipContent>
          </Tooltip>
        {/if}
        <Button
          variant="outline"
          size="sm"
          onclick={onToggleAdvanced}
          aria-expanded={advancedOpen}
          aria-label={m.tool_convert_advanced_options()}
        >
          <SlidersHorizontalIcon />
          {m.tool_convert_advanced_options()}
        </Button>
      </div>
    </TooltipProvider>
  </div>
  <!-- 内联错误条：转换失败时展示（输出端保留上次成功内容），成功即消失；有行号时可点击跳转 -->
  {#if error}
    <div
      role="alert"
      class={cn("shrink-0 border-b border-destructive/30 bg-destructive/10 px-3 py-2")}
    >
      {#if canJump && onErrorJump}
        <button
          onclick={onErrorJump}
          class={cn(
            "flex w-full items-center gap-1 text-left text-xs font-medium text-destructive",
            "cursor-pointer underline-offset-2 hover:underline",
          )}
          aria-label={lineText}
        >
          <span>{convertErrorText(error.code, error.format)}</span>
          <span class={cn("shrink-0")}>· {lineText}</span>
        </button>
      {:else}
        <p class={cn("text-xs font-medium text-destructive")}>
          {convertErrorText(error.code, error.format)}
          {#if lineText}<span> · {lineText}</span>{/if}
        </p>
      {/if}
      {#if error.message}
        <p class={cn("mt-0.5 line-clamp-2 text-xs text-muted-foreground")} title={error.message}>
          {error.message}
        </p>
      {/if}
    </div>
  {/if}
  <!-- 高级选项栏：折叠时不渲染；输出端内容按输出格式分支 -->
  {#if advancedOpen}
    <div class={cn("shrink-0 border-b px-3 py-2")}>
      {#if side === "input"}
        <p class={cn("text-xs text-muted-foreground")}>{m.tool_convert_no_input_options()}</p>
      {:else if format === "json"}
        <div class={cn("flex items-center justify-between gap-2")}>
          <span class={cn("text-xs text-muted-foreground")}>{m.tool_convert_indent_label()}</span>
          <Select
            type="single"
            value={options.json_indent}
            items={indentItems}
            onValueChange={handleIndentChange}
          >
            <SelectTrigger class="w-28" aria-label={m.tool_convert_indent_label()}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each indentItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
      {:else if format === "xml"}
        <div class={cn("flex flex-col gap-2")}>
          <div class={cn("flex items-center justify-between gap-2")}>
            <span class={cn("text-xs text-muted-foreground")}
              >{m.tool_convert_xml_root_label()}</span
            >
            <Input
              type="text"
              value={options.xml_root_name}
              placeholder={m.tool_convert_xml_root_placeholder()}
              aria-label={m.tool_convert_xml_root_label()}
              oninput={handleRootNameInput}
              class="w-40"
            />
          </div>
          <div class={cn("flex items-center justify-between gap-2")}>
            <span class={cn("text-xs text-muted-foreground")}
              >{m.tool_convert_xml_declaration_label()}</span
            >
            <Switch
              checked={options.xml_declaration}
              onCheckedChange={handleDeclarationChange}
              aria-label={m.tool_convert_xml_declaration_label()}
            />
          </div>
          <div class={cn("flex items-center justify-between gap-2")}>
            <span class={cn("text-xs text-muted-foreground")}>{m.tool_convert_indent_label()}</span>
            <Select
              type="single"
              value={options.xml_indent}
              items={indentItems}
              onValueChange={handleXmlIndentChange}
            >
              <SelectTrigger class="w-28" aria-label={m.tool_convert_indent_label()}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each indentItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </div>
          <div class={cn("flex items-center justify-between gap-2")}>
            <span class={cn("text-xs text-muted-foreground")}
              >{m.tool_convert_xml_trailing_newline_label()}</span
            >
            <Switch
              checked={options.xml_trailing_newline}
              onCheckedChange={handleTrailingNewlineChange}
              aria-label={m.tool_convert_xml_trailing_newline_label()}
            />
          </div>
        </div>
      {:else if format === "ini"}
        <div class={cn("flex items-center justify-between gap-2")}>
          <span class={cn("text-xs text-muted-foreground")}
            >{m.tool_convert_ini_separator_label()}</span
          >
          <Select
            type="single"
            value={options.ini_kv_separator}
            items={separatorItems}
            onValueChange={handleSeparatorChange}
          >
            <SelectTrigger class="w-28" aria-label={m.tool_convert_ini_separator_label()}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each separatorItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
      {:else if format === "properties"}
        <div class={cn("flex items-center justify-between gap-2")}>
          <span class={cn("text-xs text-muted-foreground")}
            >{m.tool_convert_properties_escape_label()}</span
          >
          <Switch
            checked={options.properties_escape_unicode}
            onCheckedChange={handleEscapeChange}
            aria-label={m.tool_convert_properties_escape_label()}
          />
        </div>
      {:else if isConvertFormat(format)}
        <p class={cn("text-xs text-muted-foreground")}>
          {m.tool_convert_no_output_options({ format: formatLabel(format) })}
        </p>
      {:else}
        <!-- 输出端运行时恒为具体格式，此分支仅为类型完备 -->
        <p class={cn("text-xs text-muted-foreground")}>{m.tool_convert_no_input_options()}</p>
      {/if}
    </div>
  {/if}
  <!-- 编辑器：撑满工具栏下方剩余高度 -->
  <div class={cn("min-h-0 flex-1")}>
    <CodeEditor
      value={editorValue}
      format={editorFormat}
      readonly={side === "output"}
      label={editorLabel}
      onInput={onEditorInput}
      {scrollTick}
    />
  </div>
</section>
