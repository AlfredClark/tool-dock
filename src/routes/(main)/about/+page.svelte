<script lang="ts">
  // 关于页组装：滚动壳 + 标题 + 应用/项目/平台/诊断四分组，无业务逻辑。
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import AppInfoSection from "$components/widget/about/app-info-section.svelte";
  import DiagnosticsSection from "$components/widget/about/diagnostics-section.svelte";
  import PlatformInfoSection from "$components/widget/about/platform-info-section.svelte";
  import ProjectLinksSection from "$components/widget/about/project-links-section.svelte";
  import { m } from "$libs/i18n/paraglide/messages";
</script>

<!-- 页面级纵向滚动：内容超高时在视口内滚动；内层保持水平居中、垂直置顶 -->
<ScrollArea class="h-full w-full">
  <div class="mx-auto flex w-full max-w-2xl flex-col gap-6 px-4 py-6">
    <h1 class="text-2xl font-medium">{m.page_about_title()}</h1>

    <AppInfoSection />
    <ProjectLinksSection />
    <PlatformInfoSection />
    <DiagnosticsSection />
  </div>
</ScrollArea>
