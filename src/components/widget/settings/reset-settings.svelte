<script lang="ts">
  // 恢复默认设置：全宽 destructive 按钮 + 确认弹窗；先重置前端外观与颜色模式，再重置后端配置。
  // 语言回落由后端重置结果驱动：重置前后 locale 不一致时走 Paraglide 默认重载生效。
  import { setMode } from "mode-watcher";
  import { resetAppearance } from "$hooks/appearance.svelte";
  import { configState, resetConfig } from "$hooks/config.svelte";
  import ConfirmDialog from "$components/common/confirm-dialog.svelte";
  import { Button } from "$components/shadcn-svelte/button";
  import { m } from "$libs/i18n/paraglide/messages";
  import { getLocale, setLocale } from "$libs/i18n/paraglide/runtime";
  import { toast } from "$libs/utils/toast";

  /** 确认弹窗开关 */
  let open = $state(false);
  /** 重置在途时禁用按钮，避免重复提交 */
  let busy = $state(false);

  /** 确认恢复：弹窗已由通用组件关闭；前端三件套即时生效，后端失败仅 toast（前端已重置的不回滚） */
  async function handleConfirm(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const before = configState.value?.locale ?? getLocale();
      resetAppearance();
      setMode("system");
      const after = await resetConfig();
      if (after === null) {
        toast.error(m.settings_reset_failed());
        return;
      }
      toast.success(m.settings_reset_success());
      if (after.locale !== before) setLocale(after.locale);
    } finally {
      busy = false;
    }
  }
</script>

<Button variant="destructive" class="w-full" disabled={busy} onclick={() => (open = true)}>
  {m.settings_reset_button()}
</Button>

<ConfirmDialog
  bind:open
  title={m.settings_reset_confirm_title()}
  description={m.settings_reset_confirm_description()}
  cancelLabel={m.settings_close_confirm_cancel()}
  confirmLabel={m.settings_reset_confirm_ok()}
  onConfirm={() => void handleConfirm()}
/>
