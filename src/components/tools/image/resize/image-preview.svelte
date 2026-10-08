<script lang="ts">
  // 图片预览（中栏信息面板）：原图 + 下方元信息网格（原尺寸/大小/格式/预计输出），
  // `done` 后追加输出尺寸与输出路径，`failed` 显示错误详情。
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import { formatBytes } from "./resize-math";
  import type { ResizeImageItem } from "./resize-types";

  interface Props {
    /** 当前选中项：`null` 即空态提示 */
    item: ResizeImageItem | null;
    /** 预计输出尺寸：参数合法且选中项就绪时有值 */
    estimated: { width: number; height: number } | null;
    /** 目标大小是否被忽略：有目标但选中项非 JPEG 输出（与“未达成”区分展示） */
    targetIgnored: boolean;
  }

  let { item, estimated, targetIgnored }: Props = $props();
</script>

<section
  aria-label={m.tool_resize_preview_title()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <div class={cn("flex h-10 shrink-0 items-center justify-between gap-2 border-b px-3")}>
    <span class={cn("truncate text-xs font-medium text-muted-foreground")}>
      {item ? `${m.tool_resize_preview_original()} · ${item.name}` : m.tool_resize_preview_title()}
    </span>
    {#if item?.info && estimated}
      <span class={cn("shrink-0 text-xs text-muted-foreground tabular-nums")}>
        {m.tool_resize_preview_estimated({ width: estimated.width, height: estimated.height })}
      </span>
    {/if}
  </div>
  {#if !item}
    <div class={cn("flex flex-1 items-center justify-center p-8")}>
      <p class={cn("text-center text-sm text-muted-foreground")}>
        {m.tool_resize_preview_empty()}
      </p>
    </div>
  {:else}
    <div
      class={cn("flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-muted/40 p-4")}
    >
      {#if item.previewUrl !== ""}
        <img
          src={item.previewUrl}
          alt={item.name}
          class={cn("max-h-full max-w-full rounded border object-contain shadow-sm")}
        />
      {:else}
        <!-- 缩略图失败兜底：元信息仍在下方展示，此处仅提示无图（`loading` 态短暂经过） -->
        <p class={cn("text-center text-sm text-muted-foreground")}>
          {m.tool_resize_list_invalid()}
        </p>
      {/if}
    </div>
    <div
      class={cn(
        "flex shrink-0 flex-wrap items-center gap-x-4 gap-y-1 border-t px-3 py-2",
        "text-xs text-muted-foreground tabular-nums",
      )}
    >
      {#if item.info}
        {@const original = `${item.info.width}×${item.info.height}`}
        <span>{original}</span>
        <span>{item.info.format.toUpperCase()}</span>
        <span>{formatBytes(item.info.file_size)}</span>
      {/if}
      {#if item.outputSize}
        {@const produced = `→ ${item.outputSize.width}×${item.outputSize.height}`}
        <span class={cn("text-emerald-600 dark:text-emerald-400")}>{produced}</span>
      {/if}
      {#if item.info && targetIgnored}
        <span class={cn("text-amber-600 dark:text-amber-400")}>
          {m.tool_resize_target_not_applicable()}
        </span>
      {:else if item.targetMet === false}
        <span class={cn("text-amber-600 dark:text-amber-400")}>
          {m.tool_resize_target_missed()}
        </span>
      {/if}
      {#if item.output}
        <span class={cn("max-w-full truncate text-emerald-600 dark:text-emerald-400")}>
          {item.output}
        </span>
      {/if}
      {#if item.errorDetail}
        <!-- 跳过是正常分支，用次要色；失败/不可读才用红色 -->
        <span class={cn(item.status === "skipped" ? "text-muted-foreground" : "text-destructive")}>
          {item.errorDetail}
        </span>
      {/if}
    </div>
  {/if}
</section>
