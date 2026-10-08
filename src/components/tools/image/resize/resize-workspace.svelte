<script lang="ts">
  // 图片尺寸 workspace：左列（上列表 + 下参数）+ 右预览的三栏组装，持有全部页面级状态。
  // 文件接入三轨：桌面端对话框/拖放给路径走后端读元信息 + 缩略图，浏览器降级读 File。
  // 处理逐张调命令以驱动进度，单张失败标红跳过继续，最后统一出成功/失败汇总。
  import { onDestroy, onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import type {
    CommandError,
    ImageFormat,
    ResizeMode,
    ResizeOptions,
    ResizeSingleOutcome,
  } from "$libs/commands/bindings";
  import type { AnyResult } from "$libs/commands/types";
  import ImageList from "$components/tools/image/resize/image-list.svelte";
  import ImagePreview from "$components/tools/image/resize/image-preview.svelte";
  import ResizeParams from "$components/tools/image/resize/resize-params.svelte";
  import {
    ResizableHandle,
    ResizablePane,
    ResizablePaneGroup,
  } from "$components/shadcn-svelte/resizable";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";
  import { cn } from "$libs/utils/shadcn-svelte";
  import {
    IMAGE_EXTENSIONS,
    basenameOf,
    extensionOf,
    isImageExtension,
    isJpegOutput,
    planDimensions,
    rotateSize,
  } from "./resize-math";
  import { DEFAULT_RESIZE_PARAMS } from "./resize-types";
  import { resizeErrorText } from "./resize-errors";
  import type { ResizeImageItem, ResizeParamsState, ResizeSummary } from "./resize-types";

  /** 桌面端判定：有 Tauri 注入才走路径 + 命令链，否则走浏览器降级 */
  const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  /** 预览缩略图边长：列表 40px 与右侧大预览共用一张，768 兼顾清晰度与内存 */
  const THUMB_MAX_SIDE = 768;

  /** 列表项：加入顺序即处理顺序 */
  let items = $state<ResizeImageItem[]>([]);

  /** 选中项 id：`null` 即右侧空态 */
  let selectedId = $state<string | null>(null);

  /** 调整参数：面板读写，提交时解析为契约 `ResizeMode` */
  let params = $state<ResizeParamsState>({ ...DEFAULT_RESIZE_PARAMS });

  /** 批量处理中：锁表单与开始按钮，进度条展示 */
  let processing = $state(false);

  /** 进度：`null` 即空闲不展示 */
  let progress = $state<{ done: number; total: number } | null>(null);

  /** 批量汇总：处理完成后常驻列表底部（改列表即失效，随增删清空） */
  let summary = $state<ResizeSummary | null>(null);

  // 输出目录是否被动过（手动选择 / 文件夹自动设置）：动过后挂载默认值与后续自动设置都不再覆盖
  let outputDirTouched = false;

  // 拖放悬浮计数：dragenter/dragleave 成对增减，>0 显示 overlay（同数据互转）
  let dragDepth = $state(0);
  const dragActive = $derived(dragDepth > 0);

  // Tauri 拖放监听就绪后，DOM drop 只做 preventDefault，不再重复读取
  let tauriDropReady = false;

  /** 当前选中项：右侧预览消费 */
  const selectedItem = $derived(items.find((item) => item.id === selectedId) ?? null);

  /** 列表含 GIF：提示首帧转 PNG（后端既定行为，参数区附注） */
  const hasGif = $derived(items.some((item) => item.info?.format === "gif"));

  /** 当前模式解析：非法返回 `null`（开始按钮禁用） */
  const activeMode = $derived(buildMode(params));

  /** 预计输出尺寸：选中项就绪 + 模式合法时有值（先套用旋转，与后端管线同顺序） */
  const estimated = $derived(
    selectedItem?.info && activeMode
      ? planDimensions(
          ...dimsAfterRotation(selectedItem.info.width, selectedItem.info.height, params.rotation),
          activeMode,
          params.noUpscale,
        )
      : null,
  );

  /** 旋转后宽高元组：90/270 系交换宽高，供 `planDimensions` 消费 */
  function dimsAfterRotation(
    width: number,
    height: number,
    rotation: ResizeParamsState["rotation"],
  ): [number, number] {
    const rotated = rotateSize({ width, height }, rotation);
    return [rotated.width, rotated.height];
  }

  /** 可处理项：就绪 + 失败 + 已完成 + 已跳过（调参后可重新生成；跳过重跑仅复查存在性，开销可忽略） */
  const processableCount = $derived(
    items.filter(
      (item) =>
        item.status === "ready" ||
        item.status === "failed" ||
        item.status === "done" ||
        item.status === "skipped",
    ).length,
  );

  /** 目标大小状态：无关（非 JPEG）/关闭/合法/非法三态；非法时禁用开始 */
  const targetState = $derived.by<"off" | "ok" | "bad">(() => {
    if (params.format !== "original" && params.format !== "jpeg") return "off";
    const text = params.targetSizeKb.trim();
    if (text === "") return "off";
    if (!/^\d+$/.test(text)) return "bad";
    const kb = Number.parseInt(text, 10);
    return kb >= 10 && kb <= 51200 ? "ok" : "bad";
  });

  /** 目标大小数值：合法时有值，其余为 `null`（非法已被开始按钮拦截） */
  const targetKbValue = $derived(
    targetState === "ok" ? Number.parseInt(params.targetSizeKb.trim(), 10) : null,
  );

  /** 目标大小是否被忽略：有目标但选中项非 JPEG 输出（后端常规编码，前端明示不适用） */
  const targetIgnored = $derived(
    targetState === "ok" && selectedItem?.info
      ? !isJpegOutput(selectedItem.info.format, params.format)
      : false,
  );

  /** 开始可用态：模式合法 + 目标大小合法 + 有可处理项 + 输出目录已选 + 空闲 + 无读取中 */
  const canStart = $derived(
    activeMode !== null &&
      targetState !== "bad" &&
      processableCount > 0 &&
      params.outputDir.trim() !== "" &&
      !processing &&
      !items.some((item) => item.status === "loading"),
  );

  /** 开始按钮文案：处理中显示进度 */
  const startLabel = $derived(
    progress
      ? m.tool_resize_processing({ done: progress.done, total: progress.total })
      : m.tool_resize_start(),
  );

  /** 解析正整数文本：空/非法返回 `null`（调用方区分“未填”与“非法”） */
  function parsePositiveInt(text: string): number | null {
    const trimmed = text.trim();
    if (trimmed === "") return null;
    if (!/^\d+$/.test(trimmed)) return Number.NaN;
    return Number.parseInt(trimmed, 10);
  }

  /** 参数面板状态转契约模式：任一非法即 `null`（零值视为非法，后端同样拒绝） */
  function buildMode(state: ResizeParamsState): ResizeMode | null {
    switch (state.modeKind) {
      case "box": {
        const width = parsePositiveInt(state.boxWidth);
        const height = parsePositiveInt(state.boxHeight);
        if (Number.isNaN(width) || Number.isNaN(height)) return null;
        if (width == null && height == null) return null;
        if ((width ?? 1) <= 0 || (height ?? 1) <= 0) return null;
        return { kind: "widthheight", width, height, lock_ratio: state.lockRatio };
      }
      case "percent": {
        if (!Number.isInteger(state.percent) || state.percent < 1 || state.percent > 1000) {
          return null;
        }
        return { kind: "percent", percent: state.percent };
      }
      case "exact": {
        const width = parsePositiveInt(state.exactWidth);
        const height = parsePositiveInt(state.exactHeight);
        if (width == null || height == null || Number.isNaN(width) || Number.isNaN(height)) {
          return null;
        }
        if (width <= 0 || height <= 0) return null;
        return { kind: "exact", width, height, fit: state.fit };
      }
    }
  }

  /** 错误值转可读文本：`CommandError` 取 message，IPC 异常取 Error 文本 */
  function failureMessage(failure: unknown): string {
    if (failure instanceof Error) return `${failure.name}: ${failure.message}`;
    if (typeof failure === "object" && failure !== null && "message" in failure) {
      return String((failure as { message: unknown }).message);
    }
    return String(failure);
  }

  /** 扩展名转契约格式：白名单外返回 `null`（浏览器降级路径用） */
  function formatFromExtension(filename: string): ImageFormat | null {
    switch (extensionOf(filename)) {
      case "jpg":
      case "jpeg":
        return "jpeg";
      case "png":
        return "png";
      case "webp":
        return "webp";
      case "bmp":
        return "bmp";
      case "tif":
      case "tiff":
        return "tiff";
      case "gif":
        return "gif";
      default:
        return null;
    }
  }

  /**
   * 路径批量载入（桌面端）：文件夹先展平为图片 + 输出目录自动设为首个文件夹的 `resized/`； 再逐张读元信息 +
   * 缩略图（失败标红，不中断其余）。展开失败回落原路径。
   */
  async function addPaths(paths: string[]): Promise<void> {
    const dropped = paths.filter((path) => path !== "");
    if (dropped.length === 0) return;
    let files = dropped;
    const expanded = await commands.expandDroppedPaths(dropped).result();
    if (expanded.status === "ok") {
      files = expanded.data.files;
      const suggested = expanded.data.output_dir;
      // 仅在输出目录没被动过、且确有文件时自动设置：空文件夹不改目录
      if (suggested && files.length > 0 && !outputDirTouched) {
        params = { ...params, outputDir: suggested };
        outputDirTouched = true;
        toast.info(m.tool_resize_output_auto_set({ dir: suggested }));
      }
    }
    const fresh = files.filter((path) => path !== "" && !items.some((item) => item.id === path));
    if (fresh.length === 0) return;
    const shells: ResizeImageItem[] = fresh.map((path) => ({
      id: path,
      path,
      name: basenameOf(path),
      previewUrl: "",
      revokePreview: false,
      info: null,
      errorDetail: null,
      status: "loading",
      output: null,
      outputSize: null,
      targetMet: null,
    }));
    items = [...items, ...shells];
    if (selectedId === null) selectedId = shells[0].id;
    // 新一批次进列表，旧汇总失效
    summary = null;
    for (const shell of shells) {
      let detail: string | null = null;
      const result = await commands
        .readImageInfo(shell.path)
        .failed((failure) => {
          detail = failureMessage(failure);
        })
        .result();
      if (result.status === "error") {
        items = items.map((item) =>
          item.id === shell.id
            ? { ...item, status: "invalid", errorDetail: detail ?? m.tool_resize_info_failed() }
            : item,
        );
        continue;
      }
      // 元信息成功后再取缩略图 `data:` URL（与预览/列表共用；失败仅空预览，不降级条目）
      const thumb = await commands.getImageThumbnail(shell.path, THUMB_MAX_SIDE).result();
      items = items.map((item) =>
        item.id === shell.id
          ? {
              ...item,
              status: "ready",
              info: result.data,
              previewUrl: thumb.status === "ok" ? thumb.data : "",
            }
          : item,
      );
    }
  }

  /** 浏览器降级载入：`File` 直读，`createImageBitmap` 量尺寸（桌面端不用此路径） */
  async function addBrowserFiles(files: File[]): Promise<void> {
    const fresh = files.filter((file) => file.size > 0);
    for (const file of fresh) {
      const id = `browser:${file.name}:${file.size}:${file.lastModified}`;
      if (items.some((item) => item.id === id)) continue;
      const previewUrl = URL.createObjectURL(file);
      const format = formatFromExtension(file.name);
      let info: ResizeImageItem["info"] = null;
      let errorDetail: string | null = null;
      if (format === null || !isImageExtension(file.name)) {
        errorDetail = m.tool_resize_info_failed();
      } else {
        try {
          const bitmap = await createImageBitmap(file);
          info = {
            width: bitmap.width,
            height: bitmap.height,
            format,
            file_size: file.size,
          };
          bitmap.close();
        } catch {
          errorDetail = m.tool_resize_info_failed();
        }
      }
      const item: ResizeImageItem = {
        id,
        path: "",
        name: file.name,
        previewUrl,
        revokePreview: true,
        info,
        errorDetail,
        status: info ? "ready" : "invalid",
        output: null,
        outputSize: null,
        targetMet: null,
      };
      items = [...items, item];
      if (selectedId === null) selectedId = id;
      summary = null;
    }
  }

  /** 添加图片：桌面端走对话框多选，浏览器走文件框（对话框不可用时降级） */
  async function handleAddImages(): Promise<void> {
    try {
      const picked = await openDialog({
        multiple: true,
        filters: [{ name: "Images", extensions: [...IMAGE_EXTENSIONS] }],
      });
      if (!picked) return;
      await addPaths(Array.isArray(picked) ? picked : [picked]);
    } catch {
      fileInput?.click();
    }
  }

  /** 选择输出目录：对话框目录模式，取消即保持原值 */
  async function handleChooseOutputDir(): Promise<void> {
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked === "string" && picked !== "") {
        params = { ...params, outputDir: picked };
        outputDirTouched = true;
      }
    } catch {
      toast.error(m.tool_resize_request_failed());
    }
  }

  /** 隐藏文件框：浏览器降级入口（桌面端对话框失败时也兜底） */
  let fileInput: HTMLInputElement | null = $state(null);

  /** 文件框变更：取 File 列表走降级载入（可重复选同一文件，变更后清空 value） */
  async function handleFileInput(event: Event): Promise<void> {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    const files = [...(target.files ?? [])];
    target.value = "";
    if (files.length === 0) return;
    await addBrowserFiles(files);
  }

  /** 移除单项：释放浏览器 objectURL，选中项删除后选中邻项 */
  function handleRemove(id: string): void {
    const removed = items.find((item) => item.id === id);
    if (removed?.revokePreview) URL.revokeObjectURL(removed.previewUrl);
    const rest = items.filter((item) => item.id !== id);
    items = rest;
    summary = null;
    if (selectedId === id) selectedId = rest.length > 0 ? rest[rest.length - 1].id : null;
  }

  /** 清空列表：释放全部 objectURL，重置选中/进度 */
  function handleClear(): void {
    for (const item of items) {
      if (item.revokePreview) URL.revokeObjectURL(item.previewUrl);
    }
    items = [];
    selectedId = null;
    progress = null;
    summary = null;
  }

  /**
   * 开始批量：逐张调命令驱动进度，单张失败标红继续，最后统一汇总 toast。 `finally` 保底复位：循环内任何意外抛错都不能把按钮锁在处理中。
   * `retryOnly` 为真时只重跑失败项（汇总条重试入口），否则全量重跑（含已完成，支持调参重新生成）。
   */
  async function handleStart(retryOnly = false): Promise<void> {
    const mode = buildMode(params);
    const outputDir = params.outputDir.trim();
    if (!mode || outputDir === "" || processing || targetState === "bad") return;
    const queue = items.filter((item) =>
      retryOnly ? item.status === "failed" : item.status !== "invalid" && item.status !== "loading",
    );
    if (queue.length === 0) return;
    processing = true;
    progress = { done: 0, total: queue.length };
    const options: ResizeOptions = {
      mode,
      format: params.format,
      quality: params.quality,
      output_dir: outputDir,
      no_upscale: params.noUpscale,
      filename_suffix: params.filenameSuffix,
      rotation: params.rotation,
      filter: params.filter,
      overwrite: params.overwrite,
      target_size_kb: targetKbValue,
    };
    let okCount = 0;
    let skippedCount = 0;
    try {
      for (const queued of queue) {
        const settled = await processOne(queued, options);
        if (settled === "ok") okCount += 1;
        else if (settled === "skipped") skippedCount += 1;
        progress = { done: progress.done + 1, total: queue.length };
        // 每张之间让出一帧：长耗时 IPC 间隙把 UI 线程还给渲染/拖拽，避免进度条与窗口拖动卡死
        await nextFrame();
      }
    } finally {
      // 先复位再汇总：按钮文案/可用态立即回到空闲，toast 只做通知
      processing = false;
      progress = null;
    }
    summary = { ok: okCount, failed: queue.length - okCount - skippedCount, skipped: skippedCount };
    toast.info(
      m.tool_resize_done({
        ok: okCount,
        failed: queue.length - okCount - skippedCount,
        skipped: skippedCount,
      }),
    );
  }

  /** 单张命令结算：`ok` 带业务结果，`error` 带 `CommandError` 或 IPC 异常 */
  type ResizeCallResult = AnyResult<ResizeSingleOutcome, CommandError | Error>;

  /** 单张结算结果：成功/失败/跳过（重名跳过不算失败，汇总单计数） */
  type SingleResult = "ok" | "failed" | "skipped";

  /** 处理单张：状态落盘到列表（浏览器降级项无路径，直接记失败） */
  async function processOne(
    queued: ResizeImageItem,
    options: ResizeOptions,
  ): Promise<SingleResult> {
    // 浏览器降级项无本地路径，后端不可达，直接记失败（桌面端不会走到这里）
    if (queued.path === "") {
      items = items.map((item) =>
        item.id === queued.id
          ? { ...item, status: "failed", errorDetail: m.tool_resize_request_failed() }
          : item,
      );
      return "failed";
    }
    let detail: string | null = null;
    const result: ResizeCallResult = await commands
      .resizeImage(queued.path, options)
      .failed((failure) => {
        detail = failureMessage(failure);
      })
      .result();
    return applyResizeOutcome(queued.id, result, detail);
  }

  /** 结算落盘：传输失败/业务成功/业务失败/重名跳过四分支各自标态 */
  function applyResizeOutcome(
    id: string,
    result: ResizeCallResult,
    detail: string | null,
  ): SingleResult {
    if (result.status === "error") {
      items = items.map((item) =>
        item.id === id
          ? {
              ...item,
              status: "failed",
              errorDetail: detail ?? m.tool_resize_request_failed(),
            }
          : item,
      );
      return "failed";
    }
    if (result.data.ok) {
      items = items.map((item) =>
        item.id === id
          ? {
              ...item,
              status: "done",
              output: result.data.output,
              outputSize:
                result.data.width != null && result.data.height != null
                  ? { width: result.data.width, height: result.data.height }
                  : null,
              targetMet: result.data.target_met,
              errorDetail: null,
            }
          : item,
      );
      return "ok";
    }
    const code = result.data.error?.code ?? "DecodeFailed";
    // 用户文案按码映射，英文诊断原文缀后（冒号分隔），中英混排但信息完整
    const diagnosis = result.data.error?.message ? `：${result.data.error.message}` : "";
    // 重名跳过：灰色徽章 + 映射文案，不算失败
    if (code === "Skipped") {
      items = items.map((item) =>
        item.id === id
          ? { ...item, status: "skipped", errorDetail: `${resizeErrorText(code)}${diagnosis}` }
          : item,
      );
      return "skipped";
    }
    items = items.map((item) =>
      item.id === id
        ? { ...item, status: "failed", errorDetail: `${resizeErrorText(code)}${diagnosis}` }
        : item,
    );
    return "failed";
  }

  /** 是否为文件拖拽：文本选中拖拽不过滤，避免 overlay 误显（同数据互转） */
  function hasFiles(event: DragEvent): boolean {
    return event.dataTransfer?.types.includes("Files") ?? false;
  }

  /** 让出一帧：`setTimeout 0` 把控制权还给 UI 线程（`requestAnimationFrame` 在后台窗口不触发，此处不可用） */
  function nextFrame(): Promise<void> {
    return new Promise((resolve) => {
      setTimeout(resolve, 0);
    });
  }

  onMount(() => {
    const cleanups: (() => void)[] = [];

    // 输出目录默认值：系统图片目录（浏览器无后端时静默留空；不计入“动过”，文件夹仍可自动设置）
    void commands
      .getPictureDir()
      .failed(() => undefined)
      .result()
      .then((result) => {
        if (result.status === "ok" && result.data && !outputDirTouched) {
          params = { ...params, outputDir: result.data };
        }
      });

    // Tauri 拖放事件（桌面端主路径）：路径直接进后端读元信息
    if (isDesktop) {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          const payload = event.payload;
          if (payload.type === "enter") {
            dragDepth = 1;
          } else if (payload.type === "leave") {
            dragDepth = 0;
          } else if (payload.type === "drop") {
            dragDepth = 0;
            void addPaths(payload.paths);
          }
        })
        .then((unlisten) => {
          tauriDropReady = true;
          cleanups.push(unlisten);
        })
        .catch(() => {
          tauriDropReady = false;
        });
    }

    // DOM 拖放（全环境 overlay + 浏览器降级读取）
    const handleDragEnter = (event: DragEvent): void => {
      if (!hasFiles(event)) return;
      event.preventDefault();
      dragDepth += 1;
    };
    const handleDragOver = (event: DragEvent): void => {
      event.preventDefault();
    };
    const handleDragLeave = (event: DragEvent): void => {
      if (!hasFiles(event)) return;
      dragDepth = Math.max(0, dragDepth - 1);
    };
    const handleDrop = (event: DragEvent): void => {
      event.preventDefault();
      dragDepth = 0;
      if (tauriDropReady) return;
      const files = [...(event.dataTransfer?.files ?? [])];
      if (files.length > 0) void addBrowserFiles(files);
    };
    window.addEventListener("dragenter", handleDragEnter);
    window.addEventListener("dragover", handleDragOver);
    window.addEventListener("dragleave", handleDragLeave);
    window.addEventListener("drop", handleDrop);
    return () => {
      window.removeEventListener("dragenter", handleDragEnter);
      window.removeEventListener("dragover", handleDragOver);
      window.removeEventListener("dragleave", handleDragLeave);
      window.removeEventListener("drop", handleDrop);
      for (const cleanup of cleanups) cleanup();
    };
  });

  onDestroy(() => {
    for (const item of items) {
      if (item.revokePreview) URL.revokeObjectURL(item.previewUrl);
    }
  });
