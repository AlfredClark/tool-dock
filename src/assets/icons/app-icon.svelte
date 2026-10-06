<script lang="ts">
  // ToolDock 标识：内联单色 mark（共用竖笔的斜体 T+D 连体，全矩形笔划），
  // `currentColor` 跟随文字颜色，明暗主题自适应；纯装饰，`aria-hidden` 不进入可访问名称。
  import { cn } from "$libs/utils/shadcn-svelte";

  let { class: className = "" }: { class?: string } = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 512 512"
  fill="currentColor"
  aria-hidden="true"
  class={cn("size-10", className)}
>
  <g transform="translate(40 0) skewX(-14)">
    <polygon points="221.2,102.3 290.8,70.4 290.8,441.6 221.2,441.6" />
    <rect x="58.8" y="128.4" width="232" height="63.8" />
    <rect x="290.8" y="174.8" width="174" height="63.8" />
    <rect x="401" y="174.8" width="63.8" height="243.6" />
    <rect x="290.8" y="354.6" width="174" height="63.8" />
  </g>
</svg>
