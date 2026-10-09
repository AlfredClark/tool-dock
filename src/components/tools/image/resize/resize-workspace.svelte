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

  /** 列表小图边长：96px 行 + 高分屏余量（批量一次取全；大图选中后懒加载） */
  const LIST_SIDE = 96;

  /** 大图边长：选中项按需取（与旧共用缩略图同值，行为不变） */
  const DETAIL_SIDE = 768;

  /** 批量切块：单次 IPC 至多 200 条（后端上限 500，留余量防 URL 总量爆 IPC） */
  const LOAD_CHUNK = 200;

  /** 大图缓存上限：30 张（LRU 淘汰并清空条目，防 data-URL 常驻爆内存） */
  const DETAIL_CACHE_LIMIT = 30;

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

  /** 载入进度：批量进行中有值（顶部细进度条展示，不锁开始按钮） */
  let loadProgress = $state<{ loaded: number; total: number } | null>(null);

  // 载入轮次：每次 addPaths/清空自增，在途回写先对轮次（清空与二次拖放丢弃旧批次）
  let loadRun = 0;

  // 大图缓存：选中项 768px 按需取，LRU 上限 30（普通对象，插入序即访问序，命中时先删后插刷新；
  // 刻意不用响应式 Map：缓存不是 UI 状态，不应订阅触发 effect）
  const detailCache: Record<string, string> = {};

  // 输出目录是否被动过（手动选择 / 文件夹自动设置）：动过后挂载默认值与后续自动设置都不再覆盖
  let outputDirTouched = false;

  // 拖放悬浮计数：dragenter/dragleave 成对增减，>0 显示 overlay（同数据互转）
  let dragDepth = $state(0);
  const dragActive = $derived(dragDepth > 0);

  // Tauri 拖放监听就绪后，DOM drop 只做 preventDefault，不再重复读取
  let tauriDropReady = false;

  /** 当前选中项：右侧预览消费 */
  const selectedItem = $derived(items.find((item) => item.id === selectedId) ?? null);

  // 大图懒加载：选中就绪项后按需取 768px（缓存命中直接落盘；在途与已载直接返回，天然收敛不循环）
  $effect(() => {
    const current = selectedItem;
    if (
      current === null ||
      current.path === "" ||
      current.info === null ||
      current.status !== "ready" ||
      current.detailUrl !== "" ||
      current.detailLoading ||
      current.detailFailed
    ) {
      return;
    }
    const cached = detailCache[current.id];
    if (cached !== undefined) {
      // 命中刷新 LRU 序（先删后插），落盘大图
      delete detailCache[current.id];
      detailCache[current.id] = cached;
      const hit = cached;
      const hitId = current.id;
      items = items.map((item) => (item.id === hitId ? { ...item, detailUrl: hit } : item));
      return;
    }
    const run = loadRun;
    const targetId = current.id;
    const targetPath = current.path;
    items = items.map((item) => (item.id === targetId ? { ...item, detailLoading: true } : item));
    void commands
      .getImageThumbnail(targetPath, DETAIL_SIDE)
      .result()
      .then((thumb) => {
        if (run !== loadRun || thumb.status !== "ok" || thumb.data === "") {
          // 空结果记终态失败：不再重试（`$effect` 守卫拦截，避免空转刷命令）
          if (run === loadRun) {
            items = items.map((item) =>
              item.id === targetId ? { ...item, detailLoading: false, detailFailed: true } : item,
            );
          }
          return;
        }
        detailCache[targetId] = thumb.data;
        if (Object.keys(detailCache).length > DETAIL_CACHE_LIMIT) {
          const oldest = Object.keys(detailCache)[0];
          if (oldest !== undefined && oldest !== targetId) {
            delete detailCache[oldest];
            // 淘汰即释放：清空被逐条目的大图（小图保留，列表不受影响）
            const evicted = oldest;
            items = items.map((item) => (item.id === evicted ? { ...item, detailUrl: "" } : item));
          }
        }
        const large = thumb.data;
        items = items.map((item) =>
          item.id === targetId ? { ...item, detailUrl: large, detailLoading: false } : item,
        );
      });
  });

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
   * 路径批量载入（桌面端）：文件夹先展平为图片 + 输出目录自动设为首个文件夹的 `resized/`； 再按 200 切块调批量命令（后端文件间 4
   * 并发单解码），每块合并一次落盘并让出一帧。 单项失败标红不中断其余；整块传输失败按块标红（罕见，常见是单项解码失败）。 展开失败回落原路径。
   */
  async function addPaths(paths: string[]): Promise<void> {
    // 处理中锁定列表变更：拖放与对话框入口统一在此拦截（按钮侧同时禁用，双保险）
    if (processing) return;
    const run = ++loadRun;
    const dropped = paths.filter((path) => path !== "");
    if (dropped.length === 0) return;
    let files = dropped;
    const expanded = await commands.expandDroppedPaths(dropped).result();
    if (run !== loadRun) return;
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
    const seen = new Set(items.map((item) => item.id));
    const fresh = files.filter((path) => path !== "" && !seen.has(path));
    if (fresh.length === 0) return;
    const shells: ResizeImageItem[] = fresh.map((path) => ({
      id: path,
      path,
      name: basenameOf(path),
      previewUrl: "",
      detailUrl: "",
      detailLoading: false,
      detailFailed: false,
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
    loadProgress = { loaded: 0, total: fresh.length };
    let done = 0;
    for (let start = 0; start < fresh.length; start += LOAD_CHUNK) {
      if (run !== loadRun) return;
      const chunk = fresh.slice(start, start + LOAD_CHUNK);
      let detail: string | null = null;
      const result = await commands
        .readImageBatch(chunk, LIST_SIDE)
        .failed((failure) => {
          detail = failureMessage(failure);
        })
        .result();
      if (run !== loadRun) return;
      if (result.status === "error") {
        const bad = new Set(chunk);
        const message = detail ?? m.tool_resize_request_failed();
        items = items.map((item) =>
          item.status === "loading" && bad.has(item.id)
            ? { ...item, status: "invalid", errorDetail: message }
            : item,
        );
      } else {
        const byPath = new Map(result.data.map((entry) => [entry.path, entry]));
        items = items.map((item) => {
          if (item.status !== "loading" || !byPath.has(item.id)) return item;
          const entry = byPath.get(item.id);
          if (entry === undefined || !entry.ok || entry.info === null) {
            return {
              ...item,
              status: "invalid",
              errorDetail: entry?.error ?? m.tool_resize_info_failed(),
            };
          }
          // 小图失败仅空预览，不降级条目（大图选中后仍可按需取）
          return { ...item, status: "ready", info: entry.info, previewUrl: entry.thumb ?? "" };
        });
      }
      done += chunk.length;
      loadProgress = { loaded: done, total: fresh.length };
      // 每块之间让出一帧：批量回写间隙把 UI 线程还给渲染/拖拽
      await nextFrame();
    }
    if (run !== loadRun) return;
    loadProgress = null;
  }

  /** 浏览器降级载入：`File` 直读，`createImageBitmap` 量尺寸（桌面端不用此路径） */
  async function addBrowserFiles(files: File[]): Promise<void> {
    if (processing) return;
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
        detailUrl: "",
        detailLoading: false,
        detailFailed: false,
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
    if (processing) return;
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

  /** 移除单项：释放浏览器 objectURL 与大图缓存，选中项删除后选中邻项 */
  function handleRemove(id: string): void {
    if (processing) return;
    const removed = items.find((item) => item.id === id);
    if (removed?.revokePreview) URL.revokeObjectURL(removed.previewUrl);
    delete detailCache[id];
    const rest = items.filter((item) => item.id !== id);
    items = rest;
    summary = null;
    if (selectedId === id) selectedId = rest.length > 0 ? rest[rest.length - 1].id : null;
  }

  /** 清空列表：作废在途批量（轮次自增丢弃回写），释放全部 objectURL 与缓存，重置选中/进度 */
  function handleClear(): void {
    if (processing) return;
    loadRun += 1;
    for (const item of items) {
      if (item.revokePreview) URL.revokeObjectURL(item.previewUrl);
    }
    for (const key of Object.keys(detailCache)) delete detailCache[key];
    items = [];
    selectedId = null;
    progress = null;
    loadProgress = null;
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
          // 处理中忽略拖放：悬浮不显 overlay，松手不进列表（`addPaths` 另有守卫，双保险）
          if (processing) return;
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

    // DOM 拖放（全环境 overlay + 浏览器降级读取）：处理中一律忽略，不改悬浮计数
    const handleDragEnter = (event: DragEvent): void => {
      if (!hasFiles(event)) return;
      event.preventDefault();
      if (processing) return;
      dragDepth += 1;
    };
    const handleDragOver = (event: DragEvent): void => {
      event.preventDefault();
    };
    const handleDragLeave = (event: DragEvent): void => {
      if (!hasFiles(event)) return;
      if (processing) return;
      dragDepth = Math.max(0, dragDepth - 1);
    };
    const handleDrop = (event: DragEvent): void => {
      event.preventDefault();
      dragDepth = 0;
      if (processing || tauriDropReady) return;
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
  {#if loadProgress}
    <!-- 批量载入进度：顶部细条，不锁交互（开始按钮以就绪项为准，可边载边开始） -->
    <div class={cn("absolute inset-x-0 top-0 z-20 h-0.5 bg-muted")}>
      <div
        class={cn("h-full bg-primary transition-all")}
        style={`width: ${(loadProgress.loaded / Math.max(1, loadProgress.total)) * 100}%`}
      ></div>
    </div>
  {/if}
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
