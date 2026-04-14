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

#[cfg(target_os = "macos")]
use std::collections::HashSet;
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;

#[cfg(not(target_os = "windows"))]
use aes::Aes128;
#[cfg(target_os = "windows")]
use aes_gcm::aead::generic_array::GenericArray;
#[cfg(target_os = "windows")]
use aes_gcm::aead::{Aead, AeadCore, OsRng};
#[cfg(target_os = "windows")]
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
#[cfg(target_os = "windows")]
use base64::{engine::general_purpose, Engine as _};
#[cfg(not(target_os = "windows"))]
use cbc::cipher::block_padding::Pkcs7;
#[cfg(not(target_os = "windows"))]
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
#[cfg(not(target_os = "windows"))]
use pbkdf2::pbkdf2_hmac;
use rusqlite::Connection;
#[cfg(not(target_os = "windows"))]
use sha1::Sha1;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{LocalFree, HLOCAL};
#[cfg(target_os = "windows")]
use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

#[cfg(not(target_os = "windows"))]
type Aes128CbcEnc = cbc::Encryptor<Aes128>;
#[cfg(not(target_os = "windows"))]
type Aes128CbcDec = cbc::Decryptor<Aes128>;

const V10_PREFIX: &[u8] = b"v10";
const V11_PREFIX: &[u8] = b"v11";
#[cfg(not(target_os = "windows"))]
const CBC_IV: [u8; 16] = [b' '; 16];
#[cfg(not(target_os = "windows"))]
const SALT: &[u8] = b"saltysalt";

#[derive(Clone, Copy)]
enum SafeStorageReadMode {
    Default,
    AntigravityOnly,
    CodeBuddyOnly,
    CodeBuddyCnOnly,
    QoderOnly,
    WorkBuddyOnly,
}

// PBKDF2-HMAC-SHA1(1 iteration, key = "peanuts", salt = "saltysalt")
#[cfg(target_os = "linux")]
const LINUX_V10_KEY: [u8; 16] = [
    0xfd, 0x62, 0x1f, 0xe5, 0xa2, 0xb4, 0x02, 0x53, 0x9d, 0xfa, 0x14, 0x7c, 0xa9, 0x27, 0x27, 0x78,
];

// PBKDF2-HMAC-SHA1(1 iteration, key = "", salt = "saltysalt")
#[cfg(target_os = "linux")]
const LINUX_EMPTY_KEY: [u8; 16] = [
    0xd0, 0xd0, 0xec, 0x9c, 0x7d, 0x77, 0xd4, 0x3a, 0xc5, 0x41, 0x87, 0xfa, 0x48, 0x18, 0xd1, 0x7f,
];

fn resolve_vscode_data_root(user_data_dir: Option<&str>) -> Result<PathBuf, String> {
    crate::services::vscode_paths::resolve_vscode_data_root(user_data_dir).map_err(|err| {
        if err == "GitHub Copilot 仅支持 macOS、Windows 和 Linux" {
            "Unsupported platform".to_string()
        } else {
            err
        }
    })
}

fn get_vscode_db_path_from_data_root(data_root: &Path) -> Result<PathBuf, String> {
    let path = crate::services::vscode_paths::vscode_state_db_path(data_root);
    if path.exists() {
        Ok(path)
    } else {
        let attempted = crate::services::vscode_paths::vscode_data_root_candidates()
            .ok()
            .filter(|candidates| candidates.iter().any(|candidate| candidate == data_root))
            .map(|candidates| {
                candidates
                    .iter()
                    .map(|candidate| {
                        crate::services::vscode_paths::vscode_state_db_path(candidate)
                            .display()
                            .to_string()
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            });
        if let Some(paths) = attempted {
            Err(format!("VS Code database not found. Tried: {}", paths))
        } else {
            Err(format!("VS Code database not found: {}", path.display()))
        }
    }
}

fn build_secret_storage_item_key(extension_id: &str, key: &str) -> String {
    format!(
        r#"secret://{{"extensionId":"{}","key":"{}"}}"#,
        extension_id, key
    )
}

#[cfg(target_os = "windows")]
fn get_local_state_path(data_root: &Path) -> Result<PathBuf, String> {
    let path = crate::services::vscode_paths::vscode_local_state_path(data_root);
    if path.exists() {
        Ok(path)
    } else {
        let attempted = crate::services::vscode_paths::vscode_data_root_candidates()
            .ok()
            .filter(|candidates| candidates.iter().any(|candidate| candidate == data_root))
            .map(|candidates| {
                candidates
                    .iter()
                    .map(|candidate| {
                        crate::services::vscode_paths::vscode_local_state_path(candidate)
                            .display()
                            .to_string()
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            });
        if let Some(paths) = attempted {
            Err(format!("VS Code Local State not found. Tried: {}", paths))
        } else {
            Err(format!("VS Code Local State not found: {}", path.display()))
        }
    }
}

#[cfg(target_os = "windows")]
fn get_windows_encryption_key(data_root: Option<&Path>) -> Result<Vec<u8>, String> {
    let owned_root;
    let root = if let Some(path) = data_root {
        path
    } else {
        owned_root = crate::services::vscode_paths::resolve_vscode_data_root_for_state_db()
            .map_err(|err| {
                if err == "GitHub Copilot 仅支持 macOS、Windows 和 Linux" {
                    "Unsupported platform".to_string()
                } else {
                    err
                }
            })?;
        owned_root.as_path()
    };
    let path = get_local_state_path(root)?;
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read Local State: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Local State JSON: {}", e))?;

    let encrypted_key_b64 = json["os_crypt"]["encrypted_key"]
        .as_str()
        .ok_or("Cannot find os_crypt.encrypted_key in Local State")?;

    let encrypted_key_bytes = general_purpose::STANDARD
        .decode(encrypted_key_b64)
        .map_err(|e| format!("Base64 decode failed for encrypted_key: {}", e))?;

    if encrypted_key_bytes.len() < 6 {
        return Err("encrypted_key data too short".to_string());
    }

    let prefix = String::from_utf8_lossy(&encrypted_key_bytes[..5]);
    if prefix != "DPAPI" {
        return Err(format!(
            "encrypted_key prefix is not DPAPI, got: {}",
            prefix
        ));
    }

    let dpapi_blob = &encrypted_key_bytes[5..];
    let key = dpapi_decrypt(dpapi_blob)?;
    if key.len() != 32 {
        return Err(format!(
            "Decrypted AES key has unexpected length: {}",
            key.len()
        ));
    }
    Ok(key)
}

#[cfg(target_os = "windows")]
fn dpapi_decrypt(encrypted: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
            .map_err(|_| "DPAPI CryptUnprotectData call failed".to_string())?;

        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(HLOCAL(output.pbData as *mut _));
        Ok(result)
    }
}

#[cfg(target_os = "windows")]
fn decrypt_windows_gcm_v10(key: &[u8], encrypted: &[u8]) -> Result<Vec<u8>, String> {
    if encrypted.len() < 31 {
        return Err("Encrypted data too short".to_string());
    }
    if &encrypted[..3] != V10_PREFIX {
        return Err(format!(
            "Not Windows v10 format, prefix: {:?}",
            &encrypted[..3]
        ));
    }

    let nonce_bytes = &encrypted[3..15];
    let ciphertext = &encrypted[15..];

    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("AES-GCM decryption failed: {}", e))
}

#[cfg(target_os = "windows")]
fn encrypt_windows_gcm_v10(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("AES-GCM encryption failed: {}", e))?;

    let mut result = Vec::with_capacity(3 + 12 + ciphertext.len());
    result.extend_from_slice(V10_PREFIX);
    result.extend_from_slice(nonce.as_slice());
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

#[cfg(not(target_os = "windows"))]
fn decrypt_cbc_prefixed(
    encrypted: &[u8],
    expected_prefix: &[u8],
    key: &[u8; 16],
) -> Result<Vec<u8>, String> {
    if !encrypted.starts_with(expected_prefix) {
        return Err(format!(
            "Unexpected ciphertext prefix: {:?}",
            &encrypted[..encrypted.len().min(3)]
        ));
    }
    let raw = &encrypted[expected_prefix.len()..];
    let mut buf = raw.to_vec();
    let cipher = Aes128CbcDec::new_from_slices(key, &CBC_IV)
        .map_err(|e| format!("Failed to init AES-CBC decryptor: {}", e))?;
    let plaintext = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| format!("AES-CBC decryption failed: {}", e))?
        .to_vec();
    Ok(plaintext)
}

#[cfg(not(target_os = "windows"))]
fn encrypt_cbc_prefixed(
    prefix: &[u8],
    key: &[u8; 16],
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let cipher = Aes128CbcEnc::new_from_slices(key, &CBC_IV)
        .map_err(|e| format!("Failed to init AES-CBC encryptor: {}", e))?;

    let mut buf = plaintext.to_vec();
    let msg_len = buf.len();
    let pad_len = 16 - (msg_len % 16);
    buf.resize(msg_len + pad_len, 0);
    let ciphertext = cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buf, msg_len)
        .map_err(|e| format!("AES-CBC encryption failed: {}", e))?
        .to_vec();

    let mut result = Vec::with_capacity(prefix.len() + ciphertext.len());
    result.extend_from_slice(prefix);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

#[cfg(not(target_os = "windows"))]
fn pbkdf2_sha1_key(password: &str, iterations: u32) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2_hmac::<Sha1>(password.as_bytes(), SALT, iterations, &mut key);
    key
}

