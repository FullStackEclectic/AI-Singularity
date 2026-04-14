use chrono::{DateTime, Local};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Serialize)]
pub struct DesktopLogFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesktopLogReadResult {
    pub name: String,
    pub path: String,
    pub total_lines: usize,
    pub matched_lines: usize,
    pub content: String,
}

pub struct LogsService;

impl LogsService {
    pub fn init_runtime_logging(logs_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(logs_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;
        let runtime_log_path = logs_dir.join("runtime.log");
        let writer_path = runtime_log_path.clone();

        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(move || {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&writer_path)
                    .expect("failed to open runtime.log")
            })
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_level(true)
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);
        Ok(())
    }

    pub fn list_logs(logs_dir: &Path) -> Result<Vec<DesktopLogFile>, String> {
        fs::create_dir_all(logs_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;
        let mut files = Vec::new();

        for entry in fs::read_dir(logs_dir).map_err(|e| format!("读取日志目录失败: {}", e))? {
            let entry = entry.map_err(|e| format!("读取日志条目失败: {}", e))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !name.to_ascii_lowercase().ends_with(".log") {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|e| format!("读取日志文件信息失败: {}", e))?;
            let modified_at = metadata.modified().ok().map(|time| {
                DateTime::<Local>::from(time)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            });
            let kind = if name.starts_with("crash_") {
                "crash"
            } else if name == "runtime.log" {
                "runtime"
            } else {
                "custom"
            };
            files.push(DesktopLogFile {
                name: name.to_string(),
                path: path.to_string_lossy().to_string(),
                size: metadata.len(),
                modified_at,
                kind: kind.to_string(),
            });
        }

        files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at).then_with(|| a.name.cmp(&b.name)));
        Ok(files)
    }

    pub fn read_log(
        logs_dir: &Path,
        name: &str,
        lines: usize,
        query: Option<&str>,
    ) -> Result<DesktopLogReadResult, String> {
        let path = Self::resolve_log_path(logs_dir, name)?;
        let file = fs::File::open(&path).map_err(|e| format!("打开日志文件失败: {}", e))?;
        let reader = BufReader::new(file);
        let query_lower = query
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty());

        let mut all_lines = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| format!("读取日志内容失败: {}", e))?;
            all_lines.push(line);
        }
        let total_lines = all_lines.len();

        let matched: Vec<String> = match query_lower {
            Some(ref filter) => all_lines
                .into_iter()
                .filter(|line| line.to_ascii_lowercase().contains(filter))
                .collect(),
            None => all_lines,
        };
        let matched_lines = matched.len();
        let start = matched_lines.saturating_sub(lines);
        let content = matched[start..].join("\n");

        Ok(DesktopLogReadResult {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            total_lines,
            matched_lines,
            content,
        })
    }

    pub fn export_log(logs_dir: &Path, name: &str, destination: &Path) -> Result<(), String> {
        let source = Self::resolve_log_path(logs_dir, name)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建导出目录失败: {}", e))?;
        }
        fs::copy(&source, destination).map_err(|e| format!("导出日志失败: {}", e))?;
        Ok(())
    }

    fn resolve_log_path(logs_dir: &Path, name: &str) -> Result<PathBuf, String> {
        let safe_name = Path::new(name)
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "日志文件名无效".to_string())?;
        let path = logs_dir.join(safe_name);
        if !path.exists() || !path.is_file() {
            return Err("指定日志文件不存在".to_string());
        }
        Ok(path)
    }
}
