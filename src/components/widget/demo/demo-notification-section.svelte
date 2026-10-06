<script lang="ts">
  // 演示：系统通知分组，一键发送固定文案通知，标题正文取 i18n 保证双语，零 props。
  import { Button } from "$components/shadcn-svelte/button";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 发送进行中时禁用按钮，避免重复通知刷屏 */
  let busy = $state(false);

  /** 发送固定文案的本地通知，Linux 无通知守护时走失败 toast */
  async function handleSend(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoNotify(m.demo_notification_sample_title(), m.demo_notification_sample_body())
        .failed(() => {
          toast.error(m.demo_notification_send_failed());
        })
        .result();
      if (result.status === "ok") toast.success(m.demo_notification_send_success());
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.demo_notification_title()} description={m.demo_notification_description()}>
  <CardRow
    label={m.demo_notification_send_label()}
    description={m.demo_notification_sample_title()}
  >
    {#snippet control()}
      <Button size="sm" onclick={() => void handleSend()} disabled={busy}>
        {m.demo_notification_send_button()}
      </Button>
    {/snippet}
  </CardRow>
</CardSection>
