use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};

use crate::entities::account_credentials;

#[async_trait]
pub trait AccountCredentialsRepo: Send + Sync {
    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: account_credentials::ActiveModel,
    ) -> Result<account_credentials::Model, sea_orm::DbErr>;
    async fn find_by_provider_subject_with_txn(
        &self,
        txn: &DatabaseTransaction,
        provider: &str,
        provider_subject: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr>;
}

pub struct SeaOrmAccountCredentialsRepo;

impl SeaOrmAccountCredentialsRepo {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AccountCredentialsRepo for SeaOrmAccountCredentialsRepo {
    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: account_credentials::ActiveModel,
    ) -> Result<account_credentials::Model, sea_orm::DbErr> {
        model.insert(txn).await
    }

    async fn find_by_provider_subject_with_txn(
        &self,
        txn: &DatabaseTransaction,
        provider: &str,
        provider_subject: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr> {
        account_credentials::Entity::find()
            .filter(account_credentials::Column::Provider.eq(provider))
            .filter(account_credentials::Column::ProviderSubject.eq(provider_subject))
            .filter(account_credentials::Column::DeletedAt.is_null())
            .one(txn)
            .await
    }
}
