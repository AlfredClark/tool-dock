//! 结构化存储插件初始化：经 `@tauri-apps/plugin-sql` 注册 `SQLite` 能力。
//!
//! 当前无示例表，业务表由派生项目按迁移规范追加（新增迁移禁止改已发布版本）；
//! 数据面无 Rust 薄封装——插件不暴露 Rust 查询 API，前端经插件 JS API 直调。

use tauri::{Runtime, plugin::Plugin};
use tauri_plugin_sql::Builder;

/// 结构化存储插件：桌面 / 移动通用，故在 `lib.rs` 直接注册，无需 `BuilderExt` 空实现。
pub fn init<R: Runtime>() -> impl Plugin<R> {
    Builder::default().build()
}
