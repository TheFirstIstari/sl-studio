pub mod analytics;
pub mod annotations;
pub mod backup;
pub mod chains;
pub mod config;
pub mod entities;
pub mod export;
pub mod extraction;
pub mod facets;
pub mod facts;
pub mod hardware;
pub mod inference;
pub mod language;
pub mod metadata;
pub mod migration;
pub mod model_mgmt;
pub mod network;
pub mod notify;
pub mod pipelines;
pub mod project;
pub mod quality;
pub mod registry;
pub mod search;
pub mod structured;
pub mod workflow;

pub use analytics::*;
pub use annotations::*;
pub use backup::*;
pub use chains::*;
pub use config::*;
pub use entities::*;
pub use export::*;
pub use extraction::*;
pub use facets::*;
pub use facts::*;
pub use hardware::*;
pub use inference::*;
pub use language::*;
pub use metadata::*;
pub use migration::*;
pub use model_mgmt::*;
pub use network::*;
pub use notify::*;
pub use pipelines::*;
pub use project::*;
pub use quality::*;
pub use registry::*;
pub use search::*;
pub use structured::*;
pub use workflow::*;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

use crate::core::Database;
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// Acquire the database handle from `AppState`, returning a descriptive error
/// if the RwLock is poisoned or the project has not yet been initialised.
/// Use this at the top of any command that needs DB access instead of
/// repeating the three-line lock/unwrap/clone boilerplate.
pub(crate) fn require_db(state: &State<AppState>) -> Result<Arc<Database>, String> {
    state
        .db
        .read()
        .map_err(|e| format!("Database lock poisoned: {e}"))?
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())
        .cloned()
}
