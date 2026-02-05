use sea_orm::{ConnectionTrait, DbBackend, DatabaseConnection, Statement};
use sea_orm_migration::prelude::*;

pub async fn apply(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let manager = SchemaManager::new(conn);
    let conn = manager.get_connection();

    conn
        .execute(Statement::from_string(
            DbBackend::Postgres,
            "CREATE EXTENSION IF NOT EXISTS pgcrypto".to_string(),
        ))
        .await?;

    if !manager.has_table("accounts").await? {
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Accounts::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Accounts::Uid)
                            .uuid()
                            .not_null()
                            .default(SimpleExpr::Custom("gen_random_uuid()".into())),
                    )
                    .col(
                        ColumnDef::new(Accounts::AccountType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Accounts::Username).string().not_null())
                    .col(ColumnDef::new(Accounts::Email).string())
                    .col(ColumnDef::new(Accounts::Phone).string())
                    .col(
                        ColumnDef::new(Accounts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(
                        ColumnDef::new(Accounts::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(ColumnDef::new(Accounts::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Accounts::CreatedBy).uuid())
                    .col(ColumnDef::new(Accounts::UpdatedBy).uuid())
                    .col(ColumnDef::new(Accounts::DeletedBy).uuid())
                    .col(ColumnDef::new(Accounts::PurgeAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        conn
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "ALTER TABLE accounts ADD CONSTRAINT accounts_account_type_check \
                 CHECK (account_type IN ('user','team','robot'))"
                    .to_string(),
            ))
            .await?;

        conn
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS accounts_uid_unique \
                 ON accounts (uid)"
                    .to_string(),
            ))
            .await?;

        conn
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "CREATE UNIQUE INDEX IF NOT EXISTS accounts_username_unique \
                 ON accounts (lower(username)) WHERE deleted_at IS NULL"
                    .to_string(),
            ))
            .await?;
    }

    if !manager.has_table("account_settings").await? {
        manager
            .create_table(
                Table::create()
                    .table(AccountSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AccountSettings::AccountId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AccountSettings::Nickname).string())
                    .col(ColumnDef::new(AccountSettings::AvatarUrl).string())
                    .col(
                        ColumnDef::new(AccountSettings::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(
                        ColumnDef::new(AccountSettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("now()".into())),
                    )
                    .col(ColumnDef::new(AccountSettings::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(AccountSettings::CreatedBy).uuid())
                    .col(ColumnDef::new(AccountSettings::UpdatedBy).uuid())
                    .col(ColumnDef::new(AccountSettings::DeletedBy).uuid())
                    .col(ColumnDef::new(AccountSettings::PurgeAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;
    }

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
                    .col(ColumnDef::new(AccountAuthorizations::ExpiresAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(AccountAuthorizations::RevokedAt).timestamp_with_time_zone())
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
                    .col(ColumnDef::new(AccountAuthorizations::DeletedAt).timestamp_with_time_zone())
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
enum Accounts {
    Table,
    Id,
    Uid,
    AccountType,
    Username,
    Email,
    Phone,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    CreatedBy,
    UpdatedBy,
    DeletedBy,
    PurgeAt,
}

#[derive(Iden)]
enum AccountSettings {
    Table,
    AccountId,
    Nickname,
    AvatarUrl,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    CreatedBy,
    UpdatedBy,
    DeletedBy,
    PurgeAt,
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
