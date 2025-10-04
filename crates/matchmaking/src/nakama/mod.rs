use std::{marker::PhantomData, sync::Arc};

use skillratings::mhth::MhthRating;
use tracing::{debug, error};

use crate::nakama::{
    endpoints::{
        AUTH_PATH, AuthRequestBody, AuthResponseBody, CreateUserRequestBody, HEALTHCHECK_PATH,
        NEW_USER,
    },
    helpers::{
        get_env_encryption_key, get_env_endpoint, get_env_password, get_env_server_key_name,
        get_env_server_key_value, get_env_user, get_password,
    },
};

pub mod endpoints;
pub mod helpers;

const SALTING_KEY: &str = "fL@.P47H$P!fmcdc";

#[derive(Debug, Clone)]
pub struct DefaultNakama;
#[derive(Debug, Clone)]
pub struct NoUserRegistered;
#[derive(Debug, Clone)]
pub struct Authenticated;
#[derive(Debug, Clone)]
pub struct Unauthenticated;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(".env `NAKAMA_PASSWORD` not set")]
    PasswordEnvNotSet,
    #[error("request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NakamaClient<T = DefaultNakama> {
    /// NAKAMA_USERNAME
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) token: Option<String>,
    /// NAKAMA_HOST
    pub(crate) url: String,
    /// NAKAMA_SERVER_KEY_NAME
    pub(crate) server_key_name: String,
    /// NAKAMA_SERVER_KEY
    pub(crate) server_key_value: String,
    /// Session Encryption Key
    pub(crate) encryption_key: String,
    pub(crate) _state: PhantomData<T>,
}

impl NakamaClient<DefaultNakama> {
    pub fn try_new() -> Result<NakamaClient<Unauthenticated>, Error> {
        let username = get_env_user();
        let url = get_env_endpoint();
        let server_key_name = get_env_server_key_name();
        let server_key_value = get_env_server_key_value();
        let env_password = get_env_password()?;
        let password = get_password(&env_password);
        let encryption_key = get_env_encryption_key();

        Ok(NakamaClient {
            username,
            password,
            url,
            server_key_name,
            server_key_value,
            encryption_key,
            _state: PhantomData::<Unauthenticated>,
            token: None,
        })
    }
}

impl NakamaClient<NoUserRegistered> {
    pub async fn register_admin(
        self,
        http_client: &reqwest::Client,
    ) -> Result<NakamaClient<Unauthenticated>, Error> {
        let new_admin =
            CreateUserRequestBody::new_admin(self.username.clone(), self.password.clone());
        let body = serde_json::to_string(&new_admin)?;

        let res = http_client
            .request(NEW_USER.0, format!("{}{}", &self.url, NEW_USER.1))
            .body(body)
            .basic_auth(&self.server_key_name, Some(&self.server_key_value))
            .send()
            .await
            // TODO: Remove
            .inspect(|body| debug!("Req Admin: {body:?}"))
            .inspect_err(|err| error!("Req Admin Err: {err:?}"));

        if let Ok(res) = res {
            let _ = res
                .text()
                .await
                // TODO: Remove
                .inspect(|body| debug!("Body Admin: {body:?}"))
                .inspect_err(|err| error!("Body Admin Err: {err:?}"));
        }

        Ok(NakamaClient {
            username: self.username,
            password: self.password,
            token: self.token,
            url: self.url,
            server_key_name: self.server_key_name,
            server_key_value: self.server_key_value,
            encryption_key: self.encryption_key,
            _state: PhantomData::<Unauthenticated>,
        })
    }
}

