<script lang="ts">
  // FFmpeg 管理页（右栏标签页）：状态展示 + 路径 + 重新检查 + 下载/修复重装。
  // 纯展示 + 回调转发：下载进度与结果状态由 workspace 持有，两份确认弹窗自管开关。
  import RefreshCwIcon from "@lucide/svelte/icons/refresh-cw";
  import ConfirmDialog from "$components/common/confirm-dialog.svelte";
  import { Button } from "$components/shadcn-svelte/button";
  import { Progress } from "$components/shadcn-svelte/progress";
  import type { FfmpegStatus } from "$libs/commands/bindings";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";

  interface Props {
    /** 引擎状态：查询中为 `null` */
    status: FfmpegStatus | null;
    /** 任一进行中（查询/下载/重装）：锁全部按钮 */
    busy: boolean;
    /** 下载或重装进行中：展示进度条 */
    downloading: boolean;
    /** 进度行文本：进行中才有值 */
    progressText: string | null;
    /** 进度条百分比：总数未知时为 0（不确定态） */
    progressValue: number;
    /** 失败原因：失败时展示 */
    error: string | null;
    /** 重新查询状态回调 */
    onRefresh: () => void;
    /** 确认后执行下载回调 */
    onEnsure: () => void;
    /** 确认后执行修复重装回调 */
    onRepair: () => void;
  }

  let {
    status,
    busy,
    downloading,
    progressText,
    progressValue,
    error,
    onRefresh,
    onEnsure,
    onRepair,
  }: Props = $props();

  /** 下载确认与重装确认各自开关 */
  let ensureOpen = $state(false);
  let repairOpen = $state(false);

  /** 来源文案：未知（查询中）不展示 */
  const originLabel = $derived(
    status?.origin === "managed"
      ? m.tool_ffmpeg_managed()
      : status?.origin === "system"
        ? m.tool_ffmpeg_system()
        : m.tool_ffmpeg_missing(),
  );
</script>

<div class={cn("flex flex-col gap-3")}>
  <div class={cn("flex items-center justify-between gap-2")}>
    <span class={cn("text-xs font-medium text-muted-foreground")}>
      {m.tool_ffmpeg_title()}
      {#if status}
        · {originLabel}
      {/if}
    </span>
    <Button variant="ghost" size="sm" disabled={busy} onclick={onRefresh}>
      <RefreshCwIcon />
      {m.tool_ffmpeg_refresh()}
    </Button>
  </div>

  {#if status?.available}
    <div class={cn("flex flex-col gap-1 text-sm")}>
      <span class={cn("tabular-nums")}>
        {m.tool_ffmpeg_ffmpeg_version({ version: status.ffmpeg_version ?? "?" })}
      </span>
      <span class={cn("tabular-nums")}>
        {m.tool_ffmpeg_ffprobe_version({ version: status.ffprobe_version ?? "?" })}
      </span>
      {#if status.ffmpeg_path}
        <span class={cn("text-xs text-muted-foreground")}>
          {m.tool_ffmpeg_binary_path()}
        </span>
        <span
          class={cn("truncate text-xs text-muted-foreground tabular-nums")}
          title={status.ffmpeg_path}
        >
          {status.ffmpeg_path}
        </span>
      {/if}
    </div>
    <Button variant="outline" disabled={busy} onclick={() => (repairOpen = true)} class="w-full">
      {m.tool_ffmpeg_repair()}
    </Button>
  {:else if !downloading}
    <Button disabled={busy} onclick={() => (ensureOpen = true)} class="w-full">
      {m.tool_ffmpeg_download()}
    </Button>
  {/if}

  {#if downloading}
    <Progress value={progressValue} />
    {#if progressText}
      <p class={cn("text-xs text-muted-foreground tabular-nums")}>{progressText}</p>
    {/if}
  {/if}

  {#if error}
    <p class={cn("text-xs text-destructive")}>{error}</p>
  {/if}

  <ConfirmDialog
    bind:open={ensureOpen}
    title={m.tool_ffmpeg_download_confirm_title()}
    description={m.tool_ffmpeg_download_confirm_description()}
    cancelLabel={m.tool_ffmpeg_download_cancel()}
    confirmLabel={m.tool_ffmpeg_download_ok()}
    onConfirm={onEnsure}
  />
  <ConfirmDialog
    bind:open={repairOpen}
    title={m.tool_ffmpeg_repair_confirm_title()}
    description={m.tool_ffmpeg_repair_confirm_description()}
    cancelLabel={m.tool_ffmpeg_download_cancel()}
    confirmLabel={m.tool_ffmpeg_download_ok()}
    onConfirm={onRepair}
  />
</div>