fn detect_prefix(encrypted: &[u8]) -> Option<&'static str> {
    if encrypted.starts_with(V10_PREFIX) {
        Some("v10")
    } else if encrypted.starts_with(V11_PREFIX) {
        Some("v11")
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn run_command_get_trimmed(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(target_os = "macos")]
fn build_macos_safe_storage_candidates(
    data_root: Option<&Path>,
    mode: SafeStorageReadMode,
) -> Vec<(String, Option<String>)> {
    if matches!(mode, SafeStorageReadMode::AntigravityOnly) {
        return vec![
            (
                "Antigravity Safe Storage".to_string(),
                Some("Antigravity".to_string()),
            ),
            ("Antigravity Safe Storage".to_string(), None),
            (
                "Antigravity Safe Storage".to_string(),
                Some("Antigravity Safe Storage".to_string()),
            ),
        ];
    }

    if matches!(mode, SafeStorageReadMode::CodeBuddyOnly) {
        return vec![
            (
                "CodeBuddy Safe Storage".to_string(),
                Some("CodeBuddy".to_string()),
            ),
            (
                "CodeBuddy Safe Storage".to_string(),
                Some("codebuddy".to_string()),
            ),
            (
                "CodeBuddy Safe Storage".to_string(),
                Some("CodeBuddy Key".to_string()),
            ),
            ("CodeBuddy Safe Storage".to_string(), None),
            (
                "CodeBuddy Safe Storage".to_string(),
                Some("CodeBuddy Safe Storage".to_string()),
            ),
        ];
    }

    if matches!(mode, SafeStorageReadMode::CodeBuddyCnOnly) {
        return vec![
            (
                "CodeBuddy CN Safe Storage".to_string(),
                Some("CodeBuddy CN".to_string()),
            ),
            (
                "CodeBuddy CN Safe Storage".to_string(),
                Some("codebuddy cn".to_string()),
            ),
            (
                "CodeBuddy CN Safe Storage".to_string(),
                Some("CodeBuddy CN Key".to_string()),
            ),
            ("CodeBuddy CN Safe Storage".to_string(), None),
            (
                "CodeBuddy CN Safe Storage".to_string(),
                Some("CodeBuddy CN Safe Storage".to_string()),
            ),
        ];
    }

    if matches!(mode, SafeStorageReadMode::QoderOnly) {
        return vec![
            ("Qoder Safe Storage".to_string(), Some("Qoder".to_string())),
            ("Qoder Safe Storage".to_string(), Some("qoder".to_string())),
            ("Qoder Safe Storage".to_string(), None),
            (
                "Qoder Safe Storage".to_string(),
                Some("Qoder Safe Storage".to_string()),
            ),
        ];
    }

    if matches!(mode, SafeStorageReadMode::WorkBuddyOnly) {
        return vec![
            (
                "WorkBuddy Safe Storage".to_string(),
                Some("WorkBuddy".to_string()),
            ),
            (
                "WorkBuddy Safe Storage".to_string(),
                Some("workbuddy".to_string()),
            ),
            (
                "WorkBuddy Safe Storage".to_string(),
                Some("WorkBuddy Key".to_string()),
            ),
            ("WorkBuddy Safe Storage".to_string(), None),
            (
                "WorkBuddy Safe Storage".to_string(),
                Some("WorkBuddy Safe Storage".to_string()),
            ),
        ];
    }

    let mut app_names: Vec<String> = Vec::new();
    if let Some(root) = data_root {
        if let Some(name) = root.file_name().and_then(|value| value.to_str()) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                app_names.push(trimmed.to_string());
            }
        }
    }

    // Default mode is used by VS Code / GitHub Copilot injection path.
    // Keep this list strictly VS Code-family to avoid cross-platform key probing.
    app_names.extend(
        [
            "Code",
            "Code - Insiders",
            "Visual Studio Code",
            "Visual Studio Code - Insiders",
            "Code - OSS",
            "VSCodium",
        ]
        .iter()
        .map(|value| value.to_string()),
    );

    let mut candidates: Vec<(String, Option<String>)> = Vec::new();
    let mut seen = HashSet::new();

    for app_name in app_names {
        let service = format!("{} Safe Storage", app_name);
        let account = Some(app_name.clone());
        if seen.insert((service.clone(), account.clone())) {
            candidates.push((service.clone(), account));
        }
        if seen.insert((service.clone(), None)) {
            candidates.push((service.clone(), None));
        }
        let alt_account = Some(service.clone());
        if seen.insert((service.clone(), alt_account.clone())) {
            candidates.push((service, alt_account));
        }
    }

    candidates
}

