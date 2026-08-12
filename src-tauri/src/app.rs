// Tauri setup — initialise the app state and managed resources.
use tauri::Manager;
use tracing::info;

/// Register all Tauri commands and initialise the app state.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = crate::build_tauri_app()?;

    app.manage(app_state);

    info!("SL Studio Tauri app setup complete");
    Ok(())
}
