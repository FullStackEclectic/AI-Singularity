use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub source_url: Option<String>,
    pub local_path: String,
    pub status: String,
}

pub struct SkillService;

impl SkillService {
    pub fn get_primary_skills_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let skills_dir = home.join(".ai-singularity").join("skills");
        if !skills_dir.exists() {
            let _ = fs::create_dir_all(&skills_dir);
        }
        skills_dir
    }

    pub fn get_legacy_commands_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".claude").join("commands")
    }

    fn get_skill_dirs() -> Vec<(PathBuf, String)> {
        let mut dirs = Vec::new();

        let primary = Self::get_primary_skills_dir();
        dirs.push((primary, "installed".to_string()));

        let legacy = Self::get_legacy_commands_dir();
        if legacy.exists() {
            dirs.push((legacy, "legacy".to_string()));
        }

        dirs
    }

    fn resolve_skill_path(id: &str) -> Option<PathBuf> {
        for (dir, _) in Self::get_skill_dirs() {
            let candidate = dir.join(id);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// List installed skills purely by reading the directory and git remotes
    pub fn list_skills() -> Result<Vec<SkillInfo>, String> {
        let mut skills = Vec::new();

        for (dir, status) in Self::get_skill_dirs() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_dir() {
                        let id = entry.file_name().to_string_lossy().to_string();

                        if skills.iter().any(|skill: &SkillInfo| skill.id == id) {
                            continue;
                        }

                        let source_url = Self::get_git_remote(&path);

                        skills.push(SkillInfo {
                            id: id.clone(),
                            name: id.clone(),
                            source_url,
                            local_path: path.to_string_lossy().to_string(),
                            status: status.clone(),
                        });
                    }
                }
            }
        }

        skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(skills)
    }

    /// Extract git remote origin dynamically
    fn get_git_remote(repo_path: &Path) -> Option<String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(["config", "--get", "remote.origin.url"])
            .output()
            .ok()?;

        if output.status.success() {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !url.is_empty() {
                return Some(url);
            }
        }
        None
    }

    /// Install skill by git cloning
    pub fn install_skill(url: &str) -> Result<SkillInfo, String> {
        let dir = Self::get_primary_skills_dir();

        // Ensure git is installed
        if Command::new("git").arg("--version").output().is_err() {
            return Err("未找到 Git，请先在系统安装 Git 命令行工具".to_string());
        }

        // Parse repo name from URL (e.g. https://github.com/user/repo-name.git -> repo-name)
        let repo_name = url
            .trim_end_matches(".git")
            .split('/')
            .last()
            .unwrap_or("unknown_skill");
        let target_path = dir.join(repo_name);

        if target_path.exists() {
            return Err(format!(
                "技能 {} 对应的目录已存在，请尝试更新或先卸载！",
                repo_name
            ));
        }

        info!("Cloning skill from {} to {:?}", url, target_path);
        let output = Command::new("git")
            .current_dir(&dir)
            .args(["clone", url, repo_name])
            .output()
            .map_err(|e| format!("启动 git 失败: {}", e))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            error!("Git clone failed: {}", err_msg);
            return Err(format!("Clone 失败: {}", err_msg));
        }

        // Check package.json and run npm install if exists
        let pkg_json = target_path.join("package.json");
        if pkg_json.exists() {
            info!("package.json found in skill, running npm install");
            let _ = Command::new("npm")
                .current_dir(&target_path)
                .arg("install")
                .output();
        }

        Ok(SkillInfo {
            id: repo_name.to_string(),
            name: repo_name.to_string(),
            source_url: Some(url.to_string()),
            local_path: target_path.to_string_lossy().to_string(),
            status: "installed".to_string(),
        })
    }

    /// Update skill via git pull
    pub fn update_skill(id: &str) -> Result<(), String> {
        let target_path =
            Self::resolve_skill_path(id).ok_or_else(|| format!("找不到本地目录: {}", id))?;

        let output = Command::new("git")
            .current_dir(&target_path)
            .args(["pull", "--ff-only"])
            .output()
            .map_err(|e| format!("启动 git 失败: {}", e))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("更新失败: {}", err_msg));
        }

        Ok(())
    }

    /// Remove skill completely via file deletion
    pub fn remove_skill(id: &str) -> Result<(), String> {
        let Some(target_path) = Self::resolve_skill_path(id) else {
            return Ok(());
        };

        if target_path.exists() {
            fs::remove_dir_all(&target_path).map_err(|e| format!("删除本地目录失败: {}", e))?;
        }
        Ok(())
    }
}
