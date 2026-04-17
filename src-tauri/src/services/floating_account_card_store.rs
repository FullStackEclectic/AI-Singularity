use chrono::Utc;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const FLOATING_ACCOUNT_CARDS_FILE: &str = "floating_account_cards.json";
const DEFAULT_BOUND_PLATFORMS: [&str; 2] = ["codex", "gemini"];
const MIN_WIDTH: f64 = 260.0;
const MIN_HEIGHT: f64 = 120.0;

lazy_static! {
    static ref STORE_LOCK: Mutex<()> = Mutex::new(());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatingAccountCard {
    pub id: String,
    pub scope: String,
    #[serde(default)]
    pub instance_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub bound_platforms: Vec<String>,
    #[serde(default)]
    pub window_label: Option<String>,
    #[serde(default)]
    pub always_on_top: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default = "default_visible")]
    pub visible: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFloatingAccountCardInput {
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bound_platforms: Option<Vec<String>>,
    #[serde(default)]
    pub window_label: Option<String>,
    #[serde(default)]
    pub always_on_top: Option<bool>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub collapsed: Option<bool>,
    #[serde(default)]
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatingAccountCardPatch {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub instance_id: Option<Option<String>>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bound_platforms: Option<Vec<String>>,
    #[serde(default)]
    pub window_label: Option<Option<String>>,
    #[serde(default)]
    pub always_on_top: Option<bool>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub collapsed: Option<bool>,
    #[serde(default)]
    pub visible: Option<bool>,
}

pub struct FloatingAccountCardStore;

impl FloatingAccountCardStore {
    pub fn list_cards(app_data_dir: &Path) -> Result<Vec<FloatingAccountCard>, String> {
        let _guard = STORE_LOCK.lock().map_err(|_| "浮窗存储锁获取失败".to_string())?;
        let (cards, changed) = Self::load_and_normalize_cards(app_data_dir)?;
        if changed {
            Self::save_cards_unlocked(app_data_dir, &cards)?;
        }
        Ok(cards)
    }

    pub fn create_card(
        app_data_dir: &Path,
        input: CreateFloatingAccountCardInput,
    ) -> Result<FloatingAccountCard, String> {
        let _guard = STORE_LOCK.lock().map_err(|_| "浮窗存储锁获取失败".to_string())?;
        let (mut cards, changed) = Self::load_and_normalize_cards(app_data_dir)?;
        if changed {
            Self::save_cards_unlocked(app_data_dir, &cards)?;
        }

        let instance_id = normalize_optional_string(input.instance_id.as_deref());
        let scope = normalize_scope(&input.scope, instance_id.is_some());
        if scope == "instance" && instance_id.is_none() {
            return Err("实例浮窗必须绑定实例 ID".to_string());
        }

        let title = normalize_optional_string(input.title.as_deref())
            .unwrap_or_else(|| default_title(&scope, instance_id.as_deref()));

        let mut card = FloatingAccountCard {
            id: format!("floating-{}", uuid::Uuid::new_v4()),
            scope,
            instance_id,
            title,
            bound_platforms: normalize_bound_platforms(input.bound_platforms),
            window_label: normalize_optional_string(input.window_label.as_deref()),
            always_on_top: input.always_on_top.unwrap_or(false),
            x: input.x.unwrap_or(36.0),
            y: input.y.unwrap_or(96.0),
            width: input.width.unwrap_or(320.0),
            height: input.height.unwrap_or(220.0),
            collapsed: input.collapsed.unwrap_or(false),
            visible: input.visible.unwrap_or(true),
            updated_at: Utc::now().to_rfc3339(),
        };
        sanitize_card(&mut card);

        cards.push(card.clone());
        Self::save_cards_unlocked(app_data_dir, &cards)?;
        Ok(card)
    }

    pub fn update_card(
        app_data_dir: &Path,
        id: &str,
        patch: FloatingAccountCardPatch,
        expected_updated_at: Option<&str>,
    ) -> Result<FloatingAccountCard, String> {
        let _guard = STORE_LOCK.lock().map_err(|_| "浮窗存储锁获取失败".to_string())?;
        let (mut cards, changed) = Self::load_and_normalize_cards(app_data_dir)?;
        if changed {
            Self::save_cards_unlocked(app_data_dir, &cards)?;
        }

        let idx = cards
            .iter()
            .position(|card| card.id == id)
            .ok_or_else(|| "未找到对应浮窗".to_string())?;

        if let Some(expected) = expected_updated_at.map(|value| value.trim()).filter(|value| !value.is_empty()) {
            if cards[idx].updated_at != expected {
                return Err(format!("floating_card_conflict:{}", cards[idx].updated_at));
            }
        }

        let mut next = cards[idx].clone();

        if let Some(scope) = patch.scope {
            next.scope = normalize_scope(&scope, next.instance_id.is_some());
        }
        if let Some(instance_id) = patch.instance_id {
            next.instance_id = normalize_optional_string(instance_id.as_deref());
        }
        if let Some(title) = patch.title {
            next.title = normalize_optional_string(Some(title.as_str()))
                .unwrap_or_else(|| default_title(&next.scope, next.instance_id.as_deref()));
        }
        if let Some(bound_platforms) = patch.bound_platforms {
            next.bound_platforms = normalize_bound_platforms(Some(bound_platforms));
        }
        if let Some(window_label) = patch.window_label {
            next.window_label = normalize_optional_string(window_label.as_deref());
        }
        if let Some(always_on_top) = patch.always_on_top {
            next.always_on_top = always_on_top;
        }
        if let Some(x) = patch.x.filter(|value| value.is_finite()) {
            next.x = x.max(0.0);
        }
        if let Some(y) = patch.y.filter(|value| value.is_finite()) {
            next.y = y.max(0.0);
        }
        if let Some(width) = patch.width.filter(|value| value.is_finite()) {
            next.width = width.max(MIN_WIDTH);
        }
        if let Some(height) = patch.height.filter(|value| value.is_finite()) {
            next.height = height.max(MIN_HEIGHT);
        }
        if let Some(collapsed) = patch.collapsed {
            next.collapsed = collapsed;
        }
        if let Some(visible) = patch.visible {
            next.visible = visible;
        }

        let normalized_scope = normalize_scope(&next.scope, next.instance_id.is_some());
        if normalized_scope == "instance" && next.instance_id.is_none() {
            return Err("实例浮窗必须绑定实例 ID".to_string());
        }
        if normalized_scope == "global" {
            next.instance_id = None;
        }
        next.scope = normalized_scope;
        if next.title.trim().is_empty() {
            next.title = default_title(&next.scope, next.instance_id.as_deref());
        }
        next.updated_at = Utc::now().to_rfc3339();
        sanitize_card(&mut next);

        cards[idx] = next.clone();
        Self::save_cards_unlocked(app_data_dir, &cards)?;
        Ok(next)
    }

    pub fn delete_card(app_data_dir: &Path, id: &str) -> Result<bool, String> {
        let _guard = STORE_LOCK.lock().map_err(|_| "浮窗存储锁获取失败".to_string())?;
        let (mut cards, changed) = Self::load_and_normalize_cards(app_data_dir)?;
        if changed {
            Self::save_cards_unlocked(app_data_dir, &cards)?;
        }

        let before = cards.len();
        cards.retain(|card| card.id != id);
        if cards.len() == before {
            return Ok(false);
        }

        Self::save_cards_unlocked(app_data_dir, &cards)?;
        Ok(true)
    }

    pub fn reconcile_deleted_instances(
        app_data_dir: &Path,
        valid_instance_ids: &[String],
    ) -> Result<Vec<FloatingAccountCard>, String> {
        let _guard = STORE_LOCK.lock().map_err(|_| "浮窗存储锁获取失败".to_string())?;
        let (mut cards, mut changed) = Self::load_and_normalize_cards(app_data_dir)?;

        let valid_set = valid_instance_ids
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<HashSet<_>>();
        let mut downgraded = Vec::new();

        for card in &mut cards {
            if card.scope != "instance" {
                continue;
            }
            let Some(instance_id) = card.instance_id.clone() else {
                continue;
            };
            if valid_set.contains(&instance_id) {
                continue;
            }

            card.scope = "global".to_string();
            card.instance_id = None;
            card.updated_at = Utc::now().to_rfc3339();
            if card.title.trim().is_empty() {
                card.title = default_title("global", None);
            }
            downgraded.push(card.clone());
            changed = true;
        }

        if changed {
            Self::save_cards_unlocked(app_data_dir, &cards)?;
        }

        Ok(downgraded)
    }

    fn load_and_normalize_cards(app_data_dir: &Path) -> Result<(Vec<FloatingAccountCard>, bool), String> {
        let cards = Self::load_cards_unlocked(app_data_dir)?;
        let mut changed = false;
        let mut normalized = Vec::with_capacity(cards.len());
        for mut card in cards {
            let before = serde_json::to_string(&card).unwrap_or_default();
            if card.id.trim().is_empty() {
                card.id = format!("floating-{}", uuid::Uuid::new_v4());
            }
            sanitize_card(&mut card);
            let after = serde_json::to_string(&card).unwrap_or_default();
            if before != after {
                changed = true;
            }
            normalized.push(card);
        }
        normalized.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok((normalized, changed))
    }

    fn load_cards_unlocked(app_data_dir: &Path) -> Result<Vec<FloatingAccountCard>, String> {
        let path = store_path(app_data_dir);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("读取浮窗配置失败: {}", e))?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str::<Vec<FloatingAccountCard>>(&raw)
            .map_err(|e| format!("解析浮窗配置失败: {}", e))
    }

    fn save_cards_unlocked(app_data_dir: &Path, cards: &[FloatingAccountCard]) -> Result<(), String> {
        fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;
        let path = store_path(app_data_dir);
        let content = serde_json::to_string_pretty(cards)
            .map_err(|e| format!("序列化浮窗配置失败: {}", e))?;
        fs::write(path, format!("{}\n", content))
            .map_err(|e| format!("写入浮窗配置失败: {}", e))
    }
}

