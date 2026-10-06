<script lang="ts">
  // 侧边栏导航标签列表：标签清单复用 $libs/navigation/nav-tabs.ts，与顶部标签栏同源，此处只负责渲染与路由跳转。
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import {
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
  } from "$components/shadcn-svelte/sidebar";
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

<SidebarMenu class={cn("gap-1 px-2")}>
  {#each NAV_TABS as tab (tab.path)}
    {@const Icon = tab.icon}
    <SidebarMenuItem>
      <SidebarMenuButton
        isActive={tab.path === activePath}
        tooltipContent={tab.label()}
        onclick={() => handleNavigate(tab.path)}
      >
        <Icon />
        <span>{tab.label()}</span>
      </SidebarMenuButton>
    </SidebarMenuItem>
  {/each}
</SidebarMenu>
