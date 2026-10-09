<script lang="ts">
  // 文件列表工具栏：从左到右依次为全选复选框、搜索筛选输入框、排序下拉框、删除选中按钮。
  // 纯展示组件：勾选/筛选/排序状态由 workspace 持有，此处只做回调转发；无勾选时不渲染删除按钮。
  import Trash2Icon from "@lucide/svelte/icons/trash-2";
  import { Button } from "$components/shadcn-svelte/button";
  import { Checkbox } from "$components/shadcn-svelte/checkbox";
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
  import type { RenamerSortKey } from "./renamer-types";

  interface Props {
    /** 表头全选态：`true` 全勾、`"indeterminate"` 部分勾、`false` 未勾（空列表时为 `false` 且禁用） */
    headerChecked: boolean | "indeterminate";
    /** 表头是否可用：可见行数为零时禁用 */
    headerDisabled: boolean;
    /** 搜索筛选文本 */
    query: string;
    /** 当前排序键 */
    sortKey: RenamerSortKey;
    /** 全局已勾选数：为零时不渲染删除按钮 */
    selectedCount: number;
    /** 全选切换回调（只动可见行） */
    onToggleAll: (checked: boolean) => void;
    /** 搜索文本变更回调 */
    onQueryChange: (query: string) => void;
    /** 排序变更回调 */
    onSortChange: (sortKey: RenamerSortKey) => void;
    /** 删除选中回调 */
    onRemoveSelected: () => void;
  }

  let {
    headerChecked,
    headerDisabled,
    query,
    sortKey,
    selectedCount,
    onToggleAll,
    onQueryChange,
    onSortChange,
    onRemoveSelected,
  }: Props = $props();

  /** 排序下拉选项：第一步仅名称升/降序（文案求值一次，语言切换走整页重载） */
  const sortOptions: { value: RenamerSortKey; label: string }[] = [
    { value: "name-asc", label: m.tool_renamer_sort_name_asc() },
    { value: "name-desc", label: m.tool_renamer_sort_name_desc() },
  ];

  /** 复选框变更：bits-ui 可能给出 `"indeterminate"`，按勾选处理 */
  function handleCheckedChange(value: boolean | "indeterminate"): void {
    onToggleAll(value === true || value === "indeterminate");
  }

  /** 排序变更：空值保持当前选项（Select 清空时不回写） */
  function handleSortChange(value: string | undefined): void {
    if (value === "name-asc" || value === "name-desc") onSortChange(value);
  }
</script>

<div class={cn("flex h-10 shrink-0 items-center gap-2 border-b px-2")}>
  <Checkbox
    checked={headerChecked === true}
    indeterminate={headerChecked === "indeterminate"}
    disabled={headerDisabled}
    onCheckedChange={handleCheckedChange}
    aria-label={m.tool_renamer_list_title()}
  />
  <Input
    value={query}
    oninput={(event) => onQueryChange(event.currentTarget.value)}
    placeholder={m.tool_renamer_search_placeholder()}
    aria-label={m.tool_renamer_search_placeholder()}
    class={cn("min-w-0 flex-1")}
  />
  <Select type="single" value={sortKey} items={sortOptions} onValueChange={handleSortChange}>
    <SelectTrigger aria-label={m.tool_renamer_sort_label()} class={cn("w-28 shrink-0")}>
      <SelectValue />
    </SelectTrigger>
    <SelectContent>
      {#each sortOptions as option (option.value)}
        <SelectItem value={option.value}>{option.label}</SelectItem>
      {/each}
    </SelectContent>
  </Select>
  {#if selectedCount > 0}
    <Button
      variant="ghost"
      size="sm"
      onclick={onRemoveSelected}
      aria-label={m.tool_renamer_remove_selected()}
      class={cn("shrink-0")}
    >
      <Trash2Icon />
      {m.tool_renamer_remove_selected()} · {selectedCount}
    </Button>
  {/if}
</div>
