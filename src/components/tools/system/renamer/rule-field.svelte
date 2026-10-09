<script lang="ts">
  // 规则参数格：上小标签 + 下控件，双列网格的半格或独占整行由 `span` 切换。
  import type { Snippet } from "svelte";
  import { cn } from "$libs/utils/shadcn-svelte";

  interface Props {
    /** 格上标签文案 */
    label: string;
    /** 独占整行（长输入），默认半格 */
    span?: boolean;
    children: Snippet;
  }

  let { label, span = false, children }: Props = $props();
</script>

<div class={cn("flex min-w-0 flex-col gap-0.5", span && "col-span-2")}>
  <span class={cn("text-xs text-muted-foreground")}>{label}</span>
  {@render children()}
</div>
