use std::result;
use crate::model::GithubUser;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub github_id: i64,
    pub github_login: String,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug)]
pub struct UserStore {
    // pub jobs: RwLock<HashMap<Uuid, JobState>>,
    pub pool: PgPool,
}
impl UserStore {
    
    pub fn new(db:PgPool) -> UserStore {
        Self { pool: db }
    }
    pub async fn find_or_create_github_user(&self, github_user: &GithubUser) -> anyhow::Result<User> {
        let result = sqlx::query_as!(
            User,
            r#"INSERT INTO users (
                       id,
                       github_id,
                       github_login,
                       avatar_url
                       )
                       VALUES (
                               $1,
                               $2,
                               $3,
                               $4
                               )
                        ON CONFLICT (github_id)
                       DO UPDATE SET
                          github_login = EXCLUDED.github_login,
                          avatar_url = EXCLUDED.avatar_url,
                          updated_at = NOW()
                          RETURNING
                              id,
                              github_id,
                              github_login,
                              avatar_url,
                              created_at,
                              updated_at
                             "#,
            github_user.id,
            github_user.github_id,
            github_user.login,
            github_user.avatar_url,
        )
            .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }

}
