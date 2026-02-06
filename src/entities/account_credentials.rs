use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "account_credentials")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub account_id: i64,
    pub provider: String,
    pub provider_subject: Option<String>,
    pub password_hash: Option<String>,
    pub metadata: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub deleted_by: Option<Uuid>,
    pub purge_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
