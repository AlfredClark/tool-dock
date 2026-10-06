<script lang="ts">
  // 笔记页组装：滚动壳 + 标题 + 笔记管理分组，无业务逻辑。
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import NotesManager from "$components/widget/notes/notes-manager.svelte";
  import { m } from "$libs/i18n/paraglide/messages";
</script>

<!-- 页面级纵向滚动：内容超高时在视口内滚动；内层保持水平居中、垂直置顶 -->
<ScrollArea class="h-full w-full">
  <div class="mx-auto flex w-full max-w-2xl flex-col gap-6 px-4 py-6">
    <div>
      <h1 class="text-2xl font-medium">{m.notes_page_title()}</h1>
      <p class="mt-1 text-sm text-muted-foreground">{m.notes_page_description()}</p>
    </div>

    <NotesManager />
  </div>
</ScrollArea>
