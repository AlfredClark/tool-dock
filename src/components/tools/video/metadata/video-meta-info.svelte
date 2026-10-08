<script lang="ts">
  // 元数据信息（中栏下）：技术行 + 当前标签表 + 写入预览（按选中项实时展开模板）。
  // 纯展示组件：未知占位符红色高亮（开始按钮由 workspace 统一拦截），缺值占位同样标红。
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import { contextForItem, expandTemplate, formatClock } from "./metadata-template";
  import { TAG_KEYS, type TagKey } from "./metadata-types";
  import { formatDownloadSize, type VideoItem } from "./metadata-types";

  interface Props {
    /** 当前选中项：`null` 即不渲染（调用方保证选中才挂载） */
    item: VideoItem | null;
    /** 六字段模板：空即保持原值 */
    templates: Record<TagKey, string>;
    /** 清空标记：置位即该字段写空串 */
    cleared: Record<TagKey, boolean>;
    /** 封面模板：空即保持原封面 */
    coverFile: string;
    /** 封面清除：置位即清除封面 */
    coverClear: boolean;
  }

  let { item, templates, cleared, coverFile, coverClear }: Props = $props();

  /** 字段文案：与后端 `VideoTags` 键一一对应 */
  const tagLabels: Record<TagKey, string> = {
    title: m.tool_video_tag_title(),
    artist: m.tool_video_tag_artist(),
    album: m.tool_video_tag_album(),
    genre: m.tool_video_tag_genre(),
    date: m.tool_video_tag_date(),
    comment: m.tool_video_tag_comment(),
  };

  /** 当前标签读值：后端返回的原文（大小写已归一），缺失即 `null` */
  function currentValue(key: TagKey): string | null {
    return item?.info?.tags[key] ?? null;
  }

  /** 写入预览行：保持（`null`）/ 清空（空串）/ 展开后文本 + 未知标记 */
  function previewRow(key: TagKey): { text: string | null; unknown: boolean } {
    if (cleared[key]) return { text: "", unknown: false };
    const template = templates[key];
    if (template === "") return { text: null, unknown: false };
    if (!item) return { text: null, unknown: false };
    const { text, unknown } = expandTemplate(
      template,
      contextForItem(item.name, item.path, item.info),
    );
    return { text, unknown: unknown.length > 0 };
  }

  /** 行数据：当前读值 + 写入预览（`text` 为 `null` 即保持，空串即清空） */
  const rows = $derived(
    TAG_KEYS.map((key) => ({ key, current: currentValue(key), ...previewRow(key) })),
  );
  /** 选中项全部模板（含封面）的未知占位名（去重）：非空即右栏同步拦截开始 */
  const unknownNames = $derived.by(() => {
    const names: string[] = [];
    const ctx = item == null ? null : contextForItem(item.name, item.path, item.info);
    if (ctx == null) return names;
    const jobs: string[] = [];
    for (const key of TAG_KEYS) {
      if (templates[key] === "" || cleared[key]) continue;
      jobs.push(templates[key]);
    }
    if (coverFile !== "" && !coverClear) jobs.push(coverFile);
    for (const template of jobs) {
      for (const name of expandTemplate(template, ctx).unknown) {
        if (!names.includes(name)) names.push(name);
      }
    }
    return names;
  });

  /** 封面写入预览：保持（`null`）/ 清除（空串）/ 展开后文件名 + 未知标记 */
  const coverPreview = $derived.by((): { text: string | null; unknown: boolean } => {
    if (coverClear) return { text: "", unknown: false };
    if (coverFile === "" || !item) return { text: null, unknown: false };
    const { text, unknown } = expandTemplate(
      coverFile,
      contextForItem(item.name, item.path, item.info),
    );
    return { text, unknown: unknown.length > 0 };
  });
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
      {#if item.info}
        <span>
          {m.tool_video_info_cover()}
          {item.info.has_cover ? m.tool_video_info_cover_yes() : m.tool_video_info_cover_no()}
        </span>
      {/if}
    </div>
    <div class={cn("flex flex-col gap-1")}>
      <span class={cn("font-medium text-muted-foreground")}>{m.tool_video_info_tags()}</span>
      {#if rows.every((row) => row.current == null)}
        <span class={cn("text-muted-foreground")}>{m.tool_video_info_no_tags()}</span>
      {:else}
        {#each rows as row (row.key)}
          {#if row.current != null}
            <div class={cn("flex min-w-0 items-baseline gap-2")}>
              <span class={cn("shrink-0 text-muted-foreground")}>{tagLabels[row.key]}</span>
              <span class={cn("min-w-0 flex-1 truncate")} title={row.current}>{row.current}</span>
            </div>
          {/if}
        {/each}
      {/if}
    </div>
    <div class={cn("flex flex-col gap-1")}>
      <span class={cn("font-medium text-muted-foreground")}>{m.tool_video_info_expanded()}</span>
      {#each rows as row (row.key)}
        <div class={cn("flex min-w-0 items-baseline gap-2")}>
          <span class={cn("shrink-0 text-muted-foreground")}>{tagLabels[row.key]}</span>
          {#if row.text == null}
            <span class={cn("text-muted-foreground")}>—</span>
          {:else if row.text === ""}
            <span class={cn("text-amber-600 dark:text-amber-400")}>
              {m.tool_video_tag_cleared()}
            </span>
          {:else}
            <span
              class={cn("min-w-0 flex-1 truncate", row.unknown && "text-destructive")}
              title={row.text}
            >
              {row.text}
            </span>
          {/if}
        </div>
      {/each}
      {#if unknownNames.length > 0}
        <span class={cn("text-destructive")}>
          {m.tool_video_unknown_placeholder({
            names: unknownNames.map((name) => `%${name}%`).join(" "),
          })}
        </span>
      {/if}
    </div>
    <div class={cn("flex flex-col gap-1")}>
      <span class={cn("font-medium text-muted-foreground")}>{m.tool_video_cover_label()}</span>
      <div class={cn("flex min-w-0 items-baseline gap-2")}>
        {#if coverPreview.text == null}
          <span class={cn("text-muted-foreground")}>—</span>
        {:else if coverPreview.text === ""}
          <span class={cn("text-amber-600 dark:text-amber-400")}>
            {m.tool_video_tag_cleared()}
          </span>
        {:else}
          <span
            class={cn("min-w-0 flex-1 truncate", coverPreview.unknown && "text-destructive")}
            title={coverPreview.text}
          >
            {coverPreview.text}
          </span>
        {/if}
      </div>
    </div>
    {#if item.output}
      <span class={cn("truncate text-emerald-600 dark:text-emerald-400")} title={item.output}>
        → {item.output}
      </span>
    {/if}
    {#if item.coverNote}
      <span class={cn("text-muted-foreground")}>{item.coverNote}</span>
    {/if}
    {#if item.errorDetail}
      <span class={cn(item.status === "skipped" ? "text-muted-foreground" : "text-destructive")}>
        {item.errorDetail}
      </span>
    {/if}
  </div>
{/if}
