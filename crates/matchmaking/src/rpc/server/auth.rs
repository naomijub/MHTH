use std::{
    collections::BTreeMap,
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use jwt::VerifyWithKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tonic::{Request, Status};
use tracing::error;

use crate::nakama::helpers::get_env_encryption_key;

static ENCRYPTION_KEY: LazyLock<String> = LazyLock::new(get_env_encryption_key);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    token_id: String,
    user_id: String,
    username: String,
    vars: BTreeMap<String, String>,
    expires_at: i64,
    issued_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserId {
    pub(crate) player_id: String,
}

pub fn check_auth(mut req: Request<()>) -> Result<Request<()>, Status> {
    match req.metadata().get("authorization") {
        Some(t) => {
            let key: Hmac<Sha256> = Hmac::new_from_slice(ENCRYPTION_KEY.as_bytes())
                .inspect_err(|err| error!("Encryption key: {err}"))
                .map_err(|_| Status::internal("Failed to verify token"))?;
            let token = t
                .to_str()
                .inspect_err(|err| error!("Failed to parse token as str: {err}"))
                .map_err(|_| Status::internal("Failed to verify token"))?;

            let claims: SessionClaims = VerifyWithKey::verify_with_key(token, &key)
                .inspect_err(|err| error!("Failed to verify token: {err}"))
                .map_err(|_| Status::internal("Failed to verify token"))?;

            let start = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");

            req.extensions_mut().insert(UserId {
                player_id: claims.user_id.clone(),
            });

            if claims.expires_at as u64 > start.as_secs() {
                Err(Status::unauthenticated("please refresh session token"))
            } else {
                Ok(req)
            }
        }
        _ => Err(Status::unauthenticated("No valid auth token")),
    }
}
