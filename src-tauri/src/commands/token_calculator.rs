use crate::error::AppResult;
use anyhow::anyhow;
use reqwest::Client;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
pub struct FetchRemoteModelPricingRequest {
    base_url: String,
    api_key: Option<String>,
}

#[derive(Clone, Default, serde::Serialize)]
pub struct RemoteModelPricing {
    id: String,
    name: Option<String>,
    description: Option<String>,
    input_price_per_1m: Option<f64>,
    output_price_per_1m: Option<f64>,
    cache_read_price_per_1m: Option<f64>,
    fixed_price_usd: Option<f64>,
    quota_type: Option<i32>,
    model_ratio: Option<f64>,
    completion_ratio: Option<f64>,
    cache_ratio: Option<f64>,
    model_price: Option<f64>,
    enable_groups: Vec<String>,
    vendor_id: Option<i64>,
    recommended_group: Option<String>,
}

#[derive(serde::Serialize)]
pub struct FetchRemoteModelPricingResponse {
    models: Vec<RemoteModelPricing>,
    source_endpoint: String,
    warnings: Vec<String>,
    provider_kind: Option<String>,
    quota_per_unit: Option<f64>,
    group_ratios: BTreeMap<String, f64>,
    group_labels: BTreeMap<String, String>,
    auto_groups: Vec<String>,
}

#[tauri::command]
pub async fn fetch_remote_model_pricing(
    request: FetchRemoteModelPricingRequest,
) -> AppResult<FetchRemoteModelPricingResponse> {
    let base_url = normalize_base_url(&request.base_url)?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    let mut fallback_result: Option<FetchRemoteModelPricingResponse> = None;
    let mut attempt_errors: Vec<String> = Vec::new();

    for url in build_candidate_urls(&base_url) {
        let response = apply_optional_auth(client.get(&url), request.api_key.as_deref())
            .send()
            .await;

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                attempt_errors.push(format!("{url} -> {err}"));
                continue;
            }
        };

        let response = match response.error_for_status() {
            Ok(response) => response,
            Err(err) => {
                attempt_errors.push(format!("{url} -> {err}"));
                continue;
            }
        };

        let payload: Value = match response.json().await {
            Ok(payload) => payload,
            Err(err) => {
                attempt_errors.push(format!("{url} -> {err}"));
                continue;
            }
        };

        if let Some(result) = extract_newapi_pricing_response(&payload, &url) {
            return Ok(result);
        }

        let models = extract_models_from_payload(&payload);
        if models.is_empty() {
            attempt_errors.push(format!("{url} -> 未识别到模型数据"));
            continue;
        }

        let priced_count = models
            .iter()
            .filter(|model| {
                model.input_price_per_1m.is_some()
                    || model.output_price_per_1m.is_some()
                    || model.cache_read_price_per_1m.is_some()
            })
            .count();

        if priced_count > 0 {
            return Ok(FetchRemoteModelPricingResponse {
                models,
                source_endpoint: url,
                warnings: Vec::new(),
                provider_kind: None,
                quota_per_unit: None,
                group_ratios: BTreeMap::new(),
                group_labels: BTreeMap::new(),
                auto_groups: Vec::new(),
            });
        }

        if fallback_result.is_none() {
            fallback_result = Some(FetchRemoteModelPricingResponse {
                models,
                source_endpoint: url,
                warnings: vec!["已拉取到模型列表，但暂未识别到价格字段。".to_string()],
                provider_kind: None,
                quota_per_unit: None,
                group_ratios: BTreeMap::new(),
                group_labels: BTreeMap::new(),
                auto_groups: Vec::new(),
            });
        }
    }

    if let Some(mut response) = fallback_result {
        if !attempt_errors.is_empty() {
            response.warnings.push(format!(
                "其余候选接口尝试失败：{}",
                attempt_errors.join(" | ")
            ));
        }
        return Ok(response);
    }

    Err(anyhow!(
        "未能从该地址识别模型或价格信息。已尝试：{}",
        attempt_errors.join(" | ")
    )
    .into())
}

