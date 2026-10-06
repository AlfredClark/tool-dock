// 笔记领域共享类型：前后端（迁移 SQL）同源，展示层只认此契约。
// 时间全链路 UTC ISO 字符串（见 6.4），展示层转本地时区。

/** 笔记行：`id` 自增主键，时间戳 UTC ISO */
export interface Note {
  id: number;
  title: string;
  body: string;
  created_at: string;
  updated_at: string;
}

/** 校验失败键：调用方按键取文案，与迁移 SQL 的 `CHECK` 约束同源 */
export type NoteValidationError =
  "notes_title_required" | "notes_title_too_long" | "notes_body_too_long";
