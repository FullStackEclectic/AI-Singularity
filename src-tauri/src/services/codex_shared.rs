use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MANAGED_DIR_NAME: &str = ".ai-singularity-managed";
const MANAGED_FILE_NAME: &str = "codex_shared_resources.json";
const MANAGED_SCHEMA_VERSION: u32 = 2;

const SHARED_DIRS: &[&str] = &["skills", "rules", "vendor_imports/skills"];
const SHARED_FILES: &[&str] = &["AGENTS.md"];

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedResourceState {
    #[serde(default = "default_state_schema_version")]
    schema_version: u32,
    #[serde(default)]
    managed_paths: Vec<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSharedResourceStatus {
    pub has_skills: bool,
    pub has_rules: bool,
    pub has_vendor_imports_skills: bool,
    pub has_agents_file: bool,
    #[serde(default)]
    pub has_conflicts: bool,
    #[serde(default)]
    pub conflict_paths: Vec<String>,
    #[serde(default)]
    pub shared_strategy_version: String,
}

fn default_state_schema_version() -> u32 {
    1
}

pub fn ensure_instance_shared_resources(profile_dir: &Path) -> Result<(), String> {
    let default_codex_home = dirs::home_dir()
        .ok_or("无法获取用户主目录".to_string())?
        .join(".codex");

    if paths_point_to_same_location(profile_dir, &default_codex_home) {
        return Ok(());
    }

    fs::create_dir_all(profile_dir).map_err(|e| format!("创建实例目录失败: {}", e))?;
    let mut state = load_state(profile_dir)?;
    migrate_managed_state_if_needed(profile_dir, &default_codex_home, &mut state)?;

    for relative in SHARED_DIRS {
        sync_shared_directory(profile_dir, &default_codex_home, relative, &mut state)?;
    }
    for relative in SHARED_FILES {
        sync_shared_file(profile_dir, &default_codex_home, relative, &mut state)?;
    }

    state.updated_at = Some(Utc::now().to_rfc3339());
    save_state(profile_dir, &state)?;
    Ok(())
}

pub fn inspect_instance_shared_resources(profile_dir: &Path) -> CodexSharedResourceStatus {
    let mut status = CodexSharedResourceStatus {
        has_skills: profile_dir.join("skills").exists(),
        has_rules: profile_dir.join("rules").exists(),
        has_vendor_imports_skills: profile_dir.join("vendor_imports").join("skills").exists(),
        has_agents_file: profile_dir.join("AGENTS.md").exists(),
        has_conflicts: false,
        conflict_paths: Vec::new(),
        shared_strategy_version: format!("v{}", MANAGED_SCHEMA_VERSION),
    };

    let default_codex_home = dirs::home_dir().map(|home| home.join(".codex"));
    let Some(default_codex_home) = default_codex_home else {
        return status;
    };
    if paths_point_to_same_location(profile_dir, &default_codex_home) {
        return status;
    }

    let managed_state = load_state(profile_dir).unwrap_or_default();
    for relative in SHARED_DIRS.iter().chain(SHARED_FILES.iter()) {
        let source = default_codex_home.join(relative);
        let target = profile_dir.join(relative);
        let managed = managed_state.managed_paths.iter().any(|item| item == *relative);
        if source.exists() && target.exists() && !managed {
            status.conflict_paths.push((*relative).to_string());
        }
    }
    status.conflict_paths.sort();
    status.has_conflicts = !status.conflict_paths.is_empty();
    status
}

fn sync_shared_directory(
    profile_dir: &Path,
    source_root: &Path,
    relative: &str,
    state: &mut ManagedResourceState,
) -> Result<(), String> {
    let source = source_root.join(relative);
    if !source.exists() || !source.is_dir() {
        return Ok(());
    }
    let target = profile_dir.join(relative);
    let managed = state.managed_paths.iter().any(|item| item == relative);

    if target.exists() && !managed {
        tracing::warn!(
            "[CodexShared] 共享目录存在未托管冲突，跳过覆盖: {}",
            target.display()
        );
        return Ok(());
    }

    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| format!("清理已托管共享目录失败 ({}): {}", target.display(), e))?;
    }
    copy_dir_recursive(&source, &target)?;
    ensure_managed_path(state, relative);
    Ok(())
}

