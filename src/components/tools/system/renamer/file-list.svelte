<script lang="ts">
  // 文件列表（右栏下）：行复选框 + 原文件名 + 新文件名 + 执行结果徽章（跳过/失败常驻）。
  // 纯展示组件：增删勾选逻辑由 workspace 持有，此处只做回调转发；空态展示添加按钮。
  import PlusIcon from "@lucide/svelte/icons/plus";
  import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
  import XIcon from "@lucide/svelte/icons/x";
  import { Badge } from "$components/shadcn-svelte/badge";
  import { Button } from "$components/shadcn-svelte/button";
  import { Checkbox } from "$components/shadcn-svelte/checkbox";
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import type { RenamerFileItem } from "./renamer-types";

  interface Props {
    /** 可见行（已筛选 + 已排序）：顺序即展示顺序 */
    items: RenamerFileItem[];
    /** 单行勾选回调 */
    onToggleOne: (id: string, checked: boolean) => void;
    /** 移除单行回调 */
    onRemoveOne: (id: string) => void;
    /** 添加文件回调（对话框） */
    onAdd: () => void;
  }

  let { items, onToggleOne, onRemoveOne, onAdd }: Props = $props();
</script>

<section
  aria-label={m.tool_renamer_list_title()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <ScrollArea class="min-h-0 flex-1">
    {#if items.length === 0}
      <div class={cn("flex flex-col items-center gap-3 px-4 py-8")}>
        <p class={cn("text-center text-sm text-muted-foreground")}>
          {m.tool_renamer_list_empty()}
        </p>
        <Button variant="outline" size="sm" onclick={onAdd}>
          <PlusIcon />
          {m.tool_renamer_list_add()}
        </Button>
      </div>
    {:else}
      <ul class={cn("flex flex-col gap-1 p-2")}>
        {#each items as item (item.id)}
          <li
            class={cn(
              "flex w-full items-center gap-2 rounded-md border p-1.5",
              "transition-colors hover:bg-muted/60",
              item.checked ? "border-primary bg-muted/60" : "border-transparent bg-transparent",
            )}
          >
            <Checkbox
              checked={item.checked}
              onCheckedChange={(value) => onToggleOne(item.id, value === true)}
              aria-label={item.name}
              class={cn("shrink-0")}
            />
            <span class={cn("flex min-w-0 flex-1 items-center gap-2")}>
              <span class={cn("min-w-0 flex-1 truncate text-sm font-medium")} title={item.name}>
                {item.name}
              </span>
              <span class={cn("shrink-0 text-xs text-muted-foreground")} aria-hidden="true">→</span>
              <span
                class={cn("min-w-0 flex-1 truncate text-sm text-muted-foreground")}
                title={item.newName}
              >
                {item.newName}
              </span>
              {#if item.error}
                <Badge
                  variant="outline"
                  class={cn("border-transparent bg-destructive/10 text-destructive")}
                  title={item.error.detail}
                >
                  <TriangleAlertIcon />
                  {item.error.short}
                </Badge>
              {/if}
            </span>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={m.tool_renamer_list_remove()}
              onclick={() => onRemoveOne(item.id)}
              class="shrink-0"
            >
              <XIcon />
            </Button>
          </li>
        {/each}
      </ul>
    {/if}
  </ScrollArea>
</section>
