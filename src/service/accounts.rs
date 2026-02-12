use async_trait::async_trait;
use sea_orm::{DatabaseTransaction, TransactionError, TransactionTrait};
use uuid::Uuid;

use crate::{
    entities::{account_credentials, accounts},
    repo::{account_credentials::AccountCredentialsRepo, accounts::AccountsRepo},
    state::DatabaseClient,
};

pub struct CreateAccountInput {
    pub account_type: String,
    pub username: Option<String>,
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

#[derive(Clone)]
pub struct GetOrCreateByProviderSubjectInput {
    pub provider: String,
    pub provider_subject: String,
    pub account_type: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub created_by: Option<Uuid>,
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
    async fn delete(
        &self,
        uid: Uuid,
        deleted_by: Option<Uuid>,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr>;
    #[allow(dead_code)]
    async fn get_or_create_by_provider_subject(
        &self,
        input: GetOrCreateByProviderSubjectInput,
    ) -> Result<accounts::Model, sea_orm::DbErr>;
}

#[allow(dead_code)]
pub struct AccountsServiceImpl {
    db: std::sync::Arc<dyn DatabaseClient>,
    accounts_repo: std::sync::Arc<dyn AccountsRepo>,
    credentials_repo: std::sync::Arc<dyn AccountCredentialsRepo>,
}

impl AccountsServiceImpl {
    pub fn new(
        db: std::sync::Arc<dyn DatabaseClient>,
        accounts_repo: std::sync::Arc<dyn AccountsRepo>,
        credentials_repo: std::sync::Arc<dyn AccountCredentialsRepo>,
    ) -> Self {
        Self {
            db,
            accounts_repo,
            credentials_repo,
        }
    }
}

#[async_trait]
impl AccountsService for AccountsServiceImpl {
    async fn create(&self, input: CreateAccountInput) -> Result<accounts::Model, sea_orm::DbErr> {
        let model = accounts::ActiveModel {
            uid: sea_orm::Set(Uuid::new_v4()),
            account_type: sea_orm::Set(input.account_type),
            username: sea_orm::Set(input.username),
            email: sea_orm::Set(input.email),
            phone: sea_orm::Set(input.phone),
            created_by: sea_orm::Set(input.created_by),
            updated_by: sea_orm::Set(input.created_by),
            ..Default::default()
        };

        self.accounts_repo.insert(model).await
    }

    async fn get(&self, uid: Uuid) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        self.accounts_repo.find_by_uid(uid).await
    }

    async fn update(
        &self,
        uid: Uuid,
        input: UpdateAccountInput,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        let Some(model) = self.accounts_repo.find_by_uid(uid).await? else {
            return Ok(None);
        };

        let mut active: accounts::ActiveModel = model.into();
        if let Some(username) = input.username {
            active.username = sea_orm::Set(Some(username));
        }
        if let Some(email) = input.email {
            active.email = sea_orm::Set(Some(email));
        }
        if let Some(phone) = input.phone {
            active.phone = sea_orm::Set(Some(phone));
        }
        active.updated_by = sea_orm::Set(input.updated_by);

        let updated = self.accounts_repo.update(active).await?;
        Ok(Some(updated))
    }

    async fn delete(
        &self,
        uid: Uuid,
        deleted_by: Option<Uuid>,
    ) -> Result<Option<accounts::Model>, sea_orm::DbErr> {
        let Some(model) = self.accounts_repo.find_by_uid(uid).await? else {
            return Ok(None);
        };

        let actor = deleted_by
            .or(model.updated_by)
            .or(model.created_by)
            .unwrap_or_else(Uuid::nil);
        let mut active: accounts::ActiveModel = model.into();
        active.deleted_at = sea_orm::Set(Some(chrono::Utc::now().into()));
        active.deleted_by = sea_orm::Set(Some(actor));
        active.updated_by = sea_orm::Set(Some(actor));

        let updated = self.accounts_repo.update(active).await?;
        Ok(Some(updated))
    }

