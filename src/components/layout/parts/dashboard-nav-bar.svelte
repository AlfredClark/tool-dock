<script lang="ts">
  // 仪表盘导航：与侧边导航同源渲染，但不依赖 SidebarState，可在任意容器内使用。
  // 接线复用 SidebarMenuButton 的 bits-ui 形态（原生按钮 + 合并属性 + tooltip）。
  // 调用方必须提供 Tooltip.Provider 祖先（dashboard 布局的 aside 内已提供），不可独立渲染。
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import { mergeProps } from "bits-ui";
  import { buttonVariants } from "$components/shadcn-svelte/button";
  import * as Tooltip from "$components/shadcn-svelte/tooltip/index.js";
  import { NAV_TABS, isNavTabPath, resolveNavTab } from "$libs/navigation/nav-tabs";
  import { cn } from "$libs/utils/shadcn-svelte";

  // compact 为 true 时只渲染图标：仪表盘窄屏图标栏使用，标签走 tooltip 展示
  let { compact = false }: { compact?: boolean } = $props();

  // 当前路径命中注册表才高亮，未命中的路径（如子页面）不选中任何标签
  const activePath = $derived(resolveNavTab(page.url.pathname)?.path ?? "");

  function handleNavigate(value: unknown) {
    if (isNavTabPath(value) && value !== activePath) {
      void goto(resolve(value));
    }
  }
</script>

<nav class={cn("flex flex-col gap-1 p-2")}>
  {#each NAV_TABS as tab (tab.path)}
    {@const Icon = tab.icon}
    {@const active = tab.path === activePath}
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          {@const mergedProps = mergeProps(
            {
              class: cn(
                buttonVariants({ variant: active ? "secondary" : "ghost" }),
                "w-full justify-start gap-2",
                compact && "justify-center px-0",
              ),
              "aria-current": active ? ("page" as const) : undefined,
              onclick: () => handleNavigate(tab.path),
            },
            props,
          )}
          <button {...mergedProps}>
            <Icon />
            {#if !compact}
              <span class={cn("truncate")}>{tab.label()}</span>
            {/if}
          </button>
        {/snippet}
      </Tooltip.Trigger>
      {#if compact}
        <Tooltip.Content side="right">{tab.label()}</Tooltip.Content>
      {/if}
    </Tooltip.Root>
  {/each}
</nav>
