//! 视频工具链管理：ffmpeg/ffprobe 二进制的解析、校验与按需下载。
//!
//! 需要 Tauri 运行时（`app_data_dir` 定位托管目录 + 下载进度事件推送），故放在 `cores`。
//! 纯执行逻辑（标签校验、ffprobe 解析、ffmpeg 参数组装）归 `features::video_metadata`，
//! 此处只管“二进制从哪来、能不能用”。
//! Rust 侧直接 `std::process::Command` 调二进制，不经 JS 层，故 capability 无 `shell` 项
//! （与 clipboard / single-instance 的 Rust-only 前例一致）；
//! 出网域名（`BtbN` / evermeet 及它们的重定向镜像）见 `plugins/http.rs` 的注释，
//! 下载走 Rust 侧直连，不受 `WebView` CSP 约束（与 `updater` 下载端点同类例外）。

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use tauri::{Emitter, Manager};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::cores::updater::ProgressPayload;

/// 下载进度事件名：前端工具页订阅，改名需两端同步
pub const FFMPEG_DOWNLOAD_PROGRESS_EVENT: &str = "ffmpeg-download-progress";

/// 系统 ffmpeg 最低大版本：低于此的系统版视为不可用（参数兼容性不可预测），回落托管版
pub const MIN_SYSTEM_FFMPEG_MAJOR: u32 = 6;

/// 托管根目录（`app_data_dir` 之下）：版本子目录隔离，多版本不混装
const MANAGED_ROOT: &str = "bins/video";

/// 安装完成标记：解包 + 校验 + 赋权全过才写，缺失即视为未安装完成
const INSTALL_MARKER: &str = ".ok";

/// 单个解包文件上限：300MiB，超限即坏包直接拒绝
const MAX_BINARY_BYTES: u64 = 300 * 1024 * 1024;

/// 二进制来源：托管版（已校验的固定版本）优先，系统 `PATH` 版仅作兜底
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum FfmpegOrigin {
    Managed,
    System,
}

/// 引擎状态：前端状态卡的唯一数据源；不可用时版本与来源均为 `None`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct FfmpegStatus {
    pub available: bool,
    pub origin: Option<FfmpegOrigin>,
    pub ffmpeg_version: Option<String>,
    pub ffprobe_version: Option<String>,
    /// 当前 pin 的托管版本（目录名），升级即换资产表
    pub pinned_version: String,
    /// 当前二进制路径（排障展示用；托管即托管目录下，系统即 `PATH` 命中）
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
}

/// 解析到的可用二进制：调用方直接拿路径调 `std::process`
#[derive(Debug, Clone)]
pub struct ResolvedBinaries {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub ffmpeg_version: String,
    pub ffprobe_version: String,
    pub origin: FfmpegOrigin,
}

/// 单个下载资产：URL + sha256 + 包格式 + 包内要提取的二进制名
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Asset {
    url: &'static str,
    sha256: &'static str,
    format: ArchiveFormat,
    /// 包内提供的二进制（`BtbN` 单包双二进制，evermeet 单包单二进制）
    provides: &'static [&'static str],
}

// 各平台只构造自己的包格式变体（Win/mac 用 `Zip`，Linux 用 `TarXz`），
// 单平台编译时另一变体必然“未构造”，属跨平台枚举的预期情况，不视为死代码
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveFormat {
    Zip,
    TarXz,
}

/// 平台托管方案：`BtbN`（Win/Linux 单包双二进制，n9.0 LGPL 静态）与
/// evermeet（macOS 无 `BtbN` 构建，双包各单二进制，9.0.2）；资产哈希已离线核验
struct ManagedPlan {
    version: &'static str,
    assets: &'static [Asset],
}

