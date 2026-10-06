<script lang="ts">
  // 首页工具搜索：注册表单源过滤，选中即跳转对应工具路由；空查询不弹结果，保持首页整洁。
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import {
    Empty as CommandEmpty,
    Input as CommandInput,
    Item as CommandItem,
    List as CommandList,
    Root as CommandRoot,
  } from "$components/shadcn-svelte/command";
  import { categoryLabel, searchTools, type ToolEntry } from "$libs/tools/registry";
  import { m } from "$libs/i18n/paraglide/messages";

  let query = $state("");

  /** 过滤结果：注册表纯函数求值，空查询返回空数组 */
  const results = $derived(searchTools(query));

  /** 是否展示结果区：有查询串才展开，无查询保持首页整洁 */
  const showResults = $derived(query.trim() !== "");

  /** 选中跳转并清空搜索，回到干净首页 */
  function handleSelect(tool: ToolEntry): void {
    query = "";
    void goto(resolve(tool.path));
  }
</script>

<div class="w-full max-w-md">
  <CommandRoot shouldFilter={false} class="relative overflow-visible">
    <CommandInput
      bind:value={query}
      placeholder={m.page_home_search_placeholder()}
      aria-label={m.page_home_search_label()}
    />
    {#if showResults}
      <!-- 浮层结果：绝对定位脱离文档流，不推动首页英雄区位移；纵向滚动封顶 -->
      <div class="absolute inset-x-0 top-full z-50 mt-2 rounded-xl border bg-popover p-1 shadow-md">
        <CommandList>
          {#each results as tool (tool.id)}
            {@const Icon = tool.icon}
            <CommandItem value={tool.id} onSelect={() => handleSelect(tool)}>
              <span class="flex min-w-0 flex-1 items-center gap-1.5">
                <Icon class="size-4 shrink-0 opacity-70" />
                <span class="ml-1 min-w-0 truncate">{tool.name()}</span>
              </span>
              <span class="shrink-0 text-xs text-muted-foreground"
                >{categoryLabel(tool.category)}</span
              >
            </CommandItem>
          {:else}
            <CommandEmpty>{m.page_home_search_empty()}</CommandEmpty>
          {/each}
        </CommandList>
      </div>
    {/if}
  </CommandRoot>
</div>
