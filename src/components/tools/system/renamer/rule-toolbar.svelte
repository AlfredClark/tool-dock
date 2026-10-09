<script lang="ts">
  // 规则工具栏：左清空规则（无规则时隐藏）+ 右添加规则（下拉框选类型）。
  // 纯展示组件：规则数组由 workspace 持有，此处只做回调转发。
  import HashIcon from "@lucide/svelte/icons/hash";
  import PlusIcon from "@lucide/svelte/icons/plus";
  import RegexIcon from "@lucide/svelte/icons/regex";
  import ReplaceIcon from "@lucide/svelte/icons/replace";
  import ScissorsIcon from "@lucide/svelte/icons/scissors";
  import SparklesIcon from "@lucide/svelte/icons/sparkles";
  import Trash2Icon from "@lucide/svelte/icons/trash-2";
  import TypeIcon from "@lucide/svelte/icons/type";
  import { Button } from "$components/shadcn-svelte/button";
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
  } from "$components/shadcn-svelte/dropdown-menu";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import type { RenamerRuleKind } from "./renamer-types";

  interface Props {
    /** 规则条数：为零时隐藏清空按钮 */
    ruleCount: number;
    /** 清空规则回调 */
    onClear: () => void;
    /** 添加规则回调（类型由下拉菜单选定） */
    onAdd: (kind: RenamerRuleKind) => void;
  }

  let { ruleCount, onClear, onAdd }: Props = $props();

  /** 规则候选：类型 + 图标 + 文案，顺序即下拉展示顺序 */
  const kindOptions: { kind: RenamerRuleKind; icon: typeof PlusIcon; label: string }[] = [
    { kind: "affix", icon: PlusIcon, label: m.tool_renamer_rule_kind_affix() },
    { kind: "strip", icon: ScissorsIcon, label: m.tool_renamer_rule_kind_strip() },
    { kind: "case", icon: TypeIcon, label: m.tool_renamer_rule_kind_case() },
    { kind: "replace", icon: ReplaceIcon, label: m.tool_renamer_rule_kind_replace() },
    { kind: "regex", icon: RegexIcon, label: m.tool_renamer_rule_kind_regex() },
    { kind: "number", icon: HashIcon, label: m.tool_renamer_rule_kind_number() },
    { kind: "normalize", icon: SparklesIcon, label: m.tool_renamer_rule_kind_normalize() },
  ];
</script>

<div class={cn("flex h-10 shrink-0 items-center justify-between gap-2 border-b px-2")}>
  {#if ruleCount > 0}
    <Button variant="ghost" size="sm" onclick={onClear} aria-label={m.tool_renamer_rule_clear()}>
      <Trash2Icon />
      {m.tool_renamer_rule_clear()}
    </Button>
  {:else}
    <span></span>
  {/if}
  <DropdownMenu>
    <DropdownMenuTrigger>
      {#snippet child({ props })}
        <Button variant="ghost" size="sm" {...props}>
          <PlusIcon />
          {m.tool_renamer_rule_add()}
        </Button>
      {/snippet}
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end">
      {#each kindOptions as option (option.kind)}
        {@const Icon = option.icon}
        <DropdownMenuItem onSelect={() => onAdd(option.kind)}>
          <Icon />
          {option.label}
        </DropdownMenuItem>
      {/each}
    </DropdownMenuContent>
  </DropdownMenu>
</div>
