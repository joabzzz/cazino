// Database trait - always available
pub mod r#trait;
pub use r#trait::{Database, DbError, DbResult};

// SQLite implementation - only with "sqlite" feature
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;
