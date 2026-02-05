use sea_orm::{ConnectionTrait, DbBackend, DatabaseConnection, Statement};
use sea_orm_migration::prelude::*;

pub async fn apply(
    manager: &SchemaManager<'_>,
    conn: &DatabaseConnection,
) -> Result<(), DbErr> {
    if !manager.has_table("account_authorizations").await? {
        manager
            .create_table(
                Table::create()
                    .table(AccountAuthorizations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AccountAuthorizations::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::AccountId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::TokenHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::TokenType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::ExpiresAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::RevokedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(
                        ColumnDef::new(AccountAuthorizations::DeletedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(ColumnDef::new(AccountAuthorizations::CreatedBy).uuid())
                    .col(ColumnDef::new(AccountAuthorizations::UpdatedBy).uuid())
                    .col(ColumnDef::new(AccountAuthorizations::DeletedBy).uuid())
                    .col(ColumnDef::new(AccountAuthorizations::PurgeAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        conn
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS account_authorizations_token_hash_unique \
                 ON account_authorizations (token_hash)"
                    .to_string(),
            ))
            .await?;
    }

    Ok(())
}

#[derive(Iden)]
enum AccountAuthorizations {
    Table,
    Id,
    AccountId,
    TokenHash,
    TokenType,
    ExpiresAt,
    RevokedAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    CreatedBy,
    UpdatedBy,
    DeletedBy,
    PurgeAt,
}
