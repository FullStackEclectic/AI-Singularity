use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{Emitter, Window};

#[derive(Serialize, Deserialize, Clone)]
pub struct ToolStatus {
    pub id: String,
    pub is_installed: bool,
    pub version: Option<String>,
}

pub struct ProvisionerManager;

impl ProvisionerManager {
    pub fn check_status(tool_id: &str) -> AppResult<ToolStatus> {
        let (is_installed, version) = match tool_id {
            "claude_code" => {
                // windows use cmd /c claude --version
                let output = Command::new("cmd")
                    .args(&["/c", "claude", "--version"])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        (true, Some(ver))
                    } else {
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            }
            "aider" => {
                let output = Command::new("cmd")
                    .args(&["/c", "aider", "--version"])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        (true, Some(ver))
                    } else {
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            }
            _ => (false, None),
        };

        Ok(ToolStatus {
            id: tool_id.to_string(),
            is_installed,
            version,
        })
    }

    pub fn deploy_tool(tool_id: &str, window: Window) -> AppResult<()> {
        let (cmd_name, args) = match tool_id {
            "claude_code" => (
                "cmd",
                vec!["/c", "npm", "install", "-g", "@anthropic-ai/claude-code"],
            ),
            "aider" => ("cmd", vec!["/c", "pip", "install", "aider-chat"]),
            _ => return Err(AppError::Other(anyhow::anyhow!("未知的兵器库组件标识"))),
        };

        let mut child = Command::new(cmd_name)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Other(anyhow::anyhow!("无法拉起部署装载机: {}", e)))?;

        let _ = window.emit(
            "provisioner-event",
            format!("🚀 正在初始化远程装载: {}...", tool_id),
        );

        if let Some(stdout) = child.stdout.take() {
            let win_clone = window.clone();
            let tid = tool_id.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        let _ =
                            win_clone.emit("provisioner-event", format!("[{} STDOUT] {}", tid, l));
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let win_clone = window.clone();
            let tid = tool_id.to_string();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        let _ =
                            win_clone.emit("provisioner-event", format!("[{} STDERR] {}", tid, l));
                    }
                }
            });
        }

        let status = child
            .wait()
            .map_err(|e| AppError::Other(anyhow::anyhow!("进程等待异常: {}", e)))?;

        if status.success() {
            let _ = window.emit(
                "provisioner-event",
                format!("✅ [{}] 机甲核弹载入完毕！状态: ACTIVE", tool_id),
            );
            Ok(())
        } else {
            let _ = window.emit(
                "provisioner-event",
                format!("❌ [{}] 远程装载失败，脱轨终止。", tool_id),
            );
            Err(AppError::Other(anyhow::anyhow!(
                "部署失败，请检查本机基础环境 (Node/Python) 是否就绪。"
            )))
        }
    }
}