#[cfg(target_os = "macos")]
fn get_macos_safe_storage_password(
    data_root: Option<&Path>,
    mode: SafeStorageReadMode,
) -> Result<String, String> {
    let candidates = build_macos_safe_storage_candidates(data_root, mode);
    for (service, account) in candidates {
        if let Some(account_value) = account.as_deref() {
            if let Some(password) = run_command_get_trimmed(
                "security",
                &[
                    "find-generic-password",
                    "-w",
                    "-s",
                    &service,
                    "-a",
                    account_value,
                ],
            ) {
                return Ok(password);
            }
        }
        if let Some(password) =
            run_command_get_trimmed("security", &["find-generic-password", "-w", "-s", &service])
        {
            return Ok(password);
        }
    }
    Err("Failed to read Safe Storage password from Keychain".to_string())
}

#[cfg(target_os = "linux")]
fn run_command_get_trimmed(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(target_os = "linux")]
fn get_linux_v11_key(mode: SafeStorageReadMode) -> Option<[u8; 16]> {
    let app_names: &[&str] = match mode {
        SafeStorageReadMode::CodeBuddyOnly => &["CodeBuddy", "codebuddy"],
        SafeStorageReadMode::CodeBuddyCnOnly => &[
            "CodeBuddy CN",
            "codebuddy cn",
            "codebuddy-cn",
            "codebuddycn",
        ],
        SafeStorageReadMode::QoderOnly => &["Qoder", "qoder"],
        SafeStorageReadMode::WorkBuddyOnly => {
            &["WorkBuddy", "workbuddy", "workbuddy-cn", "workbuddycn"]
        }
        _ => &[
            "code",
            "Code",
            "code-insiders",
            "Code - Insiders",
            "code-oss",
            "Code - OSS",
            "VSCodium",
        ],
    };

    for app in app_names {
        if let Some(password) =
            run_command_get_trimmed("secret-tool", &["lookup", "application", app])
        {
            return Some(pbkdf2_sha1_key(&password, 1));
        }
    }

    None
}

