use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, Set,
};

use crate::{entities::account_authorizations, state::DatabaseClient};

#[async_trait]
pub trait AccountAuthorizationsRepo: Send + Sync {
    async fn insert(
        &self,
        model: account_authorizations::ActiveModel,
    ) -> Result<account_authorizations::Model, sea_orm::DbErr>;
    #[allow(dead_code)]
    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: account_authorizations::ActiveModel,
    ) -> Result<account_authorizations::Model, sea_orm::DbErr>;
    async fn find_active_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<account_authorizations::Model>, sea_orm::DbErr>;
    async fn find_active_by_account_and_type(
        &self,
        account_id: i64,
        token_type: &str,
    ) -> Result<Option<account_authorizations::Model>, sea_orm::DbErr>;
    async fn revoke_by_id(&self, id: i64) -> Result<account_authorizations::Model, sea_orm::DbErr>;
}

pub struct SeaOrmAccountAuthorizationsRepo {
    db: std::sync::Arc<dyn DatabaseClient>,
}

impl SeaOrmAccountAuthorizationsRepo {
    pub fn new(db: std::sync::Arc<dyn DatabaseClient>) -> Self {
        Self { db }
    }

    fn active_condition() -> Condition {
        let now = Utc::now();
        Condition::all()
            .add(account_authorizations::Column::DeletedAt.is_null())
            .add(account_authorizations::Column::RevokedAt.is_null())
            .add(
                Condition::any()
                    .add(account_authorizations::Column::ExpiresAt.is_null())
                    .add(account_authorizations::Column::ExpiresAt.gt(now)),
            )
    }
}

#[async_trait]
impl AccountAuthorizationsRepo for SeaOrmAccountAuthorizationsRepo {
    async fn insert(
        &self,
        model: account_authorizations::ActiveModel,
    ) -> Result<account_authorizations::Model, sea_orm::DbErr> {
        model.insert(self.db.conn()).await
    }

    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: account_authorizations::ActiveModel,
    ) -> Result<account_authorizations::Model, sea_orm::DbErr> {
        model.insert(txn).await
    }

    async fn find_active_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<account_authorizations::Model>, sea_orm::DbErr> {
        account_authorizations::Entity::find()
            .filter(account_authorizations::Column::TokenHash.eq(token_hash))
            .filter(Self::active_condition())
            .one(self.db.conn())
            .await
    }

    async fn find_active_by_account_and_type(
        &self,
        account_id: i64,
        token_type: &str,
    ) -> Result<Option<account_authorizations::Model>, sea_orm::DbErr> {
        account_authorizations::Entity::find()
            .filter(account_authorizations::Column::AccountId.eq(account_id))
            .filter(account_authorizations::Column::TokenType.eq(token_type))
            .filter(Self::active_condition())
            .one(self.db.conn())
            .await
    }

    async fn revoke_by_id(&self, id: i64) -> Result<account_authorizations::Model, sea_orm::DbErr> {
        let Some(model) = account_authorizations::Entity::find_by_id(id)
            .one(self.db.conn())
            .await?
        else {
            return Err(sea_orm::DbErr::RecordNotFound(
                "account_authorization not found".to_string(),
            ));
        };

        let mut active: account_authorizations::ActiveModel = model.into();
        active.revoked_at = Set(Some(chrono::Utc::now().into()));
        active.update(self.db.conn()).await
    }
}
