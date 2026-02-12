use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};

use crate::{entities::account_credentials, state::DatabaseClient};

#[async_trait]
pub trait AccountCredentialsRepo: Send + Sync {
    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: account_credentials::ActiveModel,
    ) -> Result<account_credentials::Model, sea_orm::DbErr>;
    async fn find_by_account_and_provider(
        &self,
        account_id: i64,
        provider: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr>;
    #[allow(dead_code)]
    async fn find_by_provider_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr>;
    async fn find_by_provider_subject_with_txn(
        &self,
        txn: &DatabaseTransaction,
        provider: &str,
        provider_subject: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr>;
}

pub struct SeaOrmAccountCredentialsRepo {
    db: std::sync::Arc<dyn crate::state::DatabaseClient>,
}

impl SeaOrmAccountCredentialsRepo {
    pub fn new(db: std::sync::Arc<dyn DatabaseClient>) -> Self {
        Self { db }
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

    async fn find_by_account_and_provider(
        &self,
        account_id: i64,
        provider: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr> {
        account_credentials::Entity::find()
            .filter(account_credentials::Column::AccountId.eq(account_id))
            .filter(account_credentials::Column::Provider.eq(provider))
            .filter(account_credentials::Column::DeletedAt.is_null())
            .one(self.db.conn())
            .await
    }

    async fn find_by_provider_subject(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> Result<Option<account_credentials::Model>, sea_orm::DbErr> {
        account_credentials::Entity::find()
            .filter(account_credentials::Column::Provider.eq(provider))
            .filter(account_credentials::Column::ProviderSubject.eq(provider_subject))
            .filter(account_credentials::Column::DeletedAt.is_null())
            .one(self.db.conn())
            .await
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
