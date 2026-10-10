<script lang="ts">
  // 单条规则参数卡：头部（左夹点 + 标题〈类型名 + 参数摘要，超长截断 + 悬浮全显〉 + 右收起/电源/删除）+ 参数区（标签在上，双列网格）。
  // 纯展示组件：参数编辑整体回写 `onRuleChange`（仿 `resize-params` 模式），非法参数标红但不拦截。
  // 排序走 HTML5 拖放：仅手柄可拖，卡片根负责悬停定位与放置，插入线由 panel 态驱动。
  import GripVerticalIcon from "@lucide/svelte/icons/grip-vertical";
  import ChevronDownIcon from "@lucide/svelte/icons/chevron-down";
  import PowerIcon from "@lucide/svelte/icons/power";
  import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
  import XIcon from "@lucide/svelte/icons/x";
  import { Badge } from "$components/shadcn-svelte/badge";
  import { Button } from "$components/shadcn-svelte/button";
  import { Card, CardContent } from "$components/shadcn-svelte/card";
  import { Input } from "$components/shadcn-svelte/input";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "$components/shadcn-svelte/select";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import RuleField from "./rule-field.svelte";
  import { validateRule } from "./renamer-rules";
  import type {
    RenamerAffixMode,
    RenamerCaseMode,
    RenamerCharsAction,
    RenamerCharsClass,
    RenamerEdge,
    RenamerNormalizePreset,
    RenamerReplaceMode,
    RenamerRule,
  } from "./renamer-types";

  interface Props {
    /** 规则：判别联合体，模板内按 `kind` 收窄 */
    rule: RenamerRule;
    /** 序号（0 起，展示 `#${index + 1}`） */
    index: number;
    /** 参数变更回调（整体替换） */
    onRuleChange: (next: RenamerRule) => void;
    /** 删除回调 */
    onRemove: (id: string) => void;
    /** 手柄拖拽开始回调（panel 记录拖拽源） */
    onHandleDragStart: (event: DragEvent, id: string) => void;
    /** 卡片悬停回调（panel 计算插入位置，需 preventDefault 才能放置） */
    onCardDragOver: (event: DragEvent, id: string) => void;
    /** 卡片放置回调 */
    onCardDrop: (event: DragEvent, id: string) => void;
    /** 拖拽结束回调（panel 清理拖拽态） */
    onDragEnd: () => void;
    /** 上方插入线显隐 */
    dropBefore: boolean;
    /** 下方插入线显隐 */
    dropAfter: boolean;
  }

  let {
    rule,
    index,
    onRuleChange,
    onRemove,
    onHandleDragStart,
    onCardDragOver,
    onCardDrop,
    onDragEnd,
    dropBefore,
    dropAfter,
  }: Props = $props();

  /** 标题栏文案：类型名 + 关键参数摘要（收起时可辨规则内容） */
  function titleLabel(): string {
    switch (rule.kind) {
      case "affix": {
        const position =
          rule.position === "prefix"
            ? m.tool_renamer_rule_position_prefix()
            : m.tool_renamer_rule_position_suffix();
        const text = rule.text === "" ? "" : ` "${rule.text}"`;
        return `${m.tool_renamer_rule_kind_affix()} · ${affixModeLabel(rule.mode)} · ${position}${text}`;
      }
      case "case":
        return `${m.tool_renamer_rule_kind_case()} · ${caseModeLabel(rule.mode)}`;
      case "normalize":
        return `${m.tool_renamer_rule_kind_normalize()} · ${normalizePresetLabel(rule.preset)}`;
      case "replace": {
        const find = rule.find === "" ? "" : ` "${rule.find}"`;
        return `${m.tool_renamer_rule_kind_replace()} · ${replaceModeLabel(rule.mode)}${find}`;
      }
      case "number":
        return m.tool_renamer_rule_kind_number();
      case "slice": {
        const anchor =
          rule.anchor === "" ? "" : ` · ${m.tool_renamer_rule_slice_anchor()} ${rule.anchor}`;
        const length =
          rule.length === "" ? "" : ` · ${m.tool_renamer_rule_slice_length()} ${rule.length}`;
        return `${m.tool_renamer_rule_kind_slice()}${anchor}${length}`;
      }
      case "insert": {
        const position =
          rule.position === "" ? "" : ` · ${m.tool_renamer_rule_position()} ${rule.position}`;
        const text = rule.text === "" ? "" : ` "${rule.text}"`;
        return `${m.tool_renamer_rule_kind_insert()}${position}${text}`;
      }
      case "chars": {
        const actionLabel =
          rule.action === "delete"
            ? m.tool_renamer_rule_chars_action_delete()
            : m.tool_renamer_rule_chars_action_keep();
        const custom = rule.class !== "custom" || rule.custom === "" ? "" : ` "${rule.custom}"`;
        return `${m.tool_renamer_rule_kind_chars()} · ${actionLabel} · ${charsClassLabel(rule.class)}${custom}`;
      }
    }
  }

  /** 参数错误文案：`null` 即合法（预览层照常跳过非法规则） */
  function errorLabel(): string | null {
    switch (validateRule(rule)) {
      case "bad-regex":
        return m.tool_renamer_rule_pattern_invalid();
      case "bad-number":
        return m.tool_renamer_rule_number_invalid();
      case "bad-slice":
        return m.tool_renamer_rule_slice_invalid();
      case "bad-insert":
        return m.tool_renamer_rule_insert_invalid();
      case null:
        return null;
    }
  }

  const error = $derived(errorLabel());

  /** 卡片收起态：纯前端展示态，收起时仅保留标题栏与控制按钮 */
  let collapsed = $state(false);

  /** 位置候选：前后缀与自动编号共用 */
  const edgeItems: { value: RenamerEdge; label: string }[] = [
    { value: "prefix", label: m.tool_renamer_rule_position_prefix() },
    { value: "suffix", label: m.tool_renamer_rule_position_suffix() },
  ];

  /** 前后缀操作候选 */
  const affixModeItems: { value: RenamerAffixMode; label: string }[] = [
    { value: "add", label: m.tool_renamer_rule_affix_mode_add() },
    { value: "remove", label: m.tool_renamer_rule_affix_mode_remove() },
  ];

  /** 前后缀操作名文案（标题栏带出当前模式用） */
  function affixModeLabel(mode: RenamerAffixMode): string {
    return mode === "add"
      ? m.tool_renamer_rule_affix_mode_add()
      : m.tool_renamer_rule_affix_mode_remove();
  }

  /** 规范化预设候选 */
  const normalizePresetItems: { value: RenamerNormalizePreset; label: string }[] = [
    { value: "trim", label: m.tool_renamer_rule_normalize_preset_trim() },
    { value: "remove-spaces", label: m.tool_renamer_rule_normalize_preset_remove_spaces() },
    { value: "remove-illegal", label: m.tool_renamer_rule_normalize_preset_remove_illegal() },
    { value: "collapse-spaces", label: m.tool_renamer_rule_normalize_preset_collapse_spaces() },
    {
      value: "spaces-to-underscore",
      label: m.tool_renamer_rule_normalize_preset_spaces_underscore(),
    },
    { value: "spaces-to-hyphen", label: m.tool_renamer_rule_normalize_preset_spaces_hyphen() },
  ];

  /** 大小写候选 */
  const caseItems: { value: RenamerCaseMode; label: string }[] = [
    { value: "upper", label: m.tool_renamer_rule_case_upper() },
    { value: "lower", label: m.tool_renamer_rule_case_lower() },
    { value: "sentence", label: m.tool_renamer_rule_case_sentence() },
    { value: "title", label: m.tool_renamer_rule_case_title() },
  ];

  /** 查找替换方式候选 */
  const replaceModeItems: { value: RenamerReplaceMode; label: string }[] = [
    { value: "plain", label: m.tool_renamer_rule_replace_mode_plain() },
    { value: "regex", label: m.tool_renamer_rule_replace_mode_regex() },
  ];

  /** 查找替换方式名文案（标题栏摘要用） */
  function replaceModeLabel(mode: RenamerReplaceMode): string {
    return replaceModeItems.find((item) => item.value === mode)?.label ?? mode;
  }

  /** 大小写模式名文案（标题栏摘要用） */
  function caseModeLabel(mode: RenamerCaseMode): string {
    return caseItems.find((item) => item.value === mode)?.label ?? mode;
  }

  /** 规范化预设名文案（标题栏摘要用） */
  function normalizePresetLabel(preset: RenamerNormalizePreset): string {
    return normalizePresetItems.find((item) => item.value === preset)?.label ?? preset;
  }

  /** 字符取舍操作候选 */
  const charsActionItems: { value: RenamerCharsAction; label: string }[] = [
    { value: "delete", label: m.tool_renamer_rule_chars_action_delete() },
    { value: "keep", label: m.tool_renamer_rule_chars_action_keep() },
  ];

  /** 字符取舍类别候选 */
  const charsClassItems: { value: RenamerCharsClass; label: string }[] = [
    { value: "digits", label: m.tool_renamer_rule_chars_class_digits() },
    { value: "letters", label: m.tool_renamer_rule_chars_class_letters() },
    { value: "chinese", label: m.tool_renamer_rule_chars_class_chinese() },
    { value: "spaces", label: m.tool_renamer_rule_chars_class_spaces() },
    { value: "custom", label: m.tool_renamer_rule_chars_class_custom() },
  ];

  /** 字符取舍类别名文案（标题栏摘要用） */
  function charsClassLabel(cls: RenamerCharsClass): string {
    return charsClassItems.find((item) => item.value === cls)?.label ?? cls;
  }

  /** 下拉回写：候选外的值直接丢弃（bits-ui 给 string，先收窄） */
  function handleEdgeChange(next: string): void {
    if (next !== "prefix" && next !== "suffix") return;
    if (rule.kind === "affix" || rule.kind === "number") {
      onRuleChange({ ...rule, position: next });
    }
  }

  /** 前后缀操作回写 */
  function handleAffixModeChange(next: string): void {
    if (next !== "add" && next !== "remove") return;
    if (rule.kind === "affix") onRuleChange({ ...rule, mode: next });
  }

  /** 大小写敏感回写 */
  function handleMatchCaseChange(next: string): void {
    if (next !== "sensitive" && next !== "insensitive") return;
    if (rule.kind === "replace") onRuleChange({ ...rule, matchCase: next === "sensitive" });
  }

  /** 字符取舍操作回写 */
  function handleCharsActionChange(next: string): void {
    if (next !== "delete" && next !== "keep") return;
    if (rule.kind === "chars") onRuleChange({ ...rule, action: next });
  }

  /** 字符取舍类别回写 */
  function handleCharsClassChange(next: string): void {
    if (
      next !== "digits" &&
      next !== "letters" &&
      next !== "chinese" &&
      next !== "spaces" &&
      next !== "custom"
    )
      return;
    if (rule.kind === "chars") onRuleChange({ ...rule, class: next });
  }

  /** 大小写回写 */
  function handleCaseChange(next: string): void {
    if (next !== "upper" && next !== "lower" && next !== "sentence" && next !== "title") return;
    if (rule.kind === "case") onRuleChange({ ...rule, mode: next });
  }

  /** 查找替换方式回写 */
  function handleReplaceModeChange(next: string): void {
    if (next !== "plain" && next !== "regex") return;
    if (rule.kind === "replace") onRuleChange({ ...rule, mode: next });
  }

  /** 大小写敏感候选（`matchCase` 布尔值的下拉映射） */
  const matchCaseItems: { value: "sensitive" | "insensitive"; label: string }[] = [
    { value: "sensitive", label: m.tool_renamer_rule_match_case() },
    { value: "insensitive", label: m.tool_renamer_rule_ignore_case() },
  ];

  /** 规范化预设回写 */
  function handleNormalizePresetChange(next: string): void {
    if (
      next !== "trim" &&
      next !== "remove-spaces" &&
      next !== "remove-illegal" &&
      next !== "collapse-spaces" &&
      next !== "spaces-to-underscore" &&
      next !== "spaces-to-hyphen"
    )
      return;
    if (rule.kind === "normalize") onRuleChange({ ...rule, preset: next });
  }