fn apply_optional_auth(
    mut builder: reqwest::RequestBuilder,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    builder = builder.header("accept", "application/json");
    if let Some(api_key) = api_key.filter(|value| !value.trim().is_empty()) {
        builder = builder.bearer_auth(api_key);
        builder = builder.header("x-api-key", api_key);
    }
    builder
}

fn normalize_base_url(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("请先填写中转 API 地址").into());
    }

    let normalized = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    reqwest::Url::parse(&normalized).map_err(|_| anyhow!("中转 API 地址格式不正确"))?;

    Ok(normalized.trim_end_matches('/').to_string())
}

fn build_candidate_urls(base_url: &str) -> Vec<String> {
    const ENDPOINT_SUFFIXES: [&str; 8] = [
        "/api/pricing",
        "/api/v1/pricing",
        "/v1/pricing",
        "/pricing",
        "/api/v1/models",
        "/v1/models",
        "/api/models",
        "/models",
    ];

    let mut candidates: Vec<String> = Vec::new();
    let exact_input = base_url.to_string();
    let base_prefix = strip_known_endpoint(base_url);

    if exact_input != base_prefix {
        candidates.push(exact_input);
    }

    for suffix in ENDPOINT_SUFFIXES {
        candidates.push(format!("{base_prefix}{suffix}"));
    }

    let mut unique: Vec<String> = Vec::new();
    for candidate in candidates {
        if !unique.iter().any(|item| item == &candidate) {
            unique.push(candidate);
        }
    }
    unique
}

fn strip_known_endpoint(url: &str) -> String {
    const KNOWN_SUFFIXES: [&str; 8] = [
        "/api/pricing",
        "/api/v1/pricing",
        "/v1/pricing",
        "/pricing",
        "/api/v1/models",
        "/v1/models",
        "/api/models",
        "/models",
    ];

    for suffix in KNOWN_SUFFIXES {
        if let Some(stripped) = url.strip_suffix(suffix) {
            return stripped.trim_end_matches('/').to_string();
        }
    }

    url.to_string()
}

