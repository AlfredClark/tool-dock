<script lang="ts">
  // 工具页组装：滚动壳 + 分类网格，无业务逻辑。
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import ToolGrid from "$components/widget/tools/tool-grid.svelte";
</script>

<!-- 页面级纵向滚动：内容超高时在视口内滚动；网格占满宽度，仅保留左右内边距 -->
<ScrollArea class="h-full w-full">
  <div class="flex w-full flex-col gap-6 px-8 py-6">
    <ToolGrid />
  </div>
</ScrollArea>
