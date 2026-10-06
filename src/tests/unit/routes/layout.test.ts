import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import commands from "$libs/commands";
import type { AnyResult, Config_Serialize } from "$libs/commands/types";
import { __resetAlignForTests, configState, hydrateAndAlignLocale } from "$hooks/config.svelte";
import { getLocale, setLocale } from "$libs/i18n/paraglide/runtime";

vi.mock("$libs/commands", () => ({
  default: { getConfig: vi.fn() },
}));

vi.mock("$libs/i18n/paraglide/runtime", () => ({
  getLocale: vi.fn(),
  setLocale: vi.fn(),
}));

const saved: Config_Serialize = {
  locale: "zh-CN",
  auto_start: true,
  remember_window: true,
  auto_check_update: true,
  tray_enabled: true,
  close_behavior: "prompt",
  schema_version: 1,
};

type FakeChain = {
  success: (handler: (data: never) => unknown) => FakeChain;
  failed: (handler: (failure: never) => unknown) => FakeChain;
  then: (resolve: (value: AnyResult) => unknown) => Promise<unknown>;
};

/** 与 `EnhancedCommand` 成功分支一致的替身：先跑回调，再结算结果 */
function fakeOkCommand(data: Config_Serialize): FakeChain {
  const onSuccess: ((data: never) => unknown)[] = [];
  const chain: FakeChain = {
    success(handler) {
      onSuccess.push(handler);
      return chain;
    },
    failed() {
      return chain;
    },
    then(resolve) {
      for (const handler of onSuccess) {
        handler(data as never);
      }
      const result: AnyResult = { status: "ok", data };
      return Promise.resolve(result).then(resolve);
    },
  };
  return chain;
}

beforeEach(() => {
  vi.clearAllMocks();
  configState.value = null;
  __resetAlignForTests();
  vi.mocked(commands.getConfig).mockReturnValue(fakeOkCommand(saved) as never);
  vi.mocked(getLocale).mockReturnValue("en");
  // 失败上报走 console，直写的白名单调用在此统一静音
  vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("启动语言对齐", () => {
  it("水合成功且语言不一致时对齐且不重载", async () => {
    await hydrateAndAlignLocale();

    expect(commands.getConfig).toHaveBeenCalledOnce();
    expect(configState.value).toEqual(saved);
    expect(setLocale).toHaveBeenCalledOnce();
    expect(setLocale).toHaveBeenCalledWith("zh-CN", { reload: false });
  });

  it("语言已一致时不调用对齐", async () => {
    vi.mocked(getLocale).mockReturnValue("zh-CN");

    await hydrateAndAlignLocale();

    expect(setLocale).not.toHaveBeenCalled();
  });

  it("命令构造期抛错时上报且不阻断", async () => {
    vi.mocked(commands.getConfig).mockImplementation(() => {
      throw new Error("boom");
    });

    await expect(hydrateAndAlignLocale()).resolves.toBeUndefined();

    expect(setLocale).not.toHaveBeenCalled();
  });

  it("重复调用只跑一次", async () => {
    await hydrateAndAlignLocale();
    await hydrateAndAlignLocale();

    expect(commands.getConfig).toHaveBeenCalledOnce();
  });
});
