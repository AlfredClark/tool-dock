//! 示例业务：问候语纯函数，不依赖 Tauri 运行时。
//!
//! 演示页（`routes/(main)/demo`）的输入校验也收敛于此：文件名、文本长度、
//! 通知长度与演示快捷键，命令层只做薄封装，删除演示页时本文件增量部分一并删除。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use rust_i18n::t;

/// 演示沙盒目录名：读写固定落在应用数据目录下的该子目录，
/// 与 capability 的 `$APPDATA/demo/*` 域保持一致，改名需两处同步
pub const DEMO_DIR_NAME: &str = "demo";
/// 演示用全局快捷键：固定键，不开放任意注册入口
pub const DEMO_SHORTCUT: &str = "Ctrl+Shift+D";
/// 演示文本长度上限：文件与剪贴板共用，超限直接拒绝，避免误贴大文本卡死命令线程
pub const MAX_DEMO_TEXT_LEN: usize = 8192;
/// 通知标题 / 正文长度上限：各平台通知中心对超长文本截断策略不一，此处先行拒绝
pub const MAX_NOTIFY_TITLE_LEN: usize = 64;
/// 通知标题 / 正文长度上限：各平台通知中心对超长文本截断策略不一，此处先行拒绝
pub const MAX_NOTIFY_BODY_LEN: usize = 256;
/// 演示文件名长度上限：`demo.txt` 固定名本用不上，供派生项目放开自定义文件名时复用
pub const MAX_DEMO_FILE_NAME_LEN: usize = 64;
/// 单次拖放文件数量上限：超限整批拒绝，避免误拖整个目录卡死命令线程
pub const MAX_DROP_FILES: usize = 10;
/// 拖放单文件大小上限（字节）：超限整批拒绝，落盘前逐个再验
pub const MAX_DROP_FILE_SIZE: u64 = 5 * 1024 * 1024;

/// 问候文案：`name` 为 `"123"` 时返回业务错误，用于演示前端 `.failed()` 分支与自动失败上报。
///
/// 后端文案与前端 `Paraglide` 是两套独立文案（演示用，可删除），英文错误文本不直接展示给用户，
/// 前端收到 `{ kind, message }` 后按通用失败提示处理。
pub fn greet(name: &str) -> anyhow::Result<String> {
    if name == "123" {
        anyhow::bail!("Invalid argument");
    }
    Ok(t!("greet", name = name).to_string())
}

/// 校验演示文件名：去首尾空白后非空、无路径分隔符、无 `..`、长度受限；
/// 返回规整后的文件名，调用方直接拼到沙盒目录下，不做二次拼接校验
pub fn validate_demo_filename(raw: &str) -> anyhow::Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        anyhow::bail!("file name must not be empty");
    }
    if name.len() > MAX_DEMO_FILE_NAME_LEN {
        anyhow::bail!("file name is too long");
    }
    if name.contains(['/', '\\']) || name.split('.').any(|part| part == "..") || name == "." {
        anyhow::bail!("file name must stay inside the demo folder");
    }
    Ok(name.to_owned())
}

/// 校验演示文本长度：文件与剪贴板共用上限，超限拒绝
pub fn validate_demo_text(text: &str) -> anyhow::Result<()> {
    if text.len() > MAX_DEMO_TEXT_LEN {
        anyhow::bail!("text exceeds {MAX_DEMO_TEXT_LEN} bytes");
    }
    Ok(())
}

/// 校验通知标题与正文：非空且各自受限，空通知在部分平台直接静默丢弃，提前拒绝更明确
pub fn validate_notify(title: &str, body: &str) -> anyhow::Result<()> {
    if title.trim().is_empty() || body.trim().is_empty() {
        anyhow::bail!("notification title and body must not be empty");
    }
    if title.len() > MAX_NOTIFY_TITLE_LEN || body.len() > MAX_NOTIFY_BODY_LEN {
        anyhow::bail!("notification text is too long");
    }
    Ok(())
}

