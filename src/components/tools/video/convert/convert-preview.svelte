<script lang="ts">
  // 转换预览（中栏上）：选中项海报帧 + 标题栏（海报 · 文件名），逻辑在 workspace。
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import type { VideoConvertItem } from "./convert-types";

  interface Props {
    /** 当前选中项：`null` 即空态提示 */
    item: VideoConvertItem | null;
  }

  let { item }: Props = $props();
</script>

<section
  aria-label={m.tool_video_preview_title()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <div class={cn("flex h-10 shrink-0 items-center justify-between gap-2 border-b px-3")}>
    <span class={cn("truncate text-xs font-medium text-muted-foreground")}>
      {item ? `${m.tool_video_preview_poster()} · ${item.name}` : m.tool_video_preview_title()}
    </span>
  </div>
  {#if !item}
    <div class={cn("flex flex-1 items-center justify-center p-8")}>
      <p class={cn("text-center text-sm text-muted-foreground")}>
        {m.tool_video_preview_empty()}
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
        <!-- 抽帧失败兜底：元信息仍在下方展示，此处仅提示无图（`loading` 态短暂经过） -->
        <p class={cn("text-center text-sm text-muted-foreground")}>
          {m.tool_video_list_invalid()}
        </p>
      {/if}
    </div>
  {/if}
</section>
