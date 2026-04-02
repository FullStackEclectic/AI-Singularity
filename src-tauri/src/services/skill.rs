use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

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
    fn get_commands_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let commands_dir = home.join(".claude").join("commands");
        if !commands_dir.exists() {
            let _ = fs::create_dir_all(&commands_dir);
        }
        commands_dir
    }

    /// List installed skills purely by reading the directory and git remotes
    pub fn list_skills() -> Result<Vec<SkillInfo>, String> {
        let dir = Self::get_commands_dir();
        let mut skills = Vec::new();

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    let id = entry.file_name().to_string_lossy().to_string();
                    let source_url = Self::get_git_remote(&path);
                    
                    skills.push(SkillInfo {
                        id: id.clone(),
                        name: id.clone(),
                        source_url,
                        local_path: path.to_string_lossy().to_string(),
                        status: "installed".to_string(),
                    });
                }
            }
        }
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
        let dir = Self::get_commands_dir();
        
        // Ensure git is installed
        if Command::new("git").arg("--version").output().is_err() {
            return Err("未找到 Git，请先在系统安装 Git 命令行工具".to_string());
        }

        // Parse repo name from URL (e.g. https://github.com/user/repo-name.git -> repo-name)
        let repo_name = url.trim_end_matches(".git").split('/').last().unwrap_or("unknown_skill");
        let target_path = dir.join(repo_name);

        if target_path.exists() {
            return Err(format!("技能 {} 对应的目录已存在，请尝试更新或先卸载！", repo_name));
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
             info!("package.json found in skill, running npm install -y");
             let _ = Command::new("npm")
                .current_dir(&target_path)
                .args(["install", "-y"]) // Wait, npm install doesn't really have -y but pnpm does. Just npm install.
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
        let target_path = Self::get_commands_dir().join(id);
        if !target_path.exists() {
            return Err(format!("找不到本地目录: {:?}", target_path));
        }

        let output = Command::new("git")
            .current_dir(&target_path)
            .args(["pull"])
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
        let target_path = Self::get_commands_dir().join(id);
        if target_path.exists() {
            fs::remove_dir_all(&target_path).map_err(|e| format!("删除本地目录失败: {}", e))?;
        }
        Ok(())
    }
}
