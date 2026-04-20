use crate::services::env_checker::{EnvChecker, EnvConflict};
use crate::AppError;
use serde_json::Value;
use std::path::Path;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEnvStatusItem {
    pub tool: String,
    pub label: String,
    pub env_name: String,
    pub configured: bool,
    pub sources: Vec<String>,
    pub note: Option<String>,
}

#[tauri::command]
pub async fn check_system_env_conflicts(app_name: String) -> Result<Vec<EnvConflict>, AppError> {
    let conflicts = EnvChecker::check_env_conflicts(&app_name);
    Ok(conflicts)
}

#[tauri::command]
pub async fn get_runtime_env_statuses() -> Result<Vec<RuntimeEnvStatusItem>, AppError> {
    let items = vec![
        RuntimeEnvStatusItem {
            tool: "claude".to_string(),
            label: "Claude / Claude Code".to_string(),
            env_name: "ANTHROPIC_API_KEY".to_string(),
            configured: false,
            sources: vec![],
            note: None,
        },
        RuntimeEnvStatusItem {
            tool: "claude".to_string(),
            label: "Claude / Claude Code".to_string(),
            env_name: "CLAUDE_API_KEY".to_string(),
            configured: false,
            sources: vec![],
            note: Some("兼容别名；若已使用 ANTHROPIC_API_KEY，通常无需重复设置。".to_string()),
        },
        RuntimeEnvStatusItem {
            tool: "claude".to_string(),
            label: "Claude / Claude Code".to_string(),
            env_name: "ANTHROPIC_AUTH_TOKEN".to_string(),
            configured: false,
            sources: vec![],
            note: Some("部分 Claude 配置使用该变量而非 API Key。".to_string()),
        },
        RuntimeEnvStatusItem {
            tool: "codex".to_string(),
            label: "Codex / OpenAI".to_string(),
            env_name: "OPENAI_API_KEY".to_string(),
            configured: false,
            sources: vec![],
            note: Some(
                "Codex 的 OAuth 登录本身不依赖 Client Secret；这里只检查 API Key 类环境变量。"
                    .to_string(),
            ),
        },
        RuntimeEnvStatusItem {
            tool: "gemini".to_string(),
            label: "Gemini".to_string(),
            env_name: "GEMINI_API_KEY".to_string(),
            configured: false,
            sources: vec![],
            note: None,
        },
        RuntimeEnvStatusItem {
            tool: "gemini".to_string(),
            label: "Gemini".to_string(),
            env_name: "GOOGLE_API_KEY".to_string(),
            configured: false,
            sources: vec![],
            note: Some("Gemini 常见兼容变量，部分工具链会读取它。".to_string()),
        },
    ];

    Ok(items
        .into_iter()
        .map(|mut item| {
            let sources = collect_env_sources(&item.env_name);
            item.configured = !sources.is_empty();
            item.sources = sources;
            item
        })
        .collect())
}

fn collect_env_sources(env_name: &str) -> Vec<String> {
    let home_dir = dirs::home_dir();
    collect_env_sources_with_home(env_name, home_dir.as_deref())
}

fn collect_env_sources_with_home(env_name: &str, home_dir: Option<&Path>) -> Vec<String> {
    let mut sources = Vec::new();

    if std::env::var_os(env_name).is_some() {
        sources.push("Process Environment".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;

        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
            if hkcu.get_raw_value(env_name).is_ok() {
                sources.push("HKEY_CURRENT_USER\\Environment".to_string());
            }
        }

        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
        {
            if hklm.get_raw_value(env_name).is_ok() {
                sources.push(
                    "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"
                        .to_string(),
                );
            }
        }
    }

    if let Some(home_dir) = home_dir {
        sources.extend(collect_tool_file_sources(env_name, home_dir));
    }

    sources
}

fn collect_tool_file_sources(env_name: &str, home_dir: &Path) -> Vec<String> {
    let mut sources = Vec::new();
    sources.extend(collect_claude_sources(env_name, home_dir));
    sources.extend(collect_codex_sources(env_name, home_dir));
    sources.extend(collect_gemini_sources(env_name, home_dir));
    sources
}

