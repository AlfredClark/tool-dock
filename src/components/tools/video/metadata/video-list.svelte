<script lang="ts">
  // 视频列表（左栏）：文件名 + 时长/分辨率/大小 + 状态徽章，点击选中联动中间预览。
  // 纯展示组件：增删选逻辑由 workspace 持有，此处只做回调转发；底部汇总条常驻上次批处理结果。
  // 行内不放海报缩略图（抽帧按次计费，中栏选中项才抽帧并缓存）。
  import FilmIcon from "@lucide/svelte/icons/film";
  import PlusIcon from "@lucide/svelte/icons/plus";
  import RotateCcwIcon from "@lucide/svelte/icons/rotate-ccw";
  import Trash2Icon from "@lucide/svelte/icons/trash-2";
  import TriangleAlertIcon from "@lucide/svelte/icons/triangle-alert";
  import XIcon from "@lucide/svelte/icons/x";
  import { Badge } from "$components/shadcn-svelte/badge";
  import { Button } from "$components/shadcn-svelte/button";
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import { formatClock } from "./metadata-template";
  import { formatDownloadSize, type VideoItem, type VideoSummary } from "./metadata-types";

  interface Props {
    /** 列表项：顺序即加入顺序 */
    items: VideoItem[];
    /** 当前选中项 id：`null` 即未选中 */
    selectedId: string | null;
    /** 选中回调 */
    onSelect: (id: string) => void;
    /** 移除单项回调 */
    onRemove: (id: string) => void;
    /** 添加视频回调（对话框） */
    onAdd: () => void;
    /** 清空列表回调 */
    onClear: () => void;
    /** 批量汇总：处理完成后常驻底部，列表变更即失效（`null` 不展示） */
    summary: VideoSummary | null;
    /** 处理中标志：禁用重试按钮 */
    processing: boolean;
    /** 重试失败项回调（失败项重入队列，零后端改动） */
    onRetry: () => void;
  }

  let {
    items,
    selectedId,
    onSelect,
    onRemove,
    onAdd,
    onClear,
    summary,
    processing,
    onRetry,
  }: Props = $props();

  /** 状态徽章样式：完成绿、失败/不可读红、就绪描边 */
  function statusClass(status: VideoItem["status"]): string {
    switch (status) {
      case "done":
        return "border-transparent bg-emerald-500/15 text-emerald-600 dark:text-emerald-400";
      case "failed":
      case "invalid":
        return "border-transparent bg-destructive/10 text-destructive";
      default:
        return "";
    }
  }

  /** 状态徽章：仅失败/不可读展示（就绪显示元信息，完成由预览区输出路径表达） */
  function showStatusBadge(status: VideoItem["status"]): boolean {
    return status === "failed" || status === "invalid";
  }

  /** 行元信息：时长 · 分辨率 · 大小（缺失项跳过，无视频流显示无流） */
  function metaLine(item: VideoItem): string {
    const parts: string[] = [];
    if (item.info?.duration_seconds != null) {
      parts.push(formatClock(item.info.duration_seconds));
    }
    const stream = item.info?.stream;
    if (stream?.width != null && stream?.height != null) {
      parts.push(`${stream.width}×${stream.height}`);
    } else if (item.info) {
      parts.push(m.tool_video_info_no_stream());
    }
    if (item.info?.file_size != null) {
      parts.push(formatDownloadSize(item.info.file_size));
    }
    return parts.join(" · ");
  }
</script>

<section
  aria-label={m.tool_video_list_title()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <div class={cn("flex h-10 shrink-0 items-center justify-between gap-2 border-b px-2")}>
    <span class={cn("text-xs font-medium text-muted-foreground")}>
      {`${m.tool_video_list_title()} · ${m.tool_video_list_count({ count: items.length })}`}
    </span>
    <div class={cn("flex items-center gap-1")}>
      <Button variant="ghost" size="sm" onclick={onAdd}>
        <PlusIcon />
        {m.tool_video_list_add()}
      </Button>
      {#if items.length > 0}
        <Button variant="ghost" size="sm" onclick={onClear} aria-label={m.tool_video_list_clear()}>
          <Trash2Icon />
        </Button>
      {/if}
    </div>
  </div>
  <ScrollArea class="min-h-0 flex-1">
    {#if items.length === 0}
      <p class={cn("px-4 py-8 text-center text-sm text-muted-foreground")}>
        {m.tool_video_list_empty()}
      </p>
    {:else}
      <ul class={cn("flex flex-col gap-1 p-2")}>
        {#each items as item (item.id)}
          <li>
            <!-- 行容器仅布局：选中用原生 button（键盘/读屏正常），移除是兄弟按钮，不再嵌套可交互元素 -->
            <div
              class={cn(
                "flex w-full items-center gap-2 rounded-md border p-1.5",
                "transition-colors hover:bg-muted/60",
                selectedId === item.id
                  ? "border-primary bg-muted/60"
                  : "border-transparent bg-transparent",
              )}
            >
              <button
                type="button"
                onclick={() => onSelect(item.id)}
                aria-pressed={selectedId === item.id}
                class={cn("flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left")}
              >
                <FilmIcon class={cn("size-10 shrink-0 rounded border p-2 opacity-60")} />
                <span class={cn("flex min-w-0 flex-1 flex-col gap-0.5")}>
                  <span class={cn("truncate text-sm font-medium")} title={item.path || item.name}>
                    {item.name}
                  </span>
                  <span class={cn("flex items-center gap-1.5 text-xs text-muted-foreground")}>
                    {#if item.status === "loading"}
                      <span class={cn("tabular-nums")}>…</span>
                    {:else}
                      {@const meta = metaLine(item)}
                      {#if meta !== ""}
                        <span class={cn("truncate tabular-nums")}>{meta}</span>
                      {/if}
                    {/if}
                    {#if showStatusBadge(item.status)}
                      <Badge
                        variant="outline"
                        class={statusClass(item.status)}
                        title={item.errorDetail ?? undefined}
                      >
                        <TriangleAlertIcon />
                        {m.tool_video_list_invalid()}
                      </Badge>
                    {/if}
                    {#if item.status === "skipped"}
                      <Badge variant="outline" title={item.errorDetail ?? undefined}>
                        {m.tool_video_list_skipped()}
                      </Badge>
                    {/if}
                    {#if item.status === "done" && item.output}
                      <span
                        class={cn("truncate text-emerald-600 tabular-nums dark:text-emerald-400")}
                        title={item.output}
                      >
                        → {item.output}
                      </span>
                    {/if}
                  </span>
                </span>
              </button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={m.tool_video_list_remove()}
                onclick={(event) => {
                  event.stopPropagation();
                  onRemove(item.id);
                }}
                class="shrink-0"
              >
                <XIcon />
              </Button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </ScrollArea>
  {#if summary}
    <!-- 批量汇总条：toast 消失后仍可查看结果，失败项一键重试 -->
    <div class={cn("flex shrink-0 items-center justify-between gap-2 border-t px-2 py-2")}>
      <span class={cn("min-w-0 flex-1 truncate text-xs text-muted-foreground")}>
        {m.tool_video_done({ ok: summary.ok, failed: summary.failed, skipped: summary.skipped })}
      </span>
      {#if summary.failed > 0}
        <Button variant="outline" size="sm" disabled={processing} onclick={onRetry}>
          <RotateCcwIcon />
          {m.tool_video_retry_failed()}
        </Button>
      {/if}
    </div>
  {/if}
</section>
