use serde::Serialize;
use tauri::AppHandle;
use tracing::info;

#[derive(Serialize, Clone)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub timestamp: String,
    pub read: bool,
}

#[tauri::command]
pub fn send_notification(_app: AppHandle, title: String, message: String) -> Result<(), String> {
    info!("Notification: {} - {}", title, message);
    Ok(())
}
