use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use agentverse_core::{
    error::{CoreError, StorageError},
    repository::UserRepository,
    user::{User, UserKind},
};

use crate::entities::user::{self, Entity as UserEntity};

pub struct UserRepo {
    pub db: DatabaseConnection,
}

impl UserRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn model_to_domain(m: user::Model) -> User {
    User {
        id: m.id,
        username: m.username,
        email: m.email,
        kind: match m.kind.as_str() {
            "agent" => UserKind::Agent,
            "system" => UserKind::System,
            _ => UserKind::Human,
        },
        capabilities: m.capabilities,
        public_key: m.public_key,
        password_hash: m.password_hash,
        created_at: m.created_at.with_timezone(&Utc),
    }
}

#[async_trait]
impl UserRepository for UserRepo {
    async fn create(&self, user: User) -> Result<User, CoreError> {
        let now = chrono::Utc::now().fixed_offset();
        let model = user::ActiveModel {
            id: Set(user.id),
            username: Set(user.username.clone()),
            email: Set(user.email.clone()),
            password_hash: Set(user.password_hash.clone()),
            kind: Set(match &user.kind {
                UserKind::Agent => "agent".into(),
                UserKind::System => "system".into(),
                UserKind::Human => "human".into(),
            }),
            capabilities: Set(user.capabilities.clone()),
            public_key: Set(user.public_key.clone()),
            is_verified: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model
            .insert(&self.db)
            .await
            .map(model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn update(&self, user: User) -> Result<User, CoreError> {
        let model = user::ActiveModel {
            id: Set(user.id),
            username: Set(user.username.clone()),
            email: Set(user.email.clone()),
            password_hash: Set(user.password_hash.clone()),
            kind: Set(match &user.kind {
                UserKind::Agent => "agent".into(),
                UserKind::System => "system".into(),
                UserKind::Human => "human".into(),
            }),
            capabilities: Set(user.capabilities.clone()),
            public_key: Set(user.public_key.clone()),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        };
        model
            .update(&self.db)
            .await
            .map(model_to_domain)
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, CoreError> {
        UserEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map(|opt| opt.map(model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, CoreError> {
        UserEntity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map(|opt| opt.map(model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, CoreError> {
        UserEntity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map(|opt| opt.map(model_to_domain))
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
    }
}

