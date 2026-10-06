<script lang="ts">
  // 无边框窗口的自定义标题栏：左侧应用名（取注入的 Tauri 配置），右侧窗口操作按钮。
  // 无 Tauri 运行时（浏览器预览 / 单测）时窗口操作静默失败，界面保持可用。
  import type { Snippet } from "svelte";
  import AppIcon from "$assets/icons/app-icon.svelte";
  import WindowButtons from "$components/layout/parts/window-buttons.svelte";
  import { cn } from "$libs/utils/shadcn-svelte";

  // 左区默认渲染应用图标与标题，传入 left 片段时由调用方接管（如侧边栏布局的折叠按钮）。
  let {
    title = __APP_TAURI_CONF__.app.windows[0].title,
    left,
  }: { title?: string; left?: Snippet } = $props();
</script>

<div class={cn("flex h-10 w-full items-center justify-between bg-background select-none")}>
  <div class={cn("flex h-full items-center justify-center pt-1 pl-1")}>
    {#if left}
      {@render left()}
    {:else}
      <AppIcon class={cn("ml-2 size-5")} />
      <span class={cn("ml-2 truncate text-sm font-medium")} data-tauri-drag-region>{title}</span>
    {/if}
  </div>
  <div class={cn("h-full w-full flex-1 pt-1")}>
    <div class={cn("h-full w-full flex-1")} data-tauri-drag-region></div>
  </div>
  <WindowButtons />
</div>
