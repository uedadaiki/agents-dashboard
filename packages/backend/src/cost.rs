use crate::types::CumulativeUsage;

struct ModelPricing {
    prefix: &'static str,
    input: f64,
    output: f64,
    cache_read: f64,
    cache_creation: f64,
}

const MODEL_PRICING: &[ModelPricing] = &[
    ModelPricing {
        prefix: "claude-opus",
        input: 15.0,
        output: 75.0,
        cache_read: 1.5,
        cache_creation: 18.75,
    },
    ModelPricing {
        prefix: "claude-sonnet",
        input: 3.0,
        output: 15.0,
        cache_read: 0.3,
        cache_creation: 3.75,
    },
    ModelPricing {
        prefix: "claude-haiku",
        input: 0.8,
        output: 4.0,
        cache_read: 0.08,
        cache_creation: 1.0,
    },
];

fn get_pricing(model: &str) -> &'static ModelPricing {
    MODEL_PRICING
        .iter()
        .find(|m| model.starts_with(m.prefix))
        .unwrap_or(&MODEL_PRICING[1]) // default: sonnet
}

pub fn calculate_cost(
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
) -> f64 {
    let pricing = get_pricing(model);
    (input_tokens as f64 * pricing.input
        + output_tokens as f64 * pricing.output
        + cache_read_tokens as f64 * pricing.cache_read
        + cache_creation_tokens as f64 * pricing.cache_creation)
        / 1_000_000.0
}

pub fn add_usage(
    current: &CumulativeUsage,
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
) -> CumulativeUsage {
    let entry_cost = calculate_cost(model, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens);
    CumulativeUsage {
        input_tokens: current.input_tokens + input_tokens,
        output_tokens: current.output_tokens + output_tokens,
        cache_read_tokens: current.cache_read_tokens + cache_read_tokens,
        cache_creation_tokens: current.cache_creation_tokens + cache_creation_tokens,
        estimated_cost: current.estimated_cost + entry_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sonnet_cost() {
        let cost = calculate_cost("claude-sonnet-4-20250514", 1000, 500, 200, 100);
        let expected = (1000.0 * 3.0 + 500.0 * 15.0 + 200.0 * 0.3 + 100.0 * 3.75) / 1_000_000.0;
        assert!((cost - expected).abs() < 1e-10);
    }

    #[test]
    fn test_opus_cost() {
        let cost = calculate_cost("claude-opus-4-20250514", 1000, 500, 0, 0);
        let expected = (1000.0 * 15.0 + 500.0 * 75.0) / 1_000_000.0;
        assert!((cost - expected).abs() < 1e-10);
    }

    #[test]
    fn test_haiku_cost() {
        let cost = calculate_cost("claude-haiku-3-20250307", 1000, 500, 0, 0);
        let expected = (1000.0 * 0.8 + 500.0 * 4.0) / 1_000_000.0;
        assert!((cost - expected).abs() < 1e-10);
    }

    #[test]
    fn test_unknown_model_defaults_to_sonnet() {
        let cost = calculate_cost("gpt-4", 1000, 500, 0, 0);
        let expected = (1000.0 * 3.0 + 500.0 * 15.0) / 1_000_000.0;
        assert!((cost - expected).abs() < 1e-10);
    }

    #[test]
    fn test_add_usage() {
        let current = CumulativeUsage::default();
        let updated = add_usage(&current, "claude-sonnet-4-20250514", 100, 200, 50, 25);
        assert_eq!(updated.input_tokens, 100);
        assert_eq!(updated.output_tokens, 200);
        assert_eq!(updated.cache_read_tokens, 50);
        assert_eq!(updated.cache_creation_tokens, 25);
        assert!(updated.estimated_cost > 0.0);
    }
}
