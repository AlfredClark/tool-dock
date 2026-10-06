//! 命令层：每个命令都是轻量的 `#[tauri::command]` 封装，业务逻辑放在 `features`，
//! 通用能力放在 `cores`，这里只做参数校验与结果转换。
//!
//! 本层按前端调用域分组（配置 / 演示 / 系统 / 更新），不与 `features` 一一对应：
//! 需 `Tauri` 运行时的命令委托给 `cores`，纯函数命令才委托给 `features`。

#![allow(clippy::unnecessary_wraps)]

pub mod config;
pub mod demo;
pub mod system;
pub mod updater;

/// 汇总全部命令：既用于 `specta` 生成前端绑定，也用于挂载 `invoke_handler`。
///
/// 未在此注册的命令不会出现在绑定里，前端也就调用不到。
macro_rules! collect_commands {
    () => {
        tauri_specta::collect_commands![
            $crate::commands::config::get_config,
            $crate::commands::config::reset_config,
            $crate::commands::config::update_config,
            $crate::commands::demo::greet,
            $crate::commands::demo::demo_app_paths,
            $crate::commands::demo::demo_write_file,
            $crate::commands::demo::demo_read_file,
            $crate::commands::demo::demo_pick_file,
            $crate::commands::demo::demo_pick_folder,
            $crate::commands::demo::demo_save_file,
            $crate::commands::demo::demo_clipboard_write,
            $crate::commands::demo::demo_clipboard_read,
            $crate::commands::demo::demo_notify,
            $crate::commands::demo::demo_shortcut_key,
            $crate::commands::demo::demo_shortcut_register,
            $crate::commands::demo::demo_shortcut_is_registered,
            $crate::commands::demo::demo_shortcut_unregister,
            $crate::commands::demo::demo_inspect_drop,
            $crate::commands::demo::demo_import_drop,
            $crate::commands::system::get_system_info,
            $crate::commands::system::quit_app,
            $crate::commands::system::open_log_dir,
            $crate::commands::system::open_config_dir,
            $crate::commands::system::copy_system_info,
            $crate::commands::updater::check_update,
            $crate::commands::updater::download_and_install_update,
            $crate::commands::updater::restart_app,
        ]
    };
}

pub(crate) use collect_commands;
