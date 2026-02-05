use sea_orm::{ConnectionTrait, DbBackend, DatabaseConnection, Statement};
use sea_orm_migration::prelude::*;

pub async fn apply(
    manager: &SchemaManager<'_>,
    conn: &DatabaseConnection,
) -> Result<(), DbErr> {
    if !manager.has_table("account_credentials").await? {
        manager
            .create_table(
                Table::create()
                    .table(AccountCredentials::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AccountCredentials::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AccountCredentials::AccountId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AccountCredentials::Provider)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AccountCredentials::ProviderSubject).string())
                    .col(ColumnDef::new(AccountCredentials::PasswordHash).string())
                    .col(ColumnDef::new(AccountCredentials::Metadata).json_binary())
                    .col(
                        ColumnDef::new(AccountCredentials::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(
                        ColumnDef::new(AccountCredentials::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(ColumnDef::new(AccountCredentials::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(AccountCredentials::CreatedBy).uuid())
                    .col(ColumnDef::new(AccountCredentials::UpdatedBy).uuid())
                    .col(ColumnDef::new(AccountCredentials::DeletedBy).uuid())
                    .col(ColumnDef::new(AccountCredentials::PurgeAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        conn
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS account_credentials_unique_provider \
                 ON account_credentials (account_id, provider)"
                    .to_string(),
            ))
            .await?;

        conn
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS account_credentials_unique_subject \
                 ON account_credentials (provider, provider_subject) \
                 WHERE provider_subject IS NOT NULL"
                    .to_string(),
            ))
            .await?;
    }

    Ok(())
}

#[derive(Iden)]
enum AccountCredentials {
    Table,
    Id,
    AccountId,
    Provider,
    ProviderSubject,
    PasswordHash,
    Metadata,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    CreatedBy,
    UpdatedBy,
    DeletedBy,
    PurgeAt,
}