/// 当前平台的托管方案：未覆盖的平台返回 `None`（仅走系统兜底）
const fn managed_plan() -> Option<ManagedPlan> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Some(ManagedPlan {
            version: "9.0",
            assets: &[Asset {
                url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n9.0-latest-win64-lgpl-9.0.zip",
                sha256: "8100c1a2b25e19a5c55ed94f1b00780bb650050afdbfce442b2045eb79c12c9d",
                format: ArchiveFormat::Zip,
                provides: &["ffmpeg.exe", "ffprobe.exe"],
            }],
        });
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return Some(ManagedPlan {
            version: "9.0",
            assets: &[Asset {
                url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n9.0-latest-winarm64-lgpl-9.0.zip",
                sha256: "7ab6cf7ff71cf515e3d194240a46effb8a0fcdff00eb32f16cd70571b15dd1d0",
                format: ArchiveFormat::Zip,
                provides: &["ffmpeg.exe", "ffprobe.exe"],
            }],
        });
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return Some(ManagedPlan {
            version: "9.0",
            assets: &[Asset {
                url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n9.0-latest-linux64-lgpl-9.0.tar.xz",
                sha256: "975ce18d1147a53b7e4c9a49c0eda547d05a49301a72d42bc572175979de1b80",
                format: ArchiveFormat::TarXz,
                provides: &["ffmpeg", "ffprobe"],
            }],
        });
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return Some(ManagedPlan {
            version: "9.0",
            assets: &[Asset {
                url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n9.0-latest-linuxarm64-lgpl-9.0.tar.xz",
                sha256: "9f941c65daeb592da2469675300fbb0ee1d218811b0905a79402cd1bb476628e",
                format: ArchiveFormat::TarXz,
                provides: &["ffmpeg", "ffprobe"],
            }],
        });
    }
    // macOS 无 BtbN 构建：evermeet Intel 包经 Rosetta 运行（元数据读写短进程，性能无感）
    #[cfg(target_os = "macos")]
    {
        return Some(ManagedPlan {
            version: "9.0.2",
            assets: &[
                Asset {
                    url: "https://evermeet.cx/ffmpeg/ffmpeg-9.0.2.zip",
                    sha256: "4acc0be580f9b2788029eb7bd4d645ff87968911b0a62aeeb3940d42d54558d5",
                    format: ArchiveFormat::Zip,
                    provides: &["ffmpeg"],
                },
                Asset {
                    url: "https://evermeet.cx/ffmpeg/ffprobe-9.0.2.zip",
                    sha256: "24a9c968cd4da72d99c7245e914b921815835eb6dff01d99868031aebaf1d439",
                    format: ArchiveFormat::Zip,
                    provides: &["ffprobe"],
                },
            ],
        });
    }
    #[allow(unreachable_code)]
    None
}

/// 查询引擎状态：永不抛错，不可用即 `available: false`（前端据此展示下载入口）
pub fn query_status(app: &tauri::AppHandle) -> FfmpegStatus {
    let pinned = managed_plan().map_or_else(String::new, |plan| plan.version.to_owned());
    match resolve_binaries(app) {
        Ok(resolved) => FfmpegStatus {
            available: true,
            origin: Some(resolved.origin),
            ffmpeg_version: Some(resolved.ffmpeg_version),
            ffprobe_version: Some(resolved.ffprobe_version),
            pinned_version: pinned,
            ffmpeg_path: resolved.ffmpeg.to_str().map(str::to_string),
            ffprobe_path: resolved.ffprobe.to_str().map(str::to_string),
        },
        Err(_) => FfmpegStatus {
            available: false,
            origin: None,
            ffmpeg_version: None,
            ffprobe_version: None,
            pinned_version: pinned,
            ffmpeg_path: None,
            ffprobe_path: None,
        },
    }
}

/// 解析可用二进制：托管优先，系统兜底；两者皆无即报错（前端据此弹下载确认）
pub fn resolve_binaries(app: &tauri::AppHandle) -> anyhow::Result<ResolvedBinaries> {
    let system_dirs = system_path_dirs();
    let managed_base = managed_base_dir(app).ok();
    resolve_with(managed_base.as_deref(), &system_dirs)
}

/// 确保可用：已可用直接返回，否则下载托管版（进度经事件推送）后复查；
/// 托管损坏但系统可用时直接回落系统（可用性优先，不强制重下，来源如实展示）
pub fn ensure_binaries(app: &tauri::AppHandle) -> anyhow::Result<ResolvedBinaries> {
    if let Ok(resolved) = resolve_binaries(app) {
        return Ok(resolved);
    }
    reinstall_managed(app)
}

/// 强制重装托管版（FFmpeg 管理页修复入口）：清空托管目录后全量重装，不看系统版
pub fn reinstall_managed(app: &tauri::AppHandle) -> anyhow::Result<ResolvedBinaries> {
    let plan = managed_plan().context("managed ffmpeg is not available for this platform")?;
    let base = managed_base_dir_for(app, plan.version)?;
    install_managed(app, &plan, &base)?;
    resolve_binaries(app).context("managed ffmpeg installed but failed to probe")
}

