use async_trait::async_trait;
use sea_orm::prelude::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{entities::accounts, state::DatabaseClient};

#[async_trait]
pub trait AccountsRepo: Send + Sync {
    async fn insert(&self, model: accounts::ActiveModel)
        -> Result<accounts::Model, sea_orm::DbErr>;
    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: accounts::ActiveModel,
    ) -> Result<accounts::Model, sea_orm::DbErr>;
    async fn find_by_uid(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    async fn find_by_email(&self, email: &str) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    async fn find_by_id_with_txn(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    async fn update(&self, model: accounts::ActiveModel)
        -> Result<accounts::Model, sea_orm::DbErr>;
}

pub struct SeaOrmAccountsRepo {
    db: std::sync::Arc<dyn DatabaseClient>,
}

impl SeaOrmAccountsRepo {
    pub fn new(db: std::sync::Arc<dyn DatabaseClient>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AccountsRepo for SeaOrmAccountsRepo {
    async fn insert(
        &self,
        model: accounts::ActiveModel,
    ) -> Result<accounts::Model, sea_orm::DbErr> {
        model.insert(self.db.conn()).await
    }

    async fn insert_with_txn(
        &self,
        txn: &DatabaseTransaction,
        model: accounts::ActiveModel,
    ) -> Result<accounts::Model, sea_orm::DbErr> {
        model.insert(txn).await
    }

    async fn find_by_uid(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        accounts::Entity::find()
            .filter(accounts::Column::Uid.eq(uid))
            .filter(accounts::Column::DeletedAt.is_null())
            .one(self.db.conn())
            .await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        let normalized = email.trim().to_lowercase();
        accounts::Entity::find()
            .filter(accounts::Column::DeletedAt.is_null())
            .filter(Expr::cust("lower(email)").eq(normalized))
            .one(self.db.conn())
            .await
    }

    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        let normalized = username.trim().to_lowercase();
        accounts::Entity::find()
            .filter(accounts::Column::DeletedAt.is_null())
            .filter(Expr::cust("lower(username)").eq(normalized))
            .one(self.db.conn())
            .await
    }

    async fn find_by_id_with_txn(
        &self,
        txn: &DatabaseTransaction,
        id: i64,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        accounts::Entity::find_by_id(id)
            .filter(accounts::Column::DeletedAt.is_null())
            .one(txn)
            .await
    }

    async fn update(
        &self,
        model: accounts::ActiveModel,
    ) -> Result<accounts::Model, sea_orm::DbErr> {
        model.update(self.db.conn()).await
    }
}