fn decrypt_secret_payload_with_mode(
    encrypted: &[u8],
    data_root: Option<&Path>,
    mode: SafeStorageReadMode,
) -> Result<Vec<u8>, String> {
    #[cfg(not(target_os = "windows"))]
    let _ = (data_root, mode);

    #[cfg(target_os = "windows")]
    {
        let _ = mode;
        let key = get_windows_encryption_key(data_root)?;
        return decrypt_windows_gcm_v10(&key, encrypted);
    }

    #[cfg(target_os = "macos")]
    {
        let password = get_macos_safe_storage_password(data_root, mode)?;
        let key = pbkdf2_sha1_key(&password, 1003);
        return decrypt_cbc_prefixed(encrypted, V10_PREFIX, &key);
    }

    #[cfg(target_os = "linux")]
    {
        match detect_prefix(encrypted) {
            Some("v11") => {
                let key = get_linux_v11_key(mode).ok_or(
                    "Cannot load Linux secret storage key for VS Code (v11 payload)".to_string(),
                )?;
                match decrypt_cbc_prefixed(encrypted, V11_PREFIX, &key) {
                    Ok(value) => Ok(value),
                    Err(_) => decrypt_cbc_prefixed(encrypted, V11_PREFIX, &LINUX_EMPTY_KEY),
                }
            }
            Some("v10") => match decrypt_cbc_prefixed(encrypted, V10_PREFIX, &LINUX_V10_KEY) {
                Ok(value) => Ok(value),
                Err(_) => decrypt_cbc_prefixed(encrypted, V10_PREFIX, &LINUX_EMPTY_KEY),
            },
            _ => Err(format!(
                "Unsupported Linux ciphertext prefix: {:?}",
                &encrypted[..encrypted.len().min(3)]
            )),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = encrypted;
        let _ = (data_root, mode);
        Err("Unsupported platform".to_string())
    }
}

fn decrypt_secret_payload(encrypted: &[u8], data_root: Option<&Path>) -> Result<Vec<u8>, String> {
    decrypt_secret_payload_with_mode(encrypted, data_root, SafeStorageReadMode::Default)
}

fn encrypt_secret_payload(
    plaintext: &[u8],
    preferred_prefix: Option<&str>,
    data_root: Option<&Path>,
) -> Result<Vec<u8>, String> {
    encrypt_secret_payload_with_mode(
        plaintext,
        preferred_prefix,
        data_root,
        SafeStorageReadMode::Default,
    )
}

fn encrypt_secret_payload_with_mode(
    plaintext: &[u8],
    preferred_prefix: Option<&str>,
    data_root: Option<&Path>,
    mode: SafeStorageReadMode,
) -> Result<Vec<u8>, String> {
    #[cfg(not(target_os = "linux"))]
    let _ = preferred_prefix;

    #[cfg(target_os = "windows")]
    {
        let _ = mode;
        let key = get_windows_encryption_key(data_root)?;
        return encrypt_windows_gcm_v10(&key, plaintext);
    }

    #[cfg(target_os = "macos")]
    {
        let password = get_macos_safe_storage_password(data_root, mode)?;
        let key = pbkdf2_sha1_key(&password, 1003);
        return encrypt_cbc_prefixed(V10_PREFIX, &key, plaintext);
    }

    #[cfg(target_os = "linux")]
    {
        let _ = data_root;
        let target_prefix = if let Some(prefix) = preferred_prefix {
            prefix
        } else if get_linux_v11_key(mode).is_some() {
            "v11"
        } else {
            "v10"
        };

        if target_prefix == "v11" {
            let key = get_linux_v11_key(mode).ok_or(
                "Cannot load Linux secret storage key for VS Code (v11 payload)".to_string(),
            )?;
            return encrypt_cbc_prefixed(V11_PREFIX, &key, plaintext);
        }

        return encrypt_cbc_prefixed(V10_PREFIX, &LINUX_V10_KEY, plaintext);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (plaintext, preferred_prefix, data_root, mode);
        Err("Unsupported platform".to_string())
    }
}

fn inject_copilot_token_with_data_root(
    data_root: &Path,
    username: &str,
    token: &str,
    github_user_id: Option<&str>,
) -> Result<String, String> {
    let db_path = get_vscode_db_path_from_data_root(data_root)?;
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open VS Code database: {}", e))?;

    let secret_key =
        r#"secret://{"extensionId":"vscode.github-authentication","key":"github.auth"}"#;
    let existing: Option<String> = match conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?",
        [secret_key],
        |row| row.get(0),
    ) {
        Ok(val) => Some(val),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("Failed to query github.auth from database: {}", e)),
    };

    let (new_sessions, existing_prefix) = build_github_auth_sessions(
        existing.as_deref(),
        Some(data_root),
        username,
        token,
        github_user_id,
    )?;

    let sessions_json = serde_json::to_string(&new_sessions)
        .map_err(|e| format!("Failed to serialize sessions: {}", e))?;
    let encrypted = encrypt_secret_payload(
        sessions_json.as_bytes(),
        existing_prefix.as_deref(),
        Some(data_root),
    )?;

    let buffer_json = serde_json::json!({
        "type": "Buffer",
        "data": encrypted
    });
    let buffer_str = serde_json::to_string(&buffer_json)
        .map_err(|e| format!("Failed to serialize Buffer: {}", e))?;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        [secret_key, &buffer_str.as_str()],
    )
    .map_err(|e| format!("Failed to write github.auth: {}", e))?;

    tx.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        ["github.copilot-github", username],
    )
    .map_err(|e| format!("Failed to write github.copilot-github: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(format!("Successfully injected {} into VS Code", username))
}

fn decode_buffer_data(buffer: &serde_json::Value) -> Result<Vec<u8>, String> {
    let data_arr = buffer["data"]
        .as_array()
        .ok_or("Secret data is not in Buffer format")?;

    let mut encrypted_bytes: Vec<u8> = Vec::with_capacity(data_arr.len());
    for (idx, v) in data_arr.iter().enumerate() {
        let n = v
            .as_u64()
            .ok_or_else(|| format!("Secret data element at index {} is not an integer", idx))?;
        if n > 255 {
            return Err(format!(
                "Secret data element at index {} is out of range ({} > 255)",
                idx, n
            ));
        }
        encrypted_bytes.push(n as u8);
    }

    Ok(encrypted_bytes)
}

fn decode_secret_storage_value_with_mode(
    raw_value: &str,
    data_root: Option<&Path>,
    mode: SafeStorageReadMode,
) -> Result<String, String> {
    let parsed: serde_json::Value = match serde_json::from_str(raw_value) {
        Ok(value) => value,
        Err(_) => return Ok(raw_value.to_string()),
    };

    if parsed.get("data").is_some() {
        let encrypted_bytes = decode_buffer_data(&parsed)?;
        let decrypted = decrypt_secret_payload_with_mode(&encrypted_bytes, data_root, mode)?;
        return String::from_utf8(decrypted)
            .map_err(|e| format!("Decrypted data is not valid UTF-8: {}", e));
    }

    if let Some(value) = parsed.as_str() {
        return Ok(value.to_string());
    }

    Ok(raw_value.to_string())
}

fn read_secret_storage_value_with_data_root_and_mode(
    data_root: &Path,
    extension_id: &str,
    key: &str,
    mode: SafeStorageReadMode,
) -> Result<Option<String>, String> {
    let db_path = data_root
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if !db_path.exists() {
        return Ok(None);
    }

    let conn = Connection::open(&db_path).map_err(|e| {
        format!(
            "Failed to open VS Code database {}: {}",
            db_path.display(),
            e
        )
    })?;
    let secret_key = build_secret_storage_item_key(extension_id, key);
    let raw_value: Option<String> = match conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1",
        [secret_key.as_str()],
        |row| row.get(0),
    ) {
        Ok(value) => Some(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(err) => {
            return Err(format!(
                "Failed to query VS Code secret '{}' for extension '{}': {}",
                key, extension_id, err
            ))
        }
    };

    match raw_value {
        Some(value) => {
            decode_secret_storage_value_with_mode(&value, Some(data_root), mode).map(Some)
        }
        None => Ok(None),
    }
}

pub fn read_antigravity_secret_storage_value(
    extension_id: &str,
    key: &str,
    user_data_dir: Option<&str>,
) -> Result<Option<String>, String> {
    let data_root = resolve_vscode_data_root(user_data_dir)?;
    read_secret_storage_value_with_data_root_and_mode(
        &data_root,
        extension_id,
        key,
        SafeStorageReadMode::AntigravityOnly,
    )
}

pub fn read_codebuddy_secret_storage_value(
    extension_id: &str,
    key: &str,
    user_data_dir: Option<&str>,
) -> Result<Option<String>, String> {
    let data_root = resolve_vscode_data_root(user_data_dir)?;
    read_secret_storage_value_with_data_root_and_mode(
        &data_root,
        extension_id,
        key,
        SafeStorageReadMode::CodeBuddyOnly,
    )
}

pub fn read_codebuddy_cn_secret_storage_value(
    extension_id: &str,
    key: &str,
    user_data_dir: Option<&str>,
) -> Result<Option<String>, String> {
    let data_root = resolve_vscode_data_root(user_data_dir)?;
    read_secret_storage_value_with_data_root_and_mode(
        &data_root,
        extension_id,
        key,
        SafeStorageReadMode::CodeBuddyCnOnly,
    )
}

pub fn read_workbuddy_secret_storage_value(
    extension_id: &str,
    key: &str,
    user_data_dir: Option<&str>,
) -> Result<Option<String>, String> {
    let data_root = resolve_vscode_data_root(user_data_dir)?;
    read_secret_storage_value_with_data_root_and_mode(
        &data_root,
        extension_id,
        key,
        SafeStorageReadMode::WorkBuddyOnly,
    )
}

fn resolve_data_root_from_state_db_path(db_path: &Path) -> Result<&Path, String> {
    db_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| {
            format!(
                "Cannot determine data root from db path: {}",
                db_path.display()
            )
        })
}

