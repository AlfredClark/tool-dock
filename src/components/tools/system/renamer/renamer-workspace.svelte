<script lang="ts">
  // 重命名器 workspace：左右可调分栏组装，持有页面级状态（规则/文件列表/筛选/排序/执行）。
  // 文件接入三轨：桌面端对话框/拖放给路径直接入列表，浏览器降级读 File 名；规则变更后新文件名
  // 经纯前端预览派生（只改主名、保留扩展名，编号按全量加入顺序）；开始改名经命令链调后端
  // 逐项执行（前端传算好的新名，后端只做二次校验与改名），浏览器项跳过不可执行。
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    ResizableHandle,
    ResizablePane,
    ResizablePaneGroup,
  } from "$components/shadcn-svelte/resizable";
  import commands from "$libs/commands";
  import type { RenameOutcome } from "$libs/commands/bindings";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";
  import { cn } from "$libs/utils/shadcn-svelte";
  import FileList from "./file-list.svelte";
  import FileToolbar from "./file-toolbar.svelte";
  import RenameFooter from "./rename-footer.svelte";
  import RulePanel from "./rule-panel.svelte";
  import type {
    RenamerFileItem,
    RenamerRule,
    RenamerRuleKind,
    RenamerSortKey,
  } from "./renamer-types";
  import { createRule, moveRuleTo, previewName, removeRule, updateRule } from "./renamer-rules";
  import {
    basenameOf,
    filterByQuery,
    removeChecked,
    setCheckedForIds,
    sortItems,
  } from "./renamer-utils";

  /** 桌面端判定：有 Tauri 注入才走路径，否则走浏览器降级 */
  const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  /** 改名规则：数组顺序即应用顺序 */
  let rules = $state<RenamerRule[]>([]);

  /** 文件列表：加入顺序即初始顺序，展示顺序由筛选 + 排序派生 */
  let items = $state<RenamerFileItem[]>([]);

  /** 搜索筛选文本 */
  let query = $state("");

  /** 排序键：默认名称升序 */
  let sortKey = $state<RenamerSortKey>("name-asc");

  // 拖放悬浮计数：dragenter/dragleave 成对增减，>0 显示 overlay（同数据互转）
  let dragDepth = $state(0);
  const dragActive = $derived(dragDepth > 0);

  // Tauri 拖放监听就绪后，DOM drop 只做 preventDefault，不再重复读取
  let tauriDropReady = false;

  /** 预览落盘：规则按全量加入顺序编号，零规则返回原数组引用（避免多余重渲染） */
  const itemsWithPreview = $derived(
    rules.length === 0
      ? items
      : items.map((item, index) => ({
          ...item,
          newName: previewName(item.name, rules, index),
        })),
  );

  /** 可见行：先筛选后排序 */
  const visibleItems = $derived(sortItems(filterByQuery(itemsWithPreview, query), sortKey));

  /** 表头全选态：空列表为 `false`（禁用），部分勾选为 `"indeterminate"` */
  const headerChecked = $derived.by<boolean | "indeterminate">(() => {
    if (visibleItems.length === 0) return false;
    const checkedCount = visibleItems.filter((item) => item.checked).length;
    if (checkedCount === 0) return false;
    if (checkedCount === visibleItems.length) return true;
    return "indeterminate";
  });

  /** 表头是否禁用：无可见行时禁用 */
  const headerDisabled = $derived(visibleItems.length === 0);

  /** 全局已勾选数：驱动删除按钮显隐 */
  const selectedCount = $derived(items.filter((item) => item.checked).length);

  /** 执行中：锁开始按钮与列表增删，进度条展示 */
  let processing = $state(false);

  /** 执行汇总：处理完成后常驻底部，文件/规则变更即失效（`null` 不展示） */
  let summary = $state<string | null>(null);

  /** 可执行项：有本地路径且新名与原名不同（浏览器项与未改名项不进后端） */
  const executableItems = $derived(
    itemsWithPreview.filter((item) => item.path !== "" && item.newName !== item.name),
  );

  /** 浏览器项数：执行时 toast 提示跳过（仅桌面端可改名） */
  const browserCount = $derived(items.filter((item) => item.path === "").length);

  /** 开始按钮禁用：非桌面端/无可执行项/处理中 */
  const startDisabled = $derived(!isDesktop || executableItems.length === 0 || processing);

  /** 禁用提示：非桌面端悬浮说明 */
  const startTitle = $derived(isDesktop ? null : m.tool_renamer_desktop_only());

  /** 汇总失效：文件增删即清空（规则变更重算预览，旧汇总不再对应当前列表） */
  function invalidateSummary(): void {
    summary = null;
  }

  /** 路径批量入列表：去重（路径即 id），新文件名恒等于原文件名 */
  function addPaths(paths: string[]): void {
    const fresh = paths.filter((path) => path !== "" && !items.some((item) => item.id === path));
    if (fresh.length === 0) return;
    const shells: RenamerFileItem[] = fresh.map((path) => {
      const name = basenameOf(path);
      return { id: path, path, name, newName: name, checked: false, error: null };
    });
    items = [...items, ...shells];
    invalidateSummary();
  }

  /** 浏览器降级入列表：只收文件名（桌面端不用此路径） */
  function addBrowserFiles(files: File[]): void {
    const fresh = files.filter((file) => file.size > 0 || file.name !== "");
    if (fresh.length === 0) return;
    const shells: RenamerFileItem[] = [];
    for (const file of fresh) {
      const id = `browser:${file.name}:${file.size}:${file.lastModified}`;
      if (items.some((item) => item.id === id) || shells.some((item) => item.id === id)) continue;
      shells.push({
        id,
        path: "",
        name: file.name,
        newName: file.name,
        checked: false,
        error: null,
      });
    }
    if (shells.length > 0) items = [...items, ...shells];
    invalidateSummary();
  }

  /** 添加文件：桌面端走对话框多选，失败（浏览器/对话框不可用）降级为文件框 */
  async function handleAddFiles(): Promise<void> {
    try {
      const picked = await openDialog({ multiple: true });
      if (!picked) return;
      addPaths(Array.isArray(picked) ? picked : [picked]);
    } catch {
      fileInput?.click();
    }
  }

  /** 隐藏文件框：浏览器降级入口（桌面端对话框失败时也兜底） */
  let fileInput: HTMLInputElement | null = $state(null);

  /** 文件框变更：取 File 列表走降级载入（可重复选同一文件，变更后清空 value） */
  function handleFileInput(event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLInputElement)) return;
    const files = [...(target.files ?? [])];
    target.value = "";
    if (files.length === 0) return;
    addBrowserFiles(files);
  }

  /** 表头全选切换：只动可见行，被筛选隐藏的行保持原勾选 */
  function handleToggleAll(checked: boolean): void {
    items = setCheckedForIds(items, new Set(visibleItems.map((item) => item.id)), checked);
  }

  /** 单行勾选切换 */
  function handleToggleOne(id: string, checked: boolean): void {
    items = setCheckedForIds(items, new Set([id]), checked);
  }

  /** 删除选中行 */
  function handleRemoveSelected(): void {
    items = removeChecked(items);
    invalidateSummary();
  }

  /** 移除单行 */
  function handleRemoveOne(id: string): void {
    items = items.filter((item) => item.id !== id);
    invalidateSummary();
  }

  /** 添加规则：工厂默认值追到列表末尾 */
  function handleAddRule(kind: RenamerRuleKind): void {
    rules = [...rules, createRule(kind)];
    invalidateSummary();
  }

  /** 清空规则：直接清空（与文件侧删除选中一致，无确认） */
  function handleClearRules(): void {
    rules = [];
    invalidateSummary();
  }

  /** 单条规则变更：整体替换 */
  function handleRuleChange(next: RenamerRule): void {
    rules = updateRule(rules, next);
    invalidateSummary();
  }

  /** 删除单条规则 */
  function handleRemoveRule(id: string): void {
    rules = removeRule(rules, id);
    invalidateSummary();
  }

  /** 规则移序：拖放松手提交的目标下标 */
  function handleMoveRuleTo(id: string, toIndex: number): void {
    rules = moveRuleTo(rules, id, toIndex);
    invalidateSummary();
  }

  /** 跳过原因转行徽章短文本 */
  function skipReasonText(skipped: RenameOutcome["skipped"]): string {
    return skipped === "exists" ? m.tool_renamer_skip_exists() : m.tool_renamer_skip_unchanged();
  }

  /** 结算落盘：成功更新路径三件套（id/path/name），跳过/失败常驻行徽章 */
  function applyRenameOutcomes(outcomes: RenameOutcome[]): { ok: number; skipped: number } {
    const byPath = new Map(outcomes.map((outcome) => [outcome.path, outcome]));
    let okCount = 0;
    let skippedCount = 0;
    items = items.map((item) => {
      const outcome = byPath.get(item.path);
      // 浏览器项与未改名项未进后端，保持原样
      if (item.path === "" || !outcome) return item;
      if (outcome.ok && outcome.new_path) {
        okCount += 1;
        const name = basenameOf(outcome.new_path);
        return {
          ...item,
          id: outcome.new_path,
          path: outcome.new_path,
          name,
          newName: name,
          error: null,
        };
      }
      if (outcome.skipped) {
        skippedCount += 1;
        const short = skipReasonText(outcome.skipped);
        return { ...item, error: { short, detail: short } };
      }
      const detail = outcome.error ?? m.tool_renamer_request_failed();
      return { ...item, error: { short: m.tool_renamer_request_failed(), detail } };
    });
    return { ok: okCount, skipped: skippedCount };
  }

  /**
   * 开始批量改名：单次命令整批执行（改名是元数据操作，无需逐张进度），`finally` 保底复位。 浏览器项跳过并 toast 提示；传输失败
   * toast 且不上汇总条。
   */
  async function handleStart(): Promise<void> {
    if (processing || !isDesktop || executableItems.length === 0) return;
    if (browserCount > 0) toast.info(m.tool_renamer_browser_skipped({ count: browserCount }));
    processing = true;
    try {
      const payload = executableItems.map((item) => ({ path: item.path, new_name: item.newName }));
      let transportError: string | null = null;
      const result = await commands
        .renameFiles(payload)
        .failed((failure) => {
          transportError =
            failure instanceof Error ? `${failure.name}: ${failure.message}` : String(failure);
        })
        .result();
      if (result.status === "error" || transportError !== null) {
        toast.error(m.tool_renamer_request_failed());
        return;
      }
      const { ok, skipped } = applyRenameOutcomes(result.data);
      const failed = result.data.length - ok - skipped;
      summary = m.tool_renamer_done({ ok, skipped, failed });
      toast.info(summary);
    } finally {
      processing = false;
    }
  }

  /** 是否为文件拖拽：文本选中拖拽不过滤，避免 overlay 误显（同数据互转） */
  function hasFiles(event: DragEvent): boolean {
    return event.dataTransfer?.types.includes("Files") ?? false;
  }

  onMount(() => {
    const cleanups: (() => void)[] = [];

    // Tauri 拖放事件（桌面端主路径）：路径直接入列表
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
            addPaths(payload.paths);
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
      if (files.length > 0) addBrowserFiles(files);
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

<!-- 左右可调两栏：左规则 35 + 右文件 65（初始比例，不持久化） -->
<div class={cn("relative h-full w-full")}>
  <ResizablePaneGroup direction="horizontal">
    <ResizablePane defaultSize={35} minSize={25}>
      <div class={cn("flex h-full min-h-0 flex-col bg-background")}>
        <RulePanel
          {rules}
          onAddRule={handleAddRule}
          onClearRules={handleClearRules}
          onRuleChange={handleRuleChange}
          onRemoveRule={handleRemoveRule}
          onMoveRuleTo={handleMoveRuleTo}
        />
      </div>
    </ResizablePane>
    <ResizableHandle withHandle />
    <ResizablePane defaultSize={65} minSize={40}>
      <div class={cn("flex h-full min-h-0 flex-col bg-background")}>
        <FileToolbar
          {headerChecked}
          {headerDisabled}
          {query}
          {sortKey}
          {selectedCount}
          onToggleAll={handleToggleAll}
          onQueryChange={(next) => (query = next)}
          onSortChange={(next) => (sortKey = next)}
          onRemoveSelected={handleRemoveSelected}
        />
        <FileList
          items={visibleItems}
          onToggleOne={handleToggleOne}
          onRemoveOne={handleRemoveOne}
          onAdd={() => void handleAddFiles()}
        />
        {#if items.length > 0}
          <RenameFooter
            executableCount={executableItems.length}
            {processing}
            {startDisabled}
            {startTitle}
            {summary}
            onStart={() => void handleStart()}
          />
        {/if}
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
      <p class={cn("text-sm font-medium text-muted-foreground")}>{m.tool_renamer_drop_hint()}</p>
    </div>
  {/if}
</div>
<!-- 隐藏文件框：浏览器降级与对话框失败兜底共用 -->
<input type="file" multiple class="hidden" bind:this={fileInput} onchange={handleFileInput} />
