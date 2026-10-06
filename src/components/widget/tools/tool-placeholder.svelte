<script lang="ts">
  // 工具占位：演示路由仅显示工具名，真实功能后续在此替换；名称取注册表，不硬编码。
  import { page } from "$app/state";
  import { resolveTool } from "$libs/tools/registry";

  const appName = __APP_TAURI_CONF__.productName;
  const toolName = $derived(resolveTool(page.url.pathname)?.name() ?? appName);
</script>

<div class="flex h-full w-full flex-col items-center justify-center gap-6 p-6 text-center">
  <h1 class="text-2xl font-medium">{toolName}</h1>
</div>
