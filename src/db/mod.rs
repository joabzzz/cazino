pub mod r#trait;

pub use r#trait::{Database, DbError, DbResult};

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;
