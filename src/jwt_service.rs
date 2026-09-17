use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    // jwt sign time
    pub iat: usize,
    // jwt id
    pub jti: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsClaims {
    pub sub: Uuid,
    pub exp: usize,
    pub job_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ExchangeCodeRequest {
    pub code: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ExchangeCodeResponse {
    pub access_token: String,
    pub token_type: String,
}

pub struct JwtConfig {
    pub(crate) secret: String,
    pub(crate) expiration: Duration,
}

impl JwtConfig {
    pub fn create_token(&self, user_id: Uuid) -> anyhow::Result<String> {
        let exp = (Utc::now() + self.expiration).timestamp() as usize;
        let now = Utc::now().timestamp() as usize;
        let claim = Claims {
            sub: user_id,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            exp,
        };
        tracing::info!("creating jwt token for user_id: {}, exp: {}", user_id, exp);
        let token = encode(
            &Header::default(),
            &claim,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )?;
        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> anyhow::Result<Claims> {
        let key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(token, &key, &validation)?;
        Ok(token_data.claims)
    }
}

pub fn create_ws_ticket(
    user_id: Uuid,
    job_id: Uuid,
    secret: &[u8],
) -> anyhow::Result<String, jsonwebtoken::errors::Error> {
    let exp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 30;
    let claims = WsClaims {
        sub: user_id,
        exp: exp as usize,
        job_id,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}
