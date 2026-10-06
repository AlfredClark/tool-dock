<script lang="ts">
  // 设置页组装：滚动壳 + 标题 + 通用/外观两分组 + 底部恢复默认按钮，无业务逻辑。
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import AppearanceSettings from "$components/widget/settings/appearance-settings.svelte";
  import GeneralSettings from "$components/widget/settings/general-settings.svelte";
  import ResetSettings from "$components/widget/settings/reset-settings.svelte";
  import { m } from "$libs/i18n/paraglide/messages";
</script>

<!-- 页面级纵向滚动：内容超高时在视口内滚动；内层保持水平居中、垂直置顶 -->
<ScrollArea class="h-full w-full">
  <div class="mx-auto flex w-full max-w-2xl flex-col gap-6 px-4 py-6">
    <h1 class="text-2xl font-medium">{m.page_settings_title()}</h1>

    <GeneralSettings />
    <AppearanceSettings />
    <ResetSettings />
  </div>
</ScrollArea>