fn read_secret_storage_value_by_db_path_and_mode(
    db_path: &Path,
    db_key: &str,
    mode: SafeStorageReadMode,
) -> Result<Option<String>, String> {
    if !db_path.exists() {
        return Ok(None);
    }

    let data_root = resolve_data_root_from_state_db_path(db_path)?;
    let conn = Connection::open(db_path).map_err(|e| {
        format!(
            "Failed to open VS Code database {}: {}",
            db_path.display(),
            e
        )
    })?;

    let raw_value: Option<String> = match conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1",
        [db_key],
        |row| row.get(0),
    ) {
        Ok(value) => Some(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(err) => {
            return Err(format!(
                "Failed to query VS Code secret key '{}': {}",
                db_key, err
            ))
        }
    };

    match raw_value {
        Some(value) => {
            decode_secret_storage_value_with_mode(&value, Some(data_root), mode).map(Some)
        }
        None => Ok(None),
    }
}

pub fn read_qoder_secret_storage_value_by_db_path(
    db_path: &Path,
    db_key: &str,
) -> Result<Option<String>, String> {
    read_secret_storage_value_by_db_path_and_mode(db_path, db_key, SafeStorageReadMode::QoderOnly)
}

fn load_existing_sessions(
    existing_encrypted_value: Option<&str>,
    data_root: Option<&Path>,
) -> Result<(Vec<serde_json::Value>, Option<String>), String> {
    let Some(value) = existing_encrypted_value else {
        return Ok((Vec::new(), None));
    };

    let parsed: serde_json::Value = serde_json::from_str(value)
        .map_err(|e| format!("Failed to parse existing secret JSON: {}", e))?;

    if parsed.is_array() {
        let sessions: Vec<serde_json::Value> = serde_json::from_value(parsed)
            .map_err(|e| format!("Existing sessions JSON is invalid: {}", e))?;
        return Ok((sessions, None));
    }

    let encrypted_bytes = decode_buffer_data(&parsed)?;
    let prefix = detect_prefix(&encrypted_bytes).map(|s| s.to_string());
    let decrypted = decrypt_secret_payload(&encrypted_bytes, data_root)?;
    let json_str = String::from_utf8(decrypted)
        .map_err(|e| format!("Decrypted data is not valid UTF-8: {}", e))?;
    let sessions: Vec<serde_json::Value> = serde_json::from_str(&json_str)
        .map_err(|e| format!("Decrypted github.auth is not a valid sessions array: {}", e))?;

    Ok((sessions, prefix))
}

fn build_github_auth_sessions(
    existing_encrypted_value: Option<&str>,
    data_root: Option<&Path>,
    username: &str,
    token: &str,
    github_user_id: Option<&str>,
) -> Result<(serde_json::Value, Option<String>), String> {
    let (mut sessions, existing_prefix) =
        load_existing_sessions(existing_encrypted_value, data_root)?;

    let user_id = github_user_id.unwrap_or("0");
    let new_session = serde_json::json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "scopes": ["user:email"],
        "accessToken": token,
        "account": {
            "label": username,
            "id": user_id
        }
    });

    let mut replaced = false;
    for session in &mut sessions {
        if let Some(scopes) = session["scopes"].as_array() {
            let has_user_email = scopes.iter().any(|s| s.as_str() == Some("user:email"));
            if has_user_email {
                *session = new_session.clone();
                replaced = true;
                break;
            }
        }
    }
    if !replaced {
        sessions.push(new_session);
    }

    Ok((serde_json::Value::Array(sessions), existing_prefix))
}

pub fn inject_copilot_token_for_user_data_dir(
    user_data_dir: &str,
    username: &str,
    token: &str,
    github_user_id: Option<&str>,
) -> Result<String, String> {
    let data_root = resolve_vscode_data_root(Some(user_data_dir))?;
    inject_copilot_token_with_data_root(&data_root, username, token, github_user_id)
}

pub fn inject_secret_to_state_db_for_codebuddy(
    db_path: &std::path::Path,
    db_key: &str,
    plaintext: &str,
) -> Result<(), String> {
    inject_secret_to_state_db_with_mode(
        db_path,
        db_key,
        plaintext,
        SafeStorageReadMode::CodeBuddyOnly,
    )
}

pub fn inject_secret_to_state_db_for_codebuddy_cn(
    db_path: &std::path::Path,
    db_key: &str,
    plaintext: &str,
) -> Result<(), String> {
    inject_secret_to_state_db_with_mode(
        db_path,
        db_key,
        plaintext,
        SafeStorageReadMode::CodeBuddyCnOnly,
    )
}

pub fn inject_secret_to_state_db_for_qoder(
    db_path: &std::path::Path,
    db_key: &str,
    plaintext: &str,
) -> Result<(), String> {
    inject_secret_to_state_db_with_mode(db_path, db_key, plaintext, SafeStorageReadMode::QoderOnly)
}

pub fn inject_secret_to_state_db_for_workbuddy(
    db_path: &std::path::Path,
    db_key: &str,
    plaintext: &str,
) -> Result<(), String> {
    inject_secret_to_state_db_with_mode(
        db_path,
        db_key,
        plaintext,
        SafeStorageReadMode::WorkBuddyOnly,
    )
}

