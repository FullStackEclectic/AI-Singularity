use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::services::codex_shared::inspect_instance_shared_resources;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexInstanceRecord {
    pub id: String,
    pub name: String,
    pub user_data_dir: String,
    #[serde(default)]
    pub extra_args: String,
    #[serde(default)]
    pub bind_account_id: Option<String>,
    #[serde(default)]
    pub bind_provider_id: Option<String>,
    #[serde(default)]
    pub last_pid: Option<u32>,
    #[serde(default)]
    pub last_launched_at: Option<String>,
    #[serde(default)]
    pub has_state_db: bool,
    #[serde(default)]
    pub has_session_index: bool,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub follow_local_account: bool,
    #[serde(default)]
    pub has_shared_skills: bool,
    #[serde(default)]
    pub has_shared_rules: bool,
    #[serde(default)]
    pub has_shared_vendor_imports_skills: bool,
    #[serde(default)]
    pub has_shared_agents_file: bool,
    #[serde(default)]
    pub has_shared_conflicts: bool,
    #[serde(default)]
    pub shared_conflict_paths: Vec<String>,
    #[serde(default)]
    pub shared_strategy_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCodexInstanceRecord {
    pub id: String,
    pub name: String,
    pub user_data_dir: String,
    #[serde(default)]
    pub extra_args: String,
    #[serde(default)]
    pub bind_account_id: Option<String>,
    #[serde(default)]
    pub bind_provider_id: Option<String>,
    #[serde(default)]
    pub last_pid: Option<u32>,
    #[serde(default)]
    pub last_launched_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexDefaultInstanceSettings {
    #[serde(default)]
    pub extra_args: String,
    #[serde(default)]
    pub bind_account_id: Option<String>,
    #[serde(default)]
    pub bind_provider_id: Option<String>,
    #[serde(default)]
    pub follow_local_account: bool,
    #[serde(default)]
    pub last_pid: Option<u32>,
    #[serde(default)]
    pub last_launched_at: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexInstanceStoreFile {
    #[serde(default)]
    pub instances: Vec<StoredCodexInstanceRecord>,
    #[serde(default)]
    pub default_settings: CodexDefaultInstanceSettings,
}

pub struct CodexInstanceStore;

impl CodexInstanceStore {
    fn store_path() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let dir = home.join(".ai-singularity");
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
        }
        Ok(dir.join("codex_instances.json"))
    }

    fn load_file() -> Result<CodexInstanceStoreFile, String> {
        let path = Self::store_path()?;
        if !path.exists() {
            return Ok(CodexInstanceStoreFile::default());
        }
        let content =
            fs::read_to_string(&path).map_err(|e| format!("读取 Codex 实例配置失败: {}", e))?;
        if content.trim().is_empty() {
            return Ok(CodexInstanceStoreFile::default());
        }
        serde_json::from_str(&content).map_err(|e| format!("解析 Codex 实例配置失败: {}", e))
    }

    fn save_file(data: &CodexInstanceStoreFile) -> Result<(), String> {
        let path = Self::store_path()?;
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| format!("序列化 Codex 实例配置失败: {}", e))?;
        fs::write(path, format!("{}\n", content))
            .map_err(|e| format!("写入 Codex 实例配置失败: {}", e))
    }

    fn build_runtime_record(
        id: String,
        name: String,
        user_data_dir: String,
        extra_args: String,
        bind_account_id: Option<String>,
        bind_provider_id: Option<String>,
        last_pid: Option<u32>,
        last_launched_at: Option<String>,
        is_default: bool,
        follow_local_account: bool,
    ) -> CodexInstanceRecord {
        let dir = PathBuf::from(&user_data_dir);
        let shared = inspect_instance_shared_resources(&dir);
        CodexInstanceRecord {
            id,
            name,
            user_data_dir,
            extra_args,
            bind_account_id,
            bind_provider_id,
            last_pid,
            last_launched_at,
            has_state_db: dir.join("state_5.sqlite").exists(),
            has_session_index: dir.join("session_index.jsonl").exists(),
            running: last_pid.is_some_and(crate::services::codex_runtime::is_pid_running),
            is_default,
            follow_local_account,
            has_shared_skills: shared.has_skills,
            has_shared_rules: shared.has_rules,
            has_shared_vendor_imports_skills: shared.has_vendor_imports_skills,
            has_shared_agents_file: shared.has_agents_file,
            has_shared_conflicts: shared.has_conflicts,
            shared_conflict_paths: shared.conflict_paths,
            shared_strategy_version: shared.shared_strategy_version,
        }
    }

    pub fn list_instances() -> Result<Vec<CodexInstanceRecord>, String> {
        let mut instances = Self::load_file()?
            .instances
            .into_iter()
            .map(|item| {
                Self::build_runtime_record(
                    item.id,
                    item.name,
                    item.user_data_dir,
                    item.extra_args,
                    item.bind_account_id,
                    item.bind_provider_id,
                    item.last_pid,
                    item.last_launched_at,
                    false,
                    false,
                )
            })
            .collect::<Vec<_>>();
        instances.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(instances)
    }

    pub fn get_default_instance() -> Result<CodexInstanceRecord, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let file = Self::load_file()?;
        let default_dir = home.join(".codex").to_string_lossy().to_string();
        Ok(Self::build_runtime_record(
            "__default__".to_string(),
            "默认实例".to_string(),
            default_dir,
            file.default_settings.extra_args,
            file.default_settings.bind_account_id,
            file.default_settings.bind_provider_id,
            file.default_settings.last_pid,
            file.default_settings.last_launched_at,
            true,
            file.default_settings.follow_local_account,
        ))
    }

    pub fn add_instance(name: String, user_data_dir: String) -> Result<CodexInstanceRecord, String> {
        let trimmed_name = name.trim();
        let trimmed_dir = user_data_dir.trim();
        if trimmed_name.is_empty() {
            return Err("实例名称不能为空".to_string());
        }
        if trimmed_dir.is_empty() {
            return Err("实例目录不能为空".to_string());
        }

        let dir = PathBuf::from(trimmed_dir);
        if !dir.exists() {
            return Err(format!("实例目录不存在: {}", dir.display()));
        }
        if !dir.join("state_5.sqlite").exists() {
            return Err(format!(
                "该目录不是有效的 Codex 实例目录，缺少 state_5.sqlite: {}",
                dir.display()
            ));
        }

        let normalized_dir = dir.to_string_lossy().to_string();
        let mut file = Self::load_file()?;
        if file
            .instances
            .iter()
            .any(|item| item.user_data_dir.eq_ignore_ascii_case(&normalized_dir))
        {
            return Err("该 Codex 实例目录已经存在".to_string());
        }

        let stored = StoredCodexInstanceRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: trimmed_name.to_string(),
            user_data_dir: normalized_dir.clone(),
            extra_args: String::new(),
            bind_account_id: None,
            bind_provider_id: None,
            last_pid: None,
            last_launched_at: None,
        };
        file.instances.push(stored.clone());
        Self::save_file(&file)?;
        Ok(Self::build_runtime_record(
            stored.id,
            stored.name,
            stored.user_data_dir,
            stored.extra_args,
            stored.bind_account_id,
            stored.bind_provider_id,
            stored.last_pid,
            stored.last_launched_at,
            false,
            false,
        ))
    }

    pub fn delete_instance(id: &str) -> Result<(), String> {
        let mut file = Self::load_file()?;
        let before = file.instances.len();
        file.instances.retain(|item| item.id != id);
        if before == file.instances.len() {
            return Err("未找到对应的 Codex 实例".to_string());
        }
        Self::save_file(&file)
    }

    pub fn update_instance_settings(
        id: &str,
        extra_args: Option<String>,
        bind_account_id: Option<Option<String>>,
        bind_provider_id: Option<Option<String>>,
    ) -> Result<CodexInstanceRecord, String> {
        let mut file = Self::load_file()?;
        let updated = {
            let instance = file
                .instances
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("未找到对应的 Codex 实例".to_string())?;

            if let Some(extra_args) = extra_args {
                instance.extra_args = extra_args.trim().to_string();
            }
            if let Some(bind_account_id) = bind_account_id {
                instance.bind_account_id = bind_account_id.filter(|item| !item.trim().is_empty());
            }
            if let Some(bind_provider_id) = bind_provider_id {
                instance.bind_provider_id = bind_provider_id.filter(|item| !item.trim().is_empty());
            }
            instance.clone()
        };
        Self::save_file(&file)?;
        Ok(Self::build_runtime_record(
            updated.id,
            updated.name,
            updated.user_data_dir,
            updated.extra_args,
            updated.bind_account_id,
            updated.bind_provider_id,
            updated.last_pid,
            updated.last_launched_at,
            false,
            false,
        ))
    }

    pub fn update_default_settings(
        extra_args: Option<String>,
        bind_account_id: Option<Option<String>>,
        bind_provider_id: Option<Option<String>>,
        follow_local_account: Option<bool>,
    ) -> Result<CodexInstanceRecord, String> {
        let mut file = Self::load_file()?;
        if let Some(extra_args) = extra_args {
            file.default_settings.extra_args = extra_args.trim().to_string();
        }
        if let Some(bind_account_id) = bind_account_id {
            file.default_settings.bind_account_id =
                bind_account_id.filter(|item| !item.trim().is_empty());
        }
        if let Some(bind_provider_id) = bind_provider_id {
            file.default_settings.bind_provider_id =
                bind_provider_id.filter(|item| !item.trim().is_empty());
        }
        if let Some(follow_local_account) = follow_local_account {
            file.default_settings.follow_local_account = follow_local_account;
            if follow_local_account {
                file.default_settings.bind_account_id = None;
            }
        }
        Self::save_file(&file)?;
        Self::get_default_instance()
    }

    pub fn set_instance_pid(id: &str, pid: Option<u32>) -> Result<CodexInstanceRecord, String> {
        let mut file = Self::load_file()?;
        let updated = {
            let instance = file
                .instances
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("未找到对应的 Codex 实例".to_string())?;
            instance.last_pid = pid;
            instance.last_launched_at = pid.map(|_| Utc::now().to_rfc3339());
            instance.clone()
        };
        Self::save_file(&file)?;
        Ok(Self::build_runtime_record(
            updated.id,
            updated.name,
            updated.user_data_dir,
            updated.extra_args,
            updated.bind_account_id,
            updated.bind_provider_id,
            updated.last_pid,
            updated.last_launched_at,
            false,
            false,
        ))
    }

    pub fn set_default_pid(pid: Option<u32>) -> Result<CodexInstanceRecord, String> {
        let mut file = Self::load_file()?;
        file.default_settings.last_pid = pid;
        file.default_settings.last_launched_at = pid.map(|_| Utc::now().to_rfc3339());
        Self::save_file(&file)?;
        Self::get_default_instance()
    }

    pub fn clear_all_pids() -> Result<(), String> {
        let mut file = Self::load_file()?;
        file.default_settings.last_pid = None;
        for item in &mut file.instances {
            item.last_pid = None;
        }
        Self::save_file(&file)
    }
}
