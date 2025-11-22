pub mod sqlite;
pub mod r#trait;

pub use r#trait::{Database, DbError, DbResult};
pub use sqlite::SqliteDatabase;
