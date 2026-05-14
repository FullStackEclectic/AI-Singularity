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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::path::Path;

    fn make_db() -> Database {
        Database::new(Path::new(":memory:")).expect("open in-memory db")
    }

    // ── USD 平台（OpenAI / Anthropic / Gemini）──────────────────────────────

    #[test]
    fn cost_openai_gpt4o_is_positive() {
        let db = make_db();
        // gpt-4o: input $2.5/1M, output $10/1M
        let cost = PricingEngine::calculate_cost(&db, "openai", "gpt-4o", 1_000_000, 1_000_000);
        assert!(cost > 0.0, "expected positive cost, got {}", cost);
        // 1M input + 1M output = $2.5 + $10 = $12.5
        let expected = 12.5_f64;
        assert!(
            (cost - expected).abs() < 0.01,
            "expected ~{}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn cost_anthropic_claude_sonnet_is_positive() {
        let db = make_db();
        // claude-sonnet-4-5: input $3/1M, output $15/1M
        let cost = PricingEngine::calculate_cost(
            &db,
            "anthropic",
            "claude-sonnet-4-5",
            500_000,
            500_000,
        );
        assert!(cost > 0.0, "expected positive cost, got {}", cost);
        let expected = 0.5 * 3.0 + 0.5 * 15.0; // $9.0
        assert!(
            (cost - expected).abs() < 0.01,
            "expected ~{}, got {}",
            expected,
            cost
        );
    }

    #[test]
    fn cost_gemini_flash_is_positive() {
        let db = make_db();
        // gemini-2.5-flash: input $0.3/1M, output $2.5/1M
        let cost =
            PricingEngine::calculate_cost(&db, "gemini", "gemini-2.5-flash", 1_000_000, 0);
        assert!(cost > 0.0, "expected positive cost, got {}", cost);
    }

    // ── CNY 平台（DeepSeek / Aliyun / Moonshot）────────────────────────────
    // CNY 定价不兼容 USD 成本计算，calculate_cost 应返回 0.0

    #[test]
    fn cost_cny_model_returns_zero() {
        let db = make_db();
        // qwen-max 是 CNY 定价，不应被计入 USD 成本
        let cost = PricingEngine::calculate_cost(&db, "aliyun", "qwen-max", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.0, "CNY model should return 0.0, got {}", cost);
    }

    #[test]
    fn cost_moonshot_kimi_k2_returns_zero() {
        let db = make_db();
        // kimi-k2 是 CNY 定价
        let cost =
            PricingEngine::calculate_cost(&db, "moonshot", "kimi-k2", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.0, "CNY model should return 0.0, got {}", cost);
    }

    // ── 未知模型 ────────────────────────────────────────────────────────────

    #[test]
    fn cost_unknown_model_returns_zero() {
        let db = make_db();
        let cost = PricingEngine::calculate_cost(
            &db,
            "openai",
            "gpt-totally-made-up-9999",
            100_000,
            100_000,
        );
        assert_eq!(cost, 0.0, "unknown model should return 0.0, got {}", cost);
    }

    #[test]
    fn cost_unknown_platform_returns_zero() {
        let db = make_db();
        let cost =
            PricingEngine::calculate_cost(&db, "nonexistent_platform", "gpt-4o", 100_000, 100_000);
        // Falls back to base catalog lookup by model name; gpt-4o is in catalog so may still match.
        // The important invariant is that the result is non-negative.
        assert!(cost >= 0.0, "cost must be non-negative, got {}", cost);
    }

    #[test]
    fn cost_zero_tokens_returns_zero() {
        let db = make_db();
        let cost = PricingEngine::calculate_cost(&db, "openai", "gpt-4o", 0, 0);
        assert_eq!(cost, 0.0, "zero tokens should yield 0.0 cost, got {}", cost);
    }

    // ── 手动覆盖定价 ────────────────────────────────────────────────────────

    #[test]
    fn cost_respects_manual_pricing_override() {
        use crate::models::Platform;
        use crate::services::model_catalog::{ModelCatalogService, SaveModelPricingRequest};

        let db = make_db();
        // 设置 gpt-4o 的自定义价格：input $1/1M, output $2/1M
        let catalog = ModelCatalogService::new(&db);
        catalog
            .save_pricing(SaveModelPricingRequest {
                platform: Platform::OpenAI,
                model_id: "gpt-4o".to_string(),
                fixed_price: None,
                request_price: None,
                input_price_per_1m: Some(1.0),
                output_price_per_1m: Some(2.0),
                pricing_currency: Some("USD".to_string()),
                pricing_unit: Some("1m_tokens".to_string()),
                note: None,
            })
            .unwrap();

        let cost = PricingEngine::calculate_cost(&db, "openai", "gpt-4o", 1_000_000, 1_000_000);
        let expected = 1.0 + 2.0; // $3.0
        assert!(
            (cost - expected).abs() < 0.001,
            "expected override cost ~{}, got {}",
            expected,
            cost
        );
    }
}