/// 校验拖放路径批：非空且数量受限；内容级校验（存在性 / 类型 / 大小）
/// 由调用方逐项 `symlink_metadata` 后判定，此处只做批次门禁
pub fn validate_drop_paths(paths: &[String]) -> anyhow::Result<()> {
    if paths.is_empty() {
        anyhow::bail!("drop must contain at least one path");
    }
    if paths.len() > MAX_DROP_FILES {
        anyhow::bail!("drop exceeds {MAX_DROP_FILES} files");
    }
    Ok(())
}

/// 装配沙盒文件路径：`base/demo/<规整文件名>`。
///
/// 后端命令的授权收敛点（capability 的 `fs` 域只约束 JS 直接调用，Rust 侧调用不走 IPC，
/// 故此处做路径收敛断言）：文件名已保证无分隔符，拼接后仍校验末段一致，
/// 防止未来放开校验规则时静默越界到沙盒之外
pub fn resolve_demo_file(base: &Path, filename: &str) -> anyhow::Result<PathBuf> {
    let name = validate_demo_filename(filename)?;
    let path = base.join(DEMO_DIR_NAME).join(&name);
    let contained = path
        .file_name()
        .is_some_and(|leaf| leaf == OsStr::new(&name));
    if contained {
        Ok(path)
    } else {
        anyhow::bail!("file name must stay inside the demo folder");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_reserved_name() {
        assert!(greet("123").is_err());
    }

    #[test]
    fn renders_greeting_with_name() {
        assert!(greet("alice").is_ok_and(|text| text.contains("alice")));
    }

    #[test]
    fn accepts_plain_demo_filename() {
        assert_eq!(
            validate_demo_filename("demo.txt").expect("plain name"),
            "demo.txt"
        );
        // 首尾空白视为误输入，直接规整而非拒绝
        assert_eq!(
            validate_demo_filename("  demo.txt  ").expect("trimmed"),
            "demo.txt"
        );
    }

    #[test]
    fn rejects_traversal_filename() {
        for raw in [
            "",
            "   ",
            "../demo.txt",
            "a/b.txt",
            "a\\b.txt",
            ".",
            "x/../y",
        ] {
            assert!(validate_demo_filename(raw).is_err(), "must reject {raw:?}");
        }
        let too_long = "a".repeat(MAX_DEMO_FILE_NAME_LEN + 1);
        assert!(validate_demo_filename(&too_long).is_err());
    }

    #[test]
    fn rejects_oversized_demo_text() {
        assert!(validate_demo_text("hello").is_ok());
        let oversized = "x".repeat(MAX_DEMO_TEXT_LEN + 1);
        assert!(validate_demo_text(&oversized).is_err());
    }

    #[test]
    fn rejects_bad_notification_text() {
        assert!(validate_notify("title", "body").is_ok());
        assert!(validate_notify("", "body").is_err());
        assert!(validate_notify("title", "  ").is_err());
        let long_title = "t".repeat(MAX_NOTIFY_TITLE_LEN + 1);
        assert!(validate_notify(&long_title, "body").is_err());
        let long_body = "b".repeat(MAX_NOTIFY_BODY_LEN + 1);
        assert!(validate_notify("title", &long_body).is_err());
    }

    #[test]
    fn resolves_demo_file_inside_sandbox() {
        let base = Path::new("/data/app");
        let path = resolve_demo_file(base, "demo.txt").expect("resolves");
        assert_eq!(path, base.join(DEMO_DIR_NAME).join("demo.txt"));
        // 装配结果必须收敛在沙盒内，前缀断言即门禁语义
        assert!(path.starts_with(base.join(DEMO_DIR_NAME)));
    }

    #[test]
    fn rejects_demo_file_outside_sandbox() {
        let base = Path::new("/data/app");
        for raw in ["../evil.txt", "sub/evil.txt", "", "."] {
            assert!(resolve_demo_file(base, raw).is_err(), "must reject {raw:?}");
        }
    }

    #[test]
    fn rejects_empty_or_oversized_drop() {
        assert!(validate_drop_paths(&[]).is_err());
        assert!(validate_drop_paths(&["a.txt".to_owned()]).is_ok());
        let oversized = (0..=MAX_DROP_FILES)
            .map(|i| format!("{i}.txt"))
            .collect::<Vec<_>>();
        assert!(validate_drop_paths(&oversized).is_err());
    }
}