impl NakamaClient<Unauthenticated> {
    pub async fn authenticate(
        self,
        http_client: &reqwest::Client,
    ) -> Result<NakamaClient<Authenticated>, Error> {
        let request = AuthRequestBody {
            username: "admin".to_string(),
            password: "password".to_string(),
        };
        let body = serde_json::to_string(&request)?;

        debug!("{} {}", AUTH_PATH.0, format!("{}{}", self.url, AUTH_PATH.1));
        let response: AuthResponseBody = http_client
            .request(AUTH_PATH.0, format!("{}{}", self.url, AUTH_PATH.1))
            .body(body)
            .basic_auth(&self.server_key_name, Some(&self.server_key_value))
            .send()
            .await
            .inspect_err(|err| error!("{err}"))?
            .json()
            .await
            .inspect_err(|err| error!("{err}"))?;

        Ok(NakamaClient {
            username: self.username,
            password: self.password,
            token: Some(response.token),
            url: self.url,
            server_key_name: self.server_key_name,
            server_key_value: self.server_key_value,
            encryption_key: self.encryption_key,
            _state: PhantomData::<Authenticated>,
        })
    }
}

impl NakamaClient<Authenticated> {
    pub async fn get_skill_rating(
        &self,
        http_client: Arc<reqwest::Client>,
        _player_id: &str,
    ) -> Result<MhthRating, Error> {
        let token = self
            .token
            .as_ref()
            .expect("Client is already authenticated");

        let response: endpoints::RpcResponse<endpoints::HealthcheckResponse> = http_client
            .request(
                HEALTHCHECK_PATH.0,
                format!("{}{}", self.url, HEALTHCHECK_PATH.1),
            )
            .bearer_auth(token)
            .send()
            .await
            .inspect_err(|err| error!("Request Error: {err:?}"))?
            .json()
            .await
            .inspect_err(|err| error!("Response Error: {err:?}"))?;
        debug!("helthcheck: {}", response.body.success);

        Ok(MhthRating::default())
    }
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn auth_nakama_client() {
        let server = MockServer::start_async().await;
        let port = server.address().port();
        dotenv::dotenv().unwrap();
        unsafe {
            std::env::set_var("NAKAMA_HOST", "127.0.0.1");
            std::env::set_var("NAKAMA_GRPC_PORT", "7349");
            std::env::set_var("NAKAMA_REST_PORT", "7350");
            std::env::set_var("NAKAMA_CONSOLE_PORT", port.to_string());
            std::env::set_var("NAKAMA_USERNAME", "admin");
            std::env::set_var("NAKAMA_PASSWORD", "password");
            std::env::set_var("NAKAMA_SERVER_KEY_NAME", "defaultkey");
            std::env::set_var("NAKAMA_SERVER_KEY", "server_key");
            std::env::set_var("NAKAMA_ENCRYPTION_KEY", "encryption");
        }

        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .host("127.0.0.1")
                    .port(port)
                    .path("/v2/console/authenticate")
                    .scheme("http")
                    .any_request();
                then.status(200)
                    .header("content-type", "application/json")
                    .json_body(json!({"token": "my-random-token"}));
            })
            .await;

        let client = NakamaClient::try_new().unwrap();
        let client = client.authenticate(&reqwest::Client::new()).await.unwrap();

        mock.assert_async().await;
        assert_eq!(client.token.unwrap(), "my-random-token");
    }

    #[tokio::test]
    async fn get_skill_rating_with_auth() {
        let server = MockServer::start_async().await;
        let port = server.address().port();
        let client = auth_client(port);

        let mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .host("127.0.0.1")
                    .port(port)
                    .path("/v2/console/api/endpoints/rpc/healthcheck")
                    .scheme("http")
                    .any_request();
                then.status(200)
                    .header("content-type", "application/json")
                    .json_body(json!({"body": "{\"success\": true}", "error_message": "error"}));
            })
            .await;
        let http_client = Arc::new(reqwest::Client::new());
        let rating = client
            .get_skill_rating(http_client, "player_id")
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(rating.rating, 25.);
    }

    pub fn auth_client(port: u16) -> NakamaClient<Authenticated> {
        NakamaClient {
            username: "username".to_string(),
            password: "password".to_string(),
            token: Some("super_random_token".to_string()),
            url: format!("http://127.0.0.1:{port}"),
            server_key_name: "defaultkey".to_string(),
            server_key_value: "server_key".to_string(),
            encryption_key: "encryption_key".to_string(),
            _state: PhantomData,
        }
    }
}
