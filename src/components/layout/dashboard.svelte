<script lang="ts">
  // 仪表盘布局：顶部标题栏 + 左侧固定导航 + 内容区 + 底边，适合页面多的后台型应用。
  // 与 sidebar 的差异：侧栏无折叠状态机，收起/展开由底部按钮控制的局部状态，窄屏自动收为图标栏。
  import type { Snippet } from "svelte";
  import { mergeProps } from "bits-ui";
  import PanelLeftCloseIcon from "@lucide/svelte/icons/panel-left-close";
  import PanelLeftOpenIcon from "@lucide/svelte/icons/panel-left-open";
  import { getSharedIsMobile } from "$hooks/is-mobile.svelte";
  import { buttonVariants } from "$components/shadcn-svelte/button";
  import * as Tooltip from "$components/shadcn-svelte/tooltip/index.js";
  import Copyright from "$components/layout/parts/copyright.svelte";
  import DashboardNavBar from "$components/layout/parts/dashboard-nav-bar.svelte";
  import TitleBar from "$components/layout/parts/title-bar.svelte";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";

  let { children }: { children: Snippet } = $props();

  // 窄屏强制图标栏；桌面端由底部按钮控制，会话级状态，切换布局重挂载后回默认展开。
  // 断点监听复用共享单例，同断点多处订阅不断重复注册。
  const isMobile = getSharedIsMobile();
  let collapsed = $state(false);
  const compact = $derived(isMobile.current || collapsed);
</script>

<div data-layout="dashboard" class={cn("flex h-screen w-full flex-col overflow-hidden")}>
  <header class={cn("w-full shrink-0 border-b")}>
    <TitleBar />
  </header>
  <div class={cn("flex min-h-0 w-full flex-1 flex-row overflow-hidden")}>
    <aside
      class={cn(
        "flex shrink-0 flex-col overflow-x-hidden overflow-y-auto border-r transition-[width] duration-200",
        compact ? "w-14" : "w-52",
      )}
    >
      <Tooltip.Provider delayDuration={0}>
        <DashboardNavBar {compact} />
        {#if !isMobile.current}
          <div class={cn("mt-auto shrink-0 p-2")}>
            <Tooltip.Root>
              <Tooltip.Trigger>
                {#snippet child({ props })}
                  {@const mergedProps = mergeProps(
                    {
                      class: cn(
                        buttonVariants({ variant: "ghost" }),
                        "w-full justify-start gap-2",
                        collapsed && "justify-center px-0",
                      ),
                      // 不写 aria-expanded：ghost 变体会给展开态加选中底色，与“中性开关”语义冲突
                      "aria-label": collapsed
                        ? m.dashboard_toggle_expand()
                        : m.dashboard_toggle_collapse(),
                      onclick: () => {
                        collapsed = !collapsed;
                      },
                    },
                    props,
                  )}
                  <button {...mergedProps}>
                    {#if collapsed}
                      <PanelLeftOpenIcon />
                    {:else}
                      <PanelLeftCloseIcon />
                    {/if}
                    {#if !collapsed}
                      <span class={cn("truncate")}>{m.dashboard_toggle_collapse()}</span>
                    {/if}
                  </button>
                {/snippet}
              </Tooltip.Trigger>
              {#if collapsed}
                <Tooltip.Content side="right">
                  {m.dashboard_toggle_expand()}
                </Tooltip.Content>
              {/if}
            </Tooltip.Root>
          </div>
        {/if}
      </Tooltip.Provider>
    </aside>
    <main class={cn("flex min-h-0 flex-1 flex-col overflow-y-auto")}>
      {@render children()}
    </main>
  </div>
  <footer class={cn("flex w-full shrink-0 items-center justify-center border-t")}>
    <Copyright class={cn("m-1 text-xs text-muted-foreground select-none")} />
  </footer>
</div>
