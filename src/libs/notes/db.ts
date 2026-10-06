// 笔记数据访问：经 `plugin-sql` 直连 `notes.db` 的最小 CRUD，全参数化防注入。
// 连接单例模块级缓存；建表由后端迁移保证，此处不写 DDL。
import type { Note } from "$libs/notes/types";

/** 数据库连接串：与后端迁移注册的 `NOTES_DB_URL` 同值，改名两边同步 */
const NOTES_DB_URL = "sqlite:notes.db";

/** 懒单例：首调加载连接，后续复用；HMR 重载模块即重建，无悬空连接 */
let dbPromise: Promise<import("@tauri-apps/plugin-sql").default> | null = null;

/** 取共享连接；非桌面环境 import 失败直接抛错，由调用方 toast */
async function getDb(): Promise<import("@tauri-apps/plugin-sql").default> {
  if (!dbPromise) {
    dbPromise = (async () => {
      const { default: Database } = await import("@tauri-apps/plugin-sql");
      return Database.load(NOTES_DB_URL);
    })();
    // 加载失败不清缓存标记位：失败的 promise 常驻，后续调用同错，避免半开连接
    dbPromise.catch(() => {
      dbPromise = null;
    });
  }
  return dbPromise;
}

/** UTC ISO 时间戳：业务时间全链路 UTC，展示层转本地 */
function nowIso(): string {
  return new Date().toISOString();
}

/** 单页条数：列表分页读取，避免几百条一次全取 */
export const NOTES_PAGE_SIZE = 50;

/** 分页列出笔记，按更新时间倒序；默认首屏一页 */
export async function listNotes(limit: number = NOTES_PAGE_SIZE, offset = 0): Promise<Note[]> {
  const db = await getDb();
  return db.select<Note[]>(
    "SELECT id, title, body, created_at, updated_at FROM notes ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
    [limit, offset],
  );
}

/** 新增笔记，返回写后全行（含自增 `id` 与时间戳） */
export async function createNote(title: string, body: string): Promise<Note> {
  const db = await getDb();
  const now = nowIso();
  const rows = await db.select<Note[]>(
    "INSERT INTO notes (title, body, created_at, updated_at) VALUES ($1, $2, $3, $3) RETURNING id, title, body, created_at, updated_at",
    [title, body, now],
  );
  const created = rows[0];
  if (!created) throw new Error("create note returned no row");
  return created;
}

/** 按 `id` 全量更新标题正文并刷新 `updated_at`，返回影响行数 */
export async function updateNote(id: number, title: string, body: string): Promise<number> {
  const db = await getDb();
  const result = await db.execute(
    "UPDATE notes SET title = $1, body = $2, updated_at = $3 WHERE id = $4",
    [title, body, nowIso(), id],
  );
  return result.rowsAffected ?? 0;
}

/** 按 `id` 删除，返回影响行数（0 即目标不存在，调用方按幂等成功处理） */
export async function deleteNote(id: number): Promise<number> {
  const db = await getDb();
  const result = await db.execute("DELETE FROM notes WHERE id = $1", [id]);
  return result.rowsAffected ?? 0;
}
