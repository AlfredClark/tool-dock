<script lang="ts">
  // 视频格式转换 workspace：左列（文件列表）+ 中列（上预览下信息）+ 右列（参数双页签）的三栏组装，持有全部页面级状态。
  // 文件接入三轨：桌面端对话框/拖放给路径走后端读元信息 + 海报帧，浏览器降级仅展示文件名。
  // 处理逐项调转换命令驱动进度，单项失败标红跳过继续，最后统一出成功/失败汇总。
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import type {
    CommandError,
    FfmpegStatus,
    VideoConvertErrorCode,
    VideoConvertOutcome,
  } from "$libs/commands/bindings";
  import type { AnyResult } from "$libs/commands/types";
  import ConvertInfo from "$components/tools/video/convert/convert-info.svelte";
  import ConvertList from "$components/tools/video/convert/convert-list.svelte";
  import ConvertParams from "$components/tools/video/convert/convert-params.svelte";
  import ConvertPreview from "$components/tools/video/convert/convert-preview.svelte";
  import {
    ResizableHandle,
    ResizablePane,
    ResizablePaneGroup,
  } from "$components/shadcn-svelte/resizable";
  import { reportCommandFailure } from "$libs/commands/cores";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";
  import { cn } from "$libs/utils/shadcn-svelte";
  import { convertErrorText, convertNoteText } from "./convert-errors";
  import {
    DEFAULT_CONVERT_PARAMS,
    formatDownloadSize,
    videoFileExt,
    videoFileStem,
    type ConvertParamsState,
    type VideoConvertItem,
    type VideoConvertSummary,
  } from "./convert-types";

  /** 桌面端判定：有 Tauri 注入才走路径 + 命令链，否则走浏览器降级 */
  const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  /** 视频扩展名：与后端白名单同源（对话框过滤器 + 拖放校验共用） */
  const VIDEO_EXTENSIONS = ["mp4", "m4v", "mov", "mkv", "webm", "avi", "ts"];

  /** 列表小海报边长：96px（批量一次取全；大海报选中后懒加载） */
  const LIST_SIDE = 96;

  /** 大海报边长：选中项按需取（与旧值一致，行为不变） */
  const DETAIL_SIDE = 768;

  /** 批量切块：单次 IPC 至多 200 条（后端上限 500，留余量防 URL 总量爆 IPC） */
  const LOAD_CHUNK = 200;

  /** 大海报缓存上限：30 张（LRU 淘汰并清空条目，防 data-URL 常驻爆内存） */
  const DETAIL_CACHE_LIMIT = 30;

  /** 下载进度事件名，与 Rust 侧 `cores::ffmpeg::FFMPEG_DOWNLOAD_PROGRESS_EVENT` 会合 */
  const PROGRESS_EVENT = "ffmpeg-download-progress";

  /** 列表项：加入顺序即处理顺序 */
  let items = $state<VideoConvertItem[]>([]);

  /** 选中项 id：`null` 即右侧空态 */
  let selectedId = $state<string | null>(null);

  /** 目标格式与输出参数：面板读写，提交时整体下发（无模板展开） */
  let params = $state<ConvertParamsState>({ ...DEFAULT_CONVERT_PARAMS });

  /** 批量处理中：锁表单与开始按钮，进度条展示 */
  let processing = $state(false);

  /** 进度：`null` 即空闲不展示 */
  let progress = $state<{ done: number; total: number } | null>(null);

  /** 批量汇总：处理完成后常驻列表底部（改列表即失效，随增删清空） */
  let summary = $state<VideoConvertSummary | null>(null);

  /** 载入进度：批量进行中有值（顶部细进度条展示，不锁开始按钮） */
  let loadProgress = $state<{ loaded: number; total: number } | null>(null);

  // 载入轮次：每次 addPaths/清空自增，在途回写先对轮次（清空与二次拖放丢弃旧批次）
  let loadRun = 0;

  // 大海报缓存：选中项 768px 按需取，LRU 上限 30（普通对象，插入序即访问序，命中时先删后插刷新；
  // 刻意不用响应式 Map：缓存不是 UI 状态，不应订阅触发 effect）
  const detailCache: Record<string, string> = {};

  /** 引擎状态：`null` 即查询中；缺失时开始按钮禁用并引导右栏 FFmpeg 页 */
  let ffmpegStatus = $state<FfmpegStatus | null>(null);

  /** FFmpeg 查询中：锁刷新按钮 */
  let ffmpegChecking = $state(false);

  /** FFmpeg 下载/重装进行中：`null` 即空闲（两者互斥，同时只跑一个） */
  let ffmpegTask = $state<"ensure" | "repair" | null>(null);

  /** FFmpeg 下载进度：命令返回终态，事件只做过程展示 */
  let ffmpegDownloaded = $state(0);
  let ffmpegTotal = $state<number | null>(null);

  /** FFmpeg 失败原因：管理页展示诊断原文 */
  let ffmpegError = $state<string | null>(null);

  // 输出目录是否被动过（手动选择 / 文件夹自动设置）：动过后挂载默认值与后续自动设置都不再覆盖
  let outputDirTouched = false;

  // 拖放悬浮计数：dragenter/dragleave 成对增减，>0 显示 overlay（同元数据工具）
  let dragDepth = $state(0);
  const dragActive = $derived(dragDepth > 0);

  // Tauri 拖放监听就绪后，DOM drop 只做 preventDefault，不再重复读取
  let tauriDropReady = false;

  let progressListening = false;

  /** 当前选中项：中栏预览与信息消费 */
  const selectedItem = $derived(items.find((item) => item.id === selectedId) ?? null);

  // 大海报懒加载：选中就绪项后按需取 768px（抽帧时刻按当时长 10% 就地计算；缓存命中直接落盘）
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
      // 命中刷新 LRU 序（先删后插），落盘大海报
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
    const seek = seekFor(current);
    items = items.map((item) => (item.id === targetId ? { ...item, detailLoading: true } : item));
    void commands
      .getVideoThumbnail(targetPath, DETAIL_SIDE, seek)
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
            // 淘汰即释放：清空被逐条目的大海报（小海报保留）
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

  /** 可处理项：就绪 + 失败 + 已完成 + 已跳过（调参后可重新生成） */
  const processableCount = $derived(
    items.filter(
      (item) =>
        item.status === "ready" ||
        item.status === "failed" ||
        item.status === "done" ||
        item.status === "skipped",
    ).length,
  );

  /** 引擎可用：状态已查到且可用（查询中/缺失/浏览器均不可开始） */
  const ffmpegReady = $derived(ffmpegStatus?.available === true);

  /** 引擎缺失：已查到且不可用，格式页顶部引导去 FFmpeg 页 */
  const ffmpegMissing = $derived(ffmpegStatus !== null && !ffmpegStatus.available);

  /** 开始可用态：有可处理项 + 输出目录已选 + 引擎可用 + 空闲 + 无读取中 */
  const canStart = $derived(
    processableCount > 0 &&
      params.outputDir.trim() !== "" &&
      ffmpegReady &&
      !processing &&
      !items.some((item) => item.status === "loading"),
  );

  /** 开始按钮文案：处理中显示进度 */
  const startLabel = $derived(
    progress
      ? m.tool_video_processing({ done: progress.done, total: progress.total })
      : m.tool_video_start(),
  );

  /** FFmpeg 进度条百分比：总数未知时为 0（不确定态） */
  const ffmpegProgressValue = $derived(
    ffmpegTotal != null && ffmpegTotal > 0 ? (ffmpegDownloaded / ffmpegTotal) * 100 : 0,
  );

  /** FFmpeg 进度行文本：下载中按事件刷新 */
  const ffmpegProgressText = $derived(
    m.tool_ffmpeg_downloading({
      downloaded: formatDownloadSize(ffmpegDownloaded),
      total: ffmpegTotal == null ? "…" : formatDownloadSize(ffmpegTotal),
    }),
  );

  /** 错误值转可读文本：`CommandError` 取 message，IPC 异常取 Error 文本 */
  function failureMessage(failure: unknown): string {
    if (failure instanceof Error) return `${failure.name}: ${failure.message}`;
    if (typeof failure === "object" && failure !== null && "message" in failure) {
      return String((failure as { message: unknown }).message);
    }
    return String(failure);
  }

  /** 显示名：路径取 basename（含后缀），浏览器取 File 名 */
  function displayName(path: string, fallback: string): string {
    if (path === "") return fallback;
    const stem = videoFileStem(path);
    const ext = videoFileExt(path);
    return ext === "" ? stem : `${stem}.${ext}`;
  }

  /** 海报抽帧时刻：时长 10% 处（钳制 0.5s–10s），未知时长回落 1s */
  function seekFor(item: VideoConvertItem): number {
    const duration = item.info?.duration_seconds;
    if (duration == null || !Number.isFinite(duration)) return 1;
    return Math.min(10, Math.max(0.5, duration * 0.1));
  }

  /**
   * 路径批量载入（桌面端）：文件夹先展平为视频 + 输出目录自动设为首个文件夹的 `edited/`； 再按 200 切块调批量命令（后端文件间 4
   * 并发，元信息 + 96px 小海报一次回），每块合并一次落盘并让出一帧。 单项失败标红不中断其余；整块传输失败按块标红。展开失败回落原路径。
   */
  async function addPaths(paths: string[]): Promise<void> {
    // 处理中锁定列表变更：拖放与对话框入口统一在此拦截（按钮侧同时禁用，双保险）
    if (processing) return;
    const run = ++loadRun;
    const dropped = paths.filter((path) => path !== "");
    if (dropped.length === 0) return;
    let files = dropped;
    const expanded = await commands.expandDroppedVideoPaths(dropped).result();
    if (run !== loadRun) return;
    if (expanded.status === "ok") {
      files = expanded.data.files;
      const suggested = expanded.data.output_dir;
      // 仅在输出目录没被动过、且确有文件时自动设置：空文件夹不改目录
      if (suggested && files.length > 0 && !outputDirTouched) {
        params = { ...params, outputDir: suggested };
        outputDirTouched = true;
        toast.info(m.tool_video_output_auto_set({ dir: suggested }));
      }
    }
    const seen = new Set(items.map((item) => item.id));
    const fresh = files.filter((path) => path !== "" && !seen.has(path));
    if (fresh.length === 0) return;
    const shells: VideoConvertItem[] = fresh.map((path) => ({
      id: path,
      path,
      name: displayName(path, path),
      previewUrl: "",
      detailUrl: "",
      detailLoading: false,
      detailFailed: false,
      info: null,
      errorDetail: null,
      status: "loading",
      output: null,
      note: null,
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
        .readVideoBatch(chunk, LIST_SIDE)
        .failed((failure) => {
          detail = failureMessage(failure);
        })
        .result();
      if (run !== loadRun) return;
      if (result.status === "error") {
        const bad = new Set(chunk);
        const message = detail ?? m.tool_video_request_failed();
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
              errorDetail: entry?.error ?? m.tool_video_info_failed(),
            };
          }
          // 小海报失败仅空预览，不降级条目（大海报选中后仍可按需取）
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

  /** 浏览器降级载入：仅展示文件名（无后端不可读），处理不可用 */
  async function addBrowserFiles(files: File[]): Promise<void> {
    if (processing) return;
    const fresh = files.filter((file) => file.size > 0);
    for (const file of fresh) {
      const id = `browser:${file.name}:${file.size}:${file.lastModified}`;
      if (items.some((item) => item.id === id)) continue;
      const item: VideoConvertItem = {
        id,
        path: "",
        name: file.name,
        previewUrl: "",
        detailUrl: "",
        detailLoading: false,
        detailFailed: false,
        info: null,
        errorDetail: m.tool_video_browser_unsupported(),
        status: "invalid",
        output: null,
        note: null,
      };
      items = [...items, item];
      if (selectedId === null) selectedId = id;
      summary = null;
    }
  }

  /** 添加视频：桌面端走对话框多选，浏览器走文件框（对话框不可用时降级） */
  async function handleAddVideos(): Promise<void> {
    if (processing) return;
    try {
      const picked = await openDialog({
        multiple: true,
        filters: [{ name: "Videos", extensions: [...VIDEO_EXTENSIONS] }],
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
      toast.error(m.tool_video_request_failed());
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

  /** 移除单项：清大海报缓存，选中项删除后选中邻项 */
  function handleRemove(id: string): void {
    if (processing) return;
    delete detailCache[id];
    const rest = items.filter((item) => item.id !== id);
    items = rest;
    summary = null;
    if (selectedId === id) selectedId = rest.length > 0 ? rest[rest.length - 1].id : null;
  }

  /** 清空列表：作废在途批量（轮次自增丢弃回写），清缓存，重置选中/进度 */
  function handleClear(): void {
    if (processing) return;
    loadRun += 1;
    for (const key of Object.keys(detailCache)) delete detailCache[key];
    items = [];
    selectedId = null;
    progress = null;
    loadProgress = null;
    summary = null;
  }

  /**
   * 开始批量：逐项调转换命令驱动进度，单项失败标红继续，最后统一汇总 toast。 `finally`
   * 保底复位：循环内任何意外抛错都不能把按钮锁在处理中。 `retryOnly`
   * 为真时只重跑失败项（汇总条重试入口），否则全量重跑（含已完成，支持调参重新生成）。
   */
  async function handleStart(retryOnly = false): Promise<void> {
    const outputDir = params.outputDir.trim();
    if (
      outputDir === "" ||
      processing ||
      !ffmpegReady ||
      items.some((item) => item.status === "loading")
    ) {
      return;
    }
    const queue = items.filter((item) =>
      retryOnly ? item.status === "failed" : item.status !== "invalid" && item.status !== "loading",
    );
    if (queue.length === 0) return;
    processing = true;
    progress = { done: 0, total: queue.length };
    let okCount = 0;
    let skippedCount = 0;
    try {
      for (const queued of queue) {
        const settled = await processOne(queued, outputDir);
        if (settled === "ok") okCount += 1;
        else if (settled === "skipped") skippedCount += 1;
        progress = { done: progress.done + 1, total: queue.length };
        // 每项之间让出一帧：长耗时 IPC 间隙把 UI 线程还给渲染/拖拽，避免进度条与窗口拖动卡死
        await nextFrame();
      }
    } finally {
      // 先复位再汇总：按钮文案/可用态立即回到空闲，toast 只做通知
      processing = false;
      progress = null;
    }
    summary = { ok: okCount, failed: queue.length - okCount - skippedCount, skipped: skippedCount };
    toast.info(
      m.tool_video_done({
        ok: okCount,
        failed: queue.length - okCount - skippedCount,
        skipped: skippedCount,
      }),
    );
  }

  /** 单项命令结算：`ok` 带业务结果，`error` 带 `CommandError` 或 IPC 异常 */
  type VideoCallResult = AnyResult<VideoConvertOutcome, CommandError | Error>;

  /** 单项结算结果：成功/失败/跳过（重名跳过不算失败，汇总单计数） */
  type SingleResult = "ok" | "failed" | "skipped";

  /** 处理单项：浏览器降级项直接记失败（无本地路径不可转换） */
  async function processOne(queued: VideoConvertItem, outputDir: string): Promise<SingleResult> {
    if (queued.path === "") {
      items = items.map((item) =>
        item.id === queued.id
          ? { ...item, status: "failed", errorDetail: m.tool_video_request_failed() }
          : item,
      );
      return "failed";
    }
    let detail: string | null = null;
    const result: VideoCallResult = await commands
      .convertVideoFormat(queued.path, {
        target: params.target,
        mode: params.mode,
        output_dir: outputDir,
        overwrite: params.overwrite,
        quality: params.quality,
        preset: params.preset,
        video_encoder: params.videoEncoder,
        audio_bitrate: params.audioBitrate,
        resolution: params.resolution,
        accelerator: params.accelerator,
      })
      .failed((failure) => {
        detail = failureMessage(failure);
      })
      .result();
    return applyOutcome(queued.id, result, detail);
  }

  /** 结算落盘：传输失败/业务成功/业务失败/重名跳过四分支各自标态（转换备注仅成功时落盘） */
  function applyOutcome(id: string, result: VideoCallResult, detail: string | null): SingleResult {
    if (result.status === "error") {
      items = items.map((item) =>
        item.id === id
          ? {
              ...item,
              status: "failed",
              errorDetail: detail ?? m.tool_video_request_failed(),
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
              errorDetail: null,
              note: convertNoteText(result.data.tried_copy, result.data.used_hw ?? false),
            }
          : item,
      );
      return "ok";
    }
    const code: VideoConvertErrorCode = result.data.error?.code ?? "FfmpegFailed";
    // 用户文案按码映射，英文诊断原文缀后（冒号分隔），中英混排但信息完整
    const diagnosis = result.data.error?.message ? `：${result.data.error.message}` : "";
    // 重名跳过：灰色徽章 + 映射文案，不算失败
    if (code === "Skipped") {
      items = items.map((item) =>
        item.id === id
          ? { ...item, status: "skipped", errorDetail: `${convertErrorText(code)}${diagnosis}` }
          : item,
      );
      return "skipped";
    }
    items = items.map((item) =>
      item.id === id
        ? { ...item, status: "failed", errorDetail: `${convertErrorText(code)}${diagnosis}` }
        : item,
    );
    return "failed";
  }

  /** 刷新引擎状态：静默更新（失败 toast 提示，保留旧状态） */
  async function refreshFfmpegStatus(): Promise<void> {
    if (ffmpegChecking) return;
    ffmpegChecking = true;
    await commands
      .getFfmpegStatus()
      .success((next) => {
        ffmpegStatus = next;
        ffmpegError = null;
      })
      .failed((failure) => {
        toast.error(m.tool_ffmpeg_status_failed());
        reportCommandFailure("[video-convert] ffmpeg status failed", failure);
      });
    ffmpegChecking = false;
  }

  /** 订阅下载进度事件；浏览器 / 单测下动态导入失败则降级为无进度条 */
  async function ensureProgressListener(): Promise<void> {
    if (progressListening) return;
    try {
      const { listen } = await import("@tauri-apps/api/event");
      await listen<{ downloaded: number; total: number | null }>(PROGRESS_EVENT, (event) => {
        ffmpegDownloaded = event.payload.downloaded;
        ffmpegTotal = event.payload.total;
      });
      progressListening = true;
    } catch (error) {
      reportCommandFailure("[video-convert] ffmpeg progress listener unavailable", error);
    }
  }

  /** 下载 / 修复重装的统一执行：成功刷新状态，失败展示原因 */
  async function runFfmpegTask(kind: "ensure" | "repair"): Promise<void> {
    if (ffmpegTask !== null) return;
    ffmpegTask = kind;
    ffmpegDownloaded = 0;
    ffmpegTotal = null;
    ffmpegError = null;
    await ensureProgressListener();
    const call = kind === "ensure" ? commands.ensureFfmpeg() : commands.reinstallManagedFfmpeg();
    await call
      .success((next) => {
        ffmpegStatus = next;
      })
      .failed((failure) => {
        ffmpegError = failureMessage(failure);
        toast.error(m.tool_ffmpeg_download_failed());
      });
    ffmpegTask = null;
  }

  /** 是否为文件拖拽：文本选中拖拽不过滤，避免 overlay 误显（同元数据工具） */
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

    // 输出目录默认值：系统视频目录（浏览器无后端时静默留空；不计入“动过”，文件夹仍可自动设置）
    void commands
      .getVideoDir()
      .failed(() => undefined)
      .result()
      .then((result) => {
        if (result.status === "ok" && result.data && !outputDirTouched) {
          params = { ...params, outputDir: result.data };
        }
      });

    // 引擎状态初查：桌面端才调命令链，浏览器保持 `null`（开始按钮天然禁用）
    if (isDesktop) {
      void refreshFfmpegStatus();
    }

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
      event.preventDefault();
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
        <ConvertList
          {items}
          {selectedId}
          onSelect={(id) => (selectedId = id)}
          onRemove={handleRemove}
          onAdd={() => void handleAddVideos()}
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
        <ConvertPreview item={selectedItem} />
        <ConvertInfo item={selectedItem} target={params.target} />
      </div>
    </ResizablePane>
    <ResizableHandle withHandle />
    <ResizablePane defaultSize={25} minSize={15}>
      <div class={cn("flex h-full min-h-0 flex-col bg-background")}>
        <ConvertParams
          {params}
          onParamsChange={(next) => (params = next)}
          onChooseOutputDir={() => void handleChooseOutputDir()}
          {canStart}
          {processing}
          {progress}
          {startLabel}
          onStart={() => void handleStart()}
          {ffmpegMissing}
          {ffmpegStatus}
          ffmpegBusy={ffmpegChecking || ffmpegTask !== null}
          ffmpegDownloading={ffmpegTask !== null}
          ffmpegProgressText={ffmpegTask !== null ? ffmpegProgressText : null}
          {ffmpegProgressValue}
          {ffmpegError}
          onRefreshFfmpeg={() => void refreshFfmpegStatus()}
          onEnsureFfmpeg={() => void runFfmpegTask("ensure")}
          onRepairFfmpeg={() => void runFfmpegTask("repair")}
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
      <p class={cn("text-sm font-medium text-muted-foreground")}>{m.tool_video_drop_hint()}</p>
    </div>
  {/if}
</div>
<!-- 隐藏文件框：浏览器降级与对话框失败兜底共用 -->
<input
  type="file"
  accept="video/*"
  multiple
  class="hidden"
  bind:this={fileInput}
  onchange={(event) => void handleFileInput(event)}
/>
