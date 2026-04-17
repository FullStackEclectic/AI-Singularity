use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const ACCOUNT_GROUPS_FILE: &str = "account_groups.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub account_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct AccountGroupStore;

impl AccountGroupStore {
    pub fn list_groups(app_data_dir: &Path, valid_account_ids: &[String]) -> Result<Vec<AccountGroup>, String> {
        let mut groups = Self::load_groups(app_data_dir)?;
        let valid_ids = valid_account_ids.iter().collect::<std::collections::HashSet<_>>();
        let mut changed = false;

        for group in &mut groups {
            let before = group.account_ids.len();
            group.account_ids.retain(|id| valid_ids.contains(id));
            if group.account_ids.len() != before {
                changed = true;
                group.updated_at = chrono::Utc::now().to_rfc3339();
            }
        }

        if changed {
            Self::save_groups(app_data_dir, &groups)?;
        }

        Ok(groups)
    }

    pub fn create_group(
        app_data_dir: &Path,
        valid_account_ids: &[String],
        name: &str,
    ) -> Result<AccountGroup, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("分组名称不能为空".to_string());
        }

        let mut groups = Self::list_groups(app_data_dir, valid_account_ids)?;
        if groups
            .iter()
            .any(|group| group.name.trim().eq_ignore_ascii_case(trimmed))
        {
            return Err("已存在同名分组".to_string());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let group = AccountGroup {
            id: format!("group-{}", uuid::Uuid::new_v4()),
            name: trimmed.to_string(),
            account_ids: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        groups.push(group.clone());
        Self::save_groups(app_data_dir, &groups)?;
        Ok(group)
    }

    pub fn rename_group(
        app_data_dir: &Path,
        valid_account_ids: &[String],
        id: &str,
        name: &str,
    ) -> Result<AccountGroup, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("分组名称不能为空".to_string());
        }

        let mut groups = Self::list_groups(app_data_dir, valid_account_ids)?;
        if groups
            .iter()
            .any(|group| group.id != id && group.name.trim().eq_ignore_ascii_case(trimmed))
        {
            return Err("已存在同名分组".to_string());
        }

        let group = groups
            .iter_mut()
            .find(|group| group.id == id)
            .ok_or_else(|| "未找到对应分组".to_string())?;
        group.name = trimmed.to_string();
        group.updated_at = chrono::Utc::now().to_rfc3339();
        let updated = group.clone();
        Self::save_groups(app_data_dir, &groups)?;
        Ok(updated)
    }

    pub fn delete_group(
        app_data_dir: &Path,
        valid_account_ids: &[String],
        id: &str,
    ) -> Result<bool, String> {
        let groups = Self::list_groups(app_data_dir, valid_account_ids)?;
        let retained = groups
            .into_iter()
            .filter(|group| group.id != id)
            .collect::<Vec<_>>();
        let deleted = retained.len() < Self::load_groups(app_data_dir)?.len();
        if deleted {
            Self::save_groups(app_data_dir, &retained)?;
        }
        Ok(deleted)
    }

    pub fn assign_accounts_to_group(
        app_data_dir: &Path,
        valid_account_ids: &[String],
        group_id: &str,
        account_ids: &[String],
    ) -> Result<AccountGroup, String> {
        let mut groups = Self::list_groups(app_data_dir, valid_account_ids)?;
        let valid_set = valid_account_ids.iter().collect::<std::collections::HashSet<_>>();
        let target_ids = account_ids
            .iter()
            .filter(|id| valid_set.contains(id))
            .cloned()
            .collect::<Vec<_>>();
        if target_ids.is_empty() {
            return Err("请至少选择一个有效账号".to_string());
        }

        for group in &mut groups {
            if group.id == group_id {
                continue;
            }
            group.account_ids.retain(|id| !target_ids.iter().any(|target| target == id));
        }

        let target_group = groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| "未找到对应分组".to_string())?;
        for id in target_ids {
            if !target_group.account_ids.iter().any(|item| item == &id) {
                target_group.account_ids.push(id);
            }
        }
        target_group.updated_at = chrono::Utc::now().to_rfc3339();
        let updated = target_group.clone();
        Self::save_groups(app_data_dir, &groups)?;
        Ok(updated)
    }

    pub fn remove_accounts_from_group(
        app_data_dir: &Path,
        valid_account_ids: &[String],
        group_id: &str,
        account_ids: &[String],
    ) -> Result<AccountGroup, String> {
        let mut groups = Self::list_groups(app_data_dir, valid_account_ids)?;
        let target_group = groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| "未找到对应分组".to_string())?;

        let remove_set = account_ids.iter().collect::<std::collections::HashSet<_>>();
        target_group
            .account_ids
            .retain(|id| !remove_set.contains(id));
        target_group.updated_at = chrono::Utc::now().to_rfc3339();
        let updated = target_group.clone();
        Self::save_groups(app_data_dir, &groups)?;
        Ok(updated)
    }

    fn load_groups(app_data_dir: &Path) -> Result<Vec<AccountGroup>, String> {
        let path = file_path(app_data_dir);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("读取账号分组失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str::<Vec<AccountGroup>>(&raw)
            .map_err(|e| format!("解析账号分组失败: {}", e))
    }

    fn save_groups(app_data_dir: &Path, groups: &[AccountGroup]) -> Result<(), String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let content = serde_json::to_string_pretty(groups)
            .map_err(|e| format!("序列化账号分组失败: {}", e))?;
        fs::write(file_path(app_data_dir), content)
            .map_err(|e| format!("写入账号分组失败: {}", e))
    }
}

fn file_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(ACCOUNT_GROUPS_FILE)
}
