use super::AppConfig;

pub fn get_default_config() -> AppConfig {
    AppConfig::default()
}

pub fn load_config() -> Result<AppConfig, String> {
    AppConfig::load().map_err(|e| e.to_string())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}
