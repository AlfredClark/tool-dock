<script lang="ts">
  // 演示页组装：滚动壳 + 标题 + 六个插件分组 + 文件拖放分组 + 网络请求分组，无业务逻辑；派生项目删除本目录即初始化。
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import DemoClipboardSection from "$components/widget/demo/demo-clipboard-section.svelte";
  import DemoDialogSection from "$components/widget/demo/demo-dialog-section.svelte";
  import DemoDropSection from "$components/widget/demo/demo-drop-section.svelte";
  import DemoFsSection from "$components/widget/demo/demo-fs-section.svelte";
  import DemoHttpSection from "$components/widget/demo/demo-http-section.svelte";
  import DemoNotificationSection from "$components/widget/demo/demo-notification-section.svelte";
  import DemoPathSection from "$components/widget/demo/demo-path-section.svelte";
  import DemoShortcutSection from "$components/widget/demo/demo-shortcut-section.svelte";
  import { m } from "$libs/i18n/paraglide/messages";
</script>

<!-- 页面级纵向滚动：内容超高时在视口内滚动；内层保持水平居中、垂直置顶 -->
<ScrollArea class="h-full w-full">
  <div class="mx-auto flex w-full max-w-2xl flex-col gap-6 px-4 py-6">
    <div>
      <h1 class="text-2xl font-medium">{m.demo_page_title()}</h1>
      <p class="mt-1 text-sm text-muted-foreground">{m.demo_page_description()}</p>
    </div>

    <DemoPathSection />
    <DemoFsSection />
    <DemoDropSection />
    <DemoDialogSection />
    <DemoClipboardSection />
    <DemoNotificationSection />
    <DemoShortcutSection />
    <DemoHttpSection />
  </div>
</ScrollArea>
