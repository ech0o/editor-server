use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Header, EncodingKey, DecodingKey, Validation, Algorithm, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub(crate) sub: Uuid,
    exp: usize,
}

#[derive(Debug, Deserialize)]
pub struct ExchangeCodeRequest{
    pub code: String,
}
#[derive(Debug, Deserialize,Serialize)]
pub struct ExchangeCodeResponse{
    pub access_token: String,
    pub token_type: String,
}

pub struct JwtConfig {
    pub(crate) secret: String,
    pub(crate) expiration: Duration,
}

impl JwtConfig {
    pub fn create_token(&self, user_id:Uuid)->anyhow::Result<String>{
        let exp = (Utc::now() +self.expiration).timestamp() as usize;
        let claim = Claims{
            sub:user_id,
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

    pub fn verify_token(&self,token:&str)->anyhow::Result<Claims>{
        let key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::new(Algorithm::HS256);
        let token_data  = decode::<Claims>(
            token,
            &key,
            &validation,
        )?;
        Ok(token_data.claims)
    }
}