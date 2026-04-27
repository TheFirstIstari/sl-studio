//! Query modules for [`Database`]. Each submodule provides an `impl Database`
//! extension block grouping related operations. The `Database` struct itself,
//! schema initialization, migrations, and shared cache helpers remain in
//! [`super::database`].

pub mod analytics;
pub mod annotations;
pub mod chains;
pub mod checkpoints;
pub mod dedup;
pub mod entities;
pub mod errors;
pub mod exports;
pub mod facets;
pub mod intelligence;
pub mod network;
pub mod pipelines;
pub mod registry;
pub mod search;
pub mod timeline;
pub mod weighting;
