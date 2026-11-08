use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Root {
    pub model: Option<Model>,
    pub effort: Option<Effort>,
    pub workspace: Option<Workspace>,
    pub cwd: Option<String>,
    pub context_window: Option<ContextWindow>,
    pub rate_limits: Option<RateLimits>,
    pub cost: Option<Cost>,
    pub prompt_cache: Option<PromptCache>,
}

#[derive(Deserialize, Default)]
pub struct Model {
    pub display_name: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Effort {
    pub level: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Workspace {
    pub current_dir: Option<String>,
    pub project_dir: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ContextWindow {
    pub context_window_size: Option<f64>,
    pub used_percentage: Option<f64>,
    pub total_input_tokens: Option<f64>,
    pub current_usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
pub struct Usage {
    pub input_tokens: Option<f64>,
    pub cache_creation_input_tokens: Option<f64>,
    pub cache_read_input_tokens: Option<f64>,
}

#[derive(Deserialize, Default)]
pub struct RateLimits {
    pub five_hour: Option<RateLimit>,
    pub seven_day: Option<RateLimit>,
}

#[derive(Deserialize, Default)]
pub struct RateLimit {
    pub used_percentage: Option<f64>,
    pub resets_at: Option<f64>,
}

#[derive(Deserialize, Default)]
pub struct Cost {
    pub total_duration_ms: Option<f64>,
}

#[derive(Deserialize, Default)]
pub struct PromptCache {
    pub warm: Option<bool>,
    pub caching_observed: Option<bool>,
    /// Unix seconds; may be null
    pub expires_at: Option<f64>,
}
