<script lang="ts">
  // 顶部导航标签栏：标签清单来自 $libs/navigation/nav-tabs.ts，此处只负责渲染与路由跳转。
  // 页面内容由 SvelteKit 路由渲染在布局的 main 中，故这里不渲染 tabs 的 content。
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { Tabs, TabsList, TabsTrigger } from "$components/shadcn-svelte/tabs";
  import { NAV_TABS, isNavTabPath, resolveNavTab } from "$libs/navigation/nav-tabs";
  import { cn } from "$libs/utils/shadcn-svelte";

  // 当前路径命中注册表才高亮，未命中的路径（如子页面）不选中任何标签
  const activePath = $derived(resolveNavTab(page.url.pathname)?.path ?? "");

  function handleNavigate(value: unknown) {
    if (isNavTabPath(value) && value !== activePath) {
      void goto(resolve(value));
    }
  }
</script>

<Tabs value={activePath} onValueChange={handleNavigate} class={cn("w-full")}>
  <TabsList variant="line">
    {#each NAV_TABS as tab (tab.path)}
      {@const Icon = tab.icon}
      <TabsTrigger value={tab.path} class={cn("gap-1.5 px-2")}>
        <Icon />
        {tab.label()}
      </TabsTrigger>
    {/each}
  </TabsList>
</Tabs>
