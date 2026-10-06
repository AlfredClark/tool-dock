<script lang="ts">
  // 演示：网络请求分组，经 libs/http 从 timeapi.io 取设备时区当前时间并展示，零 props。
  import { Button } from "$components/shadcn-svelte/button";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import { fetchCurrentTime, resolveTimeZone } from "$libs/http/time";
  import type { TimeInfo } from "$libs/http/types";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 请求在途时禁用按钮，避免并发刷接口 */
  let busy = $state(false);
  /** 最近一次成功的时间；失败不清零，保留上次结果 */
  let current: TimeInfo | null = $state(null);
  /** 展示用时区，与请求缺省值同源（取不到回落 UTC） */
  const timeZone = resolveTimeZone();

  /** 取当前时间：成功展示，失败 toast（浏览器预览无插件运行时亦走此分支） */
  async function handleFetch(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      current = await fetchCurrentTime();
    } catch {
      toast.error(m.demo_http_failed());
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.demo_http_title()} description={m.demo_http_description()}>
  <CardRow
    label={m.demo_http_fetch_label()}
    description={`${m.demo_http_timezone_label()}：${timeZone}`}
  >
    {#snippet control()}
      <Button size="sm" onclick={() => void handleFetch()} disabled={busy}>
        {busy ? m.demo_http_fetching() : m.demo_http_fetch_button()}
      </Button>
    {/snippet}
  </CardRow>

  {#if current}
    {@const info = current}
    <CardRow label={info.timeZone} description={info.dayOfWeek}>
      {#snippet control()}
        <p class="text-sm font-medium tabular-nums">{info.date} {info.time}</p>
      {/snippet}
    </CardRow>
  {/if}
</CardSection>