fn inject_secret_to_state_db_with_mode(
    db_path: &std::path::Path,
    db_key: &str,
    plaintext: &str,
    mode: SafeStorageReadMode,
) -> Result<(), String> {
    let data_root = db_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| {
            format!(
                "Cannot determine data root from db path: {}",
                db_path.display()
            )
        })?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create state.vscdb parent dir: {}", e))?;
    }

    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("Failed to open state.vscdb: {}", e))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )
    .map_err(|e| format!("Failed to init ItemTable: {}", e))?;

    let existing_prefix: Option<String> = match conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?",
        [db_key],
        |row| row.get::<_, String>(0),
    ) {
        Ok(val) => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
                if let Ok(bytes) = decode_buffer_data(&parsed) {
                    detect_prefix(&bytes).map(|s| s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    };

    let encrypted = encrypt_secret_payload_with_mode(
        plaintext.as_bytes(),
        existing_prefix.as_deref(),
        Some(data_root),
        mode,
    )?;

    let buffer_json = serde_json::json!({
        "type": "Buffer",
        "data": encrypted
    });
    let buffer_str = serde_json::to_string(&buffer_json)
        .map_err(|e| format!("Failed to serialize Buffer: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)",
        rusqlite::params![db_key, buffer_str],
    )
    .map_err(|e| format!("Failed to write to state.vscdb: {}", e))?;

    Ok(())
}

use crate::models::IdeAccount;

pub struct IdeInjector;

pub fn inject_gemini_cli_account_to_root(
    acc: &IdeAccount,
    root_dir: &Path,
    project_id_override: Option<&str>,
) -> Result<(), String> {
    let gemini_dir = root_dir.join(".gemini");
    std::fs::create_dir_all(&gemini_dir).map_err(|e| format!("创建 Gemini 目录失败: {}", e))?;

    let expiry_date =
        acc.token.updated_at.timestamp_millis() + (acc.token.expires_in as i64 * 1000);

    let oauth_creds = serde_json::json!({
        "access_token": acc.token.access_token,
        "refresh_token": if acc.token.refresh_token.trim().is_empty() { serde_json::Value::Null } else { serde_json::Value::String(acc.token.refresh_token.clone()) },
        "token_type": if acc.token.token_type.trim().is_empty() { "Bearer" } else { acc.token.token_type.as_str() },
        "expiry_date": expiry_date,
    });
    let oauth_content = serde_json::to_string_pretty(&oauth_creds)
        .map_err(|e| format!("序列化 Gemini oauth_creds.json 失败: {}", e))?;
    std::fs::write(
        gemini_dir.join("oauth_creds.json"),
        format!("{}\n", oauth_content),
    )
    .map_err(|e| format!("写入 Gemini oauth_creds.json 失败: {}", e))?;

    let google_accounts_path = gemini_dir.join("google_accounts.json");
    let existing_accounts = if google_accounts_path.exists() {
        std::fs::read_to_string(&google_accounts_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .unwrap_or_else(|| serde_json::json!({ "active": acc.email, "old": [] }))
    } else {
        serde_json::json!({ "active": acc.email, "old": [] })
    };
    let mut old_accounts = existing_accounts
        .get("old")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if let Some(previous_active) = existing_accounts.get("active").and_then(|v| v.as_str()) {
        if !previous_active.eq_ignore_ascii_case(&acc.email)
            && !old_accounts
                .iter()
                .any(|item| item.as_str() == Some(previous_active))
        {
            old_accounts.push(serde_json::Value::String(previous_active.to_string()));
        }
    }
    let google_accounts = serde_json::json!({
        "active": acc.email,
        "old": old_accounts,
    });
    let google_accounts_content = serde_json::to_string_pretty(&google_accounts)
        .map_err(|e| format!("序列化 Gemini google_accounts.json 失败: {}", e))?;
    std::fs::write(
        &google_accounts_path,
        format!("{}\n", google_accounts_content),
    )
    .map_err(|e| format!("写入 Gemini google_accounts.json 失败: {}", e))?;

    let settings_path = gemini_dir.join("settings.json");
    let mut settings = if settings_path.exists() {
        std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if !settings.is_object() {
        settings = serde_json::json!({});
    }
    if settings
        .get("security")
        .and_then(|v| v.as_object())
        .is_none()
    {
        settings["security"] = serde_json::json!({});
    }
    if settings["security"]
        .get("auth")
        .and_then(|v| v.as_object())
        .is_none()
    {
        settings["security"]["auth"] = serde_json::json!({});
    }
    settings["security"]["auth"]["selectedType"] = serde_json::json!("oauth-personal");
    let effective_project_id = project_id_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| acc.project_id.clone().filter(|value| !value.trim().is_empty()));
    if let Some(project_id) = effective_project_id {
        settings["projectId"] = serde_json::json!(project_id);
    }
    let settings_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("序列化 Gemini settings.json 失败: {}", e))?;
    std::fs::write(settings_path, format!("{}\n", settings_content))
        .map_err(|e| format!("写入 Gemini settings.json 失败: {}", e))?;

    Ok(())
}

fn inject_gemini_cli_account(acc: &IdeAccount) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    inject_gemini_cli_account_to_root(acc, &home, None)
}

pub fn inject_codex_account_to_dir(acc: &IdeAccount, codex_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(codex_dir).map_err(|e| format!("创建 Codex 目录失败: {}", e))?;

    let meta = acc
        .meta_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let auth_mode = meta
        .get("auth_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("oauth");
    let openai_api_key = meta
        .get("openai_api_key")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let api_base_url = meta
        .get("api_base_url")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let last_refresh = meta
        .get("last_refresh")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

    let tokens = serde_json::json!({
        "access_token": acc.token.access_token,
        "refresh_token": if acc.token.refresh_token.trim().is_empty() { serde_json::Value::Null } else { serde_json::Value::String(acc.token.refresh_token.clone()) },
        "id_token": meta.get("id_token").cloned().unwrap_or(serde_json::Value::Null),
        "account_id": meta.get("account_id").cloned().unwrap_or(serde_json::Value::Null),
    });

    let auth_json = if auth_mode.eq_ignore_ascii_case("apikey") {
        serde_json::json!({
            "OPENAI_API_KEY": openai_api_key,
            "base_url": api_base_url,
            "last_refresh": last_refresh,
            "tokens": tokens,
            "auth_mode": "apikey",
        })
    } else {
        serde_json::json!({
            "OPENAI_API_KEY": openai_api_key,
            "base_url": api_base_url,
            "last_refresh": last_refresh,
            "tokens": tokens,
            "auth_mode": "oauth",
        })
    };

    let auth_content = serde_json::to_string_pretty(&auth_json)
        .map_err(|e| format!("序列化 Codex auth.json 失败: {}", e))?;
    std::fs::write(codex_dir.join("auth.json"), format!("{}\n", auth_content))
        .map_err(|e| format!("写入 Codex auth.json 失败: {}", e))?;

    Ok(())
}

fn inject_codex_account(acc: &IdeAccount) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let codex_dir = home.join(".codex");
    inject_codex_account_to_dir(acc, &codex_dir)
}

fn app_data_root(app_name: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata =
            std::env::var("APPDATA").map_err(|_| "无法获取 APPDATA 环境变量".to_string())?;
        return Ok(PathBuf::from(appdata).join(app_name));
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        return Ok(home
            .join("Library")
            .join("Application Support")
            .join(app_name));
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
        return Ok(home.join(".config").join(app_name));
    }

    #[allow(unreachable_code)]
    Err(format!("当前平台暂不支持读取 {} 数据目录", app_name))
}