</script>

<div
  role="listitem"
  ondragover={(event) => onCardDragOver(event, rule.id)}
  ondrop={(event) => onCardDrop(event, rule.id)}
  class={cn("flex shrink-0 flex-col gap-0.5")}
>
  {#if dropBefore}
    <div class={cn("h-0.5 shrink-0 rounded-full bg-primary")} aria-hidden="true"></div>
  {/if}
  <Card class={cn(!rule.enabled && "opacity-60")}>
    <CardContent class={cn("flex flex-col gap-0.5")}>
      <div class={cn("flex items-center gap-1")}>
        <Button
          variant="ghost"
          size="icon-xs"
          draggable="true"
          ondragstart={(event) => onHandleDragStart(event, rule.id)}
          ondragend={onDragEnd}
          aria-label={m.tool_renamer_rule_drag()}
          class={cn("-ml-1 cursor-grab active:cursor-grabbing")}
        >
          <GripVerticalIcon />
        </Button>
        <span class={cn("text-xs font-medium tabular-nums")}>#{index + 1}</span>
        <span class={cn("min-w-0 flex-1 truncate text-sm font-medium")} title={titleLabel()}
          >{titleLabel()}</span
        >
        {#if error}
          <Badge
            variant="outline"
            class={cn("border-transparent bg-destructive/10 text-destructive")}
            title={error}
          >
            <TriangleAlertIcon />
          </Badge>
        {/if}
        <Button
          variant="ghost"
          size="icon-xs"
          onclick={() => (collapsed = !collapsed)}
          aria-label={m.tool_renamer_rule_collapse()}
          aria-expanded={!collapsed}
          title={m.tool_renamer_rule_collapse()}
          class={cn("aria-expanded:bg-transparent aria-expanded:text-current")}
        >
          <ChevronDownIcon class={cn("transition-transform", !collapsed && "rotate-180")} />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onclick={() => onRuleChange({ ...rule, enabled: !rule.enabled })}
          aria-label={m.tool_renamer_rule_enable()}
          aria-pressed={rule.enabled}
          title={m.tool_renamer_rule_enable()}
          class={cn(rule.enabled ? "text-primary" : "text-muted-foreground")}
        >
          <PowerIcon />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          onclick={() => onRemove(rule.id)}
          aria-label={m.tool_renamer_rule_remove()}
        >
          <XIcon />
        </Button>
      </div>

      {#if !collapsed}
        {#if rule.kind === "affix"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_affix_mode()}>
              <Select
                type="single"
                value={rule.mode}
                items={affixModeItems}
                onValueChange={handleAffixModeChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each affixModeItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField label={m.tool_renamer_rule_position()}>
              <Select
                type="single"
                value={rule.position}
                items={edgeItems}
                onValueChange={handleEdgeChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each edgeItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField label={m.tool_renamer_rule_text()} span>
              <Input
                class={cn("h-7")}
                value={rule.text}
                oninput={(event) => onRuleChange({ ...rule, text: event.currentTarget.value })}
              />
            </RuleField>
          </div>
        {:else if rule.kind === "case"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_case_mode()} span>
              <Select
                type="single"
                value={rule.mode}
                items={caseItems}
                onValueChange={handleCaseChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each caseItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
          </div>
        {:else if rule.kind === "replace"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_replace_mode()}>
              <Select
                type="single"
                value={rule.mode}
                items={replaceModeItems}
                onValueChange={handleReplaceModeChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each replaceModeItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField label={m.tool_renamer_rule_case_sensitivity()}>
              <Select
                type="single"
                value={rule.matchCase ? "sensitive" : "insensitive"}
                items={matchCaseItems}
                onValueChange={handleMatchCaseChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each matchCaseItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField
              label={rule.mode === "regex"
                ? m.tool_renamer_rule_pattern()
                : m.tool_renamer_rule_find()}
              span
            >
              <Input
                class={cn("h-7")}
                value={rule.find}
                spellcheck={rule.mode !== "regex"}
                placeholder={rule.mode === "regex"
                  ? m.tool_renamer_pattern_placeholder()
                  : undefined}
                oninput={(event) => onRuleChange({ ...rule, find: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_replacement()} span>
              <Input
                class={cn("h-7")}
                value={rule.replacement}
                spellcheck={rule.mode !== "regex"}
                placeholder={rule.mode === "regex"
                  ? m.tool_renamer_replacement_placeholder()
                  : undefined}
                oninput={(event) =>
                  onRuleChange({ ...rule, replacement: event.currentTarget.value })}
              />
            </RuleField>
          </div>
        {:else if rule.kind === "number"}
          <div class={cn("grid grid-cols-3 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_number_start()}>
              <Input
                class={cn("h-7")}
                value={rule.start}
                inputmode="numeric"
                oninput={(event) => onRuleChange({ ...rule, start: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_number_step()}>
              <Input
                class={cn("h-7")}
                value={rule.step}
                inputmode="numeric"
                oninput={(event) => onRuleChange({ ...rule, step: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_number_digits()}>
              <Input
                class={cn("h-7")}
                value={rule.digits}
                inputmode="numeric"
                oninput={(event) => onRuleChange({ ...rule, digits: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_position()}>
              <Select
                type="single"
                value={rule.position}
                items={edgeItems}
                onValueChange={handleEdgeChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each edgeItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField label={m.tool_renamer_rule_number_format()}>
              <Input
                class={cn("h-7")}
                value={rule.format}
                spellcheck={false}
                placeholder="$n"
                oninput={(event) => onRuleChange({ ...rule, format: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_number_separator()}>
              <Input
                class={cn("h-7")}
                value={rule.separator}
                oninput={(event) => onRuleChange({ ...rule, separator: event.currentTarget.value })}
              />
            </RuleField>
          </div>
        {:else if rule.kind === "normalize"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_normalize_preset()} span>
              <Select
                type="single"
                value={rule.preset}
                items={normalizePresetItems}
                onValueChange={handleNormalizePresetChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each normalizePresetItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
          </div>
        {:else if rule.kind === "slice"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_slice_anchor()}>
              <Input
                class={cn("h-7")}
                value={rule.anchor}
                inputmode="numeric"
                oninput={(event) => onRuleChange({ ...rule, anchor: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_slice_length()}>
              <Input
                class={cn("h-7")}
                value={rule.length}
                inputmode="numeric"
                oninput={(event) => onRuleChange({ ...rule, length: event.currentTarget.value })}
              />
            </RuleField>
          </div>
          <p class={cn("text-xs text-muted-foreground")}>{m.tool_renamer_rule_slice_hint()}</p>
        {:else if rule.kind === "insert"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_position()}>
              <Input
                class={cn("h-7")}
                value={rule.position}
                inputmode="numeric"
                oninput={(event) => onRuleChange({ ...rule, position: event.currentTarget.value })}
              />
            </RuleField>
            <RuleField label={m.tool_renamer_rule_text()}>
              <Input
                class={cn("h-7")}
                value={rule.text}
                oninput={(event) => onRuleChange({ ...rule, text: event.currentTarget.value })}
              />
            </RuleField>
          </div>
          <p class={cn("text-xs text-muted-foreground")}>{m.tool_renamer_rule_insert_hint()}</p>
        {:else if rule.kind === "chars"}
          <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
            <RuleField label={m.tool_renamer_rule_chars_action()}>
              <Select
                type="single"
                value={rule.action}
                items={charsActionItems}
                onValueChange={handleCharsActionChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each charsActionItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField label={m.tool_renamer_rule_chars_class()}>
              <Select
                type="single"
                value={rule.class}
                items={charsClassItems}
                onValueChange={handleCharsClassChange}
              >
                <SelectTrigger class={cn("h-7 w-full")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each charsClassItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </RuleField>
            <RuleField label={m.tool_renamer_rule_chars_custom()} span>
              <Input
                class={cn("h-7")}
                value={rule.custom}
                disabled={rule.class !== "custom"}
                spellcheck={false}
                placeholder={m.tool_renamer_rule_chars_custom_placeholder()}
                oninput={(event) => onRuleChange({ ...rule, custom: event.currentTarget.value })}
              />
            </RuleField>
          </div>
        {/if}
      {/if}
    </CardContent>
  </Card>
  {#if dropAfter}
    <div class={cn("h-0.5 shrink-0 rounded-full bg-primary")} aria-hidden="true"></div>
  {/if}
</div>
