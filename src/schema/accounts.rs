use sea_orm::{ConnectionTrait, DbBackend, DatabaseConnection, Statement};
use sea_orm_migration::prelude::*;

pub async fn apply(
    manager: &SchemaManager<'_>,
    conn: &DatabaseConnection,
) -> Result<(), DbErr> {
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
                    .col(ColumnDef::new(Accounts::Username).string())
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
                 ON accounts (lower(username)) WHERE deleted_at IS NULL AND username IS NOT NULL"
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
