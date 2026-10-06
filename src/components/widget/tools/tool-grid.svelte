<script lang="ts">
  // 工具网格：按分类分组渲染工具卡片，点击跳转对应工具路由，对外零 props。
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { Card, CardDescription, CardHeader, CardTitle } from "$components/shadcn-svelte/card";
  import {
    TOOL_CATEGORIES,
    TOOLS,
    categoryLabel,
    type ToolCategory,
    type ToolEntry,
  } from "$libs/tools/registry";

  /** 按分类分组：分类元组即渲染顺序，新增分类自动多出一组 */
  const toolsByCategory = $derived(
    TOOL_CATEGORIES.map((category: ToolCategory) => ({
      category,
      tools: TOOLS.filter((tool) => tool.category === category),
    })).filter((group) => group.tools.length > 0),
  );

  /** 点击卡片跳转工具路由 */
  function handleOpen(tool: ToolEntry): void {
    void goto(resolve(tool.path));
  }
</script>

<div class="flex flex-col gap-6">
  {#each toolsByCategory as group (group.category)}
    <section aria-label={categoryLabel(group.category)}>
      <h2 class="mb-3 text-lg font-medium">{categoryLabel(group.category)}</h2>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
        {#each group.tools as tool (tool.id)}
          {@const Icon = tool.icon}
          <Card>
            <button
              type="button"
              class="block w-full cursor-pointer text-left"
              onclick={() => handleOpen(tool)}
              aria-label={tool.name()}
            >
              <CardHeader class="flex items-center gap-4">
                <Icon class="size-10 shrink-0 opacity-80" />
                <div class="flex min-w-0 flex-col gap-1">
                  <CardTitle class="text-base leading-tight">{tool.name()}</CardTitle>
                  <CardDescription class="truncate" title={tool.description()}>
                    {tool.description()}
                  </CardDescription>
                </div>
              </CardHeader>
            </button>
          </Card>
        {/each}
      </div>
    </section>
  {/each}
</div>
