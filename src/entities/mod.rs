pub mod accounts;

use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct DbState {
    pub conn: DatabaseConnection,
}
