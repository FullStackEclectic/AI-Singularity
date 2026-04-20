use super::types::CURRENT_VERSION;
use super::{
    LinuxInstallResult, LinuxReleaseAssetInfo, LinuxReleaseInfo, UpdateManager, UpdateRuntimeInfo,
};
#[cfg(target_os = "linux")]
use std::fs;
use std::path::Path;
use url::Url;

impl UpdateManager {
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
        let updater_pubkey_configured =
            !pubkey.trim().is_empty() && pubkey.trim() != "YOUR_UPDATER_PUBLIC_KEY";
        let platform = std::env::consts::OS.to_string();
        let can_auto_install =
            platform != "linux" && !endpoints.is_empty() && updater_pubkey_configured;
        let linux_install_kind = linux_install_kind();
        let linux_manual_hint = linux_manual_hint(linux_install_kind.as_deref());
        let warning = if endpoints.is_empty() {
            Some("未配置 updater endpoint，更新检查可能无法正常工作。".to_string())
        } else if !updater_pubkey_configured {
            Some("updater 公钥仍是占位值，请先替换为正式 pubkey。".to_string())
        } else if !can_auto_install {
            Some(
                "Linux 下自动安装体验依赖具体打包方式，当前已补上安装方式识别与处理建议。"
                    .to_string(),
            )
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
        let preferred_kind = linux_install_kind();
        let client = reqwest::Client::builder()
            .user_agent("AI-Singularity")
            .build()
            .map_err(|e| format!("创建更新 HTTP 客户端失败: {}", e))?;
        let mut errors = Vec::new();

        for endpoint in &config.endpoints {
            match fetch_linux_release_info_from_endpoint(
                &client,
                endpoint,
                preferred_kind.as_deref(),
            )
            .await
            {
                Ok(info) => return Ok(info),
                Err(err) => errors.push(format!("{} => {}", endpoint, err)),
            }
        }

        Err(format!(
            "当前 updater endpoint 仍无法解析可用的 Linux 安装包资产。已检测：{}",
            errors.join(" | ")
        ))
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
                        (
                            "pkcon",
                            vec![
                                "-y",
                                "install-local",
                                package_path.to_string_lossy().as_ref(),
                            ],
                        ),
                        (
                            "pkexec",
                            vec![
                                "apt",
                                "install",
                                "-y",
                                package_path.to_string_lossy().as_ref(),
                            ],
                        ),
                        (
                            "pkexec",
                            vec!["dpkg", "-i", package_path.to_string_lossy().as_ref()],
                        ),
                    ])?;
                    Ok(LinuxInstallResult {
                        downloaded_path: package_path.to_string_lossy().to_string(),
                        action: "install".to_string(),
                        message: "已下载并尝试安装 .deb 更新包。".to_string(),
                    })
                }
                "rpm" => {
                    run_linux_install_command(&[
                        (
                            "pkcon",
                            vec![
                                "-y",
                                "install-local",
                                package_path.to_string_lossy().as_ref(),
                            ],
                        ),
                        (
                            "pkexec",
                            vec![
                                "dnf",
                                "install",
                                "-y",
                                package_path.to_string_lossy().as_ref(),
                            ],
                        ),
                        (
                            "pkexec",
                            vec![
                                "yum",
                                "install",
                                "-y",
                                package_path.to_string_lossy().as_ref(),
                            ],
                        ),
                        (
                            "pkexec",
                            vec![
                                "rpm",
                                "-U",
                                "--replacepkgs",
                                package_path.to_string_lossy().as_ref(),
                            ],
                        ),
                    ])?;
                    Ok(LinuxInstallResult {
                        downloaded_path: package_path.to_string_lossy().to_string(),
                        action: "install".to_string(),
                        message: "已下载并尝试安装 .rpm 更新包。".to_string(),
                    })
                }
                "pacman" => {
                    run_linux_install_command(&[(
                        "pkexec",
                        vec![
                            "pacman",
                            "-U",
                            "--noconfirm",
                            package_path.to_string_lossy().as_ref(),
                        ],
                    )])?;
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
                        message: "已下载 AppImage，并设置为可执行。你可以直接运行这个文件。"
                            .to_string(),
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

#[derive(Debug, Clone, serde::Deserialize)]
struct TauriUpdaterConfig {
    endpoints: Vec<String>,
    pubkey: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TauriPluginsConfig {
    updater: Option<TauriUpdaterConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct TauriConfig {
    plugins: Option<TauriPluginsConfig>,
}

fn read_tauri_updater_config() -> Option<TauriUpdaterConfig> {
    let raw = include_str!("../../../tauri.conf.json");
    serde_json::from_str::<TauriConfig>(raw)
        .ok()
        .and_then(|cfg| cfg.plugins.and_then(|plugins| plugins.updater))
}

fn github_release_api_url_from_endpoint(endpoint: &str) -> Option<String> {
    let parsed = Url::parse(endpoint).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let segments = parsed.path_segments()?.collect::<Vec<_>>();

    if host == "api.github.com" {
        if segments.len() >= 3 && segments.first() == Some(&"repos") {
            let owner = segments.get(1)?.trim();
            let repo = segments.get(2)?.trim();
            if !owner.is_empty() && !repo.is_empty() {
                return Some(format!(
                    "https://api.github.com/repos/{}/{}/releases/latest",
                    owner, repo
                ));
            }
        }
        return None;
    }

    if host == "github.com" || host == "www.github.com" {
        if segments.len() >= 2 {
            let owner = segments.first()?.trim();
            let repo = segments.get(1)?.trim();
            if !owner.is_empty() && !repo.is_empty() {
                return Some(format!(
                    "https://api.github.com/repos/{}/{}/releases/latest",
                    owner, repo
                ));
            }
        }
    }

    None
}

async fn fetch_linux_release_info_from_endpoint(
    client: &reqwest::Client,
    endpoint: &str,
    preferred_kind: Option<&str>,
) -> Result<LinuxReleaseInfo, String> {
    let request_url = github_release_api_url_from_endpoint(endpoint)
        .unwrap_or_else(|| endpoint.trim().to_string());
    let value = client
        .get(&request_url)
        .send()
        .await
        .map_err(|e| format!("请求更新清单失败: {}", e))?
        .error_for_status()
        .map_err(|e| format!("更新清单返回异常状态: {}", e))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析更新清单响应失败: {}", e))?;

    if request_url.contains("api.github.com/repos/") {
        parse_github_release_info(&value, preferred_kind)
    } else {
        parse_generic_linux_release_info(endpoint, &value, preferred_kind)
    }
}

fn parse_github_release_info(
    value: &serde_json::Value,
    preferred_kind: Option<&str>,
) -> Result<LinuxReleaseInfo, String> {
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
            let url = asset
                .get("browser_download_url")
                .or_else(|| asset.get("url"))
                .and_then(|v| v.as_str())?
                .to_string();
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| asset_name_from_url(&url))?;
            let kind = classify_linux_asset_kind(&name)?;
            Some(LinuxReleaseAssetInfo {
                preferred: preferred_kind == Some(kind.as_str()),
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

    if assets.is_empty() {
        return Err("已解析 GitHub Release，但未找到可识别的 Linux 安装包资产。".to_string());
    }

    Ok(LinuxReleaseInfo {
        version,
        published_at,
        body,
        assets,
    })
}

fn parse_generic_linux_release_info(
    endpoint: &str,
    value: &serde_json::Value,
    preferred_kind: Option<&str>,
) -> Result<LinuxReleaseInfo, String> {
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("name").and_then(|v| v.as_str()))
        .or_else(|| value.get("tag_name").and_then(|v| v.as_str()))
        .unwrap_or("unknown")
        .to_string();
    let published_at = value
        .get("pub_date")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("published_at").and_then(|v| v.as_str()))
        .map(|s| s.to_string());
    let body = value
        .get("notes")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("body").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    let mut assets = Vec::new();
    if let Some(platforms) = value.get("platforms").and_then(|v| v.as_object()) {
        let current_arch = std::env::consts::ARCH.to_ascii_lowercase();
        for (platform_key, item) in platforms {
            let lower_platform = platform_key.to_ascii_lowercase();
            if !lower_platform.contains("linux") {
                continue;
            }
            let Some(item_obj) = item.as_object() else {
                continue;
            };
            let Some(url) = item_obj
                .get("url")
                .or_else(|| item_obj.get("browser_download_url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            else {
                continue;
            };
            let name = item_obj
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| asset_name_from_url(&url))
                .unwrap_or_else(|| format!("linux-asset-{}", platform_key));
            let Some(kind) =
                classify_linux_asset_kind(&name).or_else(|| classify_linux_asset_kind(&url))
            else {
                continue;
            };
            assets.push(LinuxReleaseAssetInfo {
                preferred: preferred_kind == Some(kind.as_str())
                    || lower_platform.contains(&current_arch),
                name,
                kind,
                url,
                size: item_obj.get("size").and_then(|v| v.as_u64()),
                content_type: None,
            });
        }
    }

    if assets.is_empty() {
        assets = value
            .get("assets")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|asset| {
                let url = asset
                    .get("browser_download_url")
                    .or_else(|| asset.get("url"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let name = asset
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| asset_name_from_url(&url))?;
                let kind =
                    classify_linux_asset_kind(&name).or_else(|| classify_linux_asset_kind(&url))?;
                let lower_platform = asset
                    .get("platform")
                    .or_else(|| asset.get("target"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !lower_platform.is_empty() && !lower_platform.contains("linux") {
                    return None;
                }
                Some(LinuxReleaseAssetInfo {
                    preferred: preferred_kind == Some(kind.as_str()),
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
            .collect();
    }

    if assets.is_empty() {
        return Err(format!(
            "已解析 updater manifest，但未在 {} 中找到可识别的 Linux 安装包资产。",
            endpoint
        ));
    }

    Ok(LinuxReleaseInfo {
        version,
        published_at,
        body,
        assets,
    })
}

fn asset_name_from_url(url: &str) -> Option<String> {
    Url::parse(url).ok().and_then(|parsed| {
        parsed
            .path_segments()
            .and_then(|mut segments| segments.next_back().map(str::to_string))
            .filter(|name| !name.trim().is_empty())
    })
}

#[cfg(target_os = "linux")]
fn asset_file_name(url: &str) -> Result<String, String> {
    asset_name_from_url(url).ok_or_else(|| "无法从下载链接解析文件名".to_string())
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
        "rpm" => {
            "检测到 .rpm 安装。建议优先通过 dnf / yum / zypper 安装新包，保留系统包管理一致性。"
        }
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
    use super::{github_release_api_url_from_endpoint, parse_generic_linux_release_info};
    use serde_json::json;

    #[test]
    fn resolves_github_web_release_endpoint_to_api_latest() {
        let resolved = github_release_api_url_from_endpoint(
            "https://github.com/tarui/AI-Singularity/releases/latest",
        )
        .expect("should resolve github web latest url");
        assert_eq!(
            resolved,
            "https://api.github.com/repos/tarui/AI-Singularity/releases/latest"
        );
    }

    #[test]
    fn resolves_github_api_release_endpoint_to_api_latest() {
        let resolved = github_release_api_url_from_endpoint(
            "https://api.github.com/repos/tarui/AI-Singularity/releases/latest",
        )
        .expect("should resolve github api latest url");
        assert_eq!(
            resolved,
            "https://api.github.com/repos/tarui/AI-Singularity/releases/latest"
        );
    }

    #[test]
    fn resolves_github_download_manifest_endpoint_to_api_latest() {
        let resolved = github_release_api_url_from_endpoint(
            "https://github.com/tarui/AI-Singularity/releases/latest/download/latest.json",
        )
        .expect("should resolve github release asset url");
        assert_eq!(
            resolved,
            "https://api.github.com/repos/tarui/AI-Singularity/releases/latest"
        );
    }

    #[test]
    fn rejects_non_github_endpoint() {
        let resolved =
            github_release_api_url_from_endpoint("https://updates.example.com/latest.json");
        assert!(resolved.is_none());
    }

    #[test]
    fn parses_generic_latest_json_linux_assets() {
        let manifest = json!({
            "version": "v0.1.1",
            "notes": "release notes",
            "pub_date": "2026-04-20T00:00:00Z",
            "platforms": {
                "linux-x86_64": {
                    "signature": "sig",
                    "url": "https://updates.example.com/downloads/AI-Singularity_0.1.1_amd64.deb"
                },
                "windows-x86_64": {
                    "signature": "sig",
                    "url": "https://updates.example.com/downloads/AI-Singularity_0.1.1_x64-setup.exe"
                }
            }
        });

        let parsed = parse_generic_linux_release_info(
            "https://updates.example.com/latest.json",
            &manifest,
            Some("deb"),
        )
        .expect("should parse generic updater manifest");

        assert_eq!(parsed.version, "v0.1.1");
        assert_eq!(parsed.assets.len(), 1);
        assert_eq!(parsed.assets[0].kind, "deb");
        assert!(parsed.assets[0].preferred);
        assert_eq!(parsed.assets[0].name, "AI-Singularity_0.1.1_amd64.deb");
    }
}
