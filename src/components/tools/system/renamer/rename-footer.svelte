<script lang="ts">
  // 改名执行条（右栏底部）：左汇总文本 + 右开始按钮，有文件时常驻。
  // 纯展示组件：可执行数/禁用态/文案由 workspace 计算，此处只做回调转发。
  import PlayIcon from "@lucide/svelte/icons/play";
  import { Button } from "$components/shadcn-svelte/button";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";

  interface Props {
    /** 可执行数：有本地路径且新名与原名不同（按钮文案计数来源） */
    executableCount: number;
    /** 处理中：锁按钮并切换文案 */
    processing: boolean;
    /** 开始按钮禁用：无可执行项/处理中/非桌面端 */
    startDisabled: boolean;
    /** 禁用提示：非桌面端时悬浮说明，仅桌面端可用 */
    startTitle: string | null;
    /** 上次执行汇总：处理完成后常驻，文件/规则变更即失效（`null` 不展示） */
    summary: string | null;
    /** 开始改名回调 */
    onStart: () => void;
  }

  let { executableCount, processing, startDisabled, startTitle, summary, onStart }: Props =
    $props();

  /** 开始按钮文案：处理中显示进度态，否则带可执行计数 */
  const startLabel = $derived(
    processing ? m.tool_renamer_processing() : `${m.tool_renamer_start()} · ${executableCount}`,
  );
</script>

<div class={cn("flex shrink-0 items-center justify-between gap-2 border-t px-2 py-2")}>
  <span class={cn("min-w-0 flex-1 truncate text-xs text-muted-foreground")}>
    {summary ?? ""}
  </span>
  <Button
    variant="default"
    size="sm"
    disabled={startDisabled}
    title={startTitle ?? undefined}
    onclick={onStart}
  >
    <PlayIcon />
    {startLabel}
  </Button>
</div>
