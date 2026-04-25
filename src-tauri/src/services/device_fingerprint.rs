use crate::db::{Database, DeviceFingerprintRecord};
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

pub struct DeviceFingerprintService;

impl DeviceFingerprintService {
    pub fn list(db: &Database) -> Result<Vec<DeviceFingerprintRecord>, String> {
        db.list_device_fingerprints().map_err(|e| e.to_string())
    }

    pub fn create(
        db: &Database,
        name: &str,
        seed: Option<DeviceFingerprintRecord>,
    ) -> Result<DeviceFingerprintRecord, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("指纹名称不能为空".to_string());
        }
        let now = Utc::now().to_rfc3339();
        let fp = match seed {
            Some(s) => DeviceFingerprintRecord {
                id: Uuid::new_v4().to_string(),
                name: trimmed.to_string(),
                machine_id: s.machine_id,
                mac_machine_id: s.mac_machine_id,
                dev_device_id: s.dev_device_id,
                sqm_id: s.sqm_id,
                service_machine_id: s.service_machine_id,
                created_at: now,
            },
            None => DeviceFingerprintRecord {
                id: Uuid::new_v4().to_string(),
                name: trimmed.to_string(),
                machine_id: random_hex(64),
                mac_machine_id: random_hex(64),
                dev_device_id: random_uuid(),
                sqm_id: format!("{{{}}}", random_uuid().to_uppercase()),
                service_machine_id: Some(random_uuid()),
                created_at: now,
            },
        };
        db.upsert_device_fingerprint(&fp)
            .map_err(|e| e.to_string())?;
        Ok(fp)
    }

    pub fn rename(db: &Database, id: &str, name: &str) -> Result<usize, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("指纹名称不能为空".to_string());
        }
        if id == "original" {
            return Err("内置原始指纹不可重命名".to_string());
        }
        db.rename_device_fingerprint(id, trimmed)
            .map_err(|e| e.to_string())
    }

    pub fn delete(db: &Database, id: &str) -> Result<usize, String> {
        if id == "original" {
            return Err("内置原始指纹不可删除".to_string());
        }
        db.delete_device_fingerprint(id).map_err(|e| e.to_string())
    }

    pub fn apply_to_account(
        db: &Database,
        account_id: &str,
        fingerprint_id: Option<&str>,
    ) -> Result<usize, String> {
        if let Some(fp_id) = fingerprint_id {
            if fp_id != "original" {
                let exists = db
                    .get_device_fingerprint(fp_id)
                    .map_err(|e| e.to_string())?;
                if exists.is_none() {
                    return Err(format!("指纹 {} 不存在", fp_id));
                }
            }
        }
        db.update_ide_account_fingerprint(account_id, fingerprint_id)
            .map_err(|e| e.to_string())
    }

    pub fn remember_for_deleted_account(
        db: &Database,
        email: &str,
        fingerprint_id: Option<&str>,
    ) {
        let Some(fp_id) = fingerprint_id else {
            return;
        };
        if fp_id.is_empty() {
            return;
        }
        let lower = email.trim().to_lowercase();
        if lower.is_empty() {
            return;
        }
        if let Err(e) = db.remember_deleted_account_fingerprint(&lower, fp_id) {
            tracing::warn!(
                "[Fingerprint] 记录已删账号指纹绑定失败 email={} err={}",
                lower,
                e
            );
        }
    }

    pub fn lookup_for_email(db: &Database, email: &str) -> Option<String> {
        let lower = email.trim().to_lowercase();
        if lower.is_empty() {
            return None;
        }
        match db.lookup_deleted_account_fingerprint(&lower) {
            Ok(Some(fp_id)) => {
                // 验证指纹是否仍然存在
                match db.get_device_fingerprint(&fp_id) {
                    Ok(Some(_)) => Some(fp_id),
                    _ => {
                        let _ = db.forget_deleted_account_fingerprint(&lower);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// 将指纹写入对应 IDE 的 storage.json（目前仅支持 Antigravity）
    pub fn write_to_storage(
        platform: &str,
        fp: &DeviceFingerprintRecord,
    ) -> Result<(), String> {
        let platform_lower = platform.to_lowercase();
        if platform_lower != "antigravity" {
            return Ok(()); // 其它平台暂不写指纹
        }
        let storage_path = antigravity_storage_path()
            .ok_or_else(|| "无法定位 Antigravity storage.json".to_string())?;
        if !storage_path.exists() {
            return Err(format!("storage.json 不存在: {}", storage_path.display()));
        }
        let content = std::fs::read_to_string(&storage_path)
            .map_err(|e| format!("读取 storage.json 失败: {}", e))?;
        let mut value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("解析 storage.json 失败: {}", e))?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "telemetry.machineId".to_string(),
                serde_json::Value::String(fp.machine_id.clone()),
            );
            obj.insert(
                "telemetry.macMachineId".to_string(),
                serde_json::Value::String(fp.mac_machine_id.clone()),
            );
            obj.insert(
                "telemetry.devDeviceId".to_string(),
                serde_json::Value::String(fp.dev_device_id.clone()),
            );
            obj.insert(
                "telemetry.sqmId".to_string(),
                serde_json::Value::String(fp.sqm_id.clone()),
            );
        }
        let serialized = serde_json::to_string_pretty(&value)
            .map_err(|e| format!("序列化 storage.json 失败: {}", e))?;
        crate::atomic_write::atomic_write(&storage_path, serialized.as_bytes())
            .map_err(|e| format!("写入 storage.json 失败: {}", e))?;
        Ok(())
    }
}

fn random_hex(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len / 2)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect()
}

fn random_uuid() -> String {
    Uuid::new_v4().to_string()
}

fn antigravity_storage_path() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        return Some(
            std::path::PathBuf::from(appdata)
                .join("Antigravity")
                .join("User")
                .join("globalStorage")
                .join("storage.json"),
        );
    }
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        return Some(
            home.join("Library")
                .join("Application Support")
                .join("Antigravity")
                .join("User")
                .join("globalStorage")
                .join("storage.json"),
        );
    }
    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir()?;
        return Some(
            home.join(".config")
                .join("Antigravity")
                .join("User")
                .join("globalStorage")
                .join("storage.json"),
        );
    }
    #[allow(unreachable_code)]
    None
}