fn extract_newapi_pricing_response(
    payload: &Value,
    source_endpoint: &str,
) -> Option<FetchRemoteModelPricingResponse> {
    let data = payload.get("data")?.as_array()?;
    if data.is_empty() {
        return None;
    }

    let group_ratios = extract_number_map(payload.get("group_ratio"));
    let group_labels = extract_string_map(payload.get("usable_group"));
    let auto_groups = extract_string_array(payload.get("auto_groups"));
    let quota_per_unit = 500_000.0;
    let vendors = extract_vendor_group_map(payload.get("vendors"));

    let mut models = Vec::new();
    for item in data {
        let Some(id) = get_string_at_path(item, "model_name")
            .or_else(|| get_string_at_path(item, "name"))
            .or_else(|| get_string_at_path(item, "id"))
        else {
            continue;
        };

        let enable_groups = extract_string_array(
            item.get("enable_groups")
                .or_else(|| item.get("enable_group")),
        );
        let vendor_id = item.get("vendor_id").and_then(Value::as_i64);
        let quota_type = item
            .get("quota_type")
            .and_then(Value::as_i64)
            .map(|v| v as i32);
        let model_ratio = item.get("model_ratio").and_then(Value::as_f64);
        let completion_ratio = item.get("completion_ratio").and_then(Value::as_f64);
        let cache_ratio = item.get("cache_ratio").and_then(Value::as_f64);
        let model_price = item.get("model_price").and_then(Value::as_f64);

        let vendor_group = vendor_id.and_then(|id| vendors.get(&id).cloned());
        let recommended_group = pick_recommended_group(
            &enable_groups,
            &auto_groups,
            &group_ratios,
            vendor_group.as_deref(),
        );
        let recommended_group_ratio = recommended_group
            .as_ref()
            .and_then(|group| group_ratios.get(group))
            .copied()
            .or_else(|| {
                if enable_groups.is_empty() {
                    None
                } else {
                    enable_groups
                        .iter()
                        .find_map(|group| group_ratios.get(group).copied())
                }
            })
            .unwrap_or(1.0);

        let usd_per_1m_factor = 1_000_000.0 / quota_per_unit;
        let (input_price_per_1m, output_price_per_1m, cache_read_price_per_1m, fixed_price_usd) =
            match quota_type.unwrap_or(0) {
                1 => (
                    None,
                    None,
                    None,
                    model_price.map(|price| price * recommended_group_ratio),
                ),
                _ => {
                    let input = model_ratio
                        .map(|ratio| ratio * recommended_group_ratio * usd_per_1m_factor);
                    let output = match (input, completion_ratio) {
                        (Some(input), Some(completion_ratio)) => Some(input * completion_ratio),
                        _ => None,
                    };
                    let cache = match (input, cache_ratio) {
                        (Some(input), Some(cache_ratio)) => Some(input * cache_ratio),
                        _ => None,
                    };
                    (input, output, cache, None)
                }
            };

        models.push(RemoteModelPricing {
            id: normalize_model_name(&id),
            name: get_string_at_path(item, "model_name")
                .or_else(|| get_string_at_path(item, "name")),
            description: get_string_at_path(item, "description"),
            input_price_per_1m,
            output_price_per_1m,
            cache_read_price_per_1m,
            fixed_price_usd,
            quota_type,
            model_ratio,
            completion_ratio,
            cache_ratio,
            model_price,
            enable_groups,
            vendor_id,
            recommended_group,
        });
    }

    Some(FetchRemoteModelPricingResponse {
        models,
        source_endpoint: source_endpoint.to_string(),
        warnings: Vec::new(),
        provider_kind: Some("newapi".to_string()),
        quota_per_unit: Some(quota_per_unit),
        group_ratios,
        group_labels,
        auto_groups,
    })
}

fn extract_models_from_payload(payload: &Value) -> Vec<RemoteModelPricing> {
    let mut merged: BTreeMap<String, RemoteModelPricing> = BTreeMap::new();

    for (fallback_key, item) in collect_model_items(payload) {
        let Some(id) = extract_model_id(item, fallback_key.as_deref()) else {
            continue;
        };

        let entry = merged
            .entry(id.clone())
            .or_insert_with(|| RemoteModelPricing {
                id,
                name: extract_model_name(item),
                description: get_string_at_path(item, "description"),
                input_price_per_1m: None,
                output_price_per_1m: None,
                cache_read_price_per_1m: None,
                fixed_price_usd: None,
                quota_type: None,
                model_ratio: None,
                completion_ratio: None,
                cache_ratio: None,
                model_price: None,
                enable_groups: Vec::new(),
                vendor_id: None,
                recommended_group: None,
            });

        if entry.name.is_none() {
            entry.name = extract_model_name(item);
        }
        if entry.description.is_none() {
            entry.description = get_string_at_path(item, "description");
        }
        if entry.input_price_per_1m.is_none() {
            entry.input_price_per_1m = extract_input_price(item);
        }
        if entry.output_price_per_1m.is_none() {
            entry.output_price_per_1m = extract_output_price(item);
        }
        if entry.cache_read_price_per_1m.is_none() {
            entry.cache_read_price_per_1m = extract_cache_read_price(item);
        }
    }

    merged.into_values().collect()
}

fn extract_number_map(value: Option<&Value>) -> BTreeMap<String, f64> {
    let mut output = BTreeMap::new();
    let Some(Value::Object(map)) = value else {
        return output;
    };

    for (key, value) in map {
        if let Some(number) = value.as_f64() {
            output.insert(key.clone(), number);
        }
    }

    output
}

fn extract_string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    let mut output = BTreeMap::new();
    let Some(Value::Object(map)) = value else {
        return output;
    };

    for (key, value) in map {
        if let Some(text) = value
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            output.insert(key.clone(), text.to_string());
        }
    }

    output
}