fn collect_claude_sources(env_name: &str, home_dir: &Path) -> Vec<String> {
    let settings_path = home_dir.join(".claude").join("settings.json");
    let Some(root) = read_json_file(&settings_path) else {
        return Vec::new();
    };
    let Some(env) = root.get("env").and_then(Value::as_object) else {
        return Vec::new();
    };

    if env
        .get(env_name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        vec![format!(
            "{} (env.{})",
            display_path(&settings_path),
            env_name
        )]
    } else {
        Vec::new()
    }
}

fn collect_codex_sources(env_name: &str, home_dir: &Path) -> Vec<String> {
    if !env_name.eq_ignore_ascii_case("OPENAI_API_KEY") {
        return Vec::new();
    }

    let auth_path = home_dir.join(".codex").join("auth.json");
    let Some(root) = read_json_file(&auth_path) else {
        return Vec::new();
    };

    if root
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        vec![format!("{} (OPENAI_API_KEY)", display_path(&auth_path))]
    } else {
        Vec::new()
    }
}

fn collect_gemini_sources(env_name: &str, home_dir: &Path) -> Vec<String> {
    let env_path = home_dir.join(".gemini").join(".env");
    let entries = read_env_file(&env_path);
    if entries
        .iter()
        .any(|(key, value)| key.eq_ignore_ascii_case(env_name) && !value.trim().is_empty())
    {
        vec![format!("{} ({})", display_path(&env_path), env_name)]
    } else {
        Vec::new()
    }
}

fn read_json_file(path: &Path) -> Option<Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn read_env_file(path: &Path) -> Vec<(String, String)> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    content.lines().filter_map(parse_env_assignment).collect()
}

fn parse_env_assignment(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let without_export = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, value) = without_export.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    let value = value.trim();
    Some((key.to_string(), unquote_env_value(value)))
}

fn unquote_env_value(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        value.to_string()
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{collect_env_sources_with_home, parse_env_assignment};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("ai-singularity-env-test-{nanos}"))
    }

    #[test]
    fn parse_env_assignment_supports_plain_and_export_lines() {
        assert_eq!(
            parse_env_assignment("GEMINI_API_KEY=abc"),
            Some(("GEMINI_API_KEY".to_string(), "abc".to_string()))
        );
        assert_eq!(
            parse_env_assignment("export GOOGLE_API_KEY=\"quoted value\""),
            Some(("GOOGLE_API_KEY".to_string(), "quoted value".to_string()))
        );
        assert_eq!(parse_env_assignment("# comment"), None);
    }

    #[test]
    fn collect_env_sources_detects_tool_managed_files() {
        let temp_home = unique_temp_dir();
        fs::create_dir_all(temp_home.join(".claude")).expect("create claude dir");
        fs::create_dir_all(temp_home.join(".codex")).expect("create codex dir");
        fs::create_dir_all(temp_home.join(".gemini")).expect("create gemini dir");

        fs::write(
            temp_home.join(".claude").join("settings.json"),
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-claude-123"}}"#,
        )
        .expect("write claude settings");
        fs::write(
            temp_home.join(".codex").join("auth.json"),
            r#"{"OPENAI_API_KEY":"sk-openai-123"}"#,
        )
        .expect("write codex auth");
        fs::write(
            temp_home.join(".gemini").join(".env"),
            "export GEMINI_API_KEY=sk-gemini-123\nGOOGLE_API_KEY=sk-gemini-123\n",
        )
        .expect("write gemini env");

        let claude_sources =
            collect_env_sources_with_home("ANTHROPIC_AUTH_TOKEN", Some(&temp_home));
        let codex_sources = collect_env_sources_with_home("OPENAI_API_KEY", Some(&temp_home));
        let gemini_sources = collect_env_sources_with_home("GEMINI_API_KEY", Some(&temp_home));

        assert!(claude_sources
            .iter()
            .any(|item| item.contains(".claude/settings.json")));
        assert!(codex_sources
            .iter()
            .any(|item| item.contains(".codex/auth.json")));
        assert!(gemini_sources
            .iter()
            .any(|item| item.contains(".gemini/.env")));

        fs::remove_dir_all(&temp_home).expect("cleanup temp dir");
    }
}
