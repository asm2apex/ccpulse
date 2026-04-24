use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Input {
    #[allow(dead_code)]
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub model: Option<Model>,
    pub workspace: Option<Workspace>,
    pub output_style: Option<OutputStyle>,
    pub cost: Option<Cost>,
    pub context_window: Option<ContextWindow>,
    pub rate_limits: Option<RateLimits>,
    pub effort: Option<Effort>,
    #[allow(dead_code)]
    pub thinking: Option<Thinking>,
    pub fast_mode: Option<bool>,
    #[allow(dead_code)]
    pub version: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Model {
    pub id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Workspace {
    pub current_dir: Option<String>,
    #[allow(dead_code)]
    pub project_dir: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct OutputStyle {
    pub name: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    #[allow(dead_code)]
    pub total_duration_ms: Option<u64>,
    #[allow(dead_code)]
    pub total_api_duration_ms: Option<u64>,
    pub total_lines_added: Option<u64>,
    pub total_lines_removed: Option<u64>,
}

#[derive(Deserialize, Default)]
pub struct ContextWindow {
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub context_window_size: Option<u64>,
    pub current_usage: Option<CurrentUsage>,
    pub used_percentage: Option<f64>,
    #[allow(dead_code)]
    pub remaining_percentage: Option<f64>,
}

#[derive(Deserialize, Default)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    #[allow(dead_code)]
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Deserialize, Default)]
pub struct RateLimits {
    pub five_hour: Option<RateLimitWindow>,
    pub seven_day: Option<RateLimitWindow>,
}

#[derive(Deserialize, Default)]
pub struct RateLimitWindow {
    pub used_percentage: Option<f64>,
    pub resets_at: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct Effort {
    pub level: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct Thinking {
    #[allow(dead_code)]
    pub enabled: Option<bool>,
}
