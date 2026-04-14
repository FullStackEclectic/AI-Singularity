use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use url::Url;

const CONFIG_FILE: &str = "update_settings.json";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub auto_install: bool,
    pub last_check_at: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_install: false,
            last_check_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRuntimeInfo {
    pub current_version: String,
    pub platform: String,
    pub updater_endpoints: Vec<String>,
    pub updater_pubkey_configured: bool,
    pub can_auto_install: bool,
    pub linux_install_kind: Option<String>,
    pub linux_manual_hint: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxReleaseAssetInfo {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub preferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxReleaseInfo {
    pub version: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
    pub assets: Vec<LinuxReleaseAssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxInstallResult {
    pub downloaded_path: String,
    pub action: String,
    pub message: String,
}

pub struct UpdateManager;

impl UpdateManager {
    pub fn load_settings(app_data_dir: &Path) -> Result<UpdateSettings, String> {
        let path = settings_path(app_data_dir);
        if !path.exists() {
            return Ok(UpdateSettings::default());
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("读取更新设置失败: {}", e))?;
        serde_json::from_str(&raw).map_err(|e| format!("解析更新设置失败: {}", e))
    }

    pub fn save_settings(app_data_dir: &Path, settings: &UpdateSettings) -> Result<(), String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let path = settings_path(app_data_dir);
        let content =
            serde_json::to_string_pretty(settings).map_err(|e| format!("序列化更新设置失败: {}", e))?;
        fs::write(path, content).map_err(|e| format!("写入更新设置失败: {}", e))
    }

    pub fn mark_checked_now(app_data_dir: &Path) -> Result<UpdateSettings, String> {
        let mut settings = Self::load_settings(app_data_dir)?;
        settings.last_check_at = Some(Utc::now().to_rfc3339());
        Self::save_settings(app_data_dir, &settings)?;
        Ok(settings)
    }

    pub fn runtime_info() -> UpdateRuntimeInfo {
        let config = read_tauri_updater_config();
        let endpoints = config
            .as_ref()
            .map(|item| item.endpoints.clone())
            .unwrap_or_default();
        let pubkey = config
            .as_ref()
            .map(|item| item.pubkey.clone())
            .unwrap_or_default();
        let updater_pubkey_configured = !pubkey.trim().is_empty() && pubkey.trim() != "YOUR_UPDATER_PUBLIC_KEY";
        let platform = std::env::consts::OS.to_string();
        let can_auto_install = platform != "linux";
        let linux_install_kind = linux_install_kind();
        let linux_manual_hint = linux_manual_hint(linux_install_kind.as_deref());
        let warning = if endpoints.is_empty() {
            Some("未配置 updater endpoint，更新检查可能无法正常工作。".to_string())
        } else if !updater_pubkey_configured {
            Some("updater 公钥仍是占位值，请先替换为正式 pubkey。".to_string())
        } else if !can_auto_install {
            Some("Linux 下自动安装体验依赖具体打包方式，当前已补上安装方式识别与处理建议。".to_string())
        } else {
            None
        };

        UpdateRuntimeInfo {
            current_version: CURRENT_VERSION.to_string(),
            platform,
            updater_endpoints: endpoints,
            updater_pubkey_configured,
            can_auto_install,
            linux_install_kind,
            linux_manual_hint,
            warning,
        }
    }

    pub async fn fetch_linux_release_info() -> Result<LinuxReleaseInfo, String> {
        let config =
            read_tauri_updater_config().ok_or_else(|| "未找到 updater 配置".to_string())?;
        let endpoint = config
            .endpoints
            .first()
            .cloned()
            .ok_or_else(|| "未配置 updater endpoint".to_string())?;

        if !endpoint.contains("api.github.com") || !endpoint.contains("/releases/latest") {
            return Err("当前 updater endpoint 不是 GitHub latest release API，暂不支持自动解析 Linux 安装包资产。".to_string());
        }

        let preferred_kind = linux_install_kind();
        let client = reqwest::Client::builder()
            .user_agent("AI-Singularity")
            .build()
            .map_err(|e| format!("创建更新 HTTP 客户端失败: {}", e))?;
        let value = client
            .get(&endpoint)
            .send()
            .await
            .map_err(|e| format!("请求 GitHub Release 失败: {}", e))?
            .error_for_status()
            .map_err(|e| format!("GitHub Release 返回异常状态: {}", e))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("解析 GitHub Release 响应失败: {}", e))?;

        let version = value
            .get("tag_name")
            .and_then(|v| v.as_str())
            .or_else(|| value.get("name").and_then(|v| v.as_str()))
            .unwrap_or("unknown")
            .to_string();
        let published_at = value
            .get("published_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let body = value
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let assets = value
            .get("assets")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|asset| {
                let name = asset.get("name")?.as_str()?.to_string();
                let url = asset.get("browser_download_url")?.as_str()?.to_string();
                let kind = classify_linux_asset_kind(&name)?;
                Some(LinuxReleaseAssetInfo {
                    preferred: preferred_kind.as_deref() == Some(kind.as_str()),
                    name,
                    kind,
                    url,
                    size: asset.get("size").and_then(|v| v.as_u64()),
                    content_type: asset
                        .get("content_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect::<Vec<_>>();

        Ok(LinuxReleaseInfo {
            version,
            published_at,
            body,
            assets,
        })
    }

    pub async fn install_linux_release_asset(
        app_data_dir: &Path,
        url: &str,
        kind: &str,
        version: Option<&str>,
    ) -> Result<LinuxInstallResult, String> {
        #[cfg(target_os = "linux")]
        {
            let version_dir = version
                .map(|value| value.trim().trim_start_matches('v').to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "latest".to_string());
            let file_name = asset_file_name(url)?;
            let cache_dir = app_data_dir.join("updates").join("linux").join(version_dir);
            fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("创建 Linux 更新缓存目录失败: {}", e))?;
            let package_path = cache_dir.join(&file_name);

            let client = reqwest::Client::builder()
                .user_agent("AI-Singularity")
                .build()
                .map_err(|e| format!("创建下载客户端失败: {}", e))?;
            let bytes = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("下载 Linux 安装包失败: {}", e))?
                .error_for_status()
                .map_err(|e| format!("下载 Linux 安装包返回异常状态: {}", e))?
                .bytes()
                .await
                .map_err(|e| format!("读取 Linux 安装包内容失败: {}", e))?;
            fs::write(&package_path, &bytes)
                .map_err(|e| format!("写入 Linux 安装包失败: {}", e))?;

            match kind {
                "deb" => {
                    run_linux_install_command(&[
                        ("pkcon", vec!["-y", "install-local", package_path.to_string_lossy().as_ref()]),
                        ("pkexec", vec!["apt", "install", "-y", package_path.to_string_lossy().as_ref()]),
                        ("pkexec", vec!["dpkg", "-i", package_path.to_string_lossy().as_ref()]),
                    ])?;
                    Ok(LinuxInstallResult {
                        downloaded_path: package_path.to_string_lossy().to_string(),
                        action: "install".to_string(),
                        message: "已下载并尝试安装 .deb 更新包。".to_string(),
                    })
                }
                "rpm" => {
                    run_linux_install_command(&[
                        ("pkcon", vec!["-y", "install-local", package_path.to_string_lossy().as_ref()]),
                        ("pkexec", vec!["dnf", "install", "-y", package_path.to_string_lossy().as_ref()]),
                        ("pkexec", vec!["yum", "install", "-y", package_path.to_string_lossy().as_ref()]),
                        ("pkexec", vec!["rpm", "-U", "--replacepkgs", package_path.to_string_lossy().as_ref()]),
                    ])?;
                    Ok(LinuxInstallResult {
                        downloaded_path: package_path.to_string_lossy().to_string(),
                        action: "install".to_string(),
                        message: "已下载并尝试安装 .rpm 更新包。".to_string(),
                    })
                }
                "pacman" => {
                    run_linux_install_command(&[
                        ("pkexec", vec!["pacman", "-U", "--noconfirm", package_path.to_string_lossy().as_ref()]),
                    ])?;
                    Ok(LinuxInstallResult {
                        downloaded_path: package_path.to_string_lossy().to_string(),
                        action: "install".to_string(),
                        message: "已下载并尝试安装 pacman 更新包。".to_string(),
                    })
                }
                "appimage" => {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&package_path)
                        .map_err(|e| format!("读取 AppImage 权限失败: {}", e))?
                        .permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&package_path, perms)
                        .map_err(|e| format!("设置 AppImage 可执行权限失败: {}", e))?;
                    Ok(LinuxInstallResult {
                        downloaded_path: package_path.to_string_lossy().to_string(),
                        action: "appimage_ready".to_string(),
                        message: "已下载 AppImage，并设置为可执行。你可以直接运行这个文件。".to_string(),
                    })
                }
                other => Err(format!("暂不支持处理 Linux 安装包类型: {}", other)),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (app_data_dir, url, kind, version);
            Err("当前平台不支持 Linux 安装执行链".to_string())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TauriUpdaterConfig {
    endpoints: Vec<String>,
    pubkey: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TauriPluginsConfig {
    updater: Option<TauriUpdaterConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct TauriConfig {
    plugins: Option<TauriPluginsConfig>,
}

fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(CONFIG_FILE)
}

fn read_tauri_updater_config() -> Option<TauriUpdaterConfig> {
    let raw = include_str!("../../tauri.conf.json");
    serde_json::from_str::<TauriConfig>(raw)
        .ok()
        .and_then(|cfg| cfg.plugins.and_then(|plugins| plugins.updater))
}

#[cfg(target_os = "linux")]
fn asset_file_name(url: &str) -> Result<String, String> {
    Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.path_segments().and_then(|mut segments| segments.next_back().map(str::to_string)))
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "无法从下载链接解析文件名".to_string())
}

#[cfg(target_os = "linux")]
fn run_linux_install_command(attempts: &[(&str, Vec<&str>)]) -> Result<(), String> {
    use std::process::Command;

    let mut errors = Vec::new();
    for (program, args) in attempts {
        match Command::new(program).args(args).output() {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let detail = if !stderr.is_empty() {
                    stderr
                } else if !stdout.is_empty() {
                    stdout
                } else {
                    format!("exit status {:?}", output.status.code())
                };
                errors.push(format!("{} {} => {}", program, args.join(" "), detail));
            }
            Err(err) => {
                errors.push(format!("{} {} => {}", program, args.join(" "), err));
            }
        }
    }
    Err(format!("所有 Linux 安装命令均失败：{}", errors.join(" | ")))
}

fn classify_linux_asset_kind(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".deb") {
        Some("deb".to_string())
    } else if lower.ends_with(".rpm") {
        Some("rpm".to_string())
    } else if lower.ends_with(".appimage") {
        Some("appimage".to_string())
    } else if lower.ends_with(".pkg.tar.zst") || lower.ends_with(".pkg.tar.xz") {
        Some("pacman".to_string())
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn linux_install_kind() -> Option<String> {
    use std::process::Command;

    if std::env::var_os("APPIMAGE").is_some() {
        return Some("appimage".to_string());
    }

    let exe = std::env::current_exe().ok()?;
    let exe_str = exe.to_string_lossy().to_string();
    if exe_str.to_ascii_lowercase().ends_with(".appimage") {
        return Some("appimage".to_string());
    }

    let dpkg = Command::new("dpkg-query")
        .args(["-S", &exe_str])
        .output()
        .ok()
        .filter(|output| output.status.success());
    if dpkg.is_some() {
        return Some("deb".to_string());
    }

    let rpm = Command::new("rpm")
        .args(["-qf", &exe_str])
        .output()
        .ok()
        .filter(|output| output.status.success());
    if rpm.is_some() {
        return Some("rpm".to_string());
    }

    let pacman = Command::new("pacman")
        .args(["-Qo", &exe_str])
        .output()
        .ok()
        .filter(|output| output.status.success());
    if pacman.is_some() {
        return Some("pacman".to_string());
    }

    Some("unknown".to_string())
}

#[cfg(not(target_os = "linux"))]
fn linux_install_kind() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn linux_manual_hint(kind: Option<&str>) -> Option<String> {
    let hint = match kind.unwrap_or("unknown") {
        "deb" => "检测到 .deb 安装。建议优先通过 apt / 软件中心安装新包，保留系统包管理一致性。",
        "rpm" => "检测到 .rpm 安装。建议优先通过 dnf / yum / zypper 安装新包，保留系统包管理一致性。",
        "pacman" => "检测到 pacman 管理安装。建议优先通过 pacman / yay 安装新包。",
        "appimage" => "检测到 AppImage 安装。建议下载新 AppImage 后替换旧文件，并保留可执行权限。",
        _ => "未识别当前 Linux 安装方式。建议下载最新发行包后手动替换或安装。",
    };
    Some(hint.to_string())
}

#[cfg(not(target_os = "linux"))]
fn linux_manual_hint(_: Option<&str>) -> Option<String> {
    None
}
