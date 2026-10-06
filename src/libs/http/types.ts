// 网络层共享类型：错误统一形状与时间接口返回值，前端网络出口的唯一契约。
// 调用方只认这两个类型，不触碰 `Response` 与第三方插件细节。

/** 请求失败归因：超时 / 网络层 / 非 2xx / 响应体非法 */
export type HttpErrorKind = "timeout" | "network" | "http" | "decode";

/** 归一化后的请求错误：`instanceof` 可辨，`status` 仅 `http` 分支携带 */
export class HttpError extends Error {
  readonly kind: HttpErrorKind;
  readonly status?: number;

  constructor(kind: HttpErrorKind, message: string, status?: number) {
    super(message);
    this.name = "HttpError";
    this.kind = kind;
    this.status = status;
  }
}

/** Timeapi.io 当前时间返回值（展示用子集，服务端多给的字段直接忽略） */
export interface TimeInfo {
  dateTime: string;
  date: string;
  time: string;
  timeZone: string;
  dayOfWeek: string;
}
