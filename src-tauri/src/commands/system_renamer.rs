//! 批量重命名命令（工具路由 `system/renamer`）：前端算好新名，后端只做存在性/
//! 合法性二次校验与 `fs::rename` 执行，逻辑在 `features`。

use crate::cores::types::{CommandError, CommandResult};
use crate::features::system_renamer::{RenameItem, RenameOutcome};

/// 批量改名：一次调用返回整批逐项结果（顺序与入参一致），单项跳过/失败不中断其余。
/// 阻塞文件操作经 `spawn_blocking` 隔离，不饿死异步运行时。
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub async fn rename_files(items: Vec<RenameItem>) -> CommandResult<Vec<RenameOutcome>> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::features::system_renamer::rename_files(&items)
    })
    .await
    .map_err(|err| CommandError::Internal(format!("rename task failed: {err}")))
}
