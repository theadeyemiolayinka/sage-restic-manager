pub mod dashboard;
pub mod sources;
pub mod repository;
pub mod snapshots;
pub mod restore;
pub mod scheduler;
pub mod logs;
pub mod settings;

pub use dashboard::render_dashboard;
pub use sources::render_sources;
pub use repository::render_repository;
pub use snapshots::render_snapshots;
pub use restore::render_restore;
pub use scheduler::render_scheduler;
pub use logs::render_logs;
pub use settings::render_settings;
