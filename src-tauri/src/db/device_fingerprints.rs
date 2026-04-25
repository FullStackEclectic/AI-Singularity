use super::Database;
use chrono::Utc;
use rusqlite::Result as SqlResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFingerprintRecord {
    pub id: String,
    pub name: String,
    pub machine_id: String,
    pub mac_machine_id: String,
    pub dev_device_id: String,
    pub sqm_id: String,
    pub service_machine_id: Option<String>,
    pub created_at: String,
}

fn map_fingerprint_row(row: &rusqlite::Row<'_>) -> SqlResult<DeviceFingerprintRecord> {
    Ok(DeviceFingerprintRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        machine_id: row.get(2)?,
        mac_machine_id: row.get(3)?,
        dev_device_id: row.get(4)?,
        sqm_id: row.get(5)?,
        service_machine_id: row.get(6)?,
        created_at: row.get(7)?,
    })
}

impl Database {
    pub fn list_device_fingerprints(&self) -> SqlResult<Vec<DeviceFingerprintRecord>> {
        self.query_rows(
            "SELECT id, name, machine_id, mac_machine_id, dev_device_id, sqm_id, service_machine_id, created_at FROM device_fingerprints ORDER BY created_at ASC",
            &[],
            map_fingerprint_row,
        )
    }

    pub fn get_device_fingerprint(&self, id: &str) -> SqlResult<Option<DeviceFingerprintRecord>> {
        let res = self
            .query_row(
                "SELECT id, name, machine_id, mac_machine_id, dev_device_id, sqm_id, service_machine_id, created_at FROM device_fingerprints WHERE id = ?1",
                &[&id],
                map_fingerprint_row,
            )
            .ok();
        Ok(res)
    }

    pub fn upsert_device_fingerprint(&self, fp: &DeviceFingerprintRecord) -> SqlResult<usize> {
        self.execute(
            "INSERT INTO device_fingerprints (id, name, machine_id, mac_machine_id, dev_device_id, sqm_id, service_machine_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                machine_id = excluded.machine_id,
                mac_machine_id = excluded.mac_machine_id,
                dev_device_id = excluded.dev_device_id,
                sqm_id = excluded.sqm_id,
                service_machine_id = excluded.service_machine_id",
            rusqlite::params![
                fp.id, fp.name, fp.machine_id, fp.mac_machine_id, fp.dev_device_id, fp.sqm_id, fp.service_machine_id, fp.created_at
            ],
        )
    }

    pub fn rename_device_fingerprint(&self, id: &str, name: &str) -> SqlResult<usize> {
        self.execute(
            "UPDATE device_fingerprints SET name = ?1 WHERE id = ?2 AND id != 'original'",
            &[&name, &id],
        )
    }

    pub fn delete_device_fingerprint(&self, id: &str) -> SqlResult<usize> {
        if id == "original" {
            return Ok(0);
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE ide_accounts SET fingerprint_id = NULL WHERE fingerprint_id = ?1",
            [id],
        )?;
        let removed = tx.execute("DELETE FROM device_fingerprints WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(removed)
    }

    pub fn remember_deleted_account_fingerprint(
        &self,
        email_lower: &str,
        fingerprint_id: &str,
    ) -> SqlResult<usize> {
        if email_lower.is_empty() || fingerprint_id.is_empty() {
            return Ok(0);
        }
        let now = Utc::now().to_rfc3339();
        self.execute(
            "INSERT INTO deleted_account_fingerprint_bindings (email_lower, fingerprint_id, deleted_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(email_lower) DO UPDATE SET fingerprint_id = excluded.fingerprint_id, deleted_at = excluded.deleted_at",
            &[&email_lower, &fingerprint_id, &now],
        )
    }

    pub fn lookup_deleted_account_fingerprint(
        &self,
        email_lower: &str,
    ) -> SqlResult<Option<String>> {
        if email_lower.is_empty() {
            return Ok(None);
        }
        let res = self
            .query_row(
                "SELECT fingerprint_id FROM deleted_account_fingerprint_bindings WHERE email_lower = ?1",
                &[&email_lower],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(res)
    }

    pub fn forget_deleted_account_fingerprint(&self, email_lower: &str) -> SqlResult<usize> {
        self.execute(
            "DELETE FROM deleted_account_fingerprint_bindings WHERE email_lower = ?1",
            &[&email_lower],
        )
    }
}