fn parse_meta_object(acc: &IdeAccount) -> serde_json::Map<String, serde_json::Value> {
    acc.meta_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn inject_qoder_account(acc: &IdeAccount) -> Result<(), String> {
    let db_path = app_data_root("Qoder")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    let meta = parse_meta_object(acc);

    let user_info_json = meta
        .get("auth_user_info_raw")
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "id": meta.get("user_id").cloned().unwrap_or(serde_json::Value::Null),
                "email": acc.email,
                "name": acc.disabled_reason.clone().unwrap_or_default(),
            })
        });
    let user_plan_json = meta
        .get("auth_user_plan_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let credit_usage_json = meta
        .get("auth_credit_usage_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let user_info_raw = serde_json::to_string(&user_info_json)
        .map_err(|e| format!("序列化 Qoder userInfo 失败: {}", e))?;
    let user_plan_raw = serde_json::to_string(&user_plan_json)
        .map_err(|e| format!("序列化 Qoder userPlan 失败: {}", e))?;
    let credit_usage_raw = serde_json::to_string(&credit_usage_json)
        .map_err(|e| format!("序列化 Qoder creditUsage 失败: {}", e))?;

    inject_secret_to_state_db_for_qoder(
        db_path.as_path(),
        "secret://aicoding.auth.userInfo",
        &user_info_raw,
    )?;
    inject_secret_to_state_db_for_qoder(
        db_path.as_path(),
        "secret://aicoding.auth.userPlan",
        &user_plan_raw,
    )?;
    inject_secret_to_state_db_for_qoder(
        db_path.as_path(),
        "secret://aicoding.auth.creditUsage",
        &credit_usage_raw,
    )?;
    Ok(())
}

fn inject_windsurf_account(acc: &IdeAccount) -> Result<(), String> {
    let db_path = app_data_root("Windsurf")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Windsurf 目录失败: {}", e))?;
    }

    let meta = parse_meta_object(acc);
    let mut auth_status = meta
        .get("windsurf_auth_status_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !auth_status.is_object() {
        auth_status = serde_json::json!({});
    }

    if let Some(obj) = auth_status.as_object_mut() {
        if obj.get("apiKey").is_none() && obj.get("api_key").is_none() {
            obj.insert("apiKey".to_string(), serde_json::Value::String(acc.token.access_token.clone()));
        }
        if obj.get("email").is_none() {
            obj.insert("email".to_string(), serde_json::Value::String(acc.email.clone()));
        }
        if obj.get("name").is_none() && acc.disabled_reason.is_some() {
            obj.insert(
                "name".to_string(),
                serde_json::Value::String(acc.disabled_reason.clone().unwrap_or_default()),
            );
        }
        if let Some(api_server_url) = meta
            .get("windsurf_api_server_url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("apiServerUrl".to_string())
                .or_insert_with(|| serde_json::Value::String(api_server_url.to_string()));
        }
    }

    let raw = serde_json::to_string(&auth_status)
        .map_err(|e| format!("序列化 Windsurf authStatus 失败: {}", e))?;
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("打开 Windsurf state.vscdb 失败: {}", e))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )
    .map_err(|e| format!("初始化 Windsurf ItemTable 失败: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["windsurfAuthStatus", raw],
    )
    .map_err(|e| format!("写入 Windsurf authStatus 失败: {}", e))?;
    Ok(())
}

fn inject_cursor_account(acc: &IdeAccount) -> Result<(), String> {
    let db_path = app_data_root("Cursor")?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Cursor 目录失败: {}", e))?;
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("打开 Cursor state.vscdb 失败: {}", e))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )
    .map_err(|e| format!("初始化 Cursor ItemTable 失败: {}", e))?;

    let meta = parse_meta_object(acc);
    let auth_id = meta
        .get("authId")
        .or_else(|| meta.get("auth_id"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string());
    let membership_type = meta
        .get("stripeMembershipType")
        .or_else(|| meta.get("membership_type"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string());
    let subscription_status = meta
        .get("stripeSubscriptionStatus")
        .or_else(|| meta.get("subscription_status"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string());

    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursorAuth/accessToken", acc.token.access_token],
    )
    .map_err(|e| format!("写入 Cursor accessToken 失败: {}", e))?;
    if !acc.token.refresh_token.trim().is_empty() && acc.token.refresh_token != "missing" {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/refreshToken", acc.token.refresh_token],
        )
        .map_err(|e| format!("写入 Cursor refreshToken 失败: {}", e))?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursorAuth/cachedEmail", acc.email],
    )
    .map_err(|e| format!("写入 Cursor cachedEmail 失败: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursor.accessToken", acc.token.access_token],
    )
    .map_err(|e| format!("写入 Cursor cursor.accessToken 失败: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["cursor.email", acc.email],
    )
    .map_err(|e| format!("写入 Cursor cursor.email 失败: {}", e))?;

    if let Some(auth_id) = auth_id {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/authId", auth_id],
        )
        .map_err(|e| format!("写入 Cursor authId 失败: {}", e))?;
    }
    if let Some(membership_type) = membership_type {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/stripeMembershipType", membership_type],
        )
        .map_err(|e| format!("写入 Cursor stripeMembershipType 失败: {}", e))?;
    }
    if let Some(subscription_status) = subscription_status {
        conn.execute(
            "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)",
            rusqlite::params!["cursorAuth/stripeSubscriptionStatus", subscription_status],
        )
        .map_err(|e| format!("写入 Cursor stripeSubscriptionStatus 失败: {}", e))?;
    }

    Ok(())
}

fn inject_codebuddy_like_account(
    acc: &IdeAccount,
    app_name: &str,
    secret_key: &str,
    writer: fn(&std::path::Path, &str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let db_path = app_data_root(app_name)?
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    let meta = parse_meta_object(acc);
    let payload = meta
        .get("auth_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({
            "uid": meta.get("uid").cloned().unwrap_or(serde_json::Value::Null),
            "nickname": meta.get("nickname").cloned().unwrap_or(serde_json::Value::Null),
            "email": acc.email,
            "accessToken": acc.token.access_token,
            "refreshToken": if acc.token.refresh_token.trim().is_empty() || acc.token.refresh_token == "missing" {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(acc.token.refresh_token.clone())
            }
        }));
    let raw = serde_json::to_string(&payload)
        .map_err(|e| format!("序列化 {} 登录态失败: {}", app_name, e))?;
    writer(db_path.as_path(), secret_key, &raw)
}

fn inject_codebuddy_account(acc: &IdeAccount) -> Result<(), String> {
    inject_codebuddy_like_account(
        acc,
        "CodeBuddy",
        r#"secret://{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessToken"}"#,
        inject_secret_to_state_db_for_codebuddy,
    )
}

fn inject_codebuddy_cn_account(acc: &IdeAccount) -> Result<(), String> {
    inject_codebuddy_like_account(
        acc,
        "CodeBuddy CN",
        r#"secret://{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessToken"}"#,
        inject_secret_to_state_db_for_codebuddy_cn,
    )
}

fn inject_workbuddy_account(acc: &IdeAccount) -> Result<(), String> {
    inject_codebuddy_like_account(
        acc,
        "WorkBuddy",
        r#"secret://{"extensionId":"tencent-cloud.coding-copilot","key":"planning-genie.new.accessTokencn"}"#,
        inject_secret_to_state_db_for_workbuddy,
    )
}

