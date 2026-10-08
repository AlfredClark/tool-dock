<script lang="ts">
  // 调整参数（右栏）：预设芯片 + 三组（尺寸/输出/文件）+ 执行。
  // 纯展示组件：状态由 workspace 持有，数字输入保持字符串，提交时 workspace 解析。
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
  import { Slider } from "$components/shadcn-svelte/slider";
  import { Switch } from "$components/shadcn-svelte/switch";
  import { Tabs, TabsList, TabsTrigger } from "$components/shadcn-svelte/tabs";
  import type {
    FitMode,
    ImageRotation,
    OutputFormat,
    OverwritePolicy,
    ResizeFilter,
  } from "$libs/commands/bindings";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import {
    RATIO_FREE,
    RATIO_ITEMS,
    applyExactRatio,
    parseRatio,
    ratioPlaceholder,
  } from "./resize-math";
  import type { ResizeModeKind, ResizeParamsState } from "./resize-types";

  interface Props {
    /** 参数状态：workspace 持有 */
    params: ResizeParamsState;
    /** 参数变更回调（整体替换） */
    onParamsChange: (params: ResizeParamsState) => void;
    /** 选择输出目录回调（对话框） */
    onChooseOutputDir: () => void;
    /** 开始按钮可用态：模式合法 + 有可处理项 + 输出目录已选 + 非处理中 */
    canStart: boolean;
    /** 处理中标志：禁用表单与开始按钮 */
    processing: boolean;
    /** 进度：处理中展示进度条 */
    progress: { done: number; total: number } | null;
    /** 开始按钮文案：空闲“开始处理”，处理中“处理中 n/m” */
    startLabel: string;
    /** 开始处理回调 */
    onStart: () => void;
    /** 列表含 GIF：提示首帧转 PNG */
    hasGif: boolean;
    /** 参考尺寸：选中图原尺寸，锁定比例时联动另一边 placeholder（`null` 即无参考） */
    referenceSize: { width: number; height: number } | null;
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
    hasGif,
    referenceSize,
  }: Props = $props();

  const MODE_KINDS: ResizeModeKind[] = ["box", "percent", "exact"];

  /** 页签切换：bits-ui 给 string，经候选收窄后提交 */
  function handleModeChange(next: string): void {
    if (!isModeKind(next) || next === params.modeKind) return;
    onParamsChange({ ...params, modeKind: next });
  }

  function isModeKind(value: string): value is ResizeModeKind {
    return (MODE_KINDS as readonly string[]).includes(value);
  }

  /** 填充策略候选：展示文案调用时求值 */
  const fitItems: { value: FitMode; label: string }[] = [
    { value: "stretch", label: m.tool_resize_fit_stretch() },
    { value: "contain", label: m.tool_resize_fit_contain() },
    { value: "cover", label: m.tool_resize_fit_cover() },
    { value: "pad", label: m.tool_resize_fit_pad() },
  ];

  /** 填充策略切换：经候选收窄后提交 */
  function handleFitChange(next: string): void {
    const fit = fitItems.find((item) => item.value === next)?.value;
    if (!fit || fit === params.fit) return;
    onParamsChange({ ...params, fit });
  }

  const formatItems: { value: OutputFormat; label: string }[] = [
    { value: "original", label: m.tool_resize_format_original() },
    { value: "jpeg", label: "JPEG" },
    { value: "png", label: "PNG" },
    { value: "webp", label: "WebP" },
  ];

  /** 输出格式切换：经候选收窄后提交 */
  function handleFormatChange(next: string): void {
    const format = formatItems.find((item) => item.value === next)?.value;
    if (!format || format === params.format) return;
    onParamsChange({ ...params, format });
  }

  /** 旋转候选：无 180° 直观图标，统一用下拉（与其它枚举一致） */
  const rotationItems: { value: ImageRotation; label: string }[] = [
    { value: "none", label: m.tool_resize_rotation_none() },
    { value: "cw90", label: m.tool_resize_rotation_cw90() },
    { value: "ccw90", label: m.tool_resize_rotation_ccw90() },
    { value: "cw180", label: m.tool_resize_rotation_cw180() },
    { value: "fliphorizontal", label: m.tool_resize_rotation_fliph() },
  ];

  /** 旋转切换：经候选收窄后提交 */
  function handleRotationChange(next: string): void {
    const rotation = rotationItems.find((item) => item.value === next)?.value;
    if (!rotation || rotation === params.rotation) return;
    onParamsChange({ ...params, rotation });
  }

  /** 插值算法候选：默认高质量，批量大图卡顿时可切最快 */
  const filterItems: { value: ResizeFilter; label: string }[] = [
    { value: "nearest", label: m.tool_resize_filter_nearest() },
    { value: "triangle", label: m.tool_resize_filter_triangle() },
    { value: "catmullrom", label: m.tool_resize_filter_catmullrom() },
    { value: "gaussian", label: m.tool_resize_filter_gaussian() },
    { value: "lanczos3", label: m.tool_resize_filter_lanczos3() },
  ];

  /** 插值切换：经候选收窄后提交 */
  function handleFilterChange(next: string): void {
    const filter = filterItems.find((item) => item.value === next)?.value;
    if (!filter || filter === params.filter) return;
    onParamsChange({ ...params, filter });
  }

  /** 重名策略候选：默认自动重命名（历史行为，老用户无感） */
  const overwriteItems: { value: OverwritePolicy; label: string }[] = [
    { value: "increment", label: m.tool_resize_overwrite_increment() },
    { value: "overwrite", label: m.tool_resize_overwrite_overwrite() },
    { value: "skip", label: m.tool_resize_overwrite_skip() },
  ];

  /** 重名策略切换：经候选收窄后提交 */
  function handleOverwriteChange(next: string): void {
    const overwrite = overwriteItems.find((item) => item.value === next)?.value;
    if (!overwrite || overwrite === params.overwrite) return;
    onParamsChange({ ...params, overwrite });
  }

  /** 长宽比候选：自由 + 常见比例（值即 `宽:高`，提交时解析） */
  const ratioItems: { value: string; label: string }[] = [
    { value: RATIO_FREE, label: m.tool_resize_ratio_free() },
    ...RATIO_ITEMS.map((ratio) => ({ value: ratio, label: ratio })),
  ];

  /** 长宽比切换：非自由时按已填边即时换算另一边（宽优先），自由时保持原值 */
  function handleRatioChange(next: string): void {
    if (next !== RATIO_FREE && parseRatio(next) == null) return;
    if (next === params.exactRatio) return;
    let nextParams = { ...params, exactRatio: next };
    if (next !== RATIO_FREE) {
      const editedKey = nextParams.exactWidth.trim() !== "" ? "exactWidth" : "exactHeight";
      nextParams = applyExactRatio(nextParams, editedKey);
    }
    onParamsChange(nextParams);
  }

  /** 文本数字输入：原样回写（空即未填，解析在 workspace 统一做） */
  function handleTextInput(key: "boxWidth" | "boxHeight" | "targetSizeKb") {
    return (event: Event): void => {
      const target = event.target;
      if (!(target instanceof HTMLInputElement)) return;
      if (target.value === params[key]) return;
      onParamsChange({ ...params, [key]: target.value });
    };
  }

  /** 精确尺寸输入：写回后按长宽比联动另一边（自由比例时即原样回写） */
  function handleExactInput(key: "exactWidth" | "exactHeight") {
    return (event: Event): void => {
      const target = event.target;
      if (!(target instanceof HTMLInputElement)) return;
      if (target.value === params[key]) return;
      onParamsChange(applyExactRatio({ ...params, [key]: target.value }, key));
    };
  }

  /** 百分比滑块：25-400 步长 25 快调，超范围经右侧数字框输入（后端上限 1000） */
  function handlePercentSlider(next: number): void {
    if (next === params.percent) return;
    onParamsChange({ ...params, percent: next });
  }

  /** 百分比数字框：非法输入忽略（保留旧值），不写回 NaN */
  function handlePercentInput(event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    const parsed = Number.parseInt(target.value, 10);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 1000) return;
    if (parsed === params.percent) return;
    onParamsChange({ ...params, percent: parsed });
  }

  /** 质量滑块：1-100，仅 JPEG 输出且未设目标大小时生效 */
  function handleQualityChange(next: number): void {
    if (next === params.quality) return;
    onParamsChange({ ...params, quality: next });
  }

  // 质量/目标行显隐：显式 PNG/WebP 隐藏（无损编码无质量概念），其余显示并附注；
  // 目标大小置位时隐藏质量滑块（两者互斥，二分接管质量）
  const showQualityTarget = $derived(params.format === "original" || params.format === "jpeg");
  const hasTarget = $derived(params.targetSizeKb.trim() !== "");

  /** 锁定联动：参考尺寸按已填边换算另一边，仅作 placeholder 提示（不写回） */
  function linkedSide(filled: string, empty: string, alongWidth: boolean): string | null {
    if (params.modeKind !== "box" || !params.lockRatio || !referenceSize) return null;
    if (filled.trim() === "" || empty.trim() !== "") return null;
    if (!/^\d+$/.test(filled.trim())) return null;
    const base = Number.parseInt(filled.trim(), 10);
    if (base <= 0) return null;
    const scaled = alongWidth
      ? (referenceSize.height * base) / referenceSize.width
      : (referenceSize.width * base) / referenceSize.height;
    return String(Math.max(1, Math.round(scaled)));
  }

  const linkedHeight = $derived(linkedSide(params.boxWidth, params.boxHeight, true));
  const linkedWidth = $derived(linkedSide(params.boxHeight, params.boxWidth, false));

  /** 精确尺寸示例提示：跟随长宽比同步（自由时回落默认），仅输入为空时可见 */
  const ratioPh = $derived(ratioPlaceholder(params.exactRatio));

  // 进度百分比：total 为 0 时不渲染进度条（调用方保证）
  const progressValue = $derived(
    progress && progress.total > 0 ? (progress.done / progress.total) * 100 : 0,
  );
