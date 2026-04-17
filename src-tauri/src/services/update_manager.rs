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
    #[serde(default)]
    pub skip_version: Option<String>,
    #[serde(default)]
    pub disable_reminders: bool,
    #[serde(default = "default_silent_reminder_strategy")]
    pub silent_reminder_strategy: String,
    #[serde(default)]
    pub last_reminded_at: Option<String>,
    #[serde(default)]
    pub last_reminded_version: Option<String>,
    pub last_check_at: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            auto_install: false,
            skip_version: None,
            disable_reminders: false,
            silent_reminder_strategy: default_silent_reminder_strategy(),
            last_reminded_at: None,
            last_reminded_version: None,
            last_check_at: None,
        }
    }
}

fn default_silent_reminder_strategy() -> String {
    "immediate".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReminderDecision {
    pub should_notify: bool,
    pub reason: String,
    pub settings: UpdateSettings,
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
        let mut settings =
            serde_json::from_str(&raw).map_err(|e| format!("解析更新设置失败: {}", e))?;
        let changed = normalize_settings(&mut settings);
        if changed {
            Self::save_settings(app_data_dir, &settings)?;
        }
        Ok(settings)
    }

    pub fn save_settings(app_data_dir: &Path, settings: &UpdateSettings) -> Result<(), String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let path = settings_path(app_data_dir);
        let mut normalized = settings.clone();
        normalize_settings(&mut normalized);
        let content =
            serde_json::to_string_pretty(&normalized).map_err(|e| format!("序列化更新设置失败: {}", e))?;
        fs::write(path, content).map_err(|e| format!("写入更新设置失败: {}", e))
    }

    pub fn mark_checked_now(app_data_dir: &Path) -> Result<UpdateSettings, String> {
        let mut settings = Self::load_settings(app_data_dir)?;
        settings.last_check_at = Some(Utc::now().to_rfc3339());
        Self::save_settings(app_data_dir, &settings)?;
        Ok(settings)
    }

    pub fn mark_reminded_now(app_data_dir: &Path, version: &str) -> Result<UpdateSettings, String> {
        let mut settings = Self::load_settings(app_data_dir)?;
        settings.last_reminded_at = Some(Utc::now().to_rfc3339());
        settings.last_reminded_version = normalize_optional_string(Some(version));
        Self::save_settings(app_data_dir, &settings)?;
        Ok(settings)
    }

    pub fn evaluate_reminder_policy(
        app_data_dir: &Path,
        version: &str,
    ) -> Result<UpdateReminderDecision, String> {
        let settings = Self::load_settings(app_data_dir)?;
        Ok(evaluate_update_reminder_policy_with_now(
            &settings,
            version,
            Utc::now(),
        ))
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

fn normalize_optional_string(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_silent_reminder_strategy(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "daily" => "daily".to_string(),
        "weekly" => "weekly".to_string(),
        _ => "immediate".to_string(),
    }
}

fn normalize_rfc3339(raw: Option<&str>) -> Option<String> {
    normalize_optional_string(raw).and_then(|value| {
        chrono::DateTime::parse_from_rfc3339(&value)
            .ok()
            .map(|parsed| parsed.with_timezone(&Utc).to_rfc3339())
    })
}

fn normalize_settings(settings: &mut UpdateSettings) -> bool {
    let mut changed = false;
    let normalized_skip_version = normalize_optional_string(settings.skip_version.as_deref());
    if settings.skip_version != normalized_skip_version {
        settings.skip_version = normalized_skip_version;
        changed = true;
    }
    let normalized_last_reminded_version =
        normalize_optional_string(settings.last_reminded_version.as_deref());
    if settings.last_reminded_version != normalized_last_reminded_version {
        settings.last_reminded_version = normalized_last_reminded_version;
        changed = true;
    }
    let normalized_strategy = normalize_silent_reminder_strategy(&settings.silent_reminder_strategy);
    if settings.silent_reminder_strategy != normalized_strategy {
        settings.silent_reminder_strategy = normalized_strategy;
        changed = true;
    }
    let normalized_last_check_at = normalize_rfc3339(settings.last_check_at.as_deref());
    if settings.last_check_at != normalized_last_check_at {
        settings.last_check_at = normalized_last_check_at;
        changed = true;
    }
    let normalized_last_reminded_at = normalize_rfc3339(settings.last_reminded_at.as_deref());
    if settings.last_reminded_at != normalized_last_reminded_at {
        settings.last_reminded_at = normalized_last_reminded_at;
        changed = true;
    }
    changed
}

fn evaluate_update_reminder_policy_with_now(
    settings: &UpdateSettings,
    version: &str,
    now: chrono::DateTime<Utc>,
) -> UpdateReminderDecision {
    let mut normalized = settings.clone();
    normalize_settings(&mut normalized);
    let version = normalize_optional_string(Some(version));
    let Some(version) = version else {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "invalid_version".to_string(),
            settings: normalized,
        };
    };

    if normalized
        .skip_version
        .as_ref()
        .is_some_and(|skip| skip.eq_ignore_ascii_case(&version))
    {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "skipped_version".to_string(),
            settings: normalized,
        };
    }

    if normalized.disable_reminders {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "reminders_disabled".to_string(),
            settings: normalized,
        };
    }

    if normalized.silent_reminder_strategy == "immediate" {
        return UpdateReminderDecision {
            should_notify: true,
            reason: "allow_immediate".to_string(),
            settings: normalized,
        };
    }

    if normalized
        .last_reminded_version
        .as_ref()
        .is_none_or(|last| !last.eq_ignore_ascii_case(&version))
    {
        return UpdateReminderDecision {
            should_notify: true,
            reason: "allow_new_version".to_string(),
            settings: normalized,
        };
    }

    let Some(last_reminded_at) = normalized
        .last_reminded_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|dt| dt.with_timezone(&Utc))
    else {
        return UpdateReminderDecision {
            should_notify: true,
            reason: "allow_missing_history".to_string(),
            settings: normalized,
        };
    };

    let required_interval_secs = if normalized.silent_reminder_strategy == "weekly" {
        7 * 24 * 60 * 60
    } else {
        24 * 60 * 60
    };
    if (now - last_reminded_at).num_seconds() < required_interval_secs {
        return UpdateReminderDecision {
            should_notify: false,
            reason: "silent_window_active".to_string(),
            settings: normalized,
        };
    }

    UpdateReminderDecision {
        should_notify: true,
        reason: "allow_window_expired".to_string(),
        settings: normalized,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ais-update-manager-{}-{}",
            label,
            uuid::Uuid::new_v4()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn load_settings_migrates_legacy_fields() {
        let dir = unique_temp_dir("migrate");
        let legacy = r#"{
  "auto_check": true,
  "auto_install": false,
  "last_check_at": "2026-04-16T01:00:00Z"
}"#;
        fs::write(settings_path(&dir), legacy).expect("write legacy settings");
        let loaded = UpdateManager::load_settings(&dir).expect("load settings");
        assert_eq!(loaded.auto_check, true);
        assert_eq!(loaded.auto_install, false);
        assert_eq!(loaded.skip_version, None);
        assert_eq!(loaded.disable_reminders, false);
        assert_eq!(loaded.silent_reminder_strategy, "immediate");
        assert_eq!(loaded.last_reminded_at, None);
        assert_eq!(loaded.last_reminded_version, None);
    }

    #[test]
    fn reminder_policy_respects_skipped_version() {
        let settings = UpdateSettings {
            skip_version: Some("0.1.12".to_string()),
            ..UpdateSettings::default()
        };
        let decision = evaluate_update_reminder_policy_with_now(
            &settings,
            "0.1.12",
            Utc::now(),
        );
        assert!(!decision.should_notify);
        assert_eq!(decision.reason, "skipped_version");
    }

    #[test]
    fn reminder_policy_blocks_daily_window_for_same_version() {
        let now = Utc::now();
        let settings = UpdateSettings {
            silent_reminder_strategy: "daily".to_string(),
            last_reminded_version: Some("0.1.13".to_string()),
            last_reminded_at: Some((now - chrono::Duration::hours(6)).to_rfc3339()),
            ..UpdateSettings::default()
        };
        let decision = evaluate_update_reminder_policy_with_now(&settings, "0.1.13", now);
        assert!(!decision.should_notify);
        assert_eq!(decision.reason, "silent_window_active");
    }

    #[test]
    fn reminder_policy_allows_newer_version_even_in_silent_window() {
        let now = Utc::now();
        let settings = UpdateSettings {
            silent_reminder_strategy: "weekly".to_string(),
            last_reminded_version: Some("0.1.13".to_string()),
            last_reminded_at: Some((now - chrono::Duration::hours(6)).to_rfc3339()),
            ..UpdateSettings::default()
        };
        let decision = evaluate_update_reminder_policy_with_now(&settings, "0.1.14", now);
        assert!(decision.should_notify);
        assert_eq!(decision.reason, "allow_new_version");
    }
}