fn store_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FLOATING_ACCOUNT_CARDS_FILE)
}

fn default_visible() -> bool {
    true
}

fn sanitize_card(card: &mut FloatingAccountCard) {
    card.instance_id = normalize_optional_string(card.instance_id.as_deref());
    card.scope = normalize_scope(&card.scope, card.instance_id.is_some());
    if card.scope == "global" {
        card.instance_id = None;
    }
    card.title = normalize_optional_string(Some(card.title.as_str()))
        .unwrap_or_else(|| default_title(&card.scope, card.instance_id.as_deref()));
    card.bound_platforms = normalize_bound_platforms(Some(card.bound_platforms.clone()));
    card.window_label = normalize_optional_string(card.window_label.as_deref());
    if !card.x.is_finite() || card.x < 0.0 {
        card.x = 0.0;
    }
    if !card.y.is_finite() || card.y < 0.0 {
        card.y = 0.0;
    }
    if !card.width.is_finite() || card.width < MIN_WIDTH {
        card.width = MIN_WIDTH;
    }
    if !card.height.is_finite() || card.height < MIN_HEIGHT {
        card.height = MIN_HEIGHT;
    }
    if chrono::DateTime::parse_from_rfc3339(card.updated_at.as_str()).is_err() {
        card.updated_at = Utc::now().to_rfc3339();
    }
}

fn normalize_scope(raw: &str, has_instance: bool) -> String {
    let scope = raw.trim().to_ascii_lowercase();
    if scope == "instance" && has_instance {
        "instance".to_string()
    } else {
        "global".to_string()
    }
}

fn normalize_optional_string(raw: Option<&str>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_bound_platforms(raw: Option<Vec<String>>) -> Vec<String> {
    let source = raw.unwrap_or_else(|| DEFAULT_BOUND_PLATFORMS.iter().map(|item| item.to_string()).collect());
    let mut dedup = Vec::new();
    for item in source {
        let normalized = item.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            continue;
        }
        if !dedup.iter().any(|existing: &String| existing == &normalized) {
            dedup.push(normalized);
        }
    }
    if dedup.is_empty() {
        return DEFAULT_BOUND_PLATFORMS.iter().map(|item| item.to_string()).collect();
    }
    dedup
}

fn default_title(scope: &str, _instance_id: Option<&str>) -> String {
    if scope == "instance" {
        "实例账号浮窗".to_string()
    } else {
        "全局账号浮窗".to_string()
    }
}
