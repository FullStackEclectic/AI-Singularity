use crate::db::Database;
use crate::error::AppResult;
use crate::models::{Platform, ProviderConfig};
use crate::services::sync::SyncService;
use chrono::Utc;
use rusqlite::params;

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
        self.db
            .query_rows(sql, &[], |row| {
                let platform_str: String = row.get(2)?;
                let platform = serde_json::from_str(&format!("\"{}\"", platform_str))
                    .unwrap_or(Platform::Custom);
                let category_str: Option<String> = row.get(3)?;
                let category = category_str
                    .as_deref()
                    .and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok());
                let created_at_str: String = row.get(15)?;
                let updated_at_str: String = row.get(16)?;

                Ok(ProviderConfig {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    platform,
                    category,
                    base_url: row.get(4)?,
                    api_key_id: row.get(5)?,
                    model_name: row.get(6).unwrap_or_default(),
                    is_active: row.get::<_, i32>(7)? != 0,
                    tool_targets: row.get(8)?,
                    icon: row.get(9)?,
                    icon_color: row.get(10)?,
                    website_url: row.get(11)?,
                    api_key_url: row.get(12)?,
                    notes: row.get(13)?,
                    extra_config: row.get(14)?,
                    sort_order: row.get(17).unwrap_or(0),
                    created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
                    updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(Into::into)
    }

    pub fn add_provider(&self, mut provider: ProviderConfig) -> AppResult<()> {
        let now = Utc::now();
        provider.created_at = now;
        provider.updated_at = now;

        let platform_str = serde_json::to_string(&provider.platform)
            .unwrap()
            .trim_matches('"')
            .to_string();
        let category_str = provider.category.as_ref().and_then(|c| {
            serde_json::to_string(c)
                .ok()
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
                        self.db.execute(
                            "UPDATE providers SET is_active = 0 WHERE id = ?1",
                            params![p.id],
                        )?;
                    }
                }
            }
            self.db.execute(
                "UPDATE providers SET is_active = 1 WHERE id = ?1",
                params![id],
            )?;
        }

        SyncService::new(self.db).sync_all();
        Ok(())
    }

    pub fn delete_provider(&self, id: &str) -> AppResult<()> {
        self.db
            .execute("DELETE FROM providers WHERE id = ?1", &[&id])?;
        SyncService::new(self.db).sync_all();
        Ok(())
    }

    /// 更新 Provider 信息
    pub fn update_provider(&self, provider: ProviderConfig) -> AppResult<()> {
        let platform_str = serde_json::to_string(&provider.platform)
            .unwrap()
            .trim_matches('"')
            .to_string();
        let category_str = provider.category.as_ref().and_then(|c| {
            serde_json::to_string(c)
                .ok()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::Platform;
    use chrono::Utc;
    use std::path::Path;

    fn make_db() -> Database {
        Database::new(Path::new(":memory:")).expect("open in-memory db")
    }

    fn sample_provider(id: &str, name: &str, tool_targets: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            name: name.to_string(),
            platform: Platform::OpenAI,
            category: None,
            base_url: None,
            api_key_id: None,
            model_name: "gpt-4o".to_string(),
            is_active: false,
            tool_targets: tool_targets.map(|s| s.to_string()),
            icon: None,
            icon_color: None,
            website_url: None,
            api_key_url: None,
            notes: None,
            extra_config: None,
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn add_and_list_provider() {
        let db = make_db();
        let svc = ProviderService::new(&db);
        svc.add_provider(sample_provider("p1", "Provider One", None)).unwrap();
        svc.add_provider(sample_provider("p2", "Provider Two", None)).unwrap();
        let list = svc.list_providers().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|p| p.id == "p1"));
        assert!(list.iter().any(|p| p.id == "p2"));
    }

    #[test]
    fn switch_provider_deactivates_overlapping() {
        let db = make_db();
        let svc = ProviderService::new(&db);

        // Both providers share the same tool_target: claude_code
        let mut p1 = sample_provider("p1", "Provider One", Some(r#"["claude_code"]"#));
        p1.is_active = true;
        svc.add_provider(p1).unwrap();

        let p2 = sample_provider("p2", "Provider Two", Some(r#"["claude_code"]"#));
        svc.add_provider(p2).unwrap();

        // Switch to p2 — p1 should be deactivated because they share claude_code
        svc.switch_provider("p2").unwrap();

        let list = svc.list_providers().unwrap();
        let p1_entry = list.iter().find(|p| p.id == "p1").unwrap();
        let p2_entry = list.iter().find(|p| p.id == "p2").unwrap();
        assert!(!p1_entry.is_active, "p1 should be deactivated after switching to p2");
        assert!(p2_entry.is_active, "p2 should be active after switch");
    }

    #[test]
    fn switch_provider_allows_different_targets() {
        let db = make_db();
        let svc = ProviderService::new(&db);

        // p1 targets claude_code, p2 targets codex — no overlap
        let mut p1 = sample_provider("p1", "Provider One", Some(r#"["claude_code"]"#));
        p1.is_active = true;
        svc.add_provider(p1).unwrap();

        let p2 = sample_provider("p2", "Provider Two", Some(r#"["codex"]"#));
        svc.add_provider(p2).unwrap();

        // Switch to p2 — p1 should remain active because targets don't overlap
        svc.switch_provider("p2").unwrap();

        let list = svc.list_providers().unwrap();
        let p1_entry = list.iter().find(|p| p.id == "p1").unwrap();
        let p2_entry = list.iter().find(|p| p.id == "p2").unwrap();
        assert!(p1_entry.is_active, "p1 should remain active (different target)");
        assert!(p2_entry.is_active, "p2 should be active after switch");
    }

    #[test]
    fn delete_provider_removes_entry() {
        let db = make_db();
        let svc = ProviderService::new(&db);
        svc.add_provider(sample_provider("p1", "Provider One", None)).unwrap();
        svc.delete_provider("p1").unwrap();
        let list = svc.list_providers().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn update_provider_changes_name() {
        let db = make_db();
        let svc = ProviderService::new(&db);
        svc.add_provider(sample_provider("p1", "Original Name", None)).unwrap();

        let mut updated = sample_provider("p1", "Updated Name", None);
        updated.model_name = "gpt-4o-mini".to_string();
        svc.update_provider(updated).unwrap();

        let list = svc.list_providers().unwrap();
        let entry = list.iter().find(|p| p.id == "p1").unwrap();
        assert_eq!(entry.name, "Updated Name");
        assert_eq!(entry.model_name, "gpt-4o-mini");
    }

    #[test]
    fn reorder_providers_updates_sort_order() {
        let db = make_db();
        let svc = ProviderService::new(&db);

        let mut p1 = sample_provider("p1", "Provider One", None);
        p1.sort_order = 0;
        let mut p2 = sample_provider("p2", "Provider Two", None);
        p2.sort_order = 1;
        let mut p3 = sample_provider("p3", "Provider Three", None);
        p3.sort_order = 2;

        svc.add_provider(p1).unwrap();
        svc.add_provider(p2).unwrap();
        svc.add_provider(p3).unwrap();

        // Reverse the order: p3, p1, p2
        svc.reorder_providers(vec!["p3".to_string(), "p1".to_string(), "p2".to_string()]).unwrap();

        let list = svc.list_providers().unwrap();
        let get_order = |id: &str| list.iter().find(|p| p.id == id).unwrap().sort_order;

        assert_eq!(get_order("p3"), 0);
        assert_eq!(get_order("p1"), 1);
        assert_eq!(get_order("p2"), 2);
    }
}
