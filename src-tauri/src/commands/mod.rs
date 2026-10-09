//! 命令层：每个命令都是轻量的 `#[tauri::command]` 封装，业务逻辑放在 `features`，
//! 通用能力放在 `cores`，这里只做参数校验与结果转换。
//!
//! 本层按前端调用域分组（配置 / 最小契约 / 系统 / 更新），不与 `features` 一一对应：
//! 需 `Tauri` 运行时的命令委托给 `cores`，纯函数命令才委托给 `features`。

#![allow(clippy::unnecessary_wraps)]

pub mod config;
pub mod ffmpeg;
pub mod image_resize;
pub mod system;
pub mod system_renamer;
pub mod text_convert;
pub mod updater;
pub mod video_convert;
pub mod video_metadata;

/// 汇总全部命令：既用于 `specta` 生成前端绑定，也用于挂载 `invoke_handler`。
///
/// 未在此注册的命令不会出现在绑定里，前端也就调用不到。
macro_rules! collect_commands {
    () => {
        tauri_specta::collect_commands![
            $crate::commands::config::get_config,
            $crate::commands::config::reset_config,
            $crate::commands::config::update_config,
            $crate::commands::ffmpeg::get_ffmpeg_status,
            $crate::commands::ffmpeg::ensure_ffmpeg,
            $crate::commands::ffmpeg::reinstall_managed_ffmpeg,
            $crate::commands::video_metadata::read_video_metadata,
            $crate::commands::video_metadata::get_video_thumbnail,
            $crate::commands::video_metadata::read_video_batch,
            $crate::commands::video_metadata::apply_video_metadata,
            $crate::commands::video_metadata::expand_dropped_video_paths,
            $crate::commands::video_metadata::get_video_dir,
            $crate::commands::video_convert::convert_video_format,
            $crate::commands::system::get_system_info,
            $crate::commands::system::quit_app,
            $crate::commands::system::open_log_dir,
            $crate::commands::system::open_config_dir,
            $crate::commands::system::copy_system_info,
            $crate::commands::system_renamer::rename_files,
            $crate::commands::text_convert::convert_data,
            $crate::commands::text_convert::copy_text,
            $crate::commands::text_convert::read_text_file,
            $crate::commands::image_resize::read_image_info,
            $crate::commands::image_resize::get_image_thumbnail,
            $crate::commands::image_resize::read_image_batch,
            $crate::commands::image_resize::expand_dropped_paths,
            $crate::commands::image_resize::get_picture_dir,
            $crate::commands::image_resize::resize_image,
            $crate::commands::updater::check_update,
            $crate::commands::updater::download_and_install_update,
            $crate::commands::updater::restart_app,
        ]
    };
}

pub(crate) use collect_commands;
