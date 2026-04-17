use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiInstanceRecord {
    pub id: String,
    pub name: String,
    pub user_data_dir: String,
    #[serde(default)]
    pub extra_args: String,
    #[serde(default)]
    pub bind_account_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub last_launched_at: Option<String>,
    #[serde(default)]
    pub initialized: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub follow_local_account: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredGeminiInstanceRecord {
    pub id: String,
    pub name: String,
    pub user_data_dir: String,
    #[serde(default)]
    pub extra_args: String,
    #[serde(default)]
    pub bind_account_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub last_launched_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiDefaultInstanceSettings {
    #[serde(default)]
    pub extra_args: String,
    #[serde(default)]
    pub bind_account_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub last_launched_at: Option<String>,
    #[serde(default)]
    pub follow_local_account: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiInstanceStoreFile {
    #[serde(default)]
    pub instances: Vec<StoredGeminiInstanceRecord>,
    #[serde(default)]
    pub default_settings: GeminiDefaultInstanceSettings,
}

pub struct GeminiInstanceStore;

impl GeminiInstanceStore {
    fn store_path() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let dir = home.join(".ai-singularity");
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
        }
        Ok(dir.join("gemini_instances.json"))
    }

    fn load_file() -> Result<GeminiInstanceStoreFile, String> {
        let path = Self::store_path()?;
        if !path.exists() {
            return Ok(GeminiInstanceStoreFile::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取 Gemini 实例配置失败: {}", e))?;
        if content.trim().is_empty() {
            return Ok(GeminiInstanceStoreFile::default());
        }
        serde_json::from_str(&content).map_err(|e| format!("解析 Gemini 实例配置失败: {}", e))
    }

    fn save_file(data: &GeminiInstanceStoreFile) -> Result<(), String> {
        let path = Self::store_path()?;
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| format!("序列化 Gemini 实例配置失败: {}", e))?;
        fs::write(path, format!("{}\n", content))
            .map_err(|e| format!("写入 Gemini 实例配置失败: {}", e))
    }

    fn is_initialized(user_data_dir: &str) -> bool {
        PathBuf::from(user_data_dir).join(".gemini").join("oauth_creds.json").exists()
    }

    fn build_runtime_record(
        id: String,
        name: String,
        user_data_dir: String,
        extra_args: String,
        bind_account_id: Option<String>,
        project_id: Option<String>,
        last_launched_at: Option<String>,
        is_default: bool,
        follow_local_account: bool,
    ) -> GeminiInstanceRecord {
        GeminiInstanceRecord {
            id,
            name,
            initialized: Self::is_initialized(&user_data_dir),
            user_data_dir,
            extra_args,
            bind_account_id,
            project_id,
            last_launched_at,
            is_default,
            follow_local_account,
        }
    }

    pub fn list_instances() -> Result<Vec<GeminiInstanceRecord>, String> {
        let mut items = Self::load_file()?
            .instances
            .into_iter()
            .map(|item| {
                Self::build_runtime_record(
                    item.id,
                    item.name,
                    item.user_data_dir,
                    item.extra_args,
                    item.bind_account_id,
                    item.project_id,
                    item.last_launched_at,
                    false,
                    false,
                )
            })
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(items)
    }

    pub fn get_default_instance() -> Result<GeminiInstanceRecord, String> {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        let file = Self::load_file()?;
        Ok(Self::build_runtime_record(
            "__default__".to_string(),
            "默认实例".to_string(),
            home.to_string_lossy().to_string(),
            file.default_settings.extra_args,
            file.default_settings.bind_account_id,
            file.default_settings.project_id,
            file.default_settings.last_launched_at,
            true,
            file.default_settings.follow_local_account,
        ))
    }

    pub fn add_instance(name: String, user_data_dir: String) -> Result<GeminiInstanceRecord, String> {
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

        let normalized_dir = dir.to_string_lossy().to_string();
        let mut file = Self::load_file()?;
        if file
            .instances
            .iter()
            .any(|item| item.user_data_dir.eq_ignore_ascii_case(&normalized_dir))
        {
            return Err("该 Gemini 实例目录已经存在".to_string());
        }

        let stored = StoredGeminiInstanceRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: trimmed_name.to_string(),
            user_data_dir: normalized_dir.clone(),
            extra_args: String::new(),
            bind_account_id: None,
            project_id: None,
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
            stored.project_id,
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
            return Err("未找到对应的 Gemini 实例".to_string());
        }
        Self::save_file(&file)
    }

    pub fn update_instance_settings(
        id: &str,
        extra_args: Option<String>,
        bind_account_id: Option<Option<String>>,
        project_id: Option<Option<String>>,
    ) -> Result<GeminiInstanceRecord, String> {
        let mut file = Self::load_file()?;
        let updated = {
            let instance = file
                .instances
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("未找到对应的 Gemini 实例".to_string())?;
            if let Some(extra_args) = extra_args {
                instance.extra_args = extra_args.trim().to_string();
            }
            if let Some(bind_account_id) = bind_account_id {
                instance.bind_account_id = bind_account_id.filter(|item| !item.trim().is_empty());
            }
            if let Some(project_id) = project_id {
                instance.project_id = project_id.filter(|item| !item.trim().is_empty());
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
            updated.project_id,
            updated.last_launched_at,
            false,
            false,
        ))
    }

    pub fn update_default_settings(
        extra_args: Option<String>,
        bind_account_id: Option<Option<String>>,
        project_id: Option<Option<String>>,
        follow_local_account: Option<bool>,
    ) -> Result<GeminiInstanceRecord, String> {
        let mut file = Self::load_file()?;
        if let Some(extra_args) = extra_args {
            file.default_settings.extra_args = extra_args.trim().to_string();
        }
        if let Some(bind_account_id) = bind_account_id {
            file.default_settings.bind_account_id =
                bind_account_id.filter(|item| !item.trim().is_empty());
        }
        if let Some(project_id) = project_id {
            file.default_settings.project_id = project_id.filter(|item| !item.trim().is_empty());
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

    pub fn update_last_launched(id: &str) -> Result<GeminiInstanceRecord, String> {
        if id == "__default__" {
            let mut file = Self::load_file()?;
            file.default_settings.last_launched_at = Some(Utc::now().to_rfc3339());
            Self::save_file(&file)?;
            return Self::get_default_instance();
        }

        let mut file = Self::load_file()?;
        let updated = {
            let instance = file
                .instances
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("未找到对应的 Gemini 实例".to_string())?;
            instance.last_launched_at = Some(Utc::now().to_rfc3339());
            instance.clone()
        };
        Self::save_file(&file)?;
        Ok(Self::build_runtime_record(
            updated.id,
            updated.name,
            updated.user_data_dir,
            updated.extra_args,
            updated.bind_account_id,
            updated.project_id,
            updated.last_launched_at,
            false,
            false,
        ))
    }
}
