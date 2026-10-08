//! 数据互转命令（工具路由 `text/convert`）：`convert_data` 与 `copy_text` 薄封装，逻辑在 `features` 与 `cores`。

use crate::cores::types::CommandResult;
use crate::features::text_convert::{ConvertFormat, ConvertOptions, ConvertOutcome};

/// 转换数据格式；`from` 为空即自动识别。业务失败装进 `ConvertOutcome`，本命令几乎恒返回 `Ok`。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn convert_data(
    input: String,
    from: Option<ConvertFormat>,
    to: ConvertFormat,
    options: ConvertOptions,
) -> CommandResult<ConvertOutcome> {
    Ok(crate::features::text_convert::convert(
        &input, from, to, &options,
    ))
}

/// 复制转换结果到系统剪贴板；纯文本直写（需 `AppHandle`，故走 `cores`，与 `copy_system_info` 同模式）
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn copy_text(app: tauri::AppHandle, text: String) -> CommandResult<()> {
    Ok(crate::cores::system::copy_text(&app, &text)?)
}

/// 读取拖拽载入的文本文件；路径来自 OS 拖放事件，校验逻辑在 `features::read_text_file`
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn read_text_file(path: String) -> CommandResult<String> {
    Ok(crate::features::text_convert::read_text_file(&path)?)
}
