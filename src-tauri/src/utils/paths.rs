use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_default().join("slstudio")
}

pub fn models_dir() -> PathBuf {
    app_data_dir().join("models")
}

pub fn dev_models_dir() -> PathBuf {
    PathBuf::from("./models")
}

pub fn logs_dir() -> PathBuf {
    app_data_dir().join("logs")
}

pub fn ensure_app_dirs() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = [app_data_dir(), models_dir(), logs_dir()];
    for dir in dirs {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}

pub fn ensure_dev_models_dir() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&dev_models_dir())?;
    Ok(())
}
