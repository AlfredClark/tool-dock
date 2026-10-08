//! 业务逻辑层：以纯函数实现，不依赖 Tauri 运行时，便于单独测试。
//! 命令层只负责参数校验与结果转换，真正干活的是这里。
//! 工具类模块按 `<工具分类>_<工具名>` 命名，与工具路由一一对应（如 `text/convert` → `text_convert`）。
pub mod image_resize;
pub mod text_convert;
pub mod video_metadata;