</script>

<!-- 三栏可调：左列表 25 + 中预览 50 + 右参数 25（初始比例 1:2:1，不持久化） -->
<div class={cn("relative h-full w-full")}>
  <ResizablePaneGroup direction="horizontal">
    <ResizablePane defaultSize={25} minSize={15}>
      <div class={cn("flex h-full min-h-0 flex-col bg-background")}>
        <ImageList
          {items}
          {selectedId}
          onSelect={(id) => (selectedId = id)}
          onRemove={handleRemove}
          onAdd={() => void handleAddImages()}
          onClear={handleClear}
          {summary}
          {processing}
          onRetry={() => void handleStart(true)}
        />
      </div>
    </ResizablePane>
    <ResizableHandle withHandle />
    <ResizablePane defaultSize={50} minSize={30}>
      <div class={cn("flex h-full min-h-0 flex-col bg-background")}>
        <ImagePreview item={selectedItem} {estimated} {targetIgnored} />
      </div>
    </ResizablePane>
    <ResizableHandle withHandle />
    <ResizablePane defaultSize={25} minSize={15}>
      <div class={cn("flex h-full min-h-0 flex-col bg-background")}>
        <ResizeParams
          {params}
          onParamsChange={(next) => (params = next)}
          onChooseOutputDir={() => void handleChooseOutputDir()}
          {canStart}
          {processing}
          {progress}
          {startLabel}
          onStart={() => void handleStart()}
          {hasGif}
          referenceSize={selectedItem?.info
            ? { width: selectedItem.info.width, height: selectedItem.info.height }
            : null}
        />
      </div>
    </ResizablePane>
  </ResizablePaneGroup>
  {#if dragActive}
    <div
      class={cn(
        "pointer-events-none absolute inset-0 z-10 flex items-center justify-center",
        "border-2 border-dashed border-primary bg-background/80",
      )}
    >
      <p class={cn("text-sm font-medium text-muted-foreground")}>{m.tool_resize_drop_hint()}</p>
    </div>
  {/if}
</div>
<!-- 隐藏文件框：浏览器降级与对话框失败兜底共用 -->
<input
  type="file"
  accept="image/*"
  multiple
  class="hidden"
  bind:this={fileInput}
  onchange={(event) => void handleFileInput(event)}
/>
