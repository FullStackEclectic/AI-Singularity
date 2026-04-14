use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const ANNOUNCEMENT_URL: &str =
    "https://raw.githubusercontent.com/tarui/AI-Singularity/main/announcements.json";
const ANNOUNCEMENT_CACHE_FILE: &str = "announcement_cache.json";
const ANNOUNCEMENT_READ_IDS_FILE: &str = "announcement_read_ids.json";
const ANNOUNCEMENT_LOCAL_OVERRIDE_FILE: &str = "announcements.local.json";
const CACHE_TTL_MS: i64 = 3_600_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    pub id: String,
    #[serde(rename = "type", default)]
    pub announcement_type: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub action: Option<AnnouncementAction>,
    #[serde(default = "default_target_versions")]
    pub target_versions: String,
    #[serde(default)]
    pub target_languages: Option<Vec<String>>,
    #[serde(default)]
    pub show_once: Option<bool>,
    #[serde(default)]
    pub popup: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementState {
    pub announcements: Vec<Announcement>,
    pub unread_ids: Vec<String>,
    pub popup_announcement: Option<Announcement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementResponse {
    #[serde(default)]
    pub announcements: Vec<Announcement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnouncementCache {
    pub time: i64,
    pub data: Vec<Announcement>,
}

pub struct AnnouncementService;

impl AnnouncementService {
    pub async fn get_state(app_data_dir: &Path, locale: &str) -> Result<AnnouncementState, String> {
        let current_version = env!("CARGO_PKG_VERSION");
        let raw = Self::load_announcements_raw(app_data_dir).await?;
        let announcements = filter_announcements(raw, current_version, locale);
        let read_ids = get_read_ids(app_data_dir)?;
        let unread_ids: Vec<String> = announcements
            .iter()
            .filter(|item| !read_ids.contains(&item.id))
            .map(|item| item.id.clone())
            .collect();
        let popup_announcement = announcements
            .iter()
            .find(|item| item.popup && !read_ids.contains(&item.id))
            .cloned();

        Ok(AnnouncementState {
            announcements,
            unread_ids,
            popup_announcement,
        })
    }

    pub async fn mark_as_read(app_data_dir: &Path, id: &str) -> Result<(), String> {
        let mut read_ids = get_read_ids(app_data_dir)?;
        if !read_ids.iter().any(|item| item == id) {
            read_ids.push(id.to_string());
            save_read_ids(app_data_dir, &read_ids)?;
        }
        Ok(())
    }

    pub async fn mark_all_as_read(app_data_dir: &Path, locale: &str) -> Result<(), String> {
        let state = Self::get_state(app_data_dir, locale).await?;
        let ids: Vec<String> = state.announcements.iter().map(|item| item.id.clone()).collect();
        save_read_ids(app_data_dir, &ids)
    }

    pub async fn force_refresh(app_data_dir: &Path, locale: &str) -> Result<AnnouncementState, String> {
        remove_cache(app_data_dir)?;
        Self::get_state(app_data_dir, locale).await
    }

    async fn load_announcements_raw(app_data_dir: &Path) -> Result<Vec<Announcement>, String> {
        if let Some(local) = load_local_announcements(app_data_dir)? {
            return Ok(local);
        }

        if let Some(cache) = load_cache(app_data_dir)? {
            let age_ms = Utc::now().timestamp_millis() - cache.time;
            if age_ms < CACHE_TTL_MS {
                return Ok(cache.data);
            }
        }

        match fetch_remote_announcements().await {
            Ok(items) => {
                let _ = save_cache(app_data_dir, &items);
                Ok(items)
            }
            Err(err) => {
                if let Some(cache) = load_cache(app_data_dir)? {
                    return Ok(cache.data);
                }
                Err(err)
            }
        }
    }
}

fn default_target_versions() -> String {
    "*".to_string()
}

fn parse_announcements_json(content: &str) -> Result<Vec<Announcement>, String> {
    if let Ok(parsed) = serde_json::from_str::<AnnouncementResponse>(content) {
        return Ok(parsed.announcements);
    }
    serde_json::from_str::<Vec<Announcement>>(content)
        .map_err(|e| format!("解析公告内容失败: {}", e))
}

fn load_local_announcements(app_data_dir: &Path) -> Result<Option<Vec<Announcement>>, String> {
    let local_override = app_data_dir.join(ANNOUNCEMENT_LOCAL_OVERRIDE_FILE);
    if local_override.exists() {
        let content = fs::read_to_string(&local_override)
            .map_err(|e| format!("读取本地公告覆盖失败: {}", e))?;
        return parse_announcements_json(&content).map(Some);
    }
    Ok(None)
}

fn cache_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(ANNOUNCEMENT_CACHE_FILE)
}

fn read_ids_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(ANNOUNCEMENT_READ_IDS_FILE)
}