#[cfg(target_os = "macos")]
fn zed_security_command_output(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("security")
        .args(args)
        .output()
        .map_err(|e| format!("执行 Zed security 命令失败: {}", e))
}

#[cfg(target_os = "macos")]
fn inject_zed_account(acc: &IdeAccount) -> Result<(), String> {
    let meta = parse_meta_object(acc);
    let user_id = meta
        .get("user_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Zed 账号缺少 user_id，无法写入 Keychain".to_string())?;

    loop {
        let output = zed_security_command_output(&["delete-internet-password", "-s", "https://zed.dev"])?;
        if output.status.success() {
            continue;
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found") {
            break;
        }
        return Err(format!("删除 Zed Keychain 凭据失败: {}", stderr.trim()));
    }

    let output = zed_security_command_output(&[
        "add-internet-password",
        "-U",
        "-a",
        user_id,
        "-s",
        "https://zed.dev",
        "-w",
        acc.token.access_token.as_str(),
    ])?;
    if !output.status.success() {
        return Err(format!(
            "写入 Zed Keychain 凭据失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn inject_zed_account(_acc: &IdeAccount) -> Result<(), String> {
    Err("Zed 切号当前仅支持 macOS".to_string())
}

fn inject_kiro_account(acc: &IdeAccount) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录".to_string())?;
    let auth_path = home
        .join(".aws")
        .join("sso")
        .join("cache")
        .join("kiro-auth-token.json");
    if let Some(parent) = auth_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Kiro 授权目录失败: {}", e))?;
    }

    let profile_path = app_data_root("Kiro")?
        .join("User")
        .join("globalStorage")
        .join("kiro.kiroagent")
        .join("profile.json");
    if let Some(parent) = profile_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Kiro profile 目录失败: {}", e))?;
    }

    let meta = parse_meta_object(acc);
    let mut auth_json = meta
        .get("kiro_auth_token_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !auth_json.is_object() {
        auth_json = serde_json::json!({});
    }
    if let Some(obj) = auth_json.as_object_mut() {
        obj.insert(
            "accessToken".to_string(),
            serde_json::Value::String(acc.token.access_token.clone()),
        );
        obj.insert(
            "access_token".to_string(),
            serde_json::Value::String(acc.token.access_token.clone()),
        );
        if !acc.token.refresh_token.trim().is_empty() && acc.token.refresh_token != "missing" {
            obj.insert(
                "refreshToken".to_string(),
                serde_json::Value::String(acc.token.refresh_token.clone()),
            );
        }
        if obj.get("email").is_none() {
            obj.insert("email".to_string(), serde_json::Value::String(acc.email.clone()));
        }
        if let Some(user_id) = meta
            .get("user_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("userId".to_string())
                .or_insert_with(|| serde_json::Value::String(user_id.to_string()));
        }
    }

    let mut profile_json = meta
        .get("kiro_profile_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !profile_json.is_object() {
        profile_json = serde_json::json!({});
    }
    if let Some(obj) = profile_json.as_object_mut() {
        obj.entry("email".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.email.clone()));
        if let Some(user_id) = meta
            .get("user_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("userId".to_string())
                .or_insert_with(|| serde_json::Value::String(user_id.to_string()));
        }
    }

    let auth_content = serde_json::to_string_pretty(&auth_json)
        .map_err(|e| format!("序列化 Kiro 授权文件失败: {}", e))?;
    std::fs::write(&auth_path, format!("{}\n", auth_content))
        .map_err(|e| format!("写入 Kiro 授权文件失败: {}", e))?;

    let profile_content = serde_json::to_string_pretty(&profile_json)
        .map_err(|e| format!("序列化 Kiro profile.json 失败: {}", e))?;
    std::fs::write(&profile_path, format!("{}\n", profile_content))
        .map_err(|e| format!("写入 Kiro profile.json 失败: {}", e))?;
    Ok(())
}

fn inject_trae_account(acc: &IdeAccount) -> Result<(), String> {
    let storage_path = app_data_root("Trae")?
        .join("User")
        .join("globalStorage")
        .join("storage.json");
    if let Some(parent) = storage_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建 Trae 目录失败: {}", e))?;
    }

    let meta = parse_meta_object(acc);
    let mut root = meta
        .get("trae_storage_raw")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !root.is_object() {
        root = serde_json::json!({});
    }

    if let Some(obj) = root.as_object_mut() {
        obj.entry("token".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.token.access_token.clone()));
        obj.entry("accessToken".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.token.access_token.clone()));
        if !acc.token.refresh_token.trim().is_empty() && acc.token.refresh_token != "missing" {
            obj.entry("refreshToken".to_string()).or_insert_with(|| {
                serde_json::Value::String(acc.token.refresh_token.clone())
            });
        }
        obj.entry("email".to_string())
            .or_insert_with(|| serde_json::Value::String(acc.email.clone()));
        if let Some(user_id) = meta
            .get("user_id")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            obj.entry("userId".to_string())
                .or_insert_with(|| serde_json::Value::String(user_id.to_string()));
        }
    }

    let content = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 Trae storage.json 失败: {}", e))?;
    std::fs::write(&storage_path, content)
        .map_err(|e| format!("写入 Trae storage.json 失败: {}", e))?;
    Ok(())
}

impl IdeInjector {
    pub fn execute_injection(acc: &IdeAccount) -> crate::error::AppResult<()> {
        let platform = acc.origin_platform.to_lowercase();
        let tk = acc.token.access_token.clone();

        let result = if platform.contains("codex") {
            inject_codex_account(acc)
        } else if platform.contains("gemini") {
            inject_gemini_cli_account(acc)
        } else if platform.contains("cursor") {
            inject_cursor_account(acc)
        } else if platform.contains("codebuddy_cn") {
            inject_codebuddy_cn_account(acc)
        } else if platform.contains("codebuddy") {
            inject_codebuddy_account(acc)
        } else if platform.contains("workbuddy") {
            inject_workbuddy_account(acc)
        } else if platform.contains("windsurf") {
            inject_windsurf_account(acc)
        } else if platform.contains("kiro") {
            inject_kiro_account(acc)
        } else if platform.contains("zed") {
            inject_zed_account(acc)
        } else if platform.contains("qoder") {
            inject_qoder_account(acc)
        } else if platform.contains("trae") {
            inject_trae_account(acc)
        } else if platform.contains("copilot") {
            inject_copilot_token_for_user_data_dir("", &acc.email, &tk, None).map(|_| ())
        } else {
            // Default fallback
            inject_copilot_token_for_user_data_dir("", &acc.email, &tk, None).map(|_| ())
        };

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::AppError::Other(anyhow::anyhow!(
                "注入失败: {}",
                e
            ))),
        }
    }
}
