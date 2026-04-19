#![allow(dead_code)]

//! VS Code GitHub Copilot token injection module.
//!
//! Enables one-click Copilot account switching in VS Code by directly
//! writing auth sessions into VS Code's state.vscdb database.
//!
//! ## Platform crypto model
//!
//! - Windows: Local State `os_crypt.encrypted_key` + DPAPI, payload is `v10` + AES-256-GCM
//! - macOS: Keychain "Code Safe Storage" password, payload is `v10` + AES-128-CBC
//! - Linux: Secret Service password for `v11` + AES-128-CBC, fallback `v10` fixed key
//!
//! This module decrypts the existing GitHub auth sessions, replaces the token,
//! re-encrypts, and writes back.

mod platforms;
mod secret_storage;

use crate::models::IdeAccount;

pub use self::platforms::{
    inject_codex_account_to_dir, inject_gemini_cli_account_to_root,
};
#[allow(unused_imports)]
pub use self::secret_storage::{
    inject_copilot_token_for_user_data_dir, read_antigravity_secret_storage_value,
    read_codebuddy_cn_secret_storage_value, read_codebuddy_secret_storage_value,
    read_qoder_secret_storage_value_by_db_path, read_workbuddy_secret_storage_value,
};

pub struct IdeInjector;

impl IdeInjector {
    pub fn execute_injection(acc: &IdeAccount) -> crate::error::AppResult<()> {
        match platforms::inject_platform_account(acc) {
            Ok(_) => Ok(()),
            Err(err) => Err(crate::error::AppError::Other(anyhow::anyhow!(
                "注入失败: {}",
                err
            ))),
        }
    }
}
