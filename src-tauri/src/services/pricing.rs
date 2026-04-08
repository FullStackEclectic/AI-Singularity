pub struct PricingEngine;

impl PricingEngine {
    /// 计算消耗美元金额
    pub fn calculate_cost(model_name: &str, prompt_tokens: u64, completion_tokens: u64) -> f64 {
        // Approximate pricing per 1 million tokens (USD)
        let (input_rate_per_1m, output_rate_per_1m) = match model_name.to_lowercase().as_str() {
            // OpenAI
            m if m.contains("gpt-4o-mini") => (0.15, 0.60),
            m if m.contains("gpt-4o") => (2.5, 10.0), // Latest GPT-4o prices
            m if m.contains("o1-mini") => (3.0, 12.0),
            m if m.contains("o1-preview") => (15.0, 60.0),
            m if m.contains("o3-mini") => (1.1, 4.4),
            
            // Anthropic
            m if m.contains("claude-3-5-sonnet") => (3.0, 15.0),
            m if m.contains("claude-3-7-sonnet") => (3.0, 15.0),
            m if m.contains("claude-3-5-haiku") => (1.0, 5.0),
            m if m.contains("claude-3-opus") => (15.0, 75.0),
            m if m.contains("claude-3-haiku") => (0.25, 1.25),
            
            // Google
            m if m.contains("gemini-1.5-pro") => (1.25, 5.0),
            m if m.contains("gemini-1.5-flash") => (0.075, 0.3),
            m if m.contains("gemini-2.5-pro") => (2.0, 8.0),
            m if m.contains("gemini-2.0-flash") => (0.1, 0.4),
            
            // DeepSeek
            m if m.contains("deepseek-reasoner") => (0.55, 2.19),
            m if m.contains("deepseek-chat") => (0.14, 0.28),
            m if m.contains("deepseek-coder") => (0.14, 0.28),
            
            // Groq/Llama
            m if m.contains("llama-3.1-70b") => (0.59, 0.79),
            m if m.contains("llama-3.1-8b") => (0.05, 0.08),
            m if m.contains("llama-3.3-70b") => (0.59, 0.79),
            
            // Default fallback
            _ => (0.0, 0.0),
        };

        let p_cost = (prompt_tokens as f64 / 1_000_000.0) * input_rate_per_1m;
        let c_cost = (completion_tokens as f64 / 1_000_000.0) * output_rate_per_1m;

        p_cost + c_cost
    }
}