</script>

<section
  aria-label={m.tool_resize_mode_label()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <ScrollArea class="min-h-0 flex-1">
    <div class={cn("flex flex-col gap-3 p-3")}>
      <span class={cn("text-xs font-medium text-muted-foreground")}>
        {m.tool_resize_group_size()}
      </span>

      <Tabs value={params.modeKind} onValueChange={handleModeChange}>
        <TabsList class="w-full">
          <TabsTrigger value="box" class="flex-1">{m.tool_resize_mode_box()}</TabsTrigger>
          <TabsTrigger value="percent" class="flex-1">{m.tool_resize_mode_percent()}</TabsTrigger>
          <TabsTrigger value="exact" class="flex-1">{m.tool_resize_mode_exact()}</TabsTrigger>
        </TabsList>
      </Tabs>

      {#if params.modeKind === "box"}
        <div class={cn("grid grid-cols-2 gap-2")}>
          <div class={cn("flex flex-col gap-1")}>
            <Label for="resize-box-width">{m.tool_resize_width_label()}</Label>
            <Input
              id="resize-box-width"
              inputmode="numeric"
              placeholder={linkedWidth ?? "800"}
              value={params.boxWidth}
              disabled={processing}
              oninput={handleTextInput("boxWidth")}
            />
          </div>
          <div class={cn("flex flex-col gap-1")}>
            <Label for="resize-box-height">{m.tool_resize_height_label()}</Label>
            <Input
              id="resize-box-height"
              inputmode="numeric"
              placeholder={linkedHeight ?? "600"}
              value={params.boxHeight}
              disabled={processing}
              oninput={handleTextInput("boxHeight")}
            />
          </div>
        </div>
        <div class={cn("flex items-center justify-between gap-2")}>
          <Label for="resize-lock-ratio">{m.tool_resize_lock_ratio()}</Label>
          <Switch
            id="resize-lock-ratio"
            checked={params.lockRatio}
            disabled={processing}
            onCheckedChange={(next) => onParamsChange({ ...params, lockRatio: next })}
          />
        </div>
      {:else if params.modeKind === "percent"}
        <div class={cn("flex flex-col gap-1")}>
          <Label for="resize-percent">{m.tool_resize_percent_label()}</Label>
          <div class={cn("flex items-center gap-2")}>
            <Slider
              type="single"
              value={params.percent}
              min={25}
              max={400}
              step={25}
              disabled={processing}
              onValueChange={handlePercentSlider}
              class="flex-1"
            />
            <Input
              id="resize-percent"
              inputmode="numeric"
              value={String(params.percent)}
              disabled={processing}
              oninput={handlePercentInput}
              class="w-20 shrink-0 text-right tabular-nums"
            />
            <span class={cn("text-sm text-muted-foreground")}>%</span>
          </div>
        </div>
      {:else}
        <div class={cn("flex flex-col gap-1")}>
          <Label id="resize-ratio-label">{m.tool_resize_ratio_label()}</Label>
          <Select
            type="single"
            value={params.exactRatio}
            items={ratioItems}
            onValueChange={handleRatioChange}
            disabled={processing}
          >
            <SelectTrigger aria-labelledby="resize-ratio-label" class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each ratioItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
        <div class={cn("grid grid-cols-2 gap-2")}>
          <div class={cn("flex flex-col gap-1")}>
            <Label for="resize-exact-width">{m.tool_resize_width_label()}</Label>
            <Input
              id="resize-exact-width"
              inputmode="numeric"
              placeholder={ratioPh?.width ?? "800"}
              value={params.exactWidth}
              disabled={processing}
              oninput={handleExactInput("exactWidth")}
            />
          </div>
          <div class={cn("flex flex-col gap-1")}>
            <Label for="resize-exact-height">{m.tool_resize_height_label()}</Label>
            <Input
              id="resize-exact-height"
              inputmode="numeric"
              placeholder={ratioPh?.height ?? "600"}
              value={params.exactHeight}
              disabled={processing}
              oninput={handleExactInput("exactHeight")}
            />
          </div>
        </div>
        <div class={cn("flex flex-col gap-1")}>
          <Label id="resize-fit-label">{m.tool_resize_fit_label()}</Label>
          <Select
            type="single"
            value={params.fit}
            items={fitItems}
            onValueChange={handleFitChange}
            disabled={processing}
          >
            <SelectTrigger aria-labelledby="resize-fit-label" class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {#each fitItems as item (item.value)}
                <SelectItem value={item.value}>{item.label}</SelectItem>
              {/each}
            </SelectContent>
          </Select>
        </div>
      {/if}

      <div class={cn("flex items-center justify-between gap-2")}>
        <Label for="resize-no-upscale">{m.tool_resize_no_upscale()}</Label>
        <Switch
          id="resize-no-upscale"
          checked={params.noUpscale}
          disabled={processing}
          onCheckedChange={(next) => onParamsChange({ ...params, noUpscale: next })}
        />
      </div>

      <div class={cn("flex flex-col gap-1")}>
        <Label id="resize-rotation-label">{m.tool_resize_rotation_label()}</Label>
        <Select
          type="single"
          value={params.rotation}
          items={rotationItems}
          onValueChange={handleRotationChange}
          disabled={processing}
        >
          <SelectTrigger aria-labelledby="resize-rotation-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each rotationItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>

      <Separator />
      <span class={cn("text-xs font-medium text-muted-foreground")}>
        {m.tool_resize_group_output()}
      </span>

      <div class={cn("flex flex-col gap-1")}>
        <Label id="resize-format-label">{m.tool_resize_format_label()}</Label>
        <Select
          type="single"
          value={params.format}
          items={formatItems}
          onValueChange={handleFormatChange}
          disabled={processing}
        >
          <SelectTrigger aria-labelledby="resize-format-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each formatItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>

      {#if showQualityTarget}
        {#if hasTarget}
          <div class={cn("flex flex-col gap-1")}>
            <Label for="resize-target">{m.tool_resize_target_label()}</Label>
            <div class={cn("flex items-center gap-2")}>
              <Input
                id="resize-target"
                inputmode="numeric"
                placeholder="200"
                value={params.targetSizeKb}
                disabled={processing}
                oninput={handleTextInput("targetSizeKb")}
                class="flex-1 text-right tabular-nums"
              />
              <span class={cn("text-sm text-muted-foreground")}>
                {m.tool_resize_target_unit()}
              </span>
            </div>
            <p class={cn("text-xs text-muted-foreground")}>{m.tool_resize_target_jpeg_only()}</p>
            <button
              type="button"
              disabled={processing}
              onclick={() => onParamsChange({ ...params, targetSizeKb: "" })}
              class={cn(
                "self-start text-xs text-muted-foreground underline-offset-4",
                "hover:text-foreground hover:underline disabled:opacity-50",
              )}
            >
              {m.tool_resize_quality_label()} →
            </button>
          </div>
        {:else}
          <div class={cn("flex flex-col gap-1")}>
            <div class={cn("flex items-center justify-between")}>
              <Label for="resize-quality">{m.tool_resize_quality_label()}</Label>
              <span class={cn("text-xs text-muted-foreground")}>
                {`${params.quality} · ${m.tool_resize_quality_jpeg_only()}`}
              </span>
            </div>
            <Slider
              type="single"
              value={params.quality}
              min={1}
              max={100}
              step={1}
              disabled={processing}
              onValueChange={handleQualityChange}
            />
            <button
              type="button"
              disabled={processing}
              onclick={() => onParamsChange({ ...params, targetSizeKb: "200" })}
              class={cn(
                "self-start text-xs text-muted-foreground underline-offset-4",
                "hover:text-foreground hover:underline disabled:opacity-50",
              )}
            >
              {m.tool_resize_target_label()} →
            </button>
          </div>
        {/if}
      {/if}

      <div class={cn("flex flex-col gap-1")}>
        <Label id="resize-filter-label">{m.tool_resize_filter_label()}</Label>
        <Select
          type="single"
          value={params.filter}
          items={filterItems}
          onValueChange={handleFilterChange}
          disabled={processing}
        >
          <SelectTrigger aria-labelledby="resize-filter-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each filterItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>

      <Separator />
      <span class={cn("text-xs font-medium text-muted-foreground")}>
        {m.tool_resize_group_file()}
      </span>

      <div class={cn("flex flex-col gap-1")}>
        <Label for="resize-output-dir">{m.tool_resize_output_label()}</Label>
        <div class={cn("flex items-center gap-2")}>
          <Input
            id="resize-output-dir"
            readonly
            value={params.outputDir}
            placeholder={m.tool_resize_output_required()}
            class="min-w-0 flex-1 truncate"
          />
          <Button variant="outline" size="sm" disabled={processing} onclick={onChooseOutputDir}>
            <FolderOpenIcon />
            {m.tool_resize_output_choose()}
          </Button>
        </div>
      </div>

      <div class={cn("flex flex-col gap-1")}>
        <Label id="resize-overwrite-label">{m.tool_resize_overwrite_label()}</Label>
        <Select
          type="single"
          value={params.overwrite}
          items={overwriteItems}
          onValueChange={handleOverwriteChange}
          disabled={processing}
        >
          <SelectTrigger aria-labelledby="resize-overwrite-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {#each overwriteItems as item (item.value)}
              <SelectItem value={item.value}>{item.label}</SelectItem>
            {/each}
          </SelectContent>
        </Select>
      </div>

      <div class={cn("flex items-center justify-between gap-2")}>
        <Label for="resize-suffix">{m.tool_resize_suffix_label()}</Label>
        <Switch
          id="resize-suffix"
          checked={params.filenameSuffix}
          disabled={processing}
          onCheckedChange={(next) => onParamsChange({ ...params, filenameSuffix: next })}
        />
      </div>

      {#if hasGif}
        <p class={cn("text-xs text-muted-foreground")}>{m.tool_resize_gif_note()}</p>
      {/if}
    </div>
  </ScrollArea>
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
