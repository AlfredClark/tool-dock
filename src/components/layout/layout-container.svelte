<script lang="ts">
  // 布局容器：按注册表动态渲染，新增布局只需加文件并扩展映射，无需改动本文件。
  // 错误兜底由根布局的全局错误边界负责，此处不再嵌套，避免双重捕获。
  // 非默认布局懒加载：pending 期间继续渲染旧布局，settled 才翻转，不白屏不闪烁。
  import type { Snippet } from "svelte";
  import type { LayoutComponent, LayoutName } from "$hooks/appearance.svelte";
  import { LAYOUTS, initLayout, layoutState } from "$hooks/appearance.svelte";

  let { children }: { children: Snippet } = $props();

  // 已 settle 的布局与取值：请求态不翻转；脏数据回落 tabs（hooks 层已回落，此处防注册表缺 key）。
  let Layout = $state<LayoutComponent | undefined>(undefined);
  let shownName = $state<LayoutName>("tabs");

  // 只写不读：init 内仅读 localStorage（非响应式），该 effect 仅首帧跑一次，无需 untrack
  $effect.pre(() => {
    initLayout();
  });

  $effect(() => {
    const name = layoutState.name;
    const loader = LAYOUTS[name] ?? LAYOUTS.tabs;
    let cancelled = false;
    void loader().then((component) => {
      if (cancelled) {
        return;
      }
      Layout = component;
      shownName = name in LAYOUTS ? name : "tabs";
    });
    return () => {
      cancelled = true;
    };
  });
</script>

{#key shownName}
  {#if Layout}
    <Layout>
      {@render children()}
    </Layout>
  {/if}
{/key}