fn sync_shared_file(
    profile_dir: &Path,
    source_root: &Path,
    relative: &str,
    state: &mut ManagedResourceState,
) -> Result<(), String> {
    let source = source_root.join(relative);
    if !source.exists() || !source.is_file() {
        return Ok(());
    }
    let target = profile_dir.join(relative);
    let managed = state.managed_paths.iter().any(|item| item == relative);

    if target.exists() && !managed {
        tracing::warn!(
            "[CodexShared] 共享文件存在未托管冲突，跳过覆盖: {}",
            target.display()
        );
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建共享文件目录失败: {}", e))?;
    }
    fs::copy(&source, &target).map_err(|e| {
        format!(
            "同步共享文件失败 ({} -> {}): {}",
            source.display(),
            target.display(),
            e
        )
    })?;
    ensure_managed_path(state, relative);
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|e| format!("创建目录失败 ({}): {}", target.display(), e))?;
    let entries = fs::read_dir(source)
        .map_err(|e| format!("读取目录失败 ({}): {}", source.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
            }
            fs::copy(&source_path, &target_path).map_err(|e| {
                format!(
                    "复制文件失败 ({} -> {}): {}",
                    source_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

fn ensure_managed_path(state: &mut ManagedResourceState, relative: &str) {
    if !state.managed_paths.iter().any(|item| item == relative) {
        state.managed_paths.push(relative.to_string());
        state.managed_paths.sort();
    }
}

fn migrate_managed_state_if_needed(
    profile_dir: &Path,
    source_root: &Path,
    state: &mut ManagedResourceState,
) -> Result<(), String> {
    if state.schema_version >= MANAGED_SCHEMA_VERSION {
        return Ok(());
    }

    if state.schema_version < 2 {
        adopt_identical_unmanaged_paths(profile_dir, source_root, state);
    }

    state.schema_version = MANAGED_SCHEMA_VERSION;
    tracing::info!(
        "[CodexShared] 已升级共享资源托管状态到策略版本 v{}",
        MANAGED_SCHEMA_VERSION
    );
    Ok(())
}

fn adopt_identical_unmanaged_paths(
    profile_dir: &Path,
    source_root: &Path,
    state: &mut ManagedResourceState,
) {
    for relative in SHARED_DIRS.iter().chain(SHARED_FILES.iter()) {
        if state.managed_paths.iter().any(|item| item == *relative) {
            continue;
        }
        let source = source_root.join(relative);
        let target = profile_dir.join(relative);
        if !source.exists() || !target.exists() {
            continue;
        }
        if is_same_shared_resource(&source, &target) {
            ensure_managed_path(state, relative);
            tracing::info!(
                "[CodexShared] 迁移策略已接管同内容资源: {}",
                target.display()
            );
        }
    }
}

fn is_same_shared_resource(source: &Path, target: &Path) -> bool {
    if source.is_dir() && target.is_dir() {
        return directories_equal(source, target).unwrap_or(false);
    }
    if source.is_file() && target.is_file() {
        return files_equal(source, target).unwrap_or(false);
    }
    false
}

fn files_equal(left: &Path, right: &Path) -> Result<bool, String> {
    let left_meta = fs::metadata(left).map_err(|e| format!("读取文件元数据失败: {}", e))?;
    let right_meta = fs::metadata(right).map_err(|e| format!("读取文件元数据失败: {}", e))?;
    if left_meta.len() != right_meta.len() {
        return Ok(false);
    }
    let left_bytes = fs::read(left).map_err(|e| format!("读取文件失败 ({}): {}", left.display(), e))?;
    let right_bytes = fs::read(right).map_err(|e| format!("读取文件失败 ({}): {}", right.display(), e))?;
    Ok(left_bytes == right_bytes)
}

fn directories_equal(left: &Path, right: &Path) -> Result<bool, String> {
    let mut left_entries = fs::read_dir(left)
        .map_err(|e| format!("读取目录失败 ({}): {}", left.display(), e))?
        .map(|entry| entry.map_err(|e| format!("读取目录项失败: {}", e)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut right_entries = fs::read_dir(right)
        .map_err(|e| format!("读取目录失败 ({}): {}", right.display(), e))?
        .map(|entry| entry.map_err(|e| format!("读取目录项失败: {}", e)))
        .collect::<Result<Vec<_>, _>>()?;

    left_entries.sort_by_key(|entry| entry.file_name());
    right_entries.sort_by_key(|entry| entry.file_name());
    if left_entries.len() != right_entries.len() {
        return Ok(false);
    }

    for (left_entry, right_entry) in left_entries.into_iter().zip(right_entries.into_iter()) {
        if left_entry.file_name() != right_entry.file_name() {
            return Ok(false);
        }
        let left_path = left_entry.path();
        let right_path = right_entry.path();
        let left_is_dir = left_path.is_dir();
        let right_is_dir = right_path.is_dir();
        if left_is_dir != right_is_dir {
            return Ok(false);
        }
        if left_is_dir {
            if !directories_equal(&left_path, &right_path)? {
                return Ok(false);
            }
        } else if !files_equal(&left_path, &right_path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn load_state(profile_dir: &Path) -> Result<ManagedResourceState, String> {
    let path = state_file_path(profile_dir);
    if !path.exists() {
        return Ok(ManagedResourceState::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("读取共享资源状态失败: {}", e))?;
    if raw.trim().is_empty() {
        return Ok(ManagedResourceState::default());
    }
    serde_json::from_str(&raw).map_err(|e| format!("解析共享资源状态失败: {}", e))
}

fn save_state(profile_dir: &Path, state: &ManagedResourceState) -> Result<(), String> {
    let path = state_file_path(profile_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建共享资源状态目录失败: {}", e))?;
    }
    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("序列化共享资源状态失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("写入共享资源状态失败: {}", e))
}

fn state_file_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join(MANAGED_DIR_NAME).join(MANAGED_FILE_NAME)
}

fn paths_point_to_same_location(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_ascii_lowercase()
    };
    normalize(left) == normalize(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ai-singularity-{}-{}", tag, uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn adopts_identical_unmanaged_file_during_migration() {
        let source_root = make_temp_dir("codex-shared-source");
        let profile_dir = make_temp_dir("codex-shared-profile");
        fs::write(source_root.join("AGENTS.md"), "same-content").expect("write source");
        fs::write(profile_dir.join("AGENTS.md"), "same-content").expect("write target");

        let mut state = ManagedResourceState::default();
        adopt_identical_unmanaged_paths(&profile_dir, &source_root, &mut state);
        assert!(state.managed_paths.iter().any(|item| item == "AGENTS.md"));

        let _ = fs::remove_dir_all(source_root);
        let _ = fs::remove_dir_all(profile_dir);
    }

    #[test]
    fn sync_shared_file_does_not_override_unmanaged_target() {
        let source_root = make_temp_dir("codex-shared-source");
        let profile_dir = make_temp_dir("codex-shared-profile");
        fs::write(source_root.join("AGENTS.md"), "source-content").expect("write source");
        fs::write(profile_dir.join("AGENTS.md"), "user-content").expect("write target");

        let mut state = ManagedResourceState::default();
        sync_shared_file(&profile_dir, &source_root, "AGENTS.md", &mut state).expect("sync shared file");
        let content = fs::read_to_string(profile_dir.join("AGENTS.md")).expect("read target");
        assert_eq!(content, "user-content");
        assert!(!state.managed_paths.iter().any(|item| item == "AGENTS.md"));

        let _ = fs::remove_dir_all(source_root);
        let _ = fs::remove_dir_all(profile_dir);
    }

    #[test]
    fn sync_shared_file_overrides_managed_target() {
        let source_root = make_temp_dir("codex-shared-source");
        let profile_dir = make_temp_dir("codex-shared-profile");
        fs::write(source_root.join("AGENTS.md"), "source-content").expect("write source");
        fs::write(profile_dir.join("AGENTS.md"), "old-content").expect("write target");

        let mut state = ManagedResourceState::default();
        state.managed_paths.push("AGENTS.md".to_string());
        sync_shared_file(&profile_dir, &source_root, "AGENTS.md", &mut state).expect("sync shared file");
        let content = fs::read_to_string(profile_dir.join("AGENTS.md")).expect("read target");
        assert_eq!(content, "source-content");

        let _ = fs::remove_dir_all(source_root);
        let _ = fs::remove_dir_all(profile_dir);
    }
}
