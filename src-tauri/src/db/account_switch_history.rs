use super::Database;
use rusqlite::Result as SqlResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSwitchHistoryItem {
    pub id: String,
    pub ts: String,
    pub trigger: String,
    pub rule: Option<String>,
    pub from_account_id: Option<String>,
    pub from_email: Option<String>,
    pub to_account_id: String,
    pub to_email: String,
    pub reason_json: Option<String>,
}

fn map_history_row(row: &rusqlite::Row<'_>) -> SqlResult<AccountSwitchHistoryItem> {
    Ok(AccountSwitchHistoryItem {
        id: row.get(0)?,
        ts: row.get(1)?,
        trigger: row.get(2)?,
        rule: row.get(3)?,
        from_account_id: row.get(4)?,
        from_email: row.get(5)?,
        to_account_id: row.get(6)?,
        to_email: row.get(7)?,
        reason_json: row.get(8)?,
    })
}

impl Database {
    pub fn append_account_switch_history(
        &self,
        item: &AccountSwitchHistoryItem,
    ) -> SqlResult<usize> {
        self.execute(
            "INSERT INTO account_switch_history (id, ts, trigger, rule, from_account_id, from_email, to_account_id, to_email, reason_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                item.id, item.ts, item.trigger, item.rule, item.from_account_id, item.from_email, item.to_account_id, item.to_email, item.reason_json
            ],
        )
    }

    pub fn list_account_switch_history(
        &self,
        limit: u32,
    ) -> SqlResult<Vec<AccountSwitchHistoryItem>> {
        let limit_val = limit.max(1).min(500) as i64;
        self.query_rows(
            "SELECT id, ts, trigger, rule, from_account_id, from_email, to_account_id, to_email, reason_json
             FROM account_switch_history ORDER BY ts DESC LIMIT ?1",
            &[&limit_val],
            map_history_row,
        )
    }
}
