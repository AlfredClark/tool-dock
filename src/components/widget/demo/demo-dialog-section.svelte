<script lang="ts">
  // 演示：系统对话框分组，选文件 / 选目录 / 存文件三行，结果行内回显，取消是正常分支，零 props。
  import { Button } from "$components/shadcn-svelte/button";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 三行回显：选中为路径，取消为取消文案，失败留空（toast 已提示） */
  let pickedFile = $state<string | null>(null);
  let pickedFolder = $state<string | null>(null);
  let savePath = $state<string | null>(null);
  /** 对话框阻塞等待用户，期间禁用三按钮 */
  let busy = $state(false);

  /** 用户取消返回 None 是正常分支，行内展示取消文案而非报错 */
  function describeSelection(selected: string | null): string {
    return selected ?? m.demo_dialog_cancelled();
  }

  /** 弹系统选文件框 */
  async function handlePickFile(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoPickFile()
        .failed(() => {
          toast.error(m.demo_dialog_failed());
        })
        .result();
      if (result.status === "ok") pickedFile = describeSelection(result.data);
    } finally {
      busy = false;
    }
  }

  /** 弹系统选目录框 */
  async function handlePickFolder(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoPickFolder()
        .failed(() => {
          toast.error(m.demo_dialog_failed());
        })
        .result();
      if (result.status === "ok") pickedFolder = describeSelection(result.data);
    } finally {
      busy = false;
    }
  }

  /** 弹系统存文件框（只取路径不写盘） */
  async function handleSaveFile(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoSaveFile()
        .failed(() => {
          toast.error(m.demo_dialog_failed());
        })
        .result();
      if (result.status === "ok") savePath = describeSelection(result.data);
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.demo_dialog_title()} description={m.demo_dialog_description()}>
  <CardRow label={m.demo_dialog_pick_file_label()} description={pickedFile ?? "…"}>
    {#snippet control()}
      <Button size="sm" onclick={() => void handlePickFile()} disabled={busy}>
        {m.demo_dialog_pick_file_button()}
      </Button>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_dialog_pick_folder_label()} description={pickedFolder ?? "…"}>
    {#snippet control()}
      <Button size="sm" variant="secondary" onclick={() => void handlePickFolder()} disabled={busy}>
        {m.demo_dialog_pick_folder_button()}
      </Button>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_dialog_save_file_label()} description={savePath ?? "…"}>
    {#snippet control()}
      <Button size="sm" variant="secondary" onclick={() => void handleSaveFile()} disabled={busy}>
        {m.demo_dialog_save_file_button()}
      </Button>
    {/snippet}
  </CardRow>
</CardSection>
