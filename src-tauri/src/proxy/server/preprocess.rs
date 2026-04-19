use crate::db::Database;
use crate::proxy::converter::{OpenAIMessage, OpenAIRequest};

pub(super) fn apply_model_mappings(db: &Database, current_model: &str) -> String {
    let mut resolved_model = current_model.to_string();

    if let Ok(mappings) = crate::services::model_mapping::ModelMappingService::new(db).get_all() {
        for mapping in mappings {
            if mapping.is_active && resolved_model.eq_ignore_ascii_case(&mapping.source_model) {
                tracing::info!(
                    "🔄 模型重写触发: {} => {}",
                    resolved_model,
                    mapping.target_model
                );
                resolved_model = mapping.target_model;
                break;
            }
        }
    }

    resolved_model
}

pub(super) fn compress_context_if_needed(body: &mut OpenAIRequest) {
    if let Ok(cfg) = crate::commands::proxy::ENGINE_CONFIG.read() {
        if !cfg.advanced_thinking.enabled {
            return;
        }

        let estimated_tokens: usize = body.messages.iter().map(|msg| msg.content.len()).sum::<usize>() / 3;
        let budget = cfg.advanced_thinking.budget_limit as usize;
        let threshold = (budget as f64 * cfg.advanced_thinking.compression_threshold) as usize;

        if estimated_tokens <= threshold || body.messages.len() <= 4 {
            return;
        }

        tracing::info!(
            "🧠 高级思维压缩触发: 当前预估 {} tokens，已超过阈值 {} (预算: {})",
            estimated_tokens,
            threshold,
            budget
        );

        let mut compressed_msgs = vec![];
        if let Some(sys) = body.messages.first() {
            compressed_msgs.push(sys.clone());
        }
        let len = body.messages.len();
        compressed_msgs.push(OpenAIMessage {
            role: "system".to_string(),
            content: "[...Previous context compressed by AI Singularity Cognitive Engine...]"
                .to_string(),
        });
        for msg in body.messages.clone().into_iter().skip(len.saturating_sub(3)) {
            compressed_msgs.push(msg);
        }
        body.messages = compressed_msgs;
    }
}
