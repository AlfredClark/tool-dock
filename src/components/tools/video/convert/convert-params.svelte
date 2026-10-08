<script lang="ts">
  // 转换参数（右栏）：格式/FFmpeg 双页签 + 底部执行条。
  // 纯展示组件：状态由 workspace 持有，目标格式与转码模式经下拉整体回写。
  import FolderOpenIcon from "@lucide/svelte/icons/folder-open";
  import PlayIcon from "@lucide/svelte/icons/play";
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
  import type {
    Accelerator,
    AudioBitrate,
    ConvertMode,
    FfmpegStatus,
    OutputResolution,
    VideoEncoder,
    VideoPreset,
    VideoQuality,
    VideoTarget,
  } from "$libs/commands/bindings";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import FfmpegManageTab from "$components/tools/video/metadata/ffmpeg-manage-tab.svelte";
  import {
    ACCELERATORS_BY_TARGET,
    AUDIO_BITRATES,
    CONVERT_MODES,
    ENCODERS_BY_TARGET,
    OUTPUT_RESOLUTIONS,
    VIDEO_PRESETS,
    VIDEO_QUALITIES,
    VIDEO_TARGETS,
    resetForEncoder,
    resetForTarget,
    supportsPreset,
    type ConvertParamsState,
  } from "./convert-types";

  interface Props {
    /** 参数状态：workspace 持有 */
    params: ConvertParamsState;
    /** 参数变更回调（整体替换） */
    onParamsChange: (params: ConvertParamsState) => void;
    /** 选择输出目录回调（对话框） */
    onChooseOutputDir: () => void;
    /** 开始按钮可用态：有可处理项 + 输出目录已选 + 引擎可用 + 空闲 */
    canStart: boolean;
    /** 处理中标志：禁用表单与开始按钮 */
    processing: boolean;
    /** 进度：处理中展示进度条 */
    progress: { done: number; total: number } | null;
    /** 开始按钮文案：空闲“开始处理”，处理中“处理中 n/m” */
    startLabel: string;
    /** 开始处理回调 */
    onStart: () => void;
    /** 引擎缺失：格式页顶部引导横幅，切页签去下载 */
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

  /** 右栏页签：格式与 FFmpeg 管理（内部状态，缺失引导可切过去） */
  let tab = $state("format");

  /** 转码模式文案：与后端 `ConvertMode` 变体一一对应 */
  function modeLabel(mode: ConvertMode): string {
    switch (mode) {
      case "auto":
        return m.tool_video_convert_mode_auto();
      case "copyonly":
        return m.tool_video_convert_mode_copy();
      case "reencode":
        return m.tool_video_convert_mode_reencode();
    }
  }

  /** 目标格式候选：下拉渲染顺序即 `VIDEO_TARGETS` */
  const targetItems = VIDEO_TARGETS.map((value) => ({ value, label: `.${value}` }));

  /** 转码模式候选 */
  const modeItems = CONVERT_MODES.map((value) => ({ value, label: modeLabel(value) }));

  /** 目标格式切换：经候选收窄后提交，编码器归新目标默认、速度归默认 */
  function handleTargetChange(next: string): void {
    if (!VIDEO_TARGETS.includes(next as VideoTarget)) return;
    if (next === params.target) return;
    onParamsChange(resetForTarget(params, next as VideoTarget));
  }

  /** 转码模式切换：经候选收窄后提交 */
  function handleModeChange(next: string): void {
    if (!CONVERT_MODES.includes(next as ConvertMode)) return;
    if (next === params.mode) return;
    onParamsChange({ ...params, mode: next as ConvertMode });
  }

  /** 画质档文案：三档通用语义 */
  function qualityLabel(quality: VideoQuality): string {
    switch (quality) {
      case "high":
        return m.tool_video_convert_quality_high();
      case "standard":
        return m.tool_video_convert_quality_standard();
      case "compact":
        return m.tool_video_convert_quality_compact();
    }
  }

  /** 编码速度文案：与后端 `VideoPreset` 变体一一对应 */
  function presetLabel(preset: VideoPreset): string {
    switch (preset) {
      case "ultrafast":
        return m.tool_video_convert_preset_ultrafast();
      case "veryfast":
        return m.tool_video_convert_preset_veryfast();
      case "medium":
        return m.tool_video_convert_preset_medium();
      case "slow":
        return m.tool_video_convert_preset_slow();
    }
  }

  /** 音频码率文案：`default` 走文案，其余显示原始值 */
  function audioLabel(bitrate: AudioBitrate): string {
    if (bitrate === "default") return m.tool_video_convert_audio_default();
    return bitrate.replace(/^kb/, "") + "k";
  }

  /** 分辨率文案：`source` 走文案，其余显示 `1080p` 式 */
  function resolutionLabel(resolution: OutputResolution): string {
    if (resolution === "source") return m.tool_video_convert_resolution_source();
    return resolution.replace(/^p/, "") + "p";
  }

  /** 当前目标的编码器候选：随目标变化，单编码器容器（如 avi）隐藏该行 */
  const encoderItems = $derived(
    ENCODERS_BY_TARGET[params.target].map((value) => ({ value, label: value })),
  );
  const showEncoderRow = $derived(encoderItems.length > 1);

  /** 各选项候选：文案在调用时求值（与转码模式候选同惯例） */
  const qualityItems = VIDEO_QUALITIES.map((value) => ({ value, label: qualityLabel(value) }));
  const presetItems = VIDEO_PRESETS.map((value) => ({ value, label: presetLabel(value) }));
  const audioItems = AUDIO_BITRATES.map((value) => ({ value, label: audioLabel(value) }));
  const resolutionItems = OUTPUT_RESOLUTIONS.map((value) => ({
    value,
    label: resolutionLabel(value),
  }));

  /** 编码器切换：经候选收窄后提交，无硬编对应即加速回 CPU */
  function handleEncoderChange(next: string): void {
    const allowed = ENCODERS_BY_TARGET[params.target];
    if (!(allowed as readonly string[]).includes(next)) return;
    if (next === params.videoEncoder) return;
    onParamsChange(resetForEncoder(params, next as VideoEncoder));
  }

  /** 画质档切换：经候选收窄后提交 */
  function handleQualityChange(next: string): void {
    if (!VIDEO_QUALITIES.includes(next as VideoQuality)) return;
    if (next === params.quality) return;
    onParamsChange({ ...params, quality: next as VideoQuality });
  }

  /** 编码速度切换：经候选收窄后提交 */
  function handlePresetChange(next: string): void {
    if (!VIDEO_PRESETS.includes(next as VideoPreset)) return;
    if (next === params.preset) return;
    onParamsChange({ ...params, preset: next as VideoPreset });
  }

  /** 音频码率切换：经候选收窄后提交 */
  function handleAudioChange(next: string): void {
    if (!AUDIO_BITRATES.includes(next as AudioBitrate)) return;
    if (next === params.audioBitrate) return;
    onParamsChange({ ...params, audioBitrate: next as AudioBitrate });
  }

  /** 加速文案：CPU 走默认文案，硬加速项保留原文 */
  function accelLabel(accelerator: Accelerator): string {
    switch (accelerator) {
      case "cpu":
        return m.tool_video_convert_accel_cpu();
      case "nvenc":
        return m.tool_video_convert_accel_nvenc();
    }
  }

  /** 加速候选：最多两项（CPU + 目标允许的硬加速） */
  const accelItems = [
    { value: "cpu", label: accelLabel("cpu") },
    { value: "nvenc", label: accelLabel("nvenc") },
  ];

  /** 加速行显隐：目标支持且本机探测到加速器才展示（其余情况用户看不到选不了） */
  const showAccelRow = $derived(
    (ACCELERATORS_BY_TARGET[params.target] as readonly string[]).includes("nvenc") &&
      ffmpegStatus?.accelerators.includes("nvenc") === true,
  );

  /** 加速切换：经候选收窄后提交 */
  function handleAccelChange(next: string): void {
    if (next !== "cpu" && next !== "nvenc") return;
    if (next === params.accelerator) return;
    onParamsChange({ ...params, accelerator: next });
  }

  /** 分辨率切换：经候选收窄后提交 */
  function handleResolutionChange(next: string): void {
    if (!OUTPUT_RESOLUTIONS.includes(next as OutputResolution)) return;
    if (next === params.resolution) return;
    onParamsChange({ ...params, resolution: next as OutputResolution });
  }

  /** 输出目录输入：原样回写（空即未选，开始按钮禁用） */
  function handleOutputInput(event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.value === params.outputDir) return;
    onParamsChange({ ...params, outputDir: target.value });
  }

  /** 重名策略候选：与图片/元数据侧同语义 */
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
  aria-label={m.tool_video_convert_name()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <Tabs value={tab} onValueChange={(next) => (tab = next)}>
    <TabsList class="m-3 mb-0 w-[calc(100%-1.5rem)]">
      <TabsTrigger value="format" class="flex-1">{m.tool_video_convert_group_format()}</TabsTrigger>
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
          {m.tool_video_convert_group_format()}
        </span>

        <div class={cn("flex flex-col gap-1")}>
          <Label id="video-convert-target-label">{m.tool_video_convert_target_label()}</Label>
          <Select
            type="single"
            value={params.target}
            items={targetItems}
            onValueChange={handleTargetChange}
            disabled={processing}
          >
            <SelectTrigger aria-labelledby="video-convert-target-label" class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each targetItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>

        <div class={cn("flex flex-col gap-1")}>
          <Label id="video-convert-mode-label">{m.tool_video_convert_mode_label()}</Label>
          <Select
            type="single"
            value={params.mode}
            items={modeItems}
            onValueChange={handleModeChange}
            disabled={processing}
          >
            <SelectTrigger aria-labelledby="video-convert-mode-label" class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each modeItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>

        {#if params.mode === "reencode"}
          <Separator />
          <span class={cn("text-xs font-medium text-muted-foreground")}>
            {m.tool_video_convert_group_reencode()}
          </span>

          {#if showAccelRow}
            <div class={cn("flex flex-col gap-1")}>
              <Label id="video-convert-accel-label">{m.tool_video_convert_accel_label()}</Label>
              <Select
                type="single"
                value={params.accelerator}
                items={accelItems}
                onValueChange={handleAccelChange}
                disabled={processing}
              >
                <SelectTrigger aria-labelledby="video-convert-accel-label" class="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each accelItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </div>
          {/if}

          {#if showEncoderRow}
            <div class={cn("flex flex-col gap-1")}>
              <Label id="video-convert-encoder-label">{m.tool_video_convert_encoder_label()}</Label>
              <Select
                type="single"
                value={params.videoEncoder}
                items={encoderItems}
                onValueChange={handleEncoderChange}
                disabled={processing}
              >
                <SelectTrigger aria-labelledby="video-convert-encoder-label" class="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each encoderItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </div>
          {/if}

          <div class={cn("flex flex-col gap-1")}>
            <Label id="video-convert-quality-label">{m.tool_video_convert_quality_label()}</Label>
            <Select
              type="single"
              value={params.quality}
              items={qualityItems}
              onValueChange={handleQualityChange}
              disabled={processing}
            >
              <SelectTrigger aria-labelledby="video-convert-quality-label" class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each qualityItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </div>

          {#if supportsPreset(params.videoEncoder)}
            <div class={cn("flex flex-col gap-1")}>
              <Label id="video-convert-preset-label">{m.tool_video_convert_preset_label()}</Label>
              <Select
                type="single"
                value={params.preset}
                items={presetItems}
                onValueChange={handlePresetChange}
                disabled={processing}
              >
                <SelectTrigger aria-labelledby="video-convert-preset-label" class="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {#each presetItems as item (item.value)}
                    <SelectItem value={item.value}>{item.label}</SelectItem>
                  {/each}
                </SelectContent>
              </Select>
            </div>
          {/if}

          <div class={cn("flex flex-col gap-1")}>
            <Label id="video-convert-audio-label">{m.tool_video_convert_audio_label()}</Label>
            <Select
              type="single"
              value={params.audioBitrate}
              items={audioItems}
              onValueChange={handleAudioChange}
              disabled={processing}
            >
              <SelectTrigger aria-labelledby="video-convert-audio-label" class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each audioItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </div>

          <div class={cn("flex flex-col gap-1")}>
            <Label id="video-convert-resolution-label"
              >{m.tool_video_convert_resolution_label()}</Label
            >
            <Select
              type="single"
              value={params.resolution}
              items={resolutionItems}
              onValueChange={handleResolutionChange}
              disabled={processing}
            >
              <SelectTrigger aria-labelledby="video-convert-resolution-label" class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {#each resolutionItems as item (item.value)}
                  <SelectItem value={item.value}>{item.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </div>
        {/if}

        <Separator />
        <span class={cn("text-xs font-medium text-muted-foreground")}>
          {m.tool_video_group_output()}
        </span>

        <div class={cn("flex flex-col gap-1")}>
          <Label for="video-convert-output-dir">{m.tool_video_output_label()}</Label>
          <div class={cn("flex items-center gap-2")}>
            <Input
              id="video-convert-output-dir"
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
          <Label id="video-convert-overwrite-label">{m.tool_video_overwrite_label()}</Label>
          <Select
            type="single"
            value={params.overwrite}
            items={overwriteItems}
            onValueChange={handleOverwriteChange}
            disabled={processing}
          >
            <SelectTrigger aria-labelledby="video-convert-overwrite-label" class="w-full">
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
