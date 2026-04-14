use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "windows"))]
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvConflict {
    pub var_name: String,
    pub var_value: String,
    pub source_type: String, // "system" | "file"
    pub source_path: String, // Registry path or file path
}

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

pub struct EnvChecker;

impl EnvChecker {
    /// 检查指定工具可能导致系统变量越权覆盖的隐患
    pub fn check_env_conflicts(app: &str) -> Vec<EnvConflict> {
        let keywords = Self::get_keywords_for_app(app);
        let mut conflicts = Vec::new();

        // 检查系统全局或者当前进程环境变量中的强覆盖
        if let Ok(sys_conflicts) = Self::check_system_env(&keywords) {
            conflicts.extend(sys_conflicts);
        }

        // Unix 环境下还需要扫描 Bash 和 Zsh profile 中的硬编码
        #[cfg(not(target_os = "windows"))]
        if let Ok(shell_conflicts) = Self::check_shell_configs(&keywords) {
            conflicts.extend(shell_conflicts);
        }

        conflicts
    }

    /// 根据平台列出潜在的涉险密钥变量
    fn get_keywords_for_app(app: &str) -> Vec<&'static str> {
        match app.to_lowercase().as_str() {
            "claude" => vec!["ANTHROPIC_API_KEY", "CLAUDE_API_KEY"],
            "openai" | "codex" => vec!["OPENAI_API_KEY"],
            "gemini" => vec!["GEMINI_API_KEY", "GOOGLE_API_KEY"],
            "deepseek" => vec!["DEEPSEEK_API_KEY"],
            _ => vec![], // 空代表全局扫描所有可疑词？但这里为了克制，只要没匹配上就不搜。为了更广可以考虑全扫
        }
    }

    #[cfg(target_os = "windows")]
    fn check_system_env(keywords: &[&str]) -> Result<Vec<EnvConflict>, String> {
        let mut conflicts = Vec::new();

        // 1. 用户级环境变量 (HKEY_CURRENT_USER\\Environment)
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
            for (name, value) in hkcu.enum_values().filter_map(Result::ok) {
                if keywords.iter().any(|&k| name.to_uppercase() == k) {
                    conflicts.push(EnvConflict {
                        var_name: name.clone(),
                        var_value: value.to_string(), // 注意，实际展示在UI时可能需要打码打星号
                        source_type: "system(user)".to_string(),
                        source_path: "HKEY_CURRENT_USER\\Environment".to_string(),
                    });
                }
            }
        }

        // 2. 系统级环境变量 (HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment)
        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
        {
            for (name, value) in hklm.enum_values().filter_map(Result::ok) {
                if keywords.iter().any(|&k| name.to_uppercase() == k) {
                    conflicts.push(EnvConflict {
                        var_name: name.clone(),
                        var_value: value.to_string(),
                        source_type: "system(machine)".to_string(),
                        source_path: "HKEY_LOCAL_MACHINE\\SYSTEM\\...\\Environment".to_string(),
                    });
                }
            }
        }

        Ok(conflicts)
    }

    #[cfg(not(target_os = "windows"))]
    fn check_system_env(keywords: &[&str]) -> Result<Vec<EnvConflict>, String> {
        let mut conflicts = Vec::new();

        // unix 直接从当前进程继承的 env 里面找
        for (key, value) in std::env::vars() {
            if keywords.iter().any(|&k| key.to_uppercase() == k) {
                conflicts.push(EnvConflict {
                    var_name: key,
                    var_value: value,
                    source_type: "system".to_string(),
                    source_path: "Process Environment".to_string(),
                });
            }
        }

        Ok(conflicts)
    }

    #[cfg(not(target_os = "windows"))]
    fn check_shell_configs(keywords: &[&str]) -> Result<Vec<EnvConflict>, String> {
        let mut conflicts = Vec::new();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

        // 激进狂扫所有常规配置
        let config_files = vec![
            format!("{}/.bashrc", home),
            format!("{}/.bash_profile", home),
            format!("{}/.zshrc", home),
            format!("{}/.zprofile", home),
            format!("{}/.profile", home),
        ];

        for file_path in config_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (line_num, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("export ")
                        || (!trimmed.starts_with('#') && trimmed.contains('='))
                    {
                        let export_line = trimmed.strip_prefix("export ").unwrap_or(trimmed);

                        if let Some(eq_pos) = export_line.find('=') {
                            let var_name = export_line[..eq_pos].trim();
                            let var_value = export_line[eq_pos + 1..].trim();

                            if keywords.iter().any(|&k| var_name.to_uppercase() == k) {
                                conflicts.push(EnvConflict {
                                    var_name: var_name.to_string(),
                                    var_value: var_value
                                        .trim_matches('"')
                                        .trim_matches('\'')
                                        .to_string(),
                                    source_type: "file(shell)".to_string(),
                                    source_path: format!("{}:{}", file_path, line_num + 1),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }
}
