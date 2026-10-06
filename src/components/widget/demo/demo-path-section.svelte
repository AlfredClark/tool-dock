<script lang="ts">
  // 演示：应用目录分组，挂载即经命令解析三处目录，失败 toast 后行内保持占位，零 props。
  import type { DemoAppPaths } from "$libs/commands/types";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import { reportCommandFailure } from "$libs/commands/cores";
  import { m } from "$libs/i18n/paraglide/messages";
  import { toast } from "$libs/utils/toast";

  /** 三处应用目录；加载失败留空，行内回落占位 */
  let paths = $state<DemoAppPaths | null>(null);

  // 挂载即拉一次目录，失败 toast 后行内保持占位
  $effect(() => {
    void commands
      .demoAppPaths()
      .success((data) => {
        paths = data;
      })
      .failed((failure) => {
        reportCommandFailure("[demo] failed to load app paths", failure);
        toast.error(m.demo_path_failed());
      });
  });
</script>

<CardSection title={m.demo_path_title()} description={m.demo_path_description()}>
  <CardRow
    label={m.demo_path_app_data_label()}
    description={paths?.app_data ?? m.demo_path_loading()}
  />

  <CardRow
    label={m.demo_path_app_cache_label()}
    description={paths?.app_cache ?? m.demo_path_loading()}
  />

  <CardRow label={m.demo_path_temp_label()} description={paths?.temp ?? m.demo_path_loading()} />
</CardSection>