/// 可测试的解析内核：托管目录（可注入）优先，系统目录（可注入）兜底
fn resolve_with(
    managed_base: Option<&Path>,
    system_dirs: &[PathBuf],
) -> anyhow::Result<ResolvedBinaries> {
    if let Some(base) = managed_base {
        // 托管版是我们 pin 的构建，探活成功即认，不另设版本门槛
        if marker_present(base)
            && let Some((ffmpeg_version, ffprobe_version)) = probe_pair(
                &base.join(exe_name("ffmpeg")),
                &base.join(exe_name("ffprobe")),
                None,
            )
        {
            return Ok(ResolvedBinaries {
                ffmpeg: base.join(exe_name("ffmpeg")),
                ffprobe: base.join(exe_name("ffprobe")),
                ffmpeg_version,
                ffprobe_version,
                origin: FfmpegOrigin::Managed,
            });
        }
    }
    for dir in system_dirs {
        // 系统版必须成对出自同一目录（防 ffmpeg/ffprobe 版本混搭）且过最低版本门槛
        if let Some((ffmpeg_version, ffprobe_version)) = probe_pair(
            &dir.join(exe_name("ffmpeg")),
            &dir.join(exe_name("ffprobe")),
            Some(MIN_SYSTEM_FFMPEG_MAJOR),
        ) {
            return Ok(ResolvedBinaries {
                ffmpeg: dir.join(exe_name("ffmpeg")),
                ffprobe: dir.join(exe_name("ffprobe")),
                ffmpeg_version,
                ffprobe_version,
                origin: FfmpegOrigin::System,
            });
        }
    }
    anyhow::bail!(
        "ffmpeg/ffprobe unavailable: no managed bundle and no system ffmpeg >= {MIN_SYSTEM_FFMPEG_MAJOR} in PATH"
    )
}

/// 托管目录：`{app_data}/bins/video/{version}`，`app_data` 取不到直接失败
/// （不基于回落默认值决策，与 config 的存储不可用策略一致）
fn managed_base_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let plan = managed_plan().context("managed ffmpeg is not available for this platform")?;
    managed_base_dir_for(app, plan.version)
}

fn managed_base_dir_for(app: &tauri::AppHandle, version: &str) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .context("app data dir unavailable")?;
    Ok(dir.join(MANAGED_ROOT).join(version))
}

/// 安装标记是否存在
fn marker_present(base: &Path) -> bool {
    base.join(INSTALL_MARKER).is_file()
}

/// 平台可执行文件名
fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    }
}

/// 系统 `PATH` 目录列表
fn system_path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH").map_or_else(Vec::new, |paths| std::env::split_paths(&paths).collect())
}

/// 成对探活：双文件存在 + 各自 `-version` 可执行 + 解析出版本；
/// `require_min_major` 置位时要求 ffmpeg 大版本达标（系统兜底用，托管版传 `None`）
fn probe_pair(
    ffmpeg: &Path,
    ffprobe: &Path,
    require_min_major: Option<u32>,
) -> Option<(String, String)> {
    if !ffmpeg.is_file() || !ffprobe.is_file() {
        return None;
    }
    let ffmpeg_version = probe_version(ffmpeg, "ffmpeg")?;
    let ffprobe_version = probe_version(ffprobe, "ffprobe")?;
    if let Some(min) = require_min_major
        && major_of(&ffmpeg_version).is_none_or(|major| major < min)
    {
        return None;
    }
    Some((ffmpeg_version, ffprobe_version))
}

