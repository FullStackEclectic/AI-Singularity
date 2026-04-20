use crate::db::Database;
use crate::error::AppError;
use crate::models::{Model, Platform};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct ModelPricingOverride {
    platform: String,
    model_id: String,
    fixed_price: Option<f64>,
    request_price: Option<f64>,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
    pricing_currency: Option<String>,
    pricing_unit: Option<String>,
    note: Option<String>,
    updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SaveModelPricingRequest {
    pub platform: Platform,
    pub model_id: String,
    pub fixed_price: Option<f64>,
    pub request_price: Option<f64>,
    pub input_price_per_1m: Option<f64>,
    pub output_price_per_1m: Option<f64>,
    pub pricing_currency: Option<String>,
    pub pricing_unit: Option<String>,
    pub note: Option<String>,
}

pub struct ModelCatalogService {
    db: Database,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedModelPricing {
    pub input_price_per_1m: Option<f64>,
    pub output_price_per_1m: Option<f64>,
    pub request_price: Option<f64>,
}

impl ModelCatalogService {
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }

    pub fn list_models(&self) -> Result<Vec<Model>, AppError> {
        let overrides = self.load_overrides()?;
        Ok(base_catalog()
            .into_iter()
            .map(|model| merge_model_override(model, &overrides))
            .collect())
    }

    pub fn get_platform_models(&self, platform: &Platform) -> Result<Vec<Model>, AppError> {
        Ok(self
            .list_models()?
            .into_iter()
            .filter(|item| &item.platform == platform)
            .collect())
    }

    pub fn save_pricing(&self, request: SaveModelPricingRequest) -> Result<Model, AppError> {
        let model = find_base_model(&request.platform, &request.model_id)
            .ok_or_else(|| anyhow::anyhow!("模型不存在，无法保存基础价格"))?;

        let note = normalize_optional_string(request.note);
        let pricing_currency = normalize_optional_string(request.pricing_currency);
        let pricing_unit = normalize_optional_string(request.pricing_unit);
        let mut fixed_price = request.fixed_price;
        let mut request_price = request.request_price;
        let mut input_price_per_1m = request.input_price_per_1m;
        let mut output_price_per_1m = request.output_price_per_1m;

        if matches!(pricing_unit.as_deref(), Some(unit) if is_fixed_pricing_unit(unit)) {
            request_price = None;
            input_price_per_1m = None;
            output_price_per_1m = None;
        } else if matches!(pricing_unit.as_deref(), Some("1m_tokens")) {
            fixed_price = None;
        }

        let has_override = fixed_price.is_some()
            || request_price.is_some()
            || input_price_per_1m.is_some()
            || output_price_per_1m.is_some()
            || pricing_currency.is_some()
            || pricing_unit.is_some()
            || note.is_some();

        if has_override {
            let platform_key = platform_key(&request.platform);
            self.db.execute(
                "INSERT INTO model_pricing_overrides (
                    platform,
                    model_id,
                    fixed_price,
                    request_price,
                    input_price_per_1m,
                    output_price_per_1m,
                    pricing_currency,
                    pricing_unit,
                    note,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
                ON CONFLICT(platform, model_id) DO UPDATE SET
                    fixed_price = excluded.fixed_price,
                    request_price = excluded.request_price,
                    input_price_per_1m = excluded.input_price_per_1m,
                    output_price_per_1m = excluded.output_price_per_1m,
                    pricing_currency = excluded.pricing_currency,
                    pricing_unit = excluded.pricing_unit,
                    note = excluded.note,
                    updated_at = datetime('now')",
                &[
                    &platform_key,
                    &request.model_id,
                    &fixed_price,
                    &request_price,
                    &input_price_per_1m,
                    &output_price_per_1m,
                    &pricing_currency,
                    &pricing_unit,
                    &note,
                ],
            )?;
        } else {
            self.reset_pricing(&request.platform, &request.model_id)?;
        }

        let overrides = self.load_overrides()?;
        Ok(merge_model_override(model, &overrides))
    }

    pub fn reset_pricing(&self, platform: &Platform, model_id: &str) -> Result<(), AppError> {
        let platform_key = platform_key(platform);
        self.db.execute(
            "DELETE FROM model_pricing_overrides WHERE platform = ?1 AND model_id = ?2",
            &[&platform_key, &model_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn resolve_rates(
        &self,
        platform: Option<&str>,
        model_name: &str,
    ) -> Result<(Option<f64>, Option<f64>), AppError> {
        let pricing = self.resolve_pricing(platform, model_name)?;
        Ok((pricing.input_price_per_1m, pricing.output_price_per_1m))
    }

    pub fn resolve_pricing(
        &self,
        platform: Option<&str>,
        model_name: &str,
    ) -> Result<ResolvedModelPricing, AppError> {
        let normalized_model = model_name.trim().to_lowercase();
        if normalized_model.is_empty() {
            return Ok(ResolvedModelPricing::default());
        }

        if let Some(platform_name) = platform {
            let lower_platform = normalize_platform_lookup(platform_name);
            let input = self.db.query_row(
                "SELECT fixed_price, request_price, input_price_per_1m, output_price_per_1m, pricing_currency, pricing_unit
                 FROM model_pricing_overrides
                 WHERE replace(lower(platform), '_', '') = ?1 AND lower(model_id) = lower(?2)",
                &[&lower_platform, &normalized_model],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            );
            if let Ok(found) = input {
                let base_model = base_catalog().into_iter().find(|item| {
                    normalize_platform_lookup(&platform_key(&item.platform)) == lower_platform
                        && model_matches(item, &normalized_model)
                });
                let currency = found
                    .4
                    .as_deref()
                    .or(base_model.as_ref().and_then(|model| model.pricing_currency.as_deref()))
                    .unwrap_or("USD");
                let unit = found
                    .5
                    .as_deref()
                    .or(base_model.as_ref().and_then(|model| model.pricing_unit.as_deref()))
                    .unwrap_or_else(|| infer_default_pricing_unit(found.0, found.2));

                if is_cost_compatible_pricing(currency, unit) {
                    return Ok(ResolvedModelPricing {
                        input_price_per_1m: if unit == "1m_tokens" { found.2 } else { None },
                        output_price_per_1m: if unit == "1m_tokens" { found.3 } else { None },
                        request_price: resolve_request_fee(unit, found.0, found.1),
                    });
                }

                return Ok(ResolvedModelPricing::default());
            }
        }

        if let Some(model) = base_catalog()
            .into_iter()
            .find(|item| model_matches(item, &normalized_model))
        {
            let currency = model.pricing_currency.as_deref().unwrap_or("USD");
            let unit = model
                .pricing_unit
                .as_deref()
                .unwrap_or_else(|| infer_default_pricing_unit(model.fixed_price, model.input_price_per_1m));
            if is_cost_compatible_pricing(currency, unit) {
                return Ok(ResolvedModelPricing {
                    input_price_per_1m: if unit == "1m_tokens" {
                        model.input_price_per_1m
                    } else {
                        None
                    },
                    output_price_per_1m: if unit == "1m_tokens" {
                        model.output_price_per_1m
                    } else {
                        None
                    },
                    request_price: resolve_request_fee(unit, model.fixed_price, model.request_price),
                });
            }
        }

        Ok(ResolvedModelPricing::default())
    }

    fn load_overrides(&self) -> Result<HashMap<String, ModelPricingOverride>, AppError> {
        let rows = self.db.query_rows(
            "SELECT platform, model_id, fixed_price, request_price, input_price_per_1m, output_price_per_1m, pricing_currency, pricing_unit, note, updated_at
             FROM model_pricing_overrides",
            &[],
            |row| {
                Ok(ModelPricingOverride {
                    platform: row.get(0)?,
                    model_id: row.get(1)?,
                    fixed_price: row.get(2)?,
                    request_price: row.get(3)?,
                    input_price_per_1m: row.get(4)?,
                    output_price_per_1m: row.get(5)?,
                    pricing_currency: row.get(6)?,
                    pricing_unit: row.get(7)?,
                    note: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )?;

        Ok(rows
            .into_iter()
            .map(|item| (override_key(&item.platform, &item.model_id), item))
            .collect())
    }
}

fn is_cost_compatible_pricing(currency: &str, unit: &str) -> bool {
    currency.eq_ignore_ascii_case("USD") && matches!(unit, "1m_tokens" | "request")
}

fn is_fixed_pricing_unit(unit: &str) -> bool {
    matches!(unit, "request" | "image")
}

fn infer_default_pricing_unit(
    fixed_price: Option<f64>,
    input_price_per_1m: Option<f64>,
) -> &'static str {
    if fixed_price.is_some() && input_price_per_1m.is_none() {
        "request"
    } else {
        "1m_tokens"
    }
}

fn resolve_request_fee(
    pricing_unit: &str,
    fixed_price: Option<f64>,
    request_price: Option<f64>,
) -> Option<f64> {
    if pricing_unit == "request" {
        fixed_price
    } else {
        request_price
    }
}

fn normalize_platform_lookup(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn merge_model_override(model: Model, overrides: &HashMap<String, ModelPricingOverride>) -> Model {
    let key = override_key(&platform_key(&model.platform), &model.id);
    let base_fixed = model.base_fixed_price;
    let base_request = model.base_request_price;
    let base_input = model.base_input_price_per_1m;
    let base_output = model.base_output_price_per_1m;
    let base_currency = model.base_pricing_currency.clone();
    let base_unit = model.base_pricing_unit.clone();

    match overrides.get(&key) {
        Some(override_row) => Model {
            fixed_price: override_row.fixed_price,
            request_price: override_row.request_price,
            input_price_per_1m: override_row.input_price_per_1m,
            output_price_per_1m: override_row.output_price_per_1m,
            pricing_currency: override_row
                .pricing_currency
                .clone()
                .or(base_currency.clone()),
            pricing_unit: override_row.pricing_unit.clone().or(base_unit.clone()),
            pricing_source: Some("manual".to_string()),
            pricing_note: override_row.note.clone(),
            pricing_updated_at: Some(override_row.updated_at.clone()),
            base_fixed_price: base_fixed,
            base_request_price: base_request,
            base_input_price_per_1m: base_input,
            base_output_price_per_1m: base_output,
            base_pricing_currency: base_currency,
            base_pricing_unit: base_unit,
            ..model
        },
        None => {
            let source = model.pricing_source.clone().unwrap_or_else(|| {
                if model.fixed_price.is_some()
                    || model.request_price.is_some()
                    || model.input_price_per_1m.is_some()
                    || model.output_price_per_1m.is_some()
                {
                    "builtin".to_string()
                } else {
                    "unset".to_string()
                }
            });
            Model {
                pricing_source: Some(source),
                fixed_price: model.fixed_price,
                request_price: model.request_price,
                pricing_currency: model.pricing_currency.clone(),
                pricing_unit: model.pricing_unit.clone(),
                pricing_note: model.pricing_note.clone(),
                pricing_updated_at: model.pricing_updated_at.clone(),
                base_fixed_price: base_fixed,
                base_request_price: base_request,
                base_input_price_per_1m: base_input,
                base_output_price_per_1m: base_output,
                base_pricing_currency: base_currency,
                base_pricing_unit: base_unit,
                ..model
            }
        }
    }
}

fn find_base_model(platform: &Platform, model_id: &str) -> Option<Model> {
    let target = model_id.trim().to_lowercase();
    base_catalog()
        .into_iter()
        .find(|item| &item.platform == platform && item.id.trim().to_lowercase() == target)
}

fn override_key(platform: &str, model_id: &str) -> String {
    format!(
        "{}::{}",
        platform.trim().to_lowercase(),
        model_id.trim().to_lowercase()
    )
}

fn platform_key(platform: &Platform) -> String {
    serde_json::to_value(platform)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "custom".to_string())
}

fn model_matches(model: &Model, normalized_model_name: &str) -> bool {
    let id = model.id.to_lowercase();
    let name = model.name.to_lowercase();

    normalized_model_name == id
        || normalized_model_name == name
        || normalized_model_name.starts_with(&id)
        || normalized_model_name.starts_with(&name)
        || normalized_model_name.contains(&id)
}

fn model(
    id: &str,
    name: &str,
    platform: Platform,
    context_length: Option<u32>,
    supports_vision: bool,
    supports_tools: bool,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
) -> Model {
    let pricing_currency = if input_price_per_1m.is_some() || output_price_per_1m.is_some() {
        Some("USD")
    } else {
        None
    };
    let pricing_unit = if input_price_per_1m.is_some() || output_price_per_1m.is_some() {
        Some("1m_tokens")
    } else {
        None
    };

    model_with_pricing_meta(
        id,
        name,
        platform,
        context_length,
        supports_vision,
        supports_tools,
        input_price_per_1m,
        output_price_per_1m,
        None,
        None,
        pricing_currency,
        pricing_unit,
    )
}

fn model_with_meta(
    id: &str,
    name: &str,
    platform: Platform,
    context_length: Option<u32>,
    supports_vision: bool,
    supports_tools: bool,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
    pricing_note: Option<&str>,
    pricing_updated_at: Option<&str>,
) -> Model {
    model_with_pricing_meta(
        id,
        name,
        platform,
        context_length,
        supports_vision,
        supports_tools,
        input_price_per_1m,
        output_price_per_1m,
        pricing_note,
        pricing_updated_at,
        Some("USD"),
        Some("1m_tokens"),
    )
}

fn model_with_pricing_meta(
    id: &str,
    name: &str,
    platform: Platform,
    context_length: Option<u32>,
    supports_vision: bool,
    supports_tools: bool,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
    pricing_note: Option<&str>,
    pricing_updated_at: Option<&str>,
    pricing_currency: Option<&str>,
    pricing_unit: Option<&str>,
) -> Model {
    model_with_source_meta(
        id,
        name,
        platform,
        context_length,
        supports_vision,
        supports_tools,
        input_price_per_1m,
        output_price_per_1m,
        None,
        pricing_note,
        pricing_updated_at,
        pricing_currency,
        pricing_unit,
    )
}

fn model_with_request_surcharge_meta(
    id: &str,
    name: &str,
    platform: Platform,
    context_length: Option<u32>,
    supports_vision: bool,
    supports_tools: bool,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
    request_price: Option<f64>,
    pricing_note: Option<&str>,
    pricing_updated_at: Option<&str>,
    pricing_currency: Option<&str>,
    pricing_unit: Option<&str>,
) -> Model {
    let mut model = model_with_pricing_meta(
        id,
        name,
        platform,
        context_length,
        supports_vision,
        supports_tools,
        input_price_per_1m,
        output_price_per_1m,
        pricing_note,
        pricing_updated_at,
        pricing_currency,
        pricing_unit,
    );
    model.request_price = request_price;
    model.base_request_price = request_price;
    model
}

fn model_with_fixed_price_meta(
    id: &str,
    name: &str,
    platform: Platform,
    context_length: Option<u32>,
    supports_vision: bool,
    supports_tools: bool,
    fixed_price: Option<f64>,
    pricing_note: Option<&str>,
    pricing_updated_at: Option<&str>,
    pricing_currency: Option<&str>,
    pricing_unit: Option<&str>,
) -> Model {
    let mut model = model_with_source_meta(
        id,
        name,
        platform,
        context_length,
        supports_vision,
        supports_tools,
        None,
        None,
        None,
        pricing_note,
        pricing_updated_at,
        pricing_currency,
        pricing_unit,
    );
    model.fixed_price = fixed_price;
    model.base_fixed_price = fixed_price;
    model
}

fn model_with_source_meta(
    id: &str,
    name: &str,
    platform: Platform,
    context_length: Option<u32>,
    supports_vision: bool,
    supports_tools: bool,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
    pricing_source: Option<&str>,
    pricing_note: Option<&str>,
    pricing_updated_at: Option<&str>,
    pricing_currency: Option<&str>,
    pricing_unit: Option<&str>,
) -> Model {
    Model {
        id: id.to_string(),
        name: name.to_string(),
        platform,
        context_length,
        supports_vision,
        supports_tools,
        fixed_price: None,
        request_price: None,
        input_price_per_1m,
        output_price_per_1m,
        is_available: true,
        base_fixed_price: None,
        base_request_price: None,
        base_input_price_per_1m: input_price_per_1m,
        base_output_price_per_1m: output_price_per_1m,
        pricing_currency: pricing_currency.map(|value| value.to_string()),
        pricing_unit: pricing_unit.map(|value| value.to_string()),
        base_pricing_currency: pricing_currency.map(|value| value.to_string()),
        base_pricing_unit: pricing_unit.map(|value| value.to_string()),
        pricing_source: pricing_source.map(|value| value.to_string()),
        pricing_note: pricing_note.map(|value| value.to_string()),
        pricing_updated_at: pricing_updated_at.map(|value| value.to_string()),
    }
}

pub fn base_catalog() -> Vec<Model> {
    const PRICING_SNAPSHOT_AT: &str = "2026-04-20T00:00:00Z";

    vec![
        model_with_meta(
            "gpt-5.4",
            "GPT-5.4",
            Platform::OpenAI,
            Some(400_000),
            true,
            true,
            Some(2.5),
            Some(15.0),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gpt-5.4-mini",
            "GPT-5.4 mini",
            Platform::OpenAI,
            Some(400_000),
            true,
            true,
            Some(0.75),
            Some(4.5),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gpt-5.4-nano",
            "GPT-5.4 nano",
            Platform::OpenAI,
            Some(400_000),
            true,
            true,
            Some(0.20),
            Some(1.25),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gpt-5.3-codex",
            "GPT-5.3 Codex",
            Platform::OpenAI,
            Some(400_000),
            true,
            true,
            Some(1.75),
            Some(14.0),
            Some("OpenAI Codex 模型官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gpt-4.1",
            "GPT-4.1",
            Platform::OpenAI,
            Some(1_000_000),
            true,
            true,
            Some(2.0),
            Some(8.0),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gpt-4.1-mini",
            "GPT-4.1 mini",
            Platform::OpenAI,
            Some(1_000_000),
            true,
            true,
            Some(0.4),
            Some(1.6),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gpt-4.1-nano",
            "GPT-4.1 nano",
            Platform::OpenAI,
            Some(1_000_000),
            true,
            true,
            Some(0.1),
            Some(0.4),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model(
            "gpt-4o",
            "GPT-4o",
            Platform::OpenAI,
            Some(128_000),
            true,
            true,
            Some(2.5),
            Some(10.0),
        ),
        model_with_meta(
            "o4-mini",
            "o4-mini",
            Platform::OpenAI,
            Some(200_000),
            false,
            true,
            Some(1.1),
            Some(4.4),
            Some("OpenAI 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "claude-opus-4-5",
            "Claude Opus 4.5",
            Platform::Anthropic,
            Some(200_000),
            true,
            true,
            Some(5.0),
            Some(25.0),
            Some("Anthropic 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "claude-sonnet-4-5",
            "Claude Sonnet 4.5",
            Platform::Anthropic,
            Some(200_000),
            true,
            true,
            Some(3.0),
            Some(15.0),
            Some("Anthropic 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "claude-haiku-4-5",
            "Claude Haiku 4.5",
            Platform::Anthropic,
            Some(200_000),
            true,
            true,
            Some(1.0),
            Some(5.0),
            Some("Anthropic 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            Platform::Gemini,
            Some(1_000_000),
            true,
            true,
            Some(1.25),
            Some(10.0),
            Some("Gemini Developer API 标准价（提示不超过 200K token）"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash",
            Platform::Gemini,
            Some(1_000_000),
            true,
            true,
            Some(0.3),
            Some(2.5),
            Some("Gemini Developer API 标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "gemini-2.5-flash-lite",
            "Gemini 2.5 Flash-Lite",
            Platform::Gemini,
            Some(1_000_000),
            true,
            true,
            Some(0.1),
            Some(0.4),
            Some("Gemini Developer API 标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "deepseek-chat",
            "DeepSeek Chat",
            Platform::DeepSeek,
            Some(128_000),
            false,
            true,
            Some(0.28),
            Some(0.42),
            Some("DeepSeek 官方 USD 定价（cache miss / V3.2）"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "deepseek-reasoner",
            "DeepSeek Reasoner",
            Platform::DeepSeek,
            Some(128_000),
            false,
            true,
            Some(0.55),
            Some(2.19),
            Some("DeepSeek 官方 USD 定价（cache miss）"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_pricing_meta(
            "qwen-max",
            "Qwen Max",
            Platform::Aliyun,
            Some(128_000),
            false,
            true,
            Some(2.4),
            Some(9.6),
            Some("阿里云百炼官方标准价（中国站）"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_pricing_meta(
            "doubao-1.5-pro",
            "Doubao 1.5 Pro",
            Platform::Bytedance,
            Some(128_000),
            false,
            true,
            Some(0.8),
            Some(2.0),
            Some("按 Doubao-1.5-pro-32k 官方标准价推断"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_pricing_meta(
            "kimi-k2",
            "Kimi K2",
            Platform::Moonshot,
            Some(200_000),
            false,
            true,
            Some(4.0),
            Some(16.0),
            Some("Moonshot 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_source_meta(
            "glm-4.5",
            "GLM 4.5",
            Platform::Zhipu,
            Some(128_000),
            false,
            true,
            None,
            None,
            Some("special"),
            Some("官方页仅披露 GLM-4.5 系列最低价格档，当前目录不为 GLM-4.5 主型号写死固定单价"),
            Some(PRICING_SNAPSHOT_AT),
            None,
            None,
        ),
        model_with_pricing_meta(
            "glm-4.5-air",
            "GLM 4.5 Air",
            Platform::Zhipu,
            Some(128_000),
            false,
            true,
            Some(0.8),
            Some(2.0),
            Some("按 GLM-4.5-Air 当前公开价格档推断"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_meta(
            "MiniMax-M2.7",
            "MiniMax M2.7",
            Platform::MiniMax,
            Some(1_000_000),
            true,
            true,
            Some(0.3),
            Some(1.2),
            Some("MiniMax 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_pricing_meta(
            "step-2-mini",
            "Step 2 Mini",
            Platform::StepFun,
            Some(256_000),
            true,
            true,
            Some(1.0),
            Some(2.0),
            Some("StepFun 官方标准价（推荐文本模型）"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_pricing_meta(
            "step-2-16k",
            "Step 2 16K",
            Platform::StepFun,
            Some(16_000),
            true,
            true,
            Some(38.0),
            Some(120.0),
            Some("StepFun 官方标准价"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_pricing_meta(
            "step-2-16k-exp",
            "Step 2 16K Exp",
            Platform::StepFun,
            Some(16_000),
            true,
            true,
            Some(38.0),
            Some(120.0),
            Some("StepFun 官方标准价（实验版）"),
            Some(PRICING_SNAPSHOT_AT),
            Some("CNY"),
            Some("1m_tokens"),
        ),
        model_with_meta(
            "llama-3.3-70b",
            "Llama 3.3 70B",
            Platform::OpenRouter,
            Some(128_000),
            false,
            true,
            Some(0.12),
            Some(0.38),
            Some("OpenRouter 当前公开标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "qwen/qwen3-coder",
            "Qwen3 Coder",
            Platform::OpenRouter,
            Some(256_000),
            false,
            true,
            Some(0.22),
            Some(1.0),
            Some("OpenRouter 当前公开标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "mistral-large",
            "Mistral Large",
            Platform::Mistral,
            Some(128_000),
            false,
            true,
            Some(0.5),
            Some(1.5),
            Some("按 Mistral Large 3 当前官方标准价推断"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "grok-4",
            "Grok 4",
            Platform::XAi,
            Some(128_000),
            true,
            true,
            Some(2.0),
            Some(6.0),
            Some("按 xAI 当前 grok-4 最新稳定价格档推断；不含工具调用费"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_meta(
            "command-r-plus",
            "Command R+",
            Platform::Cohere,
            Some(128_000),
            false,
            true,
            Some(2.5),
            Some(10.0),
            Some("Cohere 官方标准价（Command R+ 08-2024）"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_request_surcharge_meta(
            "sonar-reasoning-pro",
            "Sonar Reasoning Pro",
            Platform::Perplexity,
            Some(128_000),
            false,
            true,
            Some(2.0),
            Some(8.0),
            Some(0.006),
            Some("Perplexity 官方标准价；请求费按 Low Context 基线 $0.006 / 次计入，高上下文更高"),
            Some(PRICING_SNAPSHOT_AT),
            Some("USD"),
            Some("1m_tokens"),
        ),
        model_with_meta(
            "meta-llama/Llama-3.1-405B-Instruct",
            "Llama 3.1 405B",
            Platform::TogetherAi,
            Some(128_000),
            false,
            true,
            Some(3.5),
            Some(3.5),
            Some("Together AI 官方模型页标准价"),
            Some(PRICING_SNAPSHOT_AT),
        ),
        model_with_source_meta(
            "llama3.2",
            "Llama 3.2",
            Platform::Ollama,
            Some(128_000),
            false,
            true,
            None,
            None,
            Some("special"),
            Some("本地模型，运行成本取决于本机资源，不维护统一 API 美元单价"),
            Some(PRICING_SNAPSHOT_AT),
            None,
            None,
        ),
        model_with_source_meta(
            "deepseek/deepseek-r1",
            "DeepSeek R1",
            Platform::HuggingFace,
            Some(128_000),
            false,
            true,
            None,
            None,
            Some("special"),
            Some("HuggingFace 价格取决于推理提供方与部署方式，暂无统一美元基线"),
            Some(PRICING_SNAPSHOT_AT),
            None,
            None,
        ),
        model_with_fixed_price_meta(
            "black-forest-labs/flux-1.1-pro",
            "FLUX 1.1 Pro",
            Platform::Replicate,
            Some(32_000),
            true,
            false,
            Some(0.04),
            Some("Replicate 官方模型页标准价，按输出图像计费"),
            Some(PRICING_SNAPSHOT_AT),
            Some("USD"),
            Some("image"),
        ),
        model_with_source_meta(
            "copilot-chat",
            "Copilot Chat",
            Platform::Copilot,
            Some(128_000),
            false,
            true,
            None,
            None,
            Some("special"),
            Some("未公开统一 API token 单价，暂不提供基础价格"),
            Some(PRICING_SNAPSHOT_AT),
            None,
            None,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        base_catalog, merge_model_override, ModelCatalogService, ModelPricingOverride,
        SaveModelPricingRequest,
    };
    use crate::db::Database;
    use crate::models::{Model, Platform};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn find_model<'a>(models: &'a [Model], model_id: &str) -> &'a Model {
        models
            .iter()
            .find(|item| item.id == model_id)
            .unwrap_or_else(|| panic!("missing model: {model_id}"))
    }

    fn temp_db_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ai_singularity_{prefix}_{nanos}.db"))
    }

    #[test]
    fn base_catalog_includes_mainstream_openai_and_gemini_prices() {
        let catalog = base_catalog();

        let codex = find_model(&catalog, "gpt-5.3-codex");
        assert_eq!(codex.platform, Platform::OpenAI);
        assert_eq!(codex.input_price_per_1m, Some(1.75));
        assert_eq!(codex.output_price_per_1m, Some(14.0));

        let gemini_flash_lite = find_model(&catalog, "gemini-2.5-flash-lite");
        assert_eq!(gemini_flash_lite.platform, Platform::Gemini);
        assert_eq!(gemini_flash_lite.input_price_per_1m, Some(0.1));
        assert_eq!(gemini_flash_lite.output_price_per_1m, Some(0.4));
    }

    #[test]
    fn base_catalog_includes_verified_multiplatform_builtin_prices() {
        let catalog = base_catalog();

        let claude_haiku = find_model(&catalog, "claude-haiku-4-5");
        assert_eq!(claude_haiku.platform, Platform::Anthropic);
        assert_eq!(claude_haiku.input_price_per_1m, Some(1.0));
        assert_eq!(claude_haiku.output_price_per_1m, Some(5.0));
        assert_eq!(
            claude_haiku.pricing_note.as_deref(),
            Some("Anthropic 官方标准价")
        );

        let qwen3_coder = find_model(&catalog, "qwen/qwen3-coder");
        assert_eq!(qwen3_coder.platform, Platform::OpenRouter);
        assert_eq!(qwen3_coder.input_price_per_1m, Some(0.22));
        assert_eq!(qwen3_coder.output_price_per_1m, Some(1.0));
        assert_eq!(
            qwen3_coder.pricing_note.as_deref(),
            Some("OpenRouter 当前公开标准价")
        );

        let command_r_plus = find_model(&catalog, "command-r-plus");
        assert_eq!(command_r_plus.platform, Platform::Cohere);
        assert_eq!(command_r_plus.input_price_per_1m, Some(2.5));
        assert_eq!(command_r_plus.output_price_per_1m, Some(10.0));
        assert_eq!(
            command_r_plus.pricing_note.as_deref(),
            Some("Cohere 官方标准价（Command R+ 08-2024）")
        );

        let sonar_reasoning_pro = find_model(&catalog, "sonar-reasoning-pro");
        assert_eq!(sonar_reasoning_pro.platform, Platform::Perplexity);
        assert_eq!(sonar_reasoning_pro.input_price_per_1m, Some(2.0));
        assert_eq!(sonar_reasoning_pro.output_price_per_1m, Some(8.0));
        assert_eq!(sonar_reasoning_pro.request_price, Some(0.006));
        assert_eq!(
            sonar_reasoning_pro.pricing_note.as_deref(),
            Some("Perplexity 官方标准价；请求费按 Low Context 基线 $0.006 / 次计入，高上下文更高")
        );

        let minimax_m27 = find_model(&catalog, "MiniMax-M2.7");
        assert_eq!(minimax_m27.platform, Platform::MiniMax);
        assert_eq!(minimax_m27.input_price_per_1m, Some(0.3));
        assert_eq!(minimax_m27.output_price_per_1m, Some(1.2));
        assert_eq!(
            minimax_m27.pricing_note.as_deref(),
            Some("MiniMax 官方标准价")
        );

        let grok_4 = find_model(&catalog, "grok-4");
        assert_eq!(grok_4.platform, Platform::XAi);
        assert_eq!(grok_4.input_price_per_1m, Some(2.0));
        assert_eq!(grok_4.output_price_per_1m, Some(6.0));
        assert_eq!(
            grok_4.pricing_note.as_deref(),
            Some("按 xAI 当前 grok-4 最新稳定价格档推断；不含工具调用费")
        );

        let together_llama_405b = find_model(&catalog, "meta-llama/Llama-3.1-405B-Instruct");
        assert_eq!(together_llama_405b.platform, Platform::TogetherAi);
        assert_eq!(together_llama_405b.input_price_per_1m, Some(3.5));
        assert_eq!(together_llama_405b.output_price_per_1m, Some(3.5));
        assert_eq!(
            together_llama_405b.pricing_note.as_deref(),
            Some("Together AI 官方模型页标准价")
        );

        let qwen_max = find_model(&catalog, "qwen-max");
        assert_eq!(qwen_max.platform, Platform::Aliyun);
        assert_eq!(qwen_max.input_price_per_1m, Some(2.4));
        assert_eq!(qwen_max.output_price_per_1m, Some(9.6));
        assert_eq!(qwen_max.pricing_currency.as_deref(), Some("CNY"));
        assert_eq!(qwen_max.pricing_unit.as_deref(), Some("1m_tokens"));

        let step_2_mini = find_model(&catalog, "step-2-mini");
        assert_eq!(step_2_mini.platform, Platform::StepFun);
        assert_eq!(step_2_mini.input_price_per_1m, Some(1.0));
        assert_eq!(step_2_mini.output_price_per_1m, Some(2.0));
        assert_eq!(step_2_mini.pricing_currency.as_deref(), Some("CNY"));

        let flux_pro = find_model(&catalog, "black-forest-labs/flux-1.1-pro");
        assert_eq!(flux_pro.platform, Platform::Replicate);
        assert_eq!(flux_pro.fixed_price, Some(0.04));
        assert_eq!(flux_pro.pricing_currency.as_deref(), Some("USD"));
        assert_eq!(flux_pro.pricing_unit.as_deref(), Some("image"));
        assert_eq!(
            flux_pro.pricing_note.as_deref(),
            Some("Replicate 官方模型页标准价，按输出图像计费")
        );
    }

    #[test]
    fn merge_model_override_keeps_builtin_pricing_metadata_without_manual_override() {
        let model = find_model(&base_catalog(), "gemini-2.5-pro").clone();
        let merged = merge_model_override(model, &HashMap::new());

        assert_eq!(merged.pricing_source.as_deref(), Some("builtin"));
        assert_eq!(
            merged.pricing_note.as_deref(),
            Some("Gemini Developer API 标准价（提示不超过 200K token）")
        );
        assert_eq!(
            merged.pricing_updated_at.as_deref(),
            Some("2026-04-20T00:00:00Z")
        );
    }

    #[test]
    fn merge_model_override_uses_manual_pricing_metadata_when_present() {
        let model = find_model(&base_catalog(), "qwen-max").clone();
        let mut overrides = HashMap::new();
        overrides.insert(
            "aliyun::qwen-max".to_string(),
            ModelPricingOverride {
                platform: "aliyun".to_string(),
                model_id: "qwen-max".to_string(),
                fixed_price: None,
                request_price: None,
                input_price_per_1m: Some(0.4),
                output_price_per_1m: Some(1.6),
                pricing_currency: Some("USD".to_string()),
                pricing_unit: Some("1m_tokens".to_string()),
                note: Some("手工换算为美元成本".to_string()),
                updated_at: "2026-04-20T12:00:00Z".to_string(),
            },
        );

        let merged = merge_model_override(model, &overrides);
        assert_eq!(merged.pricing_source.as_deref(), Some("manual"));
        assert_eq!(merged.pricing_currency.as_deref(), Some("USD"));
        assert_eq!(merged.pricing_unit.as_deref(), Some("1m_tokens"));
        assert_eq!(merged.input_price_per_1m, Some(0.4));
        assert_eq!(merged.output_price_per_1m, Some(1.6));
    }

    #[test]
    fn merge_model_override_preserves_special_source_without_manual_override() {
        let model = find_model(&base_catalog(), "llama3.2").clone();
        let merged = merge_model_override(model, &HashMap::new());

        assert_eq!(merged.pricing_source.as_deref(), Some("special"));
        assert_eq!(
            merged.pricing_note.as_deref(),
            Some("本地模型，运行成本取决于本机资源，不维护统一 API 美元单价")
        );
        assert_eq!(merged.input_price_per_1m, None);
        assert_eq!(merged.output_price_per_1m, None);
    }

    #[test]
    fn merge_model_override_marks_fixed_price_catalog_models_as_builtin() {
        let model = find_model(&base_catalog(), "black-forest-labs/flux-1.1-pro").clone();
        let merged = merge_model_override(model, &HashMap::new());

        assert_eq!(merged.pricing_source.as_deref(), Some("builtin"));
        assert_eq!(merged.fixed_price, Some(0.04));
        assert_eq!(merged.pricing_unit.as_deref(), Some("image"));
    }

    #[test]
    fn resolve_rates_ignores_non_usd_catalog_prices() {
        let db_path = temp_db_path("model_catalog_non_usd");
        let db = Database::new(&db_path).unwrap();
        let service = ModelCatalogService::new(&db);

        let rates = service.resolve_rates(Some("aliyun"), "qwen-max").unwrap();
        assert_eq!(rates, (None, None));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_rates_uses_manual_usd_override_for_non_usd_base_model() {
        let db_path = temp_db_path("model_catalog_manual_usd");
        let db = Database::new(&db_path).unwrap();
        let service = ModelCatalogService::new(&db);

        service
            .save_pricing(SaveModelPricingRequest {
                platform: Platform::Aliyun,
                model_id: "qwen-max".to_string(),
                fixed_price: None,
                request_price: None,
                input_price_per_1m: Some(0.36),
                output_price_per_1m: Some(1.44),
                pricing_currency: Some("USD".to_string()),
                pricing_unit: Some("1m_tokens".to_string()),
                note: Some("手工按汇率换算".to_string()),
            })
            .unwrap();

        let rates = service.resolve_rates(Some("aliyun"), "qwen-max").unwrap();
        assert_eq!(rates, (Some(0.36), Some(1.44)));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_rates_ignores_manual_non_usd_override_for_cost_engine() {
        let db_path = temp_db_path("model_catalog_manual_cny");
        let db = Database::new(&db_path).unwrap();
        let service = ModelCatalogService::new(&db);

        service
            .save_pricing(SaveModelPricingRequest {
                platform: Platform::Aliyun,
                model_id: "qwen-max".to_string(),
                fixed_price: None,
                request_price: None,
                input_price_per_1m: Some(2.4),
                output_price_per_1m: Some(9.6),
                pricing_currency: Some("CNY".to_string()),
                pricing_unit: Some("1m_tokens".to_string()),
                note: Some("保留人民币基线".to_string()),
            })
            .unwrap();

        let rates = service.resolve_rates(Some("aliyun"), "qwen-max").unwrap();
        assert_eq!(rates, (None, None));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_rates_ignores_stale_non_usd_override_without_base_model_match() {
        let db_path = temp_db_path("model_catalog_stale_non_usd");
        let db = Database::new(&db_path).unwrap();
        db.execute(
            "INSERT INTO model_pricing_overrides (
                platform,
                model_id,
                fixed_price,
                input_price_per_1m,
                output_price_per_1m,
                pricing_currency,
                pricing_unit,
                note,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
            &[
                &"aliyun",
                &"legacy-qwen",
                &Option::<f64>::None,
                &Some(2.4_f64),
                &Some(9.6_f64),
                &Some("CNY".to_string()),
                &Some("1m_tokens".to_string()),
                &Some("遗留人民币覆盖".to_string()),
            ],
        )
        .unwrap();

        let service = ModelCatalogService::new(&db);
        let rates = service.resolve_rates(Some("aliyun"), "legacy-qwen").unwrap();
        assert_eq!(rates, (None, None));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_rates_preserves_legacy_usd_override_without_currency_metadata() {
        let db_path = temp_db_path("model_catalog_stale_legacy_usd");
        let db = Database::new(&db_path).unwrap();
        db.execute(
            "INSERT INTO model_pricing_overrides (
                platform,
                model_id,
                fixed_price,
                input_price_per_1m,
                output_price_per_1m,
                pricing_currency,
                pricing_unit,
                note,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
            &[
                &"openai",
                &"legacy-gpt",
                &Option::<f64>::None,
                &Some(1.0_f64),
                &Some(4.0_f64),
                &Option::<String>::None,
                &Option::<String>::None,
                &Some("旧版未带币种单位字段".to_string()),
            ],
        )
        .unwrap();

        let service = ModelCatalogService::new(&db);
        let rates = service.resolve_rates(Some("openai"), "legacy-gpt").unwrap();
        assert_eq!(rates, (Some(1.0), Some(4.0)));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_pricing_returns_request_surcharge_for_hybrid_usd_model() {
        let db_path = temp_db_path("model_catalog_perplexity_request_fee");
        let db = Database::new(&db_path).unwrap();
        let service = ModelCatalogService::new(&db);

        let pricing = service
            .resolve_pricing(Some("perplexity"), "sonar-reasoning-pro")
            .unwrap();
        assert_eq!(pricing.input_price_per_1m, Some(2.0));
        assert_eq!(pricing.output_price_per_1m, Some(8.0));
        assert_eq!(pricing.request_price, Some(0.006));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_pricing_ignores_fixed_image_unit_for_cost_engine() {
        let db_path = temp_db_path("model_catalog_image_fixed");
        let db = Database::new(&db_path).unwrap();
        let service = ModelCatalogService::new(&db);

        let pricing = service
            .resolve_pricing(Some("replicate"), "black-forest-labs/flux-1.1-pro")
            .unwrap();
        assert_eq!(pricing.input_price_per_1m, None);
        assert_eq!(pricing.output_price_per_1m, None);
        assert_eq!(pricing.request_price, None);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn resolve_pricing_supports_manual_request_only_usd_model() {
        let db_path = temp_db_path("model_catalog_manual_request_only");
        let db = Database::new(&db_path).unwrap();
        let service = ModelCatalogService::new(&db);

        service
            .save_pricing(SaveModelPricingRequest {
                platform: Platform::OpenAI,
                model_id: "gpt-4o".to_string(),
                fixed_price: Some(0.02),
                request_price: Some(0.5),
                input_price_per_1m: Some(2.5),
                output_price_per_1m: Some(10.0),
                pricing_currency: Some("USD".to_string()),
                pricing_unit: Some("request".to_string()),
                note: Some("按次模型覆盖".to_string()),
            })
            .unwrap();

        let pricing = service.resolve_pricing(Some("openai"), "gpt-4o").unwrap();
        assert_eq!(pricing.input_price_per_1m, None);
        assert_eq!(pricing.output_price_per_1m, None);
        assert_eq!(pricing.request_price, Some(0.02));

        let _ = std::fs::remove_file(db_path);
    }
}
