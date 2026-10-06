// timeapi.io 时间客户端：取设备时区当前时间，返回体经类型守卫后才交出。
// 出网域与 CSP 已在后端放行 `https://timeapi.io/*`，此处只拼业务路径。
import { httpFetch } from "$libs/http/client";
import { HttpError } from "$libs/http/types";
import type { TimeInfo } from "$libs/http/types";

/** 时间接口根路径，时区经查询参数传入 */
const TIME_API_BASE = "https://timeapi.io/api/time/current/zone";
/** 取不到设备时区时的回落，保证任何环境都有确定请求 */
const FALLBACK_TIME_ZONE = "UTC";

/** 解析设备时区；隐私模式等异常一律回落，不抛错 */
export function resolveTimeZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || FALLBACK_TIME_ZONE;
  } catch {
    return FALLBACK_TIME_ZONE;
  }
}

/** 校验服务端返回体：缺字段或类型漂移即拒收，避免脏数据进展示层 */
function isTimePayload(value: unknown): value is TimeInfo {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    typeof record["dateTime"] === "string" &&
    typeof record["date"] === "string" &&
    typeof record["time"] === "string" &&
    typeof record["timeZone"] === "string" &&
    typeof record["dayOfWeek"] === "string"
  );
}

/** 取指定时区当前时间；缺省用设备时区，失败抛 `HttpError`（超时 / 网络 / 非 2xx / 坏体） */
export async function fetchCurrentTime(timeZone?: string): Promise<TimeInfo> {
  const zone = timeZone || resolveTimeZone();
  const response = await httpFetch(`${TIME_API_BASE}?timeZone=${encodeURIComponent(zone)}`);
  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new HttpError("decode", "invalid time response body");
  }
  if (!isTimePayload(payload)) {
    throw new HttpError("decode", "unexpected time response shape");
  }
  return payload;
}