/// 单二进制探活：跑 `-version` 取首行版本号
fn probe_version(binary: &Path, tool: &str) -> Option<String> {
    let output = std::process::Command::new(binary)
        .arg("-version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_version_output(&String::from_utf8_lossy(&output.stdout), tool)
}

/// 解析 `-version` 首行：`ffmpeg version 9.0-...` 取 `9.0`；
/// `BtbN` 会带 `n` 前缀（如 `n9.0`），先剥非数字前缀；工具名对不上返回 `None`
fn parse_version_output(text: &str, tool: &str) -> Option<String> {
    let first = text.lines().next()?;
    let mut parts = first.split_whitespace();
    if parts.next()? != tool || parts.next()? != "version" {
        return None;
    }
    let raw = parts.next()?;
    let trimmed = raw.trim_start_matches(|c: char| !c.is_ascii_digit());
    let version = trimmed.split(['-', '+']).next().filter(|v| !v.is_empty())?;
    if version.chars().next()?.is_ascii_digit() {
        Some(version.to_owned())
    } else {
        None
    }
}

/// 取版本号大版本：`9.0` → 9，非法返回 `None`
fn major_of(version: &str) -> Option<u32> {
    version.split('.').next()?.parse().ok()
}

/// 下载并安装托管版：全量重装（坏包不留）→ 逐资产下载校验解包 → 赋权 → 写标记 → 清旧版
fn install_managed(app: &tauri::AppHandle, plan: &ManagedPlan, base: &Path) -> anyhow::Result<()> {
    if base.exists() {
        std::fs::remove_dir_all(base).context("failed to clear broken managed dir")?;
    }
    std::fs::create_dir_all(base).context("failed to create managed dir")?;

    let grand_total = {
        let mut total = Some(0_u64);
        for asset in plan.assets {
            let Some(known) = content_length_of(asset) else {
                total = None;
                break;
            };
            total = total.map(|sum| sum.saturating_add(known));
        }
        total
    };
    let mut finished: u64 = 0;
    let mut throttle = ProgressEmit::new();
    for asset in plan.assets {
        let archive_path = base.join(archive_file_name(asset));
        let downloaded = download_asset(asset.url, &archive_path, &mut |got, _| {
            // 单资产 total 未知时按总盘子估算（grand 未知则事件 total 为 `None`）
            if throttle.allow() {
                emit_progress(app, finished.saturating_add(got), grand_total);
            }
        })
        .with_context(|| format!("failed to download {}", asset.url))?;
        finished = finished.saturating_add(downloaded);
        verify_sha256_file(&archive_path, asset.sha256)
            .with_context(|| format!("checksum mismatch for {}", asset.url))?;
        extract_binaries(&archive_path, asset.format, asset.provides, base)
            .with_context(|| format!("failed to extract {}", asset.url))?;
        let _ = std::fs::remove_file(&archive_path);
    }
    #[cfg(unix)]
    {
        for name in ["ffmpeg", "ffprobe"] {
            let path = base.join(name);
            if path.is_file() {
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                    .context("failed to chmod managed binary")?;
            }
        }
    }
    std::fs::write(base.join(INSTALL_MARKER), plan.version).context("failed to write marker")?;
    prune_sibling_versions(base);
    emit_progress(app, finished, grand_total.or(Some(finished)));
    Ok(())
}

/// 进度发射节流：首包即发，之后 200ms 一次（下载是持续流，不按百分比节流）
struct ProgressEmit {
    last_emit: Option<Instant>,
}

impl ProgressEmit {
    const fn new() -> Self {
        Self { last_emit: None }
    }

    fn allow(&mut self) -> bool {
        if self
            .last_emit
            .is_none_or(|at| at.elapsed() >= Duration::from_millis(200))
        {
            self.last_emit = Some(Instant::now());
            true
        } else {
            false
        }
    }
}

/// 推送下载进度：终态由命令返回值保证，不依赖末包事件（同 updater 语义）
fn emit_progress(app: &tauri::AppHandle, downloaded: u64, total: Option<u64>) {
    if let Err(err) = app.emit(
        FFMPEG_DOWNLOAD_PROGRESS_EVENT,
        ProgressPayload { downloaded, total },
    ) {
        log::debug!("failed to emit ffmpeg progress: {err:#}");
    }
}

/// 取远端包体长度：`HEAD` 失败即 `None`（进度条回落不确定态，不阻断下载）
fn content_length_of(asset: &Asset) -> Option<u64> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(15))
        .build();
    agent
        .head(asset.url)
        .call()
        .ok()?
        .header("Content-Length")?
        .parse()
        .ok()
}

/// 流式下载到文件：64KB 分块写盘，`on_chunk(累计, 总长)` 供进度节流
fn download_asset(
    url: &str,
    dest: &Path,
    on_chunk: &mut dyn FnMut(u64, Option<u64>),
) -> anyhow::Result<u64> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .timeout_read(Duration::from_secs(120))
        .build();
    let response = agent
        .get(url)
        .call()
        .with_context(|| format!("download request failed: {url}"))?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(dest).context("failed to create download file")?;
    // 64KB 缓冲走堆（栈上大数组会被 `large_stack_arrays` 拦截）
    let mut buf = vec![0_u8; 65536];
    let mut downloaded: u64 = 0;
    loop {
        let got = reader
            .read(&mut buf)
            .context("failed to read download stream")?;
        if got == 0 {
            break;
        }
        file.write_all(&buf[..got])
            .context("failed to write download file")?;
        downloaded = downloaded.saturating_add(got as u64);
        on_chunk(downloaded, total);
    }
    Ok(downloaded)
}

