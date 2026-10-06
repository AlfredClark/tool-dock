// 应用配置的前端内存态：后端 config.json 是唯一权威，读写一律经 commands 链。
// 首帧前水合一次，写时用命令返回的写后值回写——不为省一次 IPC 而复刻第二份读取路径，
// 否则后端启动时的探测落库、将来的托盘 / 菜单改配置都会让本地缓存变陈旧。
import commands from "$libs/commands";
import { reportCommandFailure } from "$libs/commands/cores";
import type { ConfigPatch, Config_Serialize } from "$libs/commands/types";
import { getLocale, setLocale } from "$libs/i18n/paraglide/runtime";

/** 配置的前端视图：命令返回值恒为全字段必填的 `Config_Serialize` */
export type AppConfig = Config_Serialize;

/** 跨页面共享的配置状态；首帧水合前为 `null` */
export const configState = $state<{ value: AppConfig | null }>({ value: null });

/** 首帧前水合；失败时保持未水合，由调用方决定如何回落 */
export async function hydrateConfig(): Promise<void> {
  // 存储瞬时不可用时重试一次（最多阻塞首帧约 500ms + 两次 IPC），仍失败则上报并保持未水合
  // （调用方回落显示，不阻断首帧）
  for (let attempt = 0; attempt < 2; attempt += 1) {
    let failed = false;
    await commands
      .getConfig()
      .success((config) => {
        configState.value = config;
      })
      .failed((failure) => {
        failed = true;
        reportCommandFailure(
          `[config] failed to load backend config (attempt ${attempt + 1})`,
          failure,
        );
      });
    if (!failed) return;
    if (attempt === 0) await new Promise((resolve) => setTimeout(resolve, 500));
  }
}

/** 后台水合 + 语言对齐：与首帧渲染并发，完成后把前端语言对齐到后端配置 */
let alignedOnce = false;
/** 仅测试用：重置单次守卫（模块变量无法经 `configState` 复位） */
export function __resetAlignForTests(): void {
  alignedOnce = false;
}
export async function hydrateAndAlignLocale(): Promise<void> {
  // 根布局挂载时调一次；HMR 重挂载不重复跑，避免已对齐的语言被反复重设
  if (alignedOnce) return;
  alignedOnce = true;
  try {
    await hydrateConfig();
  } catch (error) {
    // 链式 API 永不 reject，此处仅防命令构造期同步抛错导致对齐中断
    reportCommandFailure("[config] hydrate threw", error);
    return;
  }
  // 不触发整页重载，否则会出现语言闪烁；水合失败则回落 Paraglide 本地策略
  const locale = configState.value?.locale;
  if (locale && locale !== getLocale()) {
    setLocale(locale, { reload: false });
  }
}

/** 局部更新：只提交传入的字段，写后用命令返回的最新配置回写状态（失败不乐观更新） */
export async function updateConfig(patch: ConfigPatch): Promise<void> {
  await commands
    .updateConfig(patch)
    .success((config) => {
      configState.value = config;
    })
    .failed((failure) => {
      reportCommandFailure("[config] failed to update backend config", failure);
    });
}

/** 重置全部后端配置为默认值，返回写后配置；失败返回空，由调用方提示 */
export async function resetConfig(): Promise<AppConfig | null> {
  let reset: AppConfig | null = null;
  await commands
    .resetConfig()
    .success((config) => {
      configState.value = config;
      reset = config;
    })
    .failed((failure) => {
      reportCommandFailure("[config] failed to reset backend config", failure);
    });
  return reset;
}
