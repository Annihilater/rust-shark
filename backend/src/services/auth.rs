use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // user_id
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthService {
    secret: String,
    expiry_hours: u64,
}

impl AuthService {
    pub fn new(secret: String, expiry_hours: u64) -> Self {
        Self { secret, expiry_hours }
    }

    pub fn generate_token(&self, user_id: &str, email: &str, role: &str) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.expiry_hours as i64);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            iat: now.timestamp() as usize,
            exp: exp.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )?;

        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(data.claims)
    }
}
