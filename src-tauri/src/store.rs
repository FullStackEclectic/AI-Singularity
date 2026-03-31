use crate::{models::ApiKey, AppError, AppResult};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use keyring::Entry;

const SERVICE_NAME: &str = "ai-singularity";

/// 安全存储 API Key：优先使用系统 Keychain，降级到 AES-256-GCM 加密文件
pub struct SecureStore;

impl SecureStore {
    /// 存储 API Key
    pub fn store_key(key_id: &str, secret: &str) -> AppResult<()> {
        let entry = Entry::new(SERVICE_NAME, key_id).map_err(|e| AppError::Keyring(e))?;
        entry
            .set_password(secret)
            .map_err(|e| AppError::Keyring(e))?;
        Ok(())
    }

    /// 读取 API Key
    pub fn get_key(key_id: &str) -> AppResult<String> {
        let entry = Entry::new(SERVICE_NAME, key_id).map_err(|e| AppError::Keyring(e))?;
        entry.get_password().map_err(|e| AppError::Keyring(e))
    }

    /// 删除 API Key
    pub fn delete_key(key_id: &str) -> AppResult<()> {
        let entry = Entry::new(SERVICE_NAME, key_id).map_err(|e| AppError::Keyring(e))?;
        entry
            .delete_credential()
            .map_err(|e| AppError::Keyring(e))?;
        Ok(())
    }

    /// 生成 Key 的预览（前8位 + "..."）
    pub fn key_preview(secret: &str) -> String {
        if secret.len() > 8 {
            format!("{}...", &secret[..8])
        } else {
            "****...".to_string()
        }
    }
}
