use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{entities::accounts, repo::accounts::AccountsRepo};

pub struct CreateAccountInput {
    pub account_type: String,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub created_by: Option<Uuid>,
}

pub struct UpdateAccountInput {
    pub username: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub updated_by: Option<Uuid>,
}

#[async_trait]
pub trait AccountsService: Send + Sync {
    async fn create(&self, input: CreateAccountInput) -> Result<accounts::Model, sea_orm::DbErr>;
    async fn get(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    async fn update(
        &self,
        uid: Uuid,
        input: UpdateAccountInput,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    async fn delete(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
}

pub struct AccountsServiceImpl {
    repo: std::sync::Arc<dyn AccountsRepo>,
}

impl AccountsServiceImpl {
    pub fn new(repo: std::sync::Arc<dyn AccountsRepo>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl AccountsService for AccountsServiceImpl {
    async fn create(&self, input: CreateAccountInput) -> Result<accounts::Model, sea_orm::DbErr> {
        let now = Utc::now();
        let model = accounts::ActiveModel {
            uid: sea_orm::Set(Uuid::new_v4()),
            account_type: sea_orm::Set(input.account_type),
            username: sea_orm::Set(input.username),
            email: sea_orm::Set(input.email),
            phone: sea_orm::Set(input.phone),
            created_at: sea_orm::Set(now.into()),
            updated_at: sea_orm::Set(now.into()),
            created_by: sea_orm::Set(input.created_by),
            updated_by: sea_orm::Set(input.created_by),
            ..Default::default()
        };

        self.repo.insert(model).await
    }

    async fn get(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        self.repo.find_by_uid(uid).await
    }

    async fn update(
        &self,
        uid: Uuid,
        input: UpdateAccountInput,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        let Some(model) = self.repo.find_by_uid(uid).await? else {
            return Ok(None);
        };

        let now = Utc::now();
        let mut active: accounts::ActiveModel = model.into();
        if let Some(username) = input.username {
            active.username = sea_orm::Set(username);
        }
        if let Some(email) = input.email {
            active.email = sea_orm::Set(Some(email));
        }
        if let Some(phone) = input.phone {
            active.phone = sea_orm::Set(Some(phone));
        }
        active.updated_at = sea_orm::Set(now.into());
        active.updated_by = sea_orm::Set(input.updated_by);

        let updated = self.repo.update(active).await?;
        Ok(Some(updated))
    }

    async fn delete(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        let Some(model) = self.repo.find_by_uid(uid).await? else {
            return Ok(None);
        };

        let now = Utc::now();
        let mut active: accounts::ActiveModel = model.into();
        active.deleted_at = sea_orm::Set(Some(now.into()));
        active.updated_at = sea_orm::Set(now.into());

        let updated = self.repo.update(active).await?;
        Ok(Some(updated))
    }
}
