use crate::db::Database;
use crate::error::AppResult;
use crate::models::{Model, Platform};
use crate::services::event_bus::EventBus;
use crate::services::model_catalog::{
    ModelCatalogService, SaveModelPricingRequest as ServiceSaveModelPricingRequest,
};
use tauri::{AppHandle, State};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetModelPricingRequest {
    pub platform: Platform,
    pub model_id: String,
}

#[tauri::command]
pub fn list_models(db: State<'_, Database>) -> AppResult<Vec<Model>> {
    ModelCatalogService::new(&db).list_models()
}

#[tauri::command]
pub fn get_platform_models(platform: String, db: State<'_, Database>) -> AppResult<Vec<Model>> {
    let parsed = serde_json::from_str::<Platform>(&format!("\"{}\"", platform)).ok();
    match parsed {
        Some(target) => ModelCatalogService::new(&db).get_platform_models(&target),
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn save_model_pricing(
    request: SaveModelPricingRequest,
    db: State<'_, Database>,
    app: AppHandle,
) -> AppResult<Model> {
    let model = ModelCatalogService::new(&db).save_pricing(ServiceSaveModelPricingRequest {
        platform: request.platform,
        model_id: request.model_id,
        fixed_price: request.fixed_price,
        request_price: request.request_price,
        input_price_per_1m: request.input_price_per_1m,
        output_price_per_1m: request.output_price_per_1m,
        pricing_currency: request.pricing_currency,
        pricing_unit: request.pricing_unit,
        note: request.note,
    })?;
    EventBus::emit_data_changed(&app, "models", "update_pricing", "model.save_pricing");
    Ok(model)
}

#[tauri::command]
pub fn reset_model_pricing(
    request: ResetModelPricingRequest,
    db: State<'_, Database>,
    app: AppHandle,
) -> AppResult<Model> {
    let service = ModelCatalogService::new(&db);
    service.reset_pricing(&request.platform, &request.model_id)?;
    let model = service
        .get_platform_models(&request.platform)?
        .into_iter()
        .find(|item| item.id == request.model_id)
        .ok_or_else(|| anyhow::anyhow!("模型不存在，无法恢复基础价格"))?;
    EventBus::emit_data_changed(&app, "models", "reset_pricing", "model.reset_pricing");
    Ok(model)
}
