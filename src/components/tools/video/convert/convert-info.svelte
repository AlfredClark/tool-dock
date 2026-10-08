<script lang="ts">
  // 转换信息（中栏下）：技术行 + 目标行（目标格式与预期输出名，随右栏下拉实时变化）。
  // 纯展示组件：选中项与目标格式由 workspace 下发，此处只做只读渲染。
  import type { VideoTarget } from "$libs/commands/bindings";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import {
    expectedOutputName,
    formatClock,
    formatDownloadSize,
    type VideoConvertItem,
  } from "./convert-types";

  interface Props {
    /** 当前选中项：`null` 即不渲染（调用方保证选中才挂载） */
    item: VideoConvertItem | null;
    /** 目标容器：右栏下拉值，预期输出名据此实时计算 */
    target: VideoTarget;
  }

  let { item, target }: Props = $props();

  /** 预期输出名：茎名 + 目标后缀（仅展示，不预留路径） */
  const expectedName = $derived(item ? expectedOutputName(item.path || item.name, target) : "");
</script>

{#if item}
  <div
    class={cn(
      "flex shrink-0 flex-col gap-2 border-t px-3 py-2 text-xs tabular-nums",
      "max-h-64 overflow-y-auto",
    )}
  >
    <div class={cn("flex flex-wrap items-center gap-x-4 gap-y-1 text-muted-foreground")}>
      {#if item.info?.stream?.width != null && item.info?.stream?.height != null}
        <span
          >{m.tool_video_info_resolution()} {item.info.stream.width}×{item.info.stream.height}</span
        >
      {:else}
        <span>{m.tool_video_info_no_stream()}</span>
      {/if}
      {#if item.info?.duration_seconds != null}
        <span>
          {m.tool_video_info_duration()}
          {formatClock(item.info.duration_seconds)}
        </span>
      {/if}
      {#if item.info?.stream?.codec_name}
        <span>{m.tool_video_info_codec()} {item.info.stream.codec_name}</span>
      {/if}
      {#if item.info?.file_size != null}
        <span>{m.tool_video_info_size()} {formatDownloadSize(item.info.file_size)}</span>
      {/if}
      {#if item.info?.format_name}
        <span>{m.tool_video_info_format()} {item.info.format_name.split(",")[0]}</span>
      {/if}
    </div>
    <div class={cn("flex min-w-0 items-baseline gap-2")}>
      <span class={cn("shrink-0 font-medium text-muted-foreground")}>
        {m.tool_video_convert_target_label()}
      </span>
      <span class={cn("min-w-0 flex-1 truncate")} title={expectedName}>
        {target} · {expectedName}
      </span>
    </div>
    {#if item.output}
      <span class={cn("truncate text-emerald-600 dark:text-emerald-400")} title={item.output}>
        → {item.output}
      </span>
    {/if}
    {#if item.note}
      <span class={cn("text-muted-foreground")}>{item.note}</span>
    {/if}
    {#if item.errorDetail}
      <span class={cn(item.status === "skipped" ? "text-muted-foreground" : "text-destructive")}>
        {item.errorDetail}
      </span>
    {/if}
  </div>
{/if}