/// 归档文件名：取 URL 末段（BtbN/evermeet 均为稳定文件名）
fn archive_file_name(asset: &Asset) -> &str {
    asset.url.rsplit('/').next().unwrap_or("ffmpeg.pkg")
}

/// 校验文件 sha256：大小写不敏感比较
fn verify_sha256_file(path: &Path, expected: &str) -> anyhow::Result<()> {
    let mut file = std::fs::File::open(path).context("failed to open download for hashing")?;
    let mut hasher = Sha256::new();
    // 64KB 缓冲走堆（栈上大数组会被 `large_stack_arrays` 拦截）
    let mut buf = vec![0_u8; 65536];
    loop {
        let got = file.read(&mut buf).context("failed to hash download")?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        anyhow::bail!("checksum mismatch for {}", path.display())
    }
}

/// 解包并提取目标二进制：按文件名后缀匹配（BtbN 包内为 `bin/` 下，evermeet 为根目录）；
/// zip-slip 防护：拒绝绝对路径与 `..`（zip 用 `enclosed_name`，tar 逐组件检查）
fn extract_binaries(
    archive: &Path,
    format: ArchiveFormat,
    wanted: &[&str],
    out_dir: &Path,
) -> anyhow::Result<()> {
    match format {
        ArchiveFormat::Zip => extract_zip(archive, wanted, out_dir),
        ArchiveFormat::TarXz => extract_tar_xz(archive, wanted, out_dir),
    }
}

fn extract_zip(archive: &Path, wanted: &[&str], out_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive).context("failed to open zip")?;
    let mut zip = zip::ZipArchive::new(file).context("failed to read zip")?;
    let mut found = vec![false; wanted.len()];
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).context("failed to read zip entry")?;
        if entry.is_dir() {
            continue;
        }
        // `enclosed_name` 为空即路径逃逸，直接跳过（坏条目不整体失败）
        let Some(safe) = entry.enclosed_name() else {
            continue;
        };
        let Some(name) = safe.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(slot) = wanted.iter().position(|want| *want == name) else {
            continue;
        };
        if entry.size() > MAX_BINARY_BYTES {
            anyhow::bail!("zip entry too large: {name}");
        }
        let mut out =
            std::fs::File::create(out_dir.join(name)).context("failed to write binary")?;
        std::io::copy(&mut entry, &mut out).context("failed to extract binary")?;
        found[slot] = true;
    }
    missing_binaries(wanted, &found)
}

fn extract_tar_xz(archive: &Path, wanted: &[&str], out_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive).context("failed to open tar.xz")?;
    let decoder = xz2::read::XzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    let mut found = vec![false; wanted.len()];
    for entry in tar.entries().context("failed to read tar")? {
        let mut entry = entry.context("failed to read tar entry")?;
        let path = entry
            .path()
            .context("failed to read tar path")?
            .into_owned();
        // 逐组件检查：拒绝绝对路径、前缀与 `..`；`./` 前缀无害予以放行
        if !is_safe_tar_path(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(slot) = wanted.iter().position(|want| *want == name) else {
            continue;
        };
        if entry.size() > MAX_BINARY_BYTES {
            anyhow::bail!("tar entry too large: {name}");
        }
        let mut out =
            std::fs::File::create(out_dir.join(name)).context("failed to write binary")?;
        std::io::copy(&mut entry, &mut out).context("failed to extract binary")?;
        found[slot] = true;
    }
    missing_binaries(wanted, &found)
}

/// tar 条目路径是否安全：拒绝绝对路径、前缀与 `..`，`./` 前缀无害放行
fn is_safe_tar_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

/// 缺失的二进制即坏包：直报错（调用方清目录重下，不留半成品）
fn missing_binaries(wanted: &[&str], found: &[bool]) -> anyhow::Result<()> {
    let missing: Vec<&&str> = wanted
        .iter()
        .zip(found)
        .filter(|(_, hit)| !**hit)
        .map(|(name, _)| name)
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("archive missing binaries: {missing:?}")
    }
}

