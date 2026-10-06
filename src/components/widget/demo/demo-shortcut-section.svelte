<script lang="ts">
  // 演示：全局快捷键分组，固定键经后端命令（注册 / 查询 / 注销），自定义键与关闭窗口键经
  // 前端直调插件 JS API；移动端经命令报错或插件缺失降级为不可用，零 props。
  import { Button } from "$components/shadcn-svelte/button";
  import { Input } from "$components/shadcn-svelte/input";
  import CardRow from "$components/common/card-row.svelte";
  import CardSection from "$components/common/card-section.svelte";
  import commands from "$libs/commands";
  import {
    CLOSE_WINDOW_SHORTCUT,
    isShortcutRegistered,
    registerShortcut,
    unregisterShortcut,
  } from "$libs/shortcuts/shortcuts";
  import { normalizeShortcut, validateShortcut } from "$libs/shortcuts/validate";
  import { m } from "$libs/i18n/paraglide/messages";
  import { closeWindow } from "$libs/utils/window-controls";
  import { toast } from "$libs/utils/toast";

  /** 固定快捷键展示；取失败时留空，行内保持占位 */
  let shortcutKey = $state<string | null>(null);
  /** 注册态；`null` 为查询中，移动端恒 `false` 且附不可用说明 */
  let registered = $state<boolean | null>(null);
  /** 移动端无快捷键能力，命令报错即锁定为不可用展示 */
  let unavailable = $state(false);
  /** 操作进行中时禁用两按钮 */
  let busy = $state(false);

  /** 状态行文案：不可用优先，其次按注册态，查询中占位 */
  const statusText = $derived(
    unavailable
      ? m.demo_shortcut_unavailable()
      : registered === null
        ? "…"
        : registered
          ? m.demo_shortcut_registered()
          : m.demo_shortcut_unregistered(),
  );

  /** 自定义槽位输入原文；注册成功后收敛为归一形存档 */
  let customInput = $state("");
  /** 自定义槽位注册态；`null` 为查询中 */
  let customRegistered = $state<boolean | null>(null);
  /** 自定义槽位已注册的归一形；注销认存档不认输入框，避免改键后注错 */
  let customAccel: string | null = $state(null);
  /** 关闭窗口预设键注册态；`null` 为查询中 */
  let closeRegistered = $state<boolean | null>(null);

  /** 自定义槽位状态行文案，与固定键同口径 */
  const customStatusText = $derived(
    unavailable
      ? m.demo_shortcut_unavailable()
      : customRegistered === null
        ? "…"
        : customRegistered
          ? m.demo_shortcut_registered()
          : m.demo_shortcut_unregistered(),
  );

  // 挂载即取固定键与注册态；移动端查询命令报错则标不可用（toast 免打扰，行内说明即可）
  $effect(() => {
    void commands
      .demoShortcutKey()
      .success((key) => {
        shortcutKey = key;
      })
      .failed(() => {
        shortcutKey = null;
      });
    void commands
      .demoShortcutIsRegistered()
      .success((value) => {
        registered = value;
      })
      .failed(() => {
        unavailable = true;
        registered = false;
      });
  });

  /** 注册固定键；移动端报错转不可用展示 */
  async function handleRegister(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoShortcutRegister()
        .failed(() => {
          toast.error(m.demo_shortcut_failed());
        })
        .result();
      if (result.status === "ok") {
        registered = result.data;
        toast.success(m.demo_shortcut_register_success());
      } else {
        unavailable = true;
      }
    } finally {
      busy = false;
    }
  }

  /** 注销固定键 */
  async function handleUnregister(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      const result = await commands
        .demoShortcutUnregister()
        .failed(() => {
          toast.error(m.demo_shortcut_failed());
        })
        .result();
      if (result.status === "ok") {
        registered = false;
        toast.success(m.demo_shortcut_unregister_success());
      }
    } finally {
      busy = false;
    }
  }

  // 挂载即查询前端直调两槽位的注册态；插件缺失（移动端 / 浏览器预览）标不可用。
  // 切页不注销：与后端固定键一致，回来重查即可继续管理。
  $effect(() => {
    void (async () => {
      try {
        closeRegistered = await isShortcutRegistered(CLOSE_WINDOW_SHORTCUT);
      } catch {
        unavailable = true;
        closeRegistered = false;
      }
      if (customAccel) {
        try {
          customRegistered = await isShortcutRegistered(customAccel);
        } catch {
          unavailable = true;
          customRegistered = false;
        }
      } else {
        customRegistered = false;
      }
    })();
  });

  /** 注册自定义槽位：先过格式与保留键校验，通过才调插件 */
  async function handleCustomRegister(): Promise<void> {
    if (busy) return;
    const invalid = validateShortcut(customInput);
    if (invalid) {
      toast.error(m[invalid]());
      return;
    }
    const accel = normalizeShortcut(customInput);
    busy = true;
    try {
      await registerShortcut(accel, () => {
        toast.success(m.demo_shortcut_triggered());
      });
      customAccel = accel;
      customRegistered = true;
      toast.success(m.demo_shortcut_register_success());
    } catch {
      toast.error(m.demo_shortcut_failed());
    } finally {
      busy = false;
    }
  }

  /** 注销自定义槽位：认存档归一形，不认输入框当前值 */
  async function handleCustomUnregister(): Promise<void> {
    if (busy || !customAccel) return;
    busy = true;
    try {
      await unregisterShortcut(customAccel);
      customRegistered = false;
      toast.success(m.demo_shortcut_unregister_success());
    } catch {
      toast.error(m.demo_shortcut_failed());
    } finally {
      busy = false;
    }
  }

  /** 关闭窗口预设键切换：注册即按之关窗（走根布局关闭确认分流），注销即停用 */
  async function handleCloseToggle(): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      if (closeRegistered) {
        await unregisterShortcut(CLOSE_WINDOW_SHORTCUT);
        closeRegistered = false;
        toast.success(m.demo_shortcut_unregister_success());
      } else {
        await registerShortcut(CLOSE_WINDOW_SHORTCUT, () => {
          void closeWindow();
        });
        closeRegistered = true;
        toast.success(m.demo_shortcut_register_success());
      }
    } catch {
      toast.error(m.demo_shortcut_failed());
    } finally {
      busy = false;
    }
  }
