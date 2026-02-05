use sea_orm::{ConnectionTrait, DbBackend, DatabaseConnection, Statement};
use sea_orm_migration::prelude::*;

mod account_authorizations;
mod account_credentials;
mod account_settings;
mod accounts;

pub async fn apply(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let manager = SchemaManager::new(conn);

    conn
        .execute(Statement::from_string(
            DbBackend::Postgres,
            "CREATE EXTENSION IF NOT EXISTS pgcrypto".to_string(),
        ))
        .await?;

    accounts::apply(&manager, conn).await?;
    account_settings::apply(&manager).await?;
    account_credentials::apply(&manager, conn).await?;
    account_authorizations::apply(&manager, conn).await?;

    Ok(())
}
