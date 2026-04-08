use crate::error::AppResult;
use crate::models::{ProviderConfig, Platform, ProviderCategory};
use crate::db::Database;
use crate::services::sync::SyncService;
use rusqlite::params;
use chrono::Utc;

pub struct ProviderService<'a> {
    db: &'a Database,
}

impl<'a> ProviderService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_providers(&self) -> AppResult<Vec<ProviderConfig>> {
        let sql = "SELECT id, name, platform, category, base_url, api_key_id, model_name,
                          is_active, tool_targets, icon, icon_color, website_url, api_key_url,
                          notes, extra_config, created_at, updated_at, sort_order
                   FROM providers
                   ORDER BY sort_order ASC, created_at ASC";
        self.db.query_rows(sql, &[], |row| {
            let platform_str: String = row.get(2)?;
            let platform = serde_json::from_str(&format!("\"{}\"", platform_str))
                .unwrap_or(Platform::Custom);
            let category_str: Option<String> = row.get(3)?;
            let category = category_str.as_deref().and_then(|s| {
                serde_json::from_str(&format!("\"{}\"", s)).ok()
            });
            let created_at_str: String = row.get(15)?;
            let updated_at_str: String = row.get(16)?;

            Ok(ProviderConfig {
                id:           row.get(0)?,
                name:         row.get(1)?,
                platform,
                category,
                base_url:     row.get(4)?,
                api_key_id:   row.get(5)?,
                model_name:   row.get(6).unwrap_or_default(),
                is_active:    row.get::<_, i32>(7)? != 0,
                tool_targets: row.get(8)?,
                icon:         row.get(9)?,
                icon_color:   row.get(10)?,
                website_url:  row.get(11)?,
                api_key_url:  row.get(12)?,
                notes:        row.get(13)?,
                extra_config: row.get(14)?,
                sort_order:   row.get(17).unwrap_or(0),
                created_at:   created_at_str.parse().unwrap_or_else(|_| Utc::now()),
                updated_at:   updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
            })
        }).map_err(Into::into)
    }

    pub fn add_provider(&self, mut provider: ProviderConfig) -> AppResult<()> {
        let now = Utc::now();
        provider.created_at = now;
        provider.updated_at = now;

        let platform_str = serde_json::to_string(&provider.platform)
            .unwrap().trim_matches('"').to_string();
        let category_str = provider.category.as_ref().and_then(|c| {
            serde_json::to_string(c).ok()
                .map(|s| s.trim_matches('"').to_string())
        });

        self.db.execute(
            "INSERT INTO providers
             (id, name, platform, category, base_url, api_key_id, model_name,
              is_active, tool_targets, icon, icon_color, website_url, api_key_url,
              notes, extra_config, sort_order, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                provider.id,
                provider.name,
                platform_str,
                category_str,
                provider.base_url,
                provider.api_key_id,
                provider.model_name,
                if provider.is_active { 1 } else { 0 },
                provider.tool_targets,
                provider.icon,
                provider.icon_color,
                provider.website_url,
                provider.api_key_url,
                provider.notes,
                provider.extra_config,
                provider.sort_order,
                provider.created_at.to_rfc3339(),
                provider.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 切换激活 Provider（仅同步目标工具相关的 provider 互斥）
    pub fn switch_provider(&self, id: &str) -> AppResult<()> {
        let all_providers = self.list_providers()?;
        if let Some(activating_p) = all_providers.iter().find(|p| p.id == id) {
            let my_targets = activating_p.parsed_tool_targets();
            for p in &all_providers {
                if p.id != id && p.is_active {
                    let their_targets = p.parsed_tool_targets();
                    let overlap = my_targets.iter().any(|t| their_targets.contains(t));
                    if overlap {
                        self.db.execute("UPDATE providers SET is_active = 0 WHERE id = ?1", params![p.id])?;
                    }
                }
            }
            self.db.execute("UPDATE providers SET is_active = 1 WHERE id = ?1", params![id])?;
        }

        SyncService::new(self.db).sync_all();
        Ok(())
    }

    pub fn delete_provider(&self, id: &str) -> AppResult<()> {
        self.db.execute("DELETE FROM providers WHERE id = ?1", &[&id])?;
        SyncService::new(self.db).sync_all();
        Ok(())
    }

    /// 更新 Provider 信息
    pub fn update_provider(&self, provider: ProviderConfig) -> AppResult<()> {
        let platform_str = serde_json::to_string(&provider.platform)
            .unwrap().trim_matches('"').to_string();
        let category_str = provider.category.as_ref().and_then(|c| {
            serde_json::to_string(c).ok()
                .map(|s| s.trim_matches('"').to_string())
        });

        self.db.execute(
            "UPDATE providers SET
                name=?2, platform=?3, category=?4, base_url=?5, api_key_id=?6,
                model_name=?7, tool_targets=?8, icon=?9, icon_color=?10,
                website_url=?11, api_key_url=?12, notes=?13, extra_config=?14,
                updated_at=?15
             WHERE id=?1",
            params![
                provider.id,
                provider.name,
                platform_str,
                category_str,
                provider.base_url,
                provider.api_key_id,
                provider.model_name,
                provider.tool_targets,
                provider.icon,
                provider.icon_color,
                provider.website_url,
                provider.api_key_url,
                provider.notes,
                provider.extra_config,
                Utc::now().to_rfc3339(),
            ],
        )?;

        SyncService::new(self.db).sync_all();
        Ok(())
    }

    /// 重新排列 Provider
    pub fn reorder_providers(&self, ordered_ids: Vec<String>) -> AppResult<()> {
        for (index, id) in ordered_ids.iter().enumerate() {
            self.db.execute(
                "UPDATE providers SET sort_order = ?1 WHERE id = ?2",
                &[&(index as i32), id],
            )?;
        }
        Ok(())
    }
}
