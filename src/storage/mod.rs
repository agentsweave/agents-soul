pub mod fixtures;
pub mod migrations;
pub mod sqlite;

use crate::storage::{fixtures::FixtureCatalog, migrations::MigrationPlan, sqlite::SqliteStorage};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageStack {
    pub sqlite: SqliteStorage,
    pub migrations: MigrationPlan,
    pub fixtures: FixtureCatalog,
}
