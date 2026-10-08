<script lang="ts">
  // 元数据参数（右栏）：编辑/FFmpeg 双页签 + 底部执行条。
  // 纯展示组件：状态由 workspace 持有，模板输入保持字符串，提交时 workspace 逐项展开。
  import FolderOpenIcon from "@lucide/svelte/icons/folder-open";
  import PlayIcon from "@lucide/svelte/icons/play";
  import RotateCcwIcon from "@lucide/svelte/icons/rotate-ccw";
  import XIcon from "@lucide/svelte/icons/x";
  import { Button } from "$components/shadcn-svelte/button";
  import { Input } from "$components/shadcn-svelte/input";
  import { Label } from "$components/shadcn-svelte/label";
  import { Progress } from "$components/shadcn-svelte/progress";
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import { Separator } from "$components/shadcn-svelte/separator";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "$components/shadcn-svelte/select";
  import { Tabs, TabsList, TabsTrigger } from "$components/shadcn-svelte/tabs";
  import type { FfmpegStatus } from "$libs/commands/bindings";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import FfmpegManageTab from "./ffmpeg-manage-tab.svelte";
  import { TAG_KEYS, type MetadataParamsState, type TagKey } from "./metadata-types";

  interface Props {
    /** 参数状态：workspace 持有 */
    params: MetadataParamsState;
    /** 参数变更回调（整体替换） */
    onParamsChange: (params: MetadataParamsState) => void;
    /** 选择输出目录回调（对话框） */
    onChooseOutputDir: () => void;
    /** 开始按钮可用态：模板合法 + 有可处理项 + 输出目录已选 + 引擎可用 + 空闲 */
    canStart: boolean;
    /** 处理中标志：禁用表单与开始按钮 */
    processing: boolean;
    /** 进度：处理中展示进度条 */
    progress: { done: number; total: number } | null;
    /** 开始按钮文案：空闲“开始处理”，处理中“处理中 n/m” */
    startLabel: string;
    /** 开始处理回调 */
    onStart: () => void;
    /** 未知占位名：非空即拦截开始（右栏错误行 + 中栏红色高亮） */
    templateErrors: string[];
    /** 引擎缺失：编辑页顶部引导横幅，切页签去下载 */
    ffmpegMissing: boolean;
    /** FFmpeg 页状态：查询中为 `null` */
    ffmpegStatus: FfmpegStatus | null;
    /** FFmpeg 任一进行中：锁按钮 */
    ffmpegBusy: boolean;
    /** FFmpeg 下载或重装进行中：展示进度条 */
    ffmpegDownloading: boolean;
    /** FFmpeg 进度行文本 */
    ffmpegProgressText: string | null;
    /** FFmpeg 进度条百分比 */
    ffmpegProgressValue: number;
    /** FFmpeg 失败原因 */
    ffmpegError: string | null;
    /** 重新查询引擎状态回调 */
    onRefreshFfmpeg: () => void;
    /** 确认后执行下载回调 */
    onEnsureFfmpeg: () => void;
    /** 确认后执行修复重装回调 */
    onRepairFfmpeg: () => void;
  }

  let {
    params,
    onParamsChange,
    onChooseOutputDir,
    canStart,
    processing,
    progress,
    startLabel,
    onStart,
    templateErrors,
    ffmpegMissing,
    ffmpegStatus,
    ffmpegBusy,
    ffmpegDownloading,
    ffmpegProgressText,
    ffmpegProgressValue,
    ffmpegError,
    onRefreshFfmpeg,
    onEnsureFfmpeg,
    onRepairFfmpeg,
  }: Props = $props();

  /** 右栏页签：编辑与 FFmpeg 管理（内部状态，缺失引导可切过去） */
  let tab = $state("edit");

  /** 字段文案：与后端 `VideoTags` 键一一对应 */
  const tagLabels: Record<TagKey, string> = {
    title: m.tool_video_tag_title(),
    artist: m.tool_video_tag_artist(),
    album: m.tool_video_tag_album(),
    genre: m.tool_video_tag_genre(),
    date: m.tool_video_tag_date(),
    comment: m.tool_video_tag_comment(),
  };

  /** 模板输入：原样回写（空即保持原值，展开在 workspace 统一做） */
  function handleTemplateInput(key: TagKey) {
    return (event: Event): void => {
      const target = event.target;
      if (!(target instanceof HTMLInputElement)) return;
      if (target.value === params.templates[key]) return;
      onParamsChange({ ...params, templates: { ...params.templates, [key]: target.value } });
    };
  }

  /** 清空标记切换：置位即该字段写空串（模板被忽略），再点撤销 */
  function toggleCleared(key: TagKey): void {
    onParamsChange({ ...params, cleared: { ...params.cleared, [key]: !params.cleared[key] } });
  }

  /** 封面模板输入：原样回写（空即保持原封面，展开在 workspace 统一做） */
  function handleCoverInput(event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.value === params.coverFile) return;
    onParamsChange({ ...params, coverFile: target.value });
  }

  /** 封面清除切换：置位即清除封面（模板被忽略），再点撤销 */
  function toggleCoverClear(): void {
    onParamsChange({ ...params, coverClear: !params.coverClear });
  }

  /** 输出目录输入：原样回写（空即未选，开始按钮禁用） */
  function handleOutputInput(event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.value === params.outputDir) return;
    onParamsChange({ ...params, outputDir: target.value });
  }

  /** 重名策略候选：与图片侧同语义 */
  const overwriteItems = [
    { value: "increment", label: m.tool_video_overwrite_increment() },
    { value: "overwrite", label: m.tool_video_overwrite_overwrite() },
    { value: "skip", label: m.tool_video_overwrite_skip() },
  ];

  /** 重名策略切换：经候选收窄后提交 */
  function handleOverwriteChange(next: string): void {
    if (next !== "increment" && next !== "overwrite" && next !== "skip") return;
    if (next === params.overwrite) return;
    onParamsChange({ ...params, overwrite: next });
  }

  // 进度百分比：total 为 0 时不渲染进度条（调用方保证）
  const progressValue = $derived(
    progress && progress.total > 0 ? (progress.done / progress.total) * 100 : 0,
  );