</script>

<CardSection title={m.demo_shortcut_title()} description={m.demo_shortcut_description()}>
  <CardRow label={m.demo_shortcut_key_label()} description={shortcutKey ?? "…"} />

  <CardRow label={m.demo_shortcut_status_label()} description={statusText}>
    {#snippet control()}
      <div class="flex items-center gap-2">
        <Button
          size="sm"
          onclick={() => void handleRegister()}
          disabled={busy || unavailable || registered === true}
        >
          {m.demo_shortcut_register_button()}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onclick={() => void handleUnregister()}
          disabled={busy || unavailable || registered !== true}
        >
          {m.demo_shortcut_unregister_button()}
        </Button>
      </div>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_shortcut_custom_label()} description={m.demo_shortcut_custom_hint()}>
    {#snippet control()}
      <div class="flex items-center gap-2">
        <Input
          bind:value={customInput}
          maxlength={32}
          placeholder={m.demo_shortcut_custom_placeholder()}
          class="w-44"
        />
      </div>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_shortcut_status_label()} description={customStatusText}>
    {#snippet control()}
      <div class="flex items-center gap-2">
        <Button
          size="sm"
          onclick={() => void handleCustomRegister()}
          disabled={busy || unavailable || customRegistered === true}
        >
          {m.demo_shortcut_register_button()}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onclick={() => void handleCustomUnregister()}
          disabled={busy || unavailable || customRegistered !== true}
        >
          {m.demo_shortcut_unregister_button()}
        </Button>
      </div>
    {/snippet}
  </CardRow>

  <CardRow label={m.demo_shortcut_close_label()} description={CLOSE_WINDOW_SHORTCUT}>
    {#snippet control()}
      <Button
        size="sm"
        variant={closeRegistered ? "secondary" : "default"}
        onclick={() => void handleCloseToggle()}
        disabled={busy || unavailable || closeRegistered === null}
      >
        {closeRegistered ? m.demo_shortcut_unregister_button() : m.demo_shortcut_register_button()}
      </Button>
    {/snippet}
  </CardRow>
</CardSection>