    async fn get_or_create_by_provider_subject(
        &self,
        input: GetOrCreateByProviderSubjectInput,
    ) -> Result<accounts::Model, sea_orm::DbErr> {
        let accounts_repo = self.accounts_repo.clone();
        let credentials_repo = self.credentials_repo.clone();
        let input = input.clone();
        let conn = self.db.conn();

        let result = conn
            .transaction::<_, accounts::Model, sea_orm::DbErr>(|txn| {
                let accounts_repo = accounts_repo.clone();
                let credentials_repo = credentials_repo.clone();
                let input = input.clone();
                Box::pin(async move {
                    get_or_create_by_provider_subject_txn(
                        txn,
                        accounts_repo.as_ref(),
                        credentials_repo.as_ref(),
                        &input,
                    )
                    .await
                })
            })
            .await;

        match result {
            Ok(account) => Ok(account),
            Err(TransactionError::Connection(err)) => Err(err),
            Err(TransactionError::Transaction(err)) => Err(err),
        }
    }
}

async fn get_or_create_by_provider_subject_txn(
    txn: &DatabaseTransaction,
    accounts_repo: &dyn AccountsRepo,
    credentials_repo: &dyn AccountCredentialsRepo,
    input: &GetOrCreateByProviderSubjectInput,
) -> Result<accounts::Model, sea_orm::DbErr> {
    if let Some(credential) = credentials_repo
        .find_by_provider_subject_with_txn(txn, &input.provider, &input.provider_subject)
        .await?
    {
        if let Some(account) = accounts_repo
            .find_by_id_with_txn(txn, credential.account_id)
            .await?
        {
            return Ok(account);
        }

        return Err(sea_orm::DbErr::RecordNotFound(format!(
            "account not found for credential {}:{}",
            input.provider, input.provider_subject
        )));
    }

    let account_model = accounts::ActiveModel {
        uid: sea_orm::Set(Uuid::new_v4()),
        account_type: sea_orm::Set(input.account_type.clone()),
        username: sea_orm::Set(input.username.clone()),
        email: sea_orm::Set(input.email.clone()),
        phone: sea_orm::Set(None),
        created_by: sea_orm::Set(input.created_by),
        updated_by: sea_orm::Set(input.created_by),
        ..Default::default()
    };

    let account = accounts_repo.insert_with_txn(txn, account_model).await?;

    let credential_model = account_credentials::ActiveModel {
        account_id: sea_orm::Set(account.id),
        provider: sea_orm::Set(input.provider.clone()),
        provider_subject: sea_orm::Set(Some(input.provider_subject.clone())),
        password_hash: sea_orm::Set(None),
        metadata: sea_orm::Set(None),
        created_by: sea_orm::Set(input.created_by),
        updated_by: sea_orm::Set(input.created_by),
        ..Default::default()
    };

    credentials_repo
        .insert_with_txn(txn, credential_model)
        .await?;

    Ok(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repo::{account_credentials::SeaOrmAccountCredentialsRepo, accounts::SeaOrmAccountsRepo},
        schema,
    };
    use sea_orm::Database;
    use std::sync::Arc;

    struct TestDatabaseClient {
        conn: sea_orm::DatabaseConnection,
    }

    impl DatabaseClient for TestDatabaseClient {
        fn conn(&self) -> &sea_orm::DatabaseConnection {
            &self.conn
        }
    }

    #[tokio::test]
    #[ignore]
    async fn get_or_create_by_provider_subject_uses_transaction(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(()),
        };

        let conn = Database::connect(&database_url).await?;
        schema::apply(&conn).await?;

        let db = Arc::new(TestDatabaseClient { conn });
        let accounts_repo = Arc::new(SeaOrmAccountsRepo::new(db.clone()));
        let credentials_repo = Arc::new(SeaOrmAccountCredentialsRepo::new(db.clone()));
        let provider_subject = format!("test-{}", Uuid::new_v4());
        let username = Some(format!("gh_test_{}", Uuid::new_v4().simple()));
        let input = GetOrCreateByProviderSubjectInput {
            provider: "github".to_string(),
            provider_subject,
            account_type: "user".to_string(),
            username,
            email: None,
            created_by: None,
        };
        let txn = db.conn().begin().await?;
        let first = get_or_create_by_provider_subject_txn(
            &txn,
            accounts_repo.as_ref(),
            credentials_repo.as_ref(),
            &input,
        )
        .await?;
        let second = get_or_create_by_provider_subject_txn(
            &txn,
            accounts_repo.as_ref(),
            credentials_repo.as_ref(),
            &input,
        )
        .await?;

        assert_eq!(first.id, second.id);
        txn.rollback().await?;
        Ok(())
    }
}