fn load_cache(app_data_dir: &Path) -> Result<Option<AnnouncementCache>, String> {
    let path = cache_path(app_data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|e| format!("读取公告缓存失败: {}", e))?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    let cache =
        serde_json::from_str::<AnnouncementCache>(&content).map_err(|e| format!("解析公告缓存失败: {}", e))?;
    Ok(Some(cache))
}

fn save_cache(app_data_dir: &Path, announcements: &[Announcement]) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
    let cache = AnnouncementCache {
        time: Utc::now().timestamp_millis(),
        data: announcements.to_vec(),
    };
    let content =
        serde_json::to_string_pretty(&cache).map_err(|e| format!("序列化公告缓存失败: {}", e))?;
    fs::write(cache_path(app_data_dir), content).map_err(|e| format!("写入公告缓存失败: {}", e))
}

fn remove_cache(app_data_dir: &Path) -> Result<(), String> {
    let path = cache_path(app_data_dir);
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("删除公告缓存失败: {}", e))?;
    }
    Ok(())
}

fn get_read_ids(app_data_dir: &Path) -> Result<Vec<String>, String> {
    let path = read_ids_path(app_data_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| format!("读取公告已读状态失败: {}", e))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|e| format!("解析公告已读状态失败: {}", e))
}

fn save_read_ids(app_data_dir: &Path, ids: &[String]) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
    let content =
        serde_json::to_string_pretty(ids).map_err(|e| format!("序列化公告已读状态失败: {}", e))?;
    fs::write(read_ids_path(app_data_dir), content).map_err(|e| format!("写入公告已读状态失败: {}", e))
}

fn parse_datetime_millis(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
}

fn parse_version(value: &str) -> Vec<i64> {
    let trimmed = value.trim_start_matches(|c: char| !c.is_ascii_digit());
    trimmed
        .split('.')
        .map(|part| part.parse::<i64>().unwrap_or(0))
        .collect()
}

fn match_version(current_version: &str, pattern: &str) -> bool {
    if pattern.trim().is_empty() || pattern.trim() == "*" {
        return true;
    }
    let (operator, version_str) = if let Some(rest) = pattern.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = pattern.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = pattern.strip_prefix('>') {
        (">", rest)
    } else if let Some(rest) = pattern.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = pattern.strip_prefix('=') {
        ("=", rest)
    } else {
        ("=", pattern)
    };

    let current = parse_version(current_version);
    let target = parse_version(version_str);
    let mut cmp = 0;
    for idx in 0..3 {
        let c = *current.get(idx).unwrap_or(&0);
        let t = *target.get(idx).unwrap_or(&0);
        if c != t {
            cmp = if c > t { 1 } else { -1 };
            break;
        }
    }
    match operator {
        ">=" => cmp >= 0,
        "<=" => cmp <= 0,
        ">" => cmp > 0,
        "<" => cmp < 0,
        _ => cmp == 0,
    }
}

fn is_language_match(current_locale: &str, target_languages: &[String]) -> bool {
    if target_languages.is_empty() || target_languages.iter().any(|lang| lang == "*") {
        return true;
    }
    let current = current_locale.to_lowercase();
    target_languages.iter().any(|lang| {
        let normalized = lang.to_lowercase();
        normalized == current || current.starts_with(&(normalized + "-"))
    })
}

fn filter_announcements(raw: Vec<Announcement>, current_version: &str, locale: &str) -> Vec<Announcement> {
    let now = Utc::now().timestamp_millis();
    let mut filtered: Vec<Announcement> = raw
        .into_iter()
        .filter(|announcement| {
            let target_versions = if announcement.target_versions.trim().is_empty() {
                "*"
            } else {
                announcement.target_versions.as_str()
            };
            if !match_version(current_version, target_versions) {
                return false;
            }
            if let Some(target_languages) = &announcement.target_languages {
                if !is_language_match(locale, target_languages) {
                    return false;
                }
            }
            if let Some(expires_at) = &announcement.expires_at {
                if let Some(expire_ms) = parse_datetime_millis(expires_at) {
                    if expire_ms < now {
                        return false;
                    }
                }
            }
            true
        })
        .collect();

    filtered.sort_by(|a, b| {
        let a_time = parse_datetime_millis(&a.created_at).unwrap_or(0);
        let b_time = parse_datetime_millis(&b.created_at).unwrap_or(0);
        b_time.cmp(&a_time).then(b.priority.cmp(&a.priority))
    });
    filtered
}

async fn fetch_remote_announcements() -> Result<Vec<Announcement>, String> {
    let client = reqwest::Client::builder()
        .user_agent("AI-Singularity")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建公告 HTTP 客户端失败: {}", e))?;

    let url = format!("{}?t={}", ANNOUNCEMENT_URL, Utc::now().timestamp_millis());
    let response = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .send()
        .await
        .map_err(|e| format!("拉取远端公告失败: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("远端公告接口返回异常状态: {}", response.status()));
    }
    let text = response
        .text()
        .await
        .map_err(|e| format!("读取远端公告响应失败: {}", e))?;
    parse_announcements_json(&text)
}
