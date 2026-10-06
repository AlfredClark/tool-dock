<script lang="ts">
  // 演示：文件拖放分组，窗口级拖放订阅 → 后端鉴别回显 → 存入沙盒，零 props。
  // 订阅失败（浏览器预览 / 单测无 Tauri 运行时）即标不可用；移动端无拖放能力，整卡隐藏。
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import type { DropFileInfo } from "$libs/commands/types";
  import { getSharedIsMobile } from "$hooks/is-mobile.svelte";
  import { Button } from "$components/shadcn-svelte/button";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 移动端无拖放能力，整卡隐藏（而非展示禁用态）；断点监听复用共享单例 */
  const isMobile = getSharedIsMobile();

  /** 拖拽悬停中：高亮放置区；`leave` / `drop` 后复位 */
  let dragging = $state(false);
  /** 订阅可用性；动态导入失败（非 Tauri 环境）即标不可用 */
  let available = $state(true);
  /** 最近一次鉴别出的文件清单；`null` 为尚未拖入 */
  let files = $state<DropFileInfo[] | null>(null);
  /** 最近一次拖放路径；导入时复用，避免二次取路径 */
  let lastPaths = $state<string[]>([]);
  /** 鉴别或导入进行中时禁用按钮 */
  let busy = $state(false);

  /** 放置区提示文案：不可用优先，其次悬停态 */
  const zoneText = $derived(
    available
      ? dragging
        ? m.demo_drop_zone_active()
        : m.demo_drop_zone_idle()
      : m.demo_drop_unavailable(),
  );

  // 挂载即订阅窗口拖放事件，卸载时解绑；订阅失败直接标不可用（toast 免打扰，行内说明即可）
  $effect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    void subscribe().catch(() => {
      if (!cancelled) available = false;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };

    /** 动态导入窗口 API 并订阅；抛错由外层统一标不可用 */
    async function subscribe(): Promise<void> {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      if (cancelled) return;
      unlisten = await getCurrentWindow().onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          // over 悬停高频触发，状态未变直接短路，避免反复重算高亮
          if (dragging) return;
          dragging = true;
        } else if (payload.type === "leave") {
          dragging = false;
        } else {
          dragging = false;
          void inspectDrop(payload.paths);
        }
      });
    }
  });

  /** 调后端鉴别拖放批次，成功行内回显清单 */
  async function inspectDrop(paths: string[]): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoInspectDrop(paths)
        .failed(() => {
          toast.error(m.demo_drop_inspect_failed());
        })
        .result();
      if (result.status === "ok") {
        files = result.data;
        lastPaths = paths;
      }
    } finally {
      busy = false;
    }
  }

  /** 存最近一批拖放文件到沙盒，成功提示并清空清单（防重复导入） */
  async function handleImport(): Promise<void> {
    if (busy || lastPaths.length === 0) return;
    busy = true;
    try {
      const result = await commands
        .demoImportDrop(lastPaths)
        .failed(() => {
          toast.error(m.demo_drop_import_failed());
        })
        .result();
      if (result.status === "ok") {
        files = null;
        lastPaths = [];
        toast.success(m.demo_drop_import_success());
      }
    } finally {
      busy = false;
    }
  }

  /** 单文件展示行：目录标类型，否则展示字节数（`size` 可能为 `null` 时回落 `0`） */
  function fileLine(file: DropFileInfo): string {
    return file.is_dir ? m.demo_drop_dir_badge() : m.demo_drop_size_unit({ size: file.size ?? 0 });
  }
</script>

{#if !isMobile.current}
  <CardSection title={m.demo_drop_title()} description={m.demo_drop_description()}>
    <div
      class="flex flex-col items-center gap-3 rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground"
      class:border-primary={dragging}
      class:bg-muted={dragging}
      aria-label={m.demo_drop_title()}
    >
      <p>{zoneText}</p>
      <Button
        size="sm"
        onclick={() => void handleImport()}
        disabled={busy || lastPaths.length === 0}
      >
        {m.demo_drop_import_button()}
      </Button>
    </div>

    {#if files !== null}
      {#each files as file (file.name)}
        <CardRow label={file.name} description={fileLine(file)} />
      {/each}
    {/if}
  </CardSection>
{/if}
