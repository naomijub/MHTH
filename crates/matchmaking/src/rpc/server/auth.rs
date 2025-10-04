use std::{
    collections::BTreeMap,
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use jwt::{Header, Token, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tonic::{Request, Status};
use tracing::error;

use crate::nakama::helpers::get_env_encryption_key;

static ENCRYPTION_KEY: LazyLock<String> = LazyLock::new(get_env_encryption_key);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub(super) token_id: String,
    pub(super) user_id: String,
    pub(super) username: String,
    pub(super) vars: BTreeMap<String, String>,
    pub(super) expires_at: i64,
    pub(super) issued_at: i64,
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

            let token: Token<Header, SessionClaims, _> =
                VerifyWithKey::verify_with_key(token, &key)
                    .inspect_err(|err| error!("Failed to verify token: {err:?}"))
                    .map_err(|_| Status::internal("Failed to verify token"))?;

            let start = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");

            let (_, claims) = token.into();
            req.extensions_mut().insert(UserId {
                player_id: claims.user_id.clone(),
            });

            if start.as_secs() > claims.expires_at as u64 {
                Err(Status::unauthenticated("please refresh session token"))
            } else {
                Ok(req)
            }
        }
        _ => Err(Status::unauthenticated("No valid auth token")),
    }
}

#[cfg(test)]
mod tests {
    use jwt::{Header, SignWithKey, Token};

    use super::*;

    #[test]
    fn happy_path_request() {
        let mut req = Request::new(());
        assert!(req.extensions().is_empty());
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 100;
        let claims = SessionClaims {
            token_id: "token_id".to_string(),
            user_id: "player_id".to_string(),
            username: "username".to_string(),
            vars: Default::default(),
            expires_at: exp as i64,
            issued_at: 0,
        };
        let key: Hmac<Sha256> = Hmac::new_from_slice(ENCRYPTION_KEY.as_bytes()).unwrap();
        let header = Header::default();
        let token = Token::new(header, claims).sign_with_key(&key).unwrap();
        let meta = req.metadata_mut();
        meta.insert("authorization", token.as_str().parse().unwrap());

        let req = check_auth(req).unwrap();

        assert_eq!(
            req.extensions().get::<UserId>().unwrap().player_id,
            "player_id"
        );
    }

    #[test]
    fn wrong_key() {
        let mut req = Request::new(());
        assert!(req.extensions().is_empty());
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            - 100;
        let claims = SessionClaims {
            token_id: "token_id".to_string(),
            user_id: "player_id".to_string(),
            username: "username".to_string(),
            vars: Default::default(),
            expires_at: exp as i64,
            issued_at: 0,
        };
        let key: Hmac<Sha256> = Hmac::new_from_slice(b"not-an-encryption-key").unwrap();
        let header = Header::default();
        let token = Token::new(header, claims).sign_with_key(&key).unwrap();
        let meta = req.metadata_mut();
        meta.insert("authorization", token.as_str().parse().unwrap());

        let err = check_auth(req).unwrap_err();

        assert_eq!(err.message(), "Failed to verify token");
    }

    #[test]
    fn missing_auth() {
        let mut req = Request::new(());
        assert!(req.extensions().is_empty());
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            - 100;
        let claims = SessionClaims {
            token_id: "token_id".to_string(),
            user_id: "player_id".to_string(),
            username: "username".to_string(),
            vars: Default::default(),
            expires_at: exp as i64,
            issued_at: 0,
        };
        let key: Hmac<Sha256> = Hmac::new_from_slice(b"not-an-encryption-key").unwrap();
        let header = Header::default();
        let token = Token::new(header, claims).sign_with_key(&key).unwrap();
        let meta = req.metadata_mut();
        meta.insert("other-meta-key", token.as_str().parse().unwrap());

        let err = check_auth(req).unwrap_err();

        assert_eq!(err.message(), "No valid auth token");
    }

    #[test]
    fn expired_token() {
        let mut req = Request::new(());
        assert!(req.extensions().is_empty());
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            - 100;
        let claims = SessionClaims {
            token_id: "token_id".to_string(),
            user_id: "player_id".to_string(),
            username: "username".to_string(),
            vars: Default::default(),
            expires_at: exp as i64,
            issued_at: 0,
        };
        let key: Hmac<Sha256> = Hmac::new_from_slice(ENCRYPTION_KEY.as_bytes()).unwrap();
        let header = Header::default();
        let token = Token::new(header, claims).sign_with_key(&key).unwrap();
        let meta = req.metadata_mut();
        meta.insert("authorization", token.as_str().parse().unwrap());

        let req = check_auth(req).unwrap_err();

        assert_eq!(req.message(), "please refresh session token");
    }
}
