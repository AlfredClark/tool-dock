//! 批量重命名业务逻辑（工具路由 `system/renamer`）：按前端算好的新名执行改名。
//!
//! 规则引擎只在前端（`renamer-rules.ts`），此处不重实现正则语义：JS 与 Rust 正则
//! 不互通（后顾、命名组替换串写法只一边有），前端直传新名即所见即所得。
//! 业务失败全部装进 [`RenameOutcome`] 返回，不抛错（跳过/失败是常态 UI 状态）；
//! 单项失败不中断其余，顺序与入参一致。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 改名输入项：`path` 为源完整路径，`new_name` 为前端预览算好的目标文件名（含扩展名）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RenameItem {
    pub path: String,
    pub new_name: String,
}

/// 跳过原因：名称未变（预览与原名一致）/ 目标已存在（永不覆盖用户文件）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum RenameSkip {
    Unchanged,
    Exists,
}

/// 单项改名结果：成功带 `new_path`，跳过带原因，失败带英文诊断（前端映射通用文案展示）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RenameOutcome {
    pub ok: bool,
    pub path: String,
    pub new_path: Option<String>,
    pub skipped: Option<RenameSkip>,
    pub error: Option<String>,
}

impl RenameOutcome {
    /// 成功：源路径与改名后的完整路径
    const fn success(path: String, new_path: String) -> Self {
        Self {
            ok: true,
            path,
            new_path: Some(new_path),
            skipped: None,
            error: None,
        }
    }

    /// 跳过：名称未变或目标已存在
    const fn skipped(path: String, reason: RenameSkip) -> Self {
        Self {
            ok: false,
            path,
            new_path: None,
            skipped: Some(reason),
            error: None,
        }
    }

    /// 失败：源缺失/非法新名/IO 错误，英文诊断供前端缀展示
    const fn failed(path: String, message: String) -> Self {
        Self {
            ok: false,
            path,
            new_path: None,
            skipped: None,
            error: Some(message),
        }
    }
}

/// 新名合法性：非空且不含路径分隔符与 NUL（前端已做规范化，此处是后端二次校验）。
/// 跨平台保守：各系统非法字符集不同，只拦一定非法的分隔符，命名品味问题不管。
fn check_new_name(new_name: &str) -> Result<(), String> {
    if new_name.is_empty() {
        return Err("empty file name".to_string());
    }
    if new_name.contains('/') || new_name.contains('\\') || new_name.contains('\0') {
        return Err(format!("invalid file name: {new_name}"));
    }
    Ok(())
}

/// 执行单项改名：源须为已存在的文件（目录不在本工具范围内）
fn rename_one(item: &RenameItem) -> RenameOutcome {
    let source = std::path::Path::new(&item.path);
    if !source.is_file() {
        return RenameOutcome::failed(item.path.clone(), "source not found".to_string());
    }
    if let Err(message) = check_new_name(&item.new_name) {
        return RenameOutcome::failed(item.path.clone(), message);
    }
    let Some(file_name) = source.file_name().and_then(|name| name.to_str()) else {
        return RenameOutcome::failed(item.path.clone(), "invalid source path".to_string());
    };
    if file_name == item.new_name {
        return RenameOutcome::skipped(item.path.clone(), RenameSkip::Unchanged);
    }
    let Some(parent) = source.parent() else {
        return RenameOutcome::failed(item.path.clone(), "invalid source path".to_string());
    };
    let target = parent.join(&item.new_name);
    if target.exists() {
        return RenameOutcome::skipped(item.path.clone(), RenameSkip::Exists);
    }
    if let Err(err) = std::fs::rename(source, &target) {
        return RenameOutcome::failed(item.path.clone(), err.to_string());
    }
    RenameOutcome::success(item.path.clone(), target.to_string_lossy().into_owned())
}

/// 批量改名：逐项结算，单项失败不中断其余，输出顺序与入参一致
pub fn rename_files(items: &[RenameItem]) -> Vec<RenameOutcome> {
    items.iter().map(rename_one).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用例独占目录：进程 id + 计数保证并行测试不互踩，跑完清理
    fn case_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tool-dock-renamer-{pid}-{name}",
            pid = std::process::id(),
            name = name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("case dir");
        dir
    }

    /// 在目录下建指定文件，返回完整路径字符串
    fn touch(dir: &std::path::Path, name: &str) -> String {
        let path = dir.join(name);
        std::fs::write(&path, "data").expect("touch");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn renames_and_reports_new_path() {
        let dir = case_dir("ok");
        let source = touch(&dir, "a.png");
        let outcomes = rename_files(&[RenameItem {
            path: source.clone(),
            new_name: "b.png".to_string(),
        }]);
        assert_eq!(outcomes.len(), 1);
        let outcome = &outcomes[0];
        assert!(outcome.ok);
        assert_eq!(
            outcome.new_path,
            Some(dir.join("b.png").to_string_lossy().into_owned())
        );
        assert!(!std::path::Path::new(&source).exists());
        assert!(dir.join("b.png").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skips_when_target_exists() {
        let dir = case_dir("exists");
        let source = touch(&dir, "a.png");
        touch(&dir, "b.png");
        let outcomes = rename_files(&[RenameItem {
            path: source.clone(),
            new_name: "b.png".to_string(),
        }]);
        assert!(!outcomes[0].ok);
        assert_eq!(outcomes[0].skipped, Some(RenameSkip::Exists));
        // 源文件原地不动
        assert!(std::path::Path::new(&source).is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skips_when_name_unchanged() {
        let dir = case_dir("unchanged");
        let source = touch(&dir, "a.png");
        let outcomes = rename_files(&[RenameItem {
            path: source,
            new_name: "a.png".to_string(),
        }]);
        assert!(!outcomes[0].ok);
        assert_eq!(outcomes[0].skipped, Some(RenameSkip::Unchanged));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fails_on_missing_source_and_invalid_name() {
        let dir = case_dir("failed");
        let outcomes = rename_files(&[
            RenameItem {
                path: dir.join("missing.png").to_string_lossy().into_owned(),
                new_name: "b.png".to_string(),
            },
            RenameItem {
                path: dir.join("x.png").to_string_lossy().into_owned(),
                new_name: "sub/b.png".to_string(),
            },
            RenameItem {
                path: dir.join("y.png").to_string_lossy().into_owned(),
                new_name: String::new(),
            },
        ]);
        assert_eq!(outcomes.len(), 3);
        // 整批继续，无抛错；三项全失败且带诊断
        for outcome in &outcomes {
            assert!(!outcome.ok);
            assert!(outcome.skipped.is_none());
            assert!(outcome.error.as_deref().is_some());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn batch_continues_after_failure() {
        let dir = case_dir("batch");
        let source = touch(&dir, "a.png");
        let outcomes = rename_files(&[
            RenameItem {
                path: dir.join("missing.png").to_string_lossy().into_owned(),
                new_name: "x.png".to_string(),
            },
            RenameItem {
                path: source,
                new_name: "b.png".to_string(),
            },
        ]);
        assert!(!outcomes[0].ok);
        assert!(outcomes[1].ok);
        assert!(dir.join("b.png").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