fn extract_string_array(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn extract_vendor_group_map(value: Option<&Value>) -> BTreeMap<i64, String> {
    let mut output = BTreeMap::new();
    let Some(Value::Array(items)) = value else {
        return output;
    };

    for item in items {
        let Some(id) = item.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let Some(name) = item
            .get("name")
            .and_then(Value::as_str)
            .map(normalize_vendor_group)
        else {
            continue;
        };
        output.insert(id, name);
    }

    output
}

fn normalize_vendor_group(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.contains("anthropic") || lower.contains("claude") {
        return "claude".to_string();
    }
    if lower.contains("openai") || lower.contains("chatgpt") {
        return "chatgpt".to_string();
    }
    if lower.contains("google") || lower.contains("gemini") {
        return "gemini".to_string();
    }

    lower.replace(' ', "_")
}

fn pick_recommended_group(
    enable_groups: &[String],
    auto_groups: &[String],
    group_ratios: &BTreeMap<String, f64>,
    vendor_group: Option<&str>,
) -> Option<String> {
    if let Some(vendor_group) = vendor_group {
        if enable_groups.iter().any(|group| group == vendor_group)
            && group_ratios.contains_key(vendor_group)
        {
            return Some(vendor_group.to_string());
        }
    }

    for group in auto_groups {
        if enable_groups.iter().any(|enabled| enabled == group) && group_ratios.contains_key(group)
        {
            return Some(group.clone());
        }
    }

    for group in enable_groups {
        if group != "admin"
            && group != "default"
            && group != "auto"
            && group_ratios.contains_key(group)
        {
            return Some(group.clone());
        }
    }

    enable_groups
        .iter()
        .find(|group| group_ratios.contains_key(group.as_str()))
        .cloned()
}

fn collect_model_items(payload: &Value) -> Vec<(Option<String>, &Value)> {
    match payload {
        Value::Array(items) => items.iter().map(|item| (None, item)).collect(),
        Value::Object(map) => {
            for key in ["data", "models", "items", "results", "result"] {
                if let Some(Value::Array(items)) = map.get(key) {
                    return items.iter().map(|item| (None, item)).collect();
                }

                if let Some(Value::Object(items)) = map.get(key) {
                    return items
                        .iter()
                        .map(|(item_key, item)| (Some(item_key.clone()), item))
                        .collect();
                }
            }

            map.iter()
                .map(|(item_key, item)| (Some(item_key.clone()), item))
                .collect()
        }
        _ => Vec::new(),
    }
}

fn extract_model_id(item: &Value, fallback_key: Option<&str>) -> Option<String> {
    for path in ["id", "model", "name", "slug", "model_name"] {
        if let Some(value) = get_string_at_path(item, path) {
            return Some(normalize_model_name(&value));
        }
    }

    if matches!(item, Value::Object(_)) {
        return fallback_key.map(normalize_model_name);
    }

    None
}

fn extract_model_name(item: &Value) -> Option<String> {
    for path in ["name", "display_name", "label", "id", "model"] {
        if let Some(value) = get_string_at_path(item, path) {
            return Some(value);
        }
    }
    None
}

fn normalize_model_name(raw: &str) -> String {
    raw.trim()
        .strip_prefix("models/")
        .unwrap_or(raw.trim())
        .to_string()
}

fn extract_input_price(item: &Value) -> Option<f64> {
    extract_price_from_paths(
        item,
        &[
            "input_price_per_1m",
            "prompt_price_per_1m",
            "input_cost_per_1m",
            "prompt_cost_per_1m",
            "pricing.input_price_per_1m",
            "pricing.prompt_price_per_1m",
            "pricing.input",
            "pricing.prompt",
            "prices.input",
            "prices.prompt",
            "cost.input",
            "cost.prompt",
            "prompt_price",
            "input_price",
            "prompt_cost",
            "input_cost",
        ],
    )
}

fn extract_output_price(item: &Value) -> Option<f64> {
    extract_price_from_paths(
        item,
        &[
            "output_price_per_1m",
            "completion_price_per_1m",
            "output_cost_per_1m",
            "completion_cost_per_1m",
            "pricing.output_price_per_1m",
            "pricing.completion_price_per_1m",
            "pricing.output",
            "pricing.completion",
            "prices.output",
            "prices.completion",
            "cost.output",
            "cost.completion",
            "output_price",
            "completion_price",
            "output_cost",
            "completion_cost",
        ],
    )
}

fn extract_cache_read_price(item: &Value) -> Option<f64> {
    extract_price_from_paths(
        item,
        &[
            "cache_read_price_per_1m",
            "cached_read_price_per_1m",
            "pricing.cache_read_price_per_1m",
            "pricing.cache_read",
            "pricing.cached_read",
            "pricing.input_cache_read",
            "pricing.cached_input",
            "prices.cache_read",
            "cache_read_price",
            "cached_read_price",
        ],
    )
}

fn extract_price_from_paths(item: &Value, paths: &[&str]) -> Option<f64> {
    for path in paths {
        if let Some(value) = get_value_at_path(item, path) {
            if let Some(price) = normalize_price_per_1m(value, path) {
                return Some(price);
            }
        }
    }
    None
}

fn get_value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn get_string_at_path(value: &Value, path: &str) -> Option<String> {
    get_value_at_path(value, path)
        .and_then(|raw| raw.as_str().map(|text| text.trim().to_string()))
        .filter(|text| !text.is_empty())
}

fn normalize_price_per_1m(value: &Value, field_hint: &str) -> Option<f64> {
    match value {
        Value::Null => None,
        Value::Number(number) => {
            let raw = number.as_f64()?;
            apply_unit_multiplier(raw, None, field_hint)
        }
        Value::String(text) => {
            let raw = extract_first_number(text)?;
            apply_unit_multiplier(raw, Some(text), field_hint)
        }
        _ => None,
    }
}

fn apply_unit_multiplier(raw: f64, raw_text: Option<&str>, field_hint: &str) -> Option<f64> {
    if !raw.is_finite() {
        return None;
    }

    let hint = field_hint.to_ascii_lowercase();
    let raw_text_lower = raw_text.map(|text| text.to_ascii_lowercase());

    if hint.contains("per_1m")
        || hint.contains("per1m")
        || raw_text_lower
            .as_deref()
            .is_some_and(|text| text.contains("1m"))
    {
        return Some(raw);
    }

    if hint.contains("per_1k")
        || hint.contains("per1k")
        || raw_text_lower
            .as_deref()
            .is_some_and(|text| text.contains("1k"))
    {
        return Some(raw * 1000.0);
    }

    if hint.contains("per_token")
        || hint.contains("token_cost")
        || hint.contains("token_price")
        || matches!(
            hint.as_str(),
            "pricing.input"
                | "pricing.prompt"
                | "pricing.output"
                | "pricing.completion"
                | "pricing.cache_read"
                | "pricing.cached_read"
                | "pricing.input_cache_read"
                | "pricing.cached_input"
        )
        || raw_text_lower.as_deref().is_some_and(|text| {
            text.contains("per token") || text.contains("/token") || text.contains("token")
        })
        || raw <= 0.01
    {
        return Some(raw * 1_000_000.0);
    }

    Some(raw)
}

fn extract_first_number(raw: &str) -> Option<f64> {
    let mut started = false;
    let mut value = String::new();

    for ch in raw.chars() {
        if ch.is_ascii_digit() || ch == '.' || (!started && (ch == '-' || ch == '+')) {
            value.push(ch);
            started = true;
        } else if started {
            break;
        }
    }

    if value.is_empty() || value == "-" || value == "+" || value == "." {
        return None;
    }

    value.parse::<f64>().ok()
}
