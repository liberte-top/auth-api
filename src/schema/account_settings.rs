use sea_orm_migration::prelude::*;

pub async fn apply(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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

    Ok(())
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
