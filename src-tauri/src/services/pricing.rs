pub struct PricingEngine;

impl PricingEngine {
    /// 计算消耗美元金额
    pub fn calculate_cost(
        db: &crate::db::Database,
        platform: &str,
        model_name: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
    ) -> f64 {
        let service = crate::services::model_catalog::ModelCatalogService::new(db);
        let pricing = service.resolve_pricing(Some(platform), model_name).unwrap_or_default();

        let p_cost =
            (prompt_tokens as f64 / 1_000_000.0) * pricing.input_price_per_1m.unwrap_or(0.0);
        let c_cost =
            (completion_tokens as f64 / 1_000_000.0) * pricing.output_price_per_1m.unwrap_or(0.0);
        let request_cost = pricing.request_price.unwrap_or(0.0);

        p_cost + c_cost + request_cost
    }
}
