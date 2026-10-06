<script lang="ts">
  // 首页：应用标识 + 工具搜索，保持整洁，无其它业务逻辑。
  import AppIcon from "$assets/icons/app-icon.svelte";
  import ToolSearch from "$components/widget/home/tool-search.svelte";
</script>

<div class="flex h-full w-full flex-col items-center justify-start gap-6 p-6 pt-20 text-center">
  <AppIcon class="size-24" />
  <ToolSearch />
</div>
