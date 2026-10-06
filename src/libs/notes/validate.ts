// 笔记输入校验：纯函数，不触碰运行时，DB 侧 `CHECK` 约束是第二道门。
// 上限与迁移 SQL 保持一致，改一处必须改另一处。
import type { NoteValidationError } from "$libs/notes/types";

/** 标题长度上限（字符）；与迁移 `length(title) <= 100` 同源 */
export const MAX_NOTE_TITLE_LEN = 100;
/** 正文长度上限（字符）；与迁移 `length(body) <= 4000` 同源 */
export const MAX_NOTE_BODY_LEN = 4000;

/** 校验标题正文，返回首个失败键；`null` 即合法 */
export function validateNote(title: string, body: string): NoteValidationError | null {
  if (title.trim().length === 0) return "notes_title_required";
  if (title.length > MAX_NOTE_TITLE_LEN) return "notes_title_too_long";
  if (body.length > MAX_NOTE_BODY_LEN) return "notes_body_too_long";
  return null;
}
