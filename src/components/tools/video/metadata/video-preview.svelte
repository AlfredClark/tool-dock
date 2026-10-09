<script lang="ts">
  // 视频预览（中栏上）：选中项海报帧 + 标题栏（海报 · 文件名），逻辑在 workspace。
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import type { VideoItem } from "./metadata-types";

  interface Props {
    /** 当前选中项：`null` 即空态提示 */
    item: VideoItem | null;
  }

  let { item }: Props = $props();

  /** 加载态：条目批量载入中，或大海报在途且暂无任何图可显（此时显示“不可读”属误导） */
  const showLoading = $derived(
    item !== null &&
      (item.status === "loading" ||
        (item.detailLoading && item.detailUrl === "" && item.previewUrl === "")),
  );

  /** 当前展示图：大海报优先，小海报兜底（加载态下不消费，骨架占位） */
  const imageSrc = $derived(
    item !== null && item.detailUrl !== "" ? item.detailUrl : (item?.previewUrl ?? ""),
  );
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
      {#if showLoading}
        <!-- 批量载入中：骨架占位 + 文案，不再误显示“不可读” -->
        <div class={cn("flex flex-col items-center justify-center gap-2")}>
          <span class={cn("h-16 w-16 animate-pulse rounded border bg-muted/60")} aria-hidden="true"
          ></span>
          <p class={cn("text-center text-sm text-muted-foreground")}>
            {m.tool_preview_loading()}
          </p>
        </div>
      {:else if imageSrc !== ""}
        <img
          src={imageSrc}
          alt={item.name}
          decoding="async"
          class={cn("max-h-full max-w-full rounded border object-contain shadow-sm")}
        />
      {:else}
        <!-- 抽帧失败兜底：元信息仍在下方展示，此处仅提示无图 -->
        <p class={cn("text-center text-sm text-muted-foreground")}>
          {m.tool_video_list_invalid()}
        </p>
      {/if}
    </div>
  {/if}
</section>
