//! 结构化存储插件初始化：笔记示例经 `@tauri-apps/plugin-sql` 直连 `notes.db`。
//!
//! 建表迁移随插件注册（`add_migrations`，连接首次加载时执行）；后续加表改列时
//! 追加新版本迁移，禁止改已发布迁移的 SQL（老用户库已执行过，不会重跑）。
//! 数据面无 Rust 薄封装——插件不暴露 Rust 查询 API，前端直调收敛在 `libs/notes/`。

use tauri::{Runtime, plugin::Plugin};
use tauri_plugin_sql::{Builder, Migration, MigrationKind};

/// 数据库文件名：相对应用配置目录，`notes.db` 即落在该目录下
pub const NOTES_DB_URL: &str = "sqlite:notes.db";

/// 建表迁移（v1）：标题必填限长、正文限长、UTC 时间戳，约束与前端校验同源
fn migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "create_notes_table",
        sql: "CREATE TABLE IF NOT EXISTS notes (\
            id INTEGER PRIMARY KEY AUTOINCREMENT, \
            title TEXT NOT NULL CHECK (length(title) BETWEEN 1 AND 100), \
            body TEXT NOT NULL DEFAULT '' CHECK (length(body) <= 4000), \
            created_at TEXT NOT NULL, \
            updated_at TEXT NOT NULL\
            )",
        kind: MigrationKind::Up,
    }]
}

/// 结构化存储插件：桌面 / 移动通用，故在 `lib.rs` 直接注册，无需 `BuilderExt` 空实现。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    Builder::default()
        .add_migrations(NOTES_DB_URL, migrations())
        .build()
}
