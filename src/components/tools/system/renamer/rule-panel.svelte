<script lang="ts">
  // 改名规则面板（左栏）：工具栏（清空 + 添加下拉）+ 规则参数卡列表 + 空态。
  // 纯展示组件：规则数组由 workspace 持有，此处只做回调转发；拖拽排序的瞬时态
  //（拖拽源 + 悬停插入位）为面板私有，不进 workspace（松手才提交一次 `onMoveRuleTo`）。
  import { ScrollArea } from "$components/shadcn-svelte/scroll-area";
  import { m } from "$libs/i18n/paraglide/messages";
  import { cn } from "$libs/utils/shadcn-svelte";
  import RuleCard from "./rule-card.svelte";
  import RuleToolbar from "./rule-toolbar.svelte";
  import type { RenamerRule, RenamerRuleKind } from "./renamer-types";

  interface Props {
    /** 规则数组：顺序即应用顺序 */
    rules: RenamerRule[];
    /** 添加规则回调（类型由下拉菜单选定） */
    onAddRule: (kind: RenamerRuleKind) => void;
    /** 清空规则回调 */
    onClearRules: () => void;
    /** 单条规则变更回调（整体替换） */
    onRuleChange: (next: RenamerRule) => void;
    /** 删除单条回调 */
    onRemoveRule: (id: string) => void;
    /** 移序回调（拖放松手时提交目标下标） */
    onMoveRuleTo: (id: string, toIndex: number) => void;
  }

  let { rules, onAddRule, onClearRules, onRuleChange, onRemoveRule, onMoveRuleTo }: Props =
    $props();

  /** 拖拽源 id：`null` 即不在拖拽中 */
  let dragId = $state<string | null>(null);

  /** 悬停目标 id + 是否在其下半区（决定插入线在上/下方） */
  let overId = $state<string | null>(null);
  let overAfter = $state(false);

  /** 清理拖拽瞬时态 */
  function clearDrag(): void {
    dragId = null;
    overId = null;
    overAfter = false;
  }

  /** 手柄拖拽开始：记录源 + 写入 data（Firefox 无 setData 不启动拖拽） */
  function handleDragStart(event: DragEvent, id: string): void {
    dragId = id;
    const transfer = event.dataTransfer;
    if (transfer) {
      transfer.setData("text/plain", id);
      transfer.effectAllowed = "move";
    }
  }

  /** 卡片悬停：非源卡片才 preventDefault（允许放置），按鼠标纵中线定插入位 */
  function handleDragOver(event: DragEvent, id: string): void {
    if (!dragId || id === dragId) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const rect = target.getBoundingClientRect();
    overId = id;
    overAfter = event.clientY > rect.top + rect.height / 2;
  }

  /** 卡片放置：按下半区换算目标下标（剔除移除后的错位），提交后清理 */
  function handleDrop(event: DragEvent, id: string): void {
    if (!dragId) return;
    // 卡片与容器两层 ondrop 都会冒泡到容器，此处截断只提交一次
    event.preventDefault();
    event.stopPropagation();
    const from = rules.findIndex((rule) => rule.id === dragId);
    let to = rules.findIndex((rule) => rule.id === id);
    if (overId === id && overAfter) to += 1;
    if (from >= 0 && from < to) to -= 1;
    const moving = dragId;
    clearDrag();
    onMoveRuleTo(moving, to);
  }

  /** 容器留白悬停：允许放置（松手即移到末尾） */
  function handleContainerDragOver(event: DragEvent): void {
    if (dragId) event.preventDefault();
  }

  /** 容器留白放置：移到末尾 */
  function handleContainerDrop(event: DragEvent): void {
    if (!dragId) return;
    event.preventDefault();
    const moving = dragId;
    clearDrag();
    onMoveRuleTo(moving, rules.length - 1);
  }
</script>

<section
  aria-label={m.tool_renamer_rule_title()}
  class={cn("flex min-h-0 min-w-0 flex-1 flex-col bg-background")}
>
  <RuleToolbar ruleCount={rules.length} onClear={onClearRules} onAdd={onAddRule} />
  <ScrollArea class="min-h-0 flex-1">
    {#if rules.length === 0}
      <p class={cn("px-4 py-8 text-center text-sm text-muted-foreground")}>
        {m.tool_renamer_rule_empty()}
      </p>
    {:else}
      <div
        role="list"
        class={cn("flex flex-col gap-1 p-1.5")}
        ondragover={handleContainerDragOver}
        ondrop={handleContainerDrop}
      >
        {#each rules as rule, index (rule.id)}
          <RuleCard
            {rule}
            {index}
            {onRuleChange}
            onRemove={onRemoveRule}
            onHandleDragStart={handleDragStart}
            onCardDragOver={handleDragOver}
            onCardDrop={handleDrop}
            onDragEnd={clearDrag}
            dropBefore={overId === rule.id && !overAfter}
            dropAfter={overId === rule.id && overAfter}
          />
        {/each}
      </div>
    {/if}
  </ScrollArea>
</section>
