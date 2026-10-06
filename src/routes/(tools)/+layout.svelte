<script lang="ts">
  // (tools) 分组布局：仅工具详情页走独立标题栏（左返回、中工具名、右窗口控制），绕开主布局容器。
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import ArrowLeftIcon from "@lucide/svelte/icons/arrow-left";
  import type { Snippet } from "svelte";
  import WindowButtons from "$components/layout/parts/window-buttons.svelte";
  import { Button } from "$components/shadcn-svelte/button";
  import { resolveTool } from "$libs/tools/registry";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";

  let { children }: { children: Snippet } = $props();

  const appName = __APP_TAURI_CONF__.productName;

  // 按路径查注册表取工具名，未命中回落应用名，深链直达未知工具不白屏
  const toolName = $derived(resolveTool(page.url.pathname)?.name() ?? appName);

  // 返回工具列表：深链冷启动无历史栈，不用 history.back，行为确定可单测
  function handleBack(): void {
    void goto(resolve("/tools"));
  }
</script>

<div class={cn("flex h-screen w-full flex-col")}>
  <header class={cn("border-b")}>
    <div class={cn("flex h-10 w-full items-center justify-between bg-background select-none")}>
      <div class={cn("flex h-full items-center pt-1 pl-1")}>
        <Button
          variant="ghost"
          size="icon-sm"
          onclick={handleBack}
          aria-label={m.page_tools_back()}
          title={m.page_tools_back()}
        >
          <ArrowLeftIcon />
        </Button>
      </div>
      <div
        class={cn("flex h-full w-full flex-1 items-center justify-center pt-1")}
        data-tauri-drag-region
      >
        <span class={cn("truncate text-sm font-medium")} data-tauri-drag-region>{toolName}</span>
      </div>
      <WindowButtons />
    </div>
  </header>
  <main class={cn("min-h-0 flex-1")}>
    {@render children()}
  </main>
</div>
