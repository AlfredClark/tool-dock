<script lang="ts">
  // 演示：沙盒文件分组，固定读写 demo.txt，结果行内回显、成败 toast，零 props。
  import { Button } from "$components/shadcn-svelte/button";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 演示文件名：前后端约定固定值，不开放输入，避免路径校验分支散落到演示页 */
  const DEMO_FILENAME = "demo.txt";

  /** 写操作回显：成功为绝对路径，失败留空（toast 已提示） */
  let writtenPath = $state<string | null>(null);
  /** 读操作回显：成功为文件内容，失败留空（toast 已提示） */
  let fileContent = $state<string | null>(null);
  /** 任一读写进行中时禁用两按钮，避免并发交错 */
  let busy = $state(false);

  /** 写固定示例文本到沙盒，成功行内回显绝对路径 */
  async function handleWrite(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoWriteFile(DEMO_FILENAME, m.demo_fs_sample_content())
        .failed(() => {
          toast.error(m.demo_fs_write_failed());
        })
        .result();
      if (result.status === "ok") {
        writtenPath = result.data;
        toast.success(m.demo_fs_write_success());
      }
    } finally {
      busy = false;
    }
  }

  /** 读沙盒文件，成功行内回显内容 */
  async function handleRead(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoReadFile(DEMO_FILENAME)
        .failed(() => {
          toast.error(m.demo_fs_read_failed());
        })
        .result();
      if (result.status === "ok") {
        fileContent = result.data;
        toast.success(m.demo_fs_read_success());
      }
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.demo_fs_title()} description={m.demo_fs_description()}>
  <CardRow label={m.demo_fs_write_label()} description={writtenPath ?? DEMO_FILENAME}>
    {#snippet control()}
      <Button size="sm" onclick={() => void handleWrite()} disabled={busy}>
        {m.demo_fs_write_button()}
      </Button>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_fs_read_label()} description={fileContent ?? DEMO_FILENAME}>
    {#snippet control()}
      <Button size="sm" variant="secondary" onclick={() => void handleRead()} disabled={busy}>
        {m.demo_fs_read_button()}
      </Button>
    {/snippet}
  </CardRow>
</CardSection>
