<script lang="ts">
  // 演示：剪贴板分组，输入框写纯文本、按钮读回显，空剪贴板按占位展示，零 props。
  import { Button } from "$components/shadcn-svelte/button";
  import { Input } from "$components/shadcn-svelte/input";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 待写入剪贴板的输入文本，页面私有状态 */
  let draft = $state("");
  /** 读回显：成功为剪贴板内容，失败留空（toast 已提示） */
  let pasted = $state<string | null>(null);
  /** 读写进行中时禁用输入与按钮 */
  let busy = $state(false);

  /** 写输入框文本到系统剪贴板，成功清空输入框 */
  async function handleWrite(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoClipboardWrite(draft)
        .failed(() => {
          toast.error(m.demo_clipboard_write_failed());
        })
        .result();
      if (result.status === "ok") {
        draft = "";
        toast.success(m.demo_clipboard_write_success());
      }
    } finally {
      busy = false;
    }
  }

  /** 读系统剪贴板，空内容按占位展示而非报错 */
  async function handleRead(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoClipboardRead()
        .failed(() => {
          toast.error(m.demo_clipboard_read_failed());
        })
        .result();
      if (result.status === "ok")
        pasted = result.data === "" ? m.demo_clipboard_empty() : result.data;
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.demo_clipboard_title()} description={m.demo_clipboard_description()}>
  <CardRow
    label={m.demo_clipboard_write_label()}
    description={m.demo_clipboard_input_placeholder()}
  >
    {#snippet control()}
      <div class="flex items-center gap-2">
        <Input
          class="w-44"
          value={draft}
          disabled={busy}
          placeholder={m.demo_clipboard_input_placeholder()}
          aria-label={m.demo_clipboard_write_label()}
          oninput={(event) => {
            draft = event.currentTarget.value;
          }}
        />
        <Button size="sm" onclick={() => void handleWrite()} disabled={busy || draft === ""}>
          {m.demo_clipboard_write_button()}
        </Button>
      </div>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_clipboard_read_label()} description={pasted ?? "…"}>
    {#snippet control()}
      <Button size="sm" variant="secondary" onclick={() => void handleRead()} disabled={busy}>
        {m.demo_clipboard_read_button()}
      </Button>
    {/snippet}
  </CardRow>
</CardSection>
