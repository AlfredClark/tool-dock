<script lang="ts">
  // 平台信息分组：挂载即拉系统信息，失败 toast 后行内保持占位，零 props。
  import type { SystemInfo } from "$libs/commands/types";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { reportCommandFailure } from "$libs/commands/cores";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 运行平台信息；加载失败留空，行内回落占位 */
  let sysInfo = $state<SystemInfo | null>(null);

  // 挂载即拉一次平台信息，失败 toast 后行内保持占位
  $effect(() => {
    void commands
      .getSystemInfo()
      .success((info) => {
        sysInfo = info;
      })
      .failed((failure) => {
        reportCommandFailure("[about] failed to load system info", failure);
        toast.error(m.about_system_failed());
      });
  });
</script>

<CardSection title={m.about_platform_info_title()}>
  <CardRow label={m.about_platform()} description={sysInfo?.platform ?? "…"} />

  <CardRow label={m.about_os_version()} description={sysInfo?.os_version ?? "…"} />

  <CardRow label={m.about_arch()} description={sysInfo?.arch ?? "…"} />

  <CardRow label={m.about_hostname()} description={sysInfo?.hostname ?? "…"} />
</CardSection>
