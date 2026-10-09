<script lang="ts">
  // 单条规则参数卡：头部（左夹点 + 标题 + 右电源/删除）+ 参数区（标签在上，双列网格）。
  // 纯展示组件：参数编辑整体回写 `onRuleChange`（仿 `resize-params` 模式），非法参数标红但不拦截。
  // 排序走 HTML5 拖放：仅手柄可拖，卡片根负责悬停定位与放置，插入线由 panel 态驱动。
  import GripVerticalIcon from "@lucide/svelte/icons/grip-vertical";
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
  import { Switch } from "$components/shadcn-svelte/switch";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import RuleField from "./rule-field.svelte";
  import { validateRule } from "./renamer-rules";
  import type { RenamerCaseMode, RenamerEdge, RenamerRule } from "./renamer-types";

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

  /** 类型名文案 */
  function kindLabel(): string {
    switch (rule.kind) {
      case "affix":
        return m.tool_renamer_rule_kind_affix();
      case "strip":
        return m.tool_renamer_rule_kind_strip();
      case "case":
        return m.tool_renamer_rule_kind_case();
      case "replace":
        return m.tool_renamer_rule_kind_replace();
      case "regex":
        return m.tool_renamer_rule_kind_regex();
      case "number":
        return m.tool_renamer_rule_kind_number();
      case "normalize":
        return m.tool_renamer_rule_kind_normalize();
    }
  }

  /** 参数错误文案：`null` 即合法（预览层照常跳过非法规则） */
  function errorLabel(): string | null {
    switch (validateRule(rule)) {
      case "bad-count":
        return m.tool_renamer_rule_count_invalid();
      case "bad-regex":
        return m.tool_renamer_rule_pattern_invalid();
      case "bad-number":
        return m.tool_renamer_rule_number_invalid();
      case null:
        return null;
    }
  }

  const error = $derived(errorLabel());

  /** 位置候选：前后缀两规则共用 */
  const edgeItems: { value: RenamerEdge; label: string }[] = [
    { value: "prefix", label: m.tool_renamer_rule_position_prefix() },
    { value: "suffix", label: m.tool_renamer_rule_position_suffix() },
  ];

  /** 大小写候选 */
  const caseItems: { value: RenamerCaseMode; label: string }[] = [
    { value: "upper", label: m.tool_renamer_rule_case_upper() },
    { value: "lower", label: m.tool_renamer_rule_case_lower() },
    { value: "title", label: m.tool_renamer_rule_case_title() },
  ];

  /** 下拉回写：候选外的值直接丢弃（bits-ui 给 string，先收窄） */
  function handleEdgeChange(next: string): void {
    if (next !== "prefix" && next !== "suffix") return;
    if (rule.kind === "affix" || rule.kind === "strip" || rule.kind === "number") {
      onRuleChange({ ...rule, position: next });
    }
  }

  /** 大小写回写 */
  function handleCaseChange(next: string): void {
    if (next !== "upper" && next !== "lower" && next !== "title") return;
    if (rule.kind === "case") onRuleChange({ ...rule, mode: next });
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
    <CardContent class={cn("flex flex-col gap-0.5 p-1")}>
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
        <span class={cn("min-w-0 flex-1 truncate text-xs font-medium")}>{kindLabel()}</span>
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

      {#if rule.kind === "affix"}
        <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
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
          <RuleField label={m.tool_renamer_rule_text()}>
            <Input
              class={cn("h-7")}
              value={rule.text}
              oninput={(event) => onRuleChange({ ...rule, text: event.currentTarget.value })}
            />
          </RuleField>
        </div>
      {:else if rule.kind === "strip"}
        <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
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
          <RuleField label={m.tool_renamer_rule_count()}>
            <Input
              class={cn("h-7")}
              value={rule.count}
              inputmode="numeric"
              oninput={(event) => onRuleChange({ ...rule, count: event.currentTarget.value })}
            />
          </RuleField>
        </div>
      {:else if rule.kind === "case"}
        <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
          <RuleField label={m.tool_renamer_rule_case_mode()}>
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
          <RuleField label={m.tool_renamer_rule_find()} span>
            <Input
              class={cn("h-7")}
              value={rule.find}
              oninput={(event) => onRuleChange({ ...rule, find: event.currentTarget.value })}
            />
          </RuleField>
          <RuleField label={m.tool_renamer_rule_replacement()} span>
            <Input
              class={cn("h-7")}
              value={rule.replacement}
              oninput={(event) => onRuleChange({ ...rule, replacement: event.currentTarget.value })}
            />
          </RuleField>
          <div class={cn("flex items-center gap-1.5")}>
            <Switch
              checked={rule.matchCase}
              onCheckedChange={(checked) => onRuleChange({ ...rule, matchCase: checked })}
            />
            <span class={cn("text-xs text-muted-foreground")}
              >{m.tool_renamer_rule_match_case()}</span
            >
          </div>
        </div>
      {:else if rule.kind === "regex"}
        <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
          <RuleField label={m.tool_renamer_rule_pattern()} span>
            <Input
              class={cn("h-7")}
              value={rule.pattern}
              spellcheck={false}
              placeholder={m.tool_renamer_pattern_placeholder()}
              oninput={(event) => onRuleChange({ ...rule, pattern: event.currentTarget.value })}
            />
          </RuleField>
          <RuleField label={m.tool_renamer_rule_replacement()} span>
            <Input
              class={cn("h-7")}
              value={rule.replacement}
              spellcheck={false}
              placeholder={m.tool_renamer_replacement_placeholder()}
              oninput={(event) => onRuleChange({ ...rule, replacement: event.currentTarget.value })}
            />
          </RuleField>
        </div>
      {:else if rule.kind === "number"}
        <div class={cn("grid grid-cols-2 gap-x-2 gap-y-0.5")}>
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
          <RuleField label={m.tool_renamer_rule_number_separator()}>
            <Input
              class={cn("h-7")}
              value={rule.separator}
              oninput={(event) => onRuleChange({ ...rule, separator: event.currentTarget.value })}
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
        </div>
      {:else}
        <p class={cn("text-xs text-muted-foreground")}>{m.tool_renamer_rule_normalize_hint()}</p>
      {/if}
    </CardContent>
  </Card>
  {#if dropAfter}
    <div class={cn("h-0.5 shrink-0 rounded-full bg-primary")} aria-hidden="true"></div>
  {/if}
</div>