/// 清理同级旧版本目录：版本升级后老包不再被引用，占空间直接删（失败仅记日志）
fn prune_sibling_versions(current: &Path) {
    let Some(parent) = current.parent() else {
        return;
    };
    let current_name = current.file_name();
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir()
            && path.file_name() != current_name
            && let Err(err) = std::fs::remove_dir_all(&path)
        {
            log::debug!("failed to prune old ffmpeg dir {}: {err:#}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_event_name_is_stable() {
        // 前端工具页硬编码同一字面量，改名需两端同步
        assert_eq!(FFMPEG_DOWNLOAD_PROGRESS_EVENT, "ffmpeg-download-progress");
    }

    #[test]
    fn managed_plan_pins_known_assets() {
        // 当前平台（CI 为 linux）必须有托管方案，且资产表非空、哈希为 64 位十六进制
        let plan = managed_plan().expect("managed plan missing for test platform");
        assert_ne!(plan.version, "");
        assert_ne!(plan.assets, []);
        for asset in plan.assets {
            assert!(asset.url.starts_with("https://"));
            assert_eq!(asset.sha256.len(), 64);
            assert!(asset.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert_ne!(asset.provides.len(), 0);
        }
    }

    #[test]
    fn parses_version_lines() {
        assert_eq!(
            parse_version_output("ffmpeg version 9.0-static Copyright", "ffmpeg"),
            Some("9.0".to_owned())
        );
        assert_eq!(
            parse_version_output("ffprobe version 6.1.1 Copyright", "ffprobe"),
            Some("6.1.1".to_owned())
        );
        // BtbN 的 `n` 前缀构建号
        assert_eq!(
            parse_version_output("ffmpeg version n9.0-12-gabc Copyright", "ffmpeg"),
            Some("9.0".to_owned())
        );
        // 发行版后缀截断（展示与大版本比较只用干净版本）
        assert_eq!(
            parse_version_output("ffmpeg version 6.1.1-3+b1 Copyright", "ffmpeg"),
            Some("6.1.1".to_owned())
        );
        // 工具名对不上拒绝（防 ffmpeg/ffprobe 拿反）
        assert_eq!(
            parse_version_output("ffprobe version 9.0 Copyright", "ffmpeg"),
            None
        );
        assert_eq!(parse_version_output("garbage", "ffmpeg"), None);
        assert_eq!(parse_version_output("", "ffmpeg"), None);
    }

    #[test]
    fn majors_compare_as_expected() {
        assert_eq!(major_of("9.0"), Some(9));
        assert_eq!(major_of("6.1.1"), Some(6));
        assert_eq!(major_of("abc"), None);
        assert_eq!(major_of(""), None);
    }

    #[test]
    fn sha256_known_vector() {
        let mut hasher = Sha256::new();
        hasher.update(b"abc");
        assert_eq!(
            hex::encode(hasher.finalize()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn zip_extracts_wanted_and_skips_escape() {
        use std::io::Write as _;
        let dir =
            std::env::temp_dir().join(format!("tool-dock-ffmpeg-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("test.zip");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            for (name, data) in [
                ("pkg/bin/ffmpeg", b"ffmpeg-bytes".as_slice()),
                ("pkg/bin/ffprobe", b"ffprobe-bytes".as_slice()),
                ("pkg/doc/readme.txt", b"docs".as_slice()),
                ("../../evil", b"evil".as_slice()),
            ] {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        extract_binaries(&archive, ArchiveFormat::Zip, &["ffmpeg", "ffprobe"], &out).unwrap();
        assert_eq!(std::fs::read(out.join("ffmpeg")).unwrap(), b"ffmpeg-bytes");
        assert_eq!(
            std::fs::read(out.join("ffprobe")).unwrap(),
            b"ffprobe-bytes"
        );
        assert!(!out.join("evil").exists());
        assert!(!out.join("readme.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_missing_binary_fails() {
        use std::io::Write as _;
        let dir =
            std::env::temp_dir().join(format!("tool-dock-ffmpeg-miss-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("test.zip");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("ffmpeg", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        assert!(
            extract_binaries(&archive, ArchiveFormat::Zip, &["ffmpeg", "ffprobe"], &out).is_err()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tar_path_guard_allows_dot_prefix_rejects_escape() {
        assert!(is_safe_tar_path(Path::new("pkg/bin/ffmpeg")));
        assert!(is_safe_tar_path(Path::new("./pkg/bin/ffmpeg")));
        assert!(!is_safe_tar_path(Path::new("../escape-ffmpeg")));
        assert!(!is_safe_tar_path(Path::new("/abs/ffmpeg")));
    }

    #[test]
    fn tar_extracts_wanted_with_dot_prefix() {
        let dir = std::env::temp_dir().join(format!("tool-dock-ffmpeg-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("test.tar.xz");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let encoder = xz2::write::XzEncoder::new(file, 6);
            let mut tar = tar::Builder::new(encoder);
            for (name, data) in [
                ("pkg/bin/ffmpeg", b"ffmpeg-bytes".as_slice()),
                ("./pkg/bin/ffprobe", b"ffprobe-bytes".as_slice()),
            ] {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                tar.append_data(&mut header, name, data).unwrap();
            }
            let encoder = tar.into_inner().unwrap();
            encoder.finish().unwrap();
        }
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        extract_binaries(&archive, ArchiveFormat::TarXz, &["ffmpeg", "ffprobe"], &out).unwrap();
        assert_eq!(std::fs::read(out.join("ffmpeg")).unwrap(), b"ffmpeg-bytes");
        assert_eq!(
            std::fs::read(out.join("ffprobe")).unwrap(),
            b"ffprobe-bytes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 探活与解析顺序：托管优先 → 系统兜底 → 版本门槛 → 全无报错（unix 用脚本假二进制）
    #[cfg(unix)]
    mod probe_tests {
        use super::*;

        fn fake_tool(dir: &Path, name: &str, version_line: &str) {
            let path = dir.join(name);
            std::fs::write(&path, format!("#!/bin/sh\necho \"{version_line}\"\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        fn scratch(suffix: &str) -> PathBuf {
            let dir = std::env::temp_dir()
                .join(format!("tool-dock-ffmpeg-{suffix}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        #[test]
        fn managed_wins_over_system() {
            let root = scratch("order");
            let managed = root.join("managed");
            std::fs::create_dir_all(&managed).unwrap();
            std::fs::write(managed.join(INSTALL_MARKER), "9.0").unwrap();
            fake_tool(&managed, "ffmpeg", "ffmpeg version 9.0 Copyright");
            fake_tool(&managed, "ffprobe", "ffprobe version 9.0 Copyright");
            let system = root.join("system");
            std::fs::create_dir_all(&system).unwrap();
            fake_tool(&system, "ffmpeg", "ffmpeg version 8.0 Copyright");
            fake_tool(&system, "ffprobe", "ffprobe version 8.0 Copyright");

            let resolved = resolve_with(Some(managed.as_path()), &[system]).unwrap();
            assert_eq!(resolved.origin, FfmpegOrigin::Managed);
            assert_eq!(resolved.ffmpeg_version, "9.0");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn falls_back_to_system_without_marker() {
            let root = scratch("fallback");
            let managed = root.join("managed");
            std::fs::create_dir_all(&managed).unwrap();
            // 无标记即视为未安装完成，不认托管
            fake_tool(&managed, "ffmpeg", "ffmpeg version 9.0 Copyright");
            fake_tool(&managed, "ffprobe", "ffprobe version 9.0 Copyright");
            let system = root.join("system");
            std::fs::create_dir_all(&system).unwrap();
            fake_tool(&system, "ffmpeg", "ffmpeg version 7.1 Copyright");
            fake_tool(&system, "ffprobe", "ffprobe version 7.1 Copyright");

            let resolved = resolve_with(Some(managed.as_path()), &[system]).unwrap();
            assert_eq!(resolved.origin, FfmpegOrigin::System);
            assert_eq!(resolved.ffmpeg_version, "7.1");
            let _ = std::fs::remove_dir_all(&root);
        }

        #[test]
        fn rejects_old_system_and_reports_missing() {
            let root = scratch("old");
            let system = root.join("system");
            std::fs::create_dir_all(&system).unwrap();
            fake_tool(&system, "ffmpeg", "ffmpeg version 5.1 Copyright");
            fake_tool(&system, "ffprobe", "ffprobe version 5.1 Copyright");

            assert!(resolve_with(None, &[system]).is_err());
            assert!(resolve_with(None, &[]).is_err());
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}