</script>

<section
  aria-label={m.tool_video_metadata_name()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <Tabs value={tab} onValueChange={(next) => (tab = next)}>
    <TabsList class="m-3 mb-0 w-[calc(100%-1.5rem)]">
      <TabsTrigger value="edit" class="flex-1">{m.tool_video_tab_edit()}</TabsTrigger>
      <TabsTrigger value="ffmpeg" class="flex-1">{m.tool_video_tab_ffmpeg()}</TabsTrigger>
    </TabsList>
  </Tabs>

  {#if tab === "ffmpeg"}
    <ScrollArea class="min-h-0 flex-1">
      <div class={cn("flex flex-col gap-3 p-3")}>
        <FfmpegManageTab
          status={ffmpegStatus}
          busy={ffmpegBusy || processing}
          downloading={ffmpegDownloading}
          progressText={ffmpegProgressText}
          progressValue={ffmpegProgressValue}
          error={ffmpegError}
          onRefresh={onRefreshFfmpeg}
          onEnsure={onEnsureFfmpeg}
          onRepair={onRepairFfmpeg}
        />
      </div>
    </ScrollArea>
  {:else}
    <ScrollArea class="min-h-0 flex-1">
      <div class={cn("flex flex-col gap-3 p-3")}>
        {#if ffmpegMissing}
          <button
            type="button"
            onclick={() => (tab = "ffmpeg")}
            class={cn(
              "rounded-md border border-dashed border-primary/50 px-3 py-2 text-left",
              "text-xs text-muted-foreground hover:text-foreground",
            )}
          >
            {m.tool_ffmpeg_missing()} →
          </button>
        {/if}
        <span class={cn("text-xs font-medium text-muted-foreground")}>
          {m.tool_video_group_tags()}
        </span>
        <p class={cn("text-xs text-muted-foreground")}>{m.tool_video_template_hint()}</p>
        {#each TAG_KEYS as key (key)}
          <div class={cn("flex flex-col gap-1")}>
            <div class={cn("flex items-center justify-between gap-2")}>
              <Label for={`video-tag-${key}`}>{tagLabels[key]}</Label>
              <Button
                variant="ghost"
                size="sm"
                disabled={processing}
                aria-pressed={params.cleared[key]}
                title={m.tool_video_clear_tag()}
                aria-label={`${m.tool_video_clear_tag()} ${tagLabels[key]}`}
                onclick={() => toggleCleared(key)}
              >
                {#if params.cleared[key]}
                  <RotateCcwIcon />
                  {m.tool_video_tag_cleared()}
                {:else}
                  <XIcon />
                {/if}
              </Button>
            </div>
            <Input
              id={`video-tag-${key}`}
              value={params.templates[key]}
              disabled={processing || params.cleared[key]}
              placeholder={params.cleared[key] ? m.tool_video_tag_cleared() : ""}
              oninput={handleTemplateInput(key)}
              class="tabular-nums"
            />
          </div>
        {/each}
        {#if templateErrors.length > 0}
          <p class={cn("text-xs text-destructive")}>
            {m.tool_video_unknown_placeholder({
              names: templateErrors.map((n) => `%${n}%`).join(" "),
            })}
          </p>
        {/if}

        <div class={cn("flex flex-col gap-1")}>
          <div class={cn("flex items-center justify-between gap-2")}>
            <Label for="video-cover-file">{m.tool_video_cover_label()}</Label>
            <Button
              variant="ghost"
              size="sm"
              disabled={processing}
              aria-pressed={params.coverClear}
              title={m.tool_video_clear_cover()}
              aria-label={m.tool_video_clear_cover()}
              onclick={toggleCoverClear}
            >
              {#if params.coverClear}
                <RotateCcwIcon />
                {m.tool_video_tag_cleared()}
              {:else}
                <XIcon />
              {/if}
            </Button>
          </div>
          <Input
            id="video-cover-file"
            value={params.coverFile}
            disabled={processing || params.coverClear}
            placeholder={params.coverClear ? m.tool_video_tag_cleared() : ""}
            oninput={handleCoverInput}
            class="tabular-nums"
          />
          <p class={cn("text-xs text-muted-foreground")}>{m.tool_video_cover_hint()}</p>
        </div>

        <Separator />
        <span class={cn("text-xs font-medium text-muted-foreground")}>
          {m.tool_video_group_output()}
        </span>

        <div class={cn("flex flex-col gap-1")}>
          <Label for="video-output-dir">{m.tool_video_output_label()}</Label>
          <div class={cn("flex items-center gap-2")}>
            <Input
              id="video-output-dir"
              value={params.outputDir}
              placeholder={m.tool_video_output_required()}
              disabled={processing}
              oninput={handleOutputInput}
              class="min-w-0 flex-1 truncate"
            />
            <Button variant="outline" size="sm" disabled={processing} onclick={onChooseOutputDir}>
              <FolderOpenIcon />
              {m.tool_video_output_choose()}
            </Button>
          </div>
        </div>

        <div class={cn("flex flex-col gap-1")}>
          <Label id="video-overwrite-label">{m.tool_video_overwrite_label()}</Label>
          <Select
            type="single"
            value={params.overwrite}
            items={overwriteItems}
            onValueChange={handleOverwriteChange}
            disabled={processing}
          >
            <SelectTrigger aria-labelledby="video-overwrite-label" class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each overwriteItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
      </div>
    </ScrollArea>
  {/if}
  <div class={cn("flex shrink-0 flex-col gap-2 border-t p-3")}>
    {#if progress}
      <Progress value={progressValue} />
    {/if}
    <Button disabled={!canStart} onclick={onStart} class="w-full">
      <PlayIcon />
      {startLabel}
    </Button>
  </div>
</section>
