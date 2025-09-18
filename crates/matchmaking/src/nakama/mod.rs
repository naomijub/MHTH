use std::marker::PhantomData;

use skillratings::mhth::MhthRating;
use tracing::debug;
use crate::nakama::endpoints::{AuthRequestBody, AuthResponseBody, HealthcheckResponse, HEALTHCHECK_PATH, AUTH_PATH};

use crate::nakama::helpers::{get_env_endpoint, get_env_password, get_env_server_key_name, get_env_server_key_value, get_env_user, get_passord};

pub mod helpers;
pub mod endpoints;

const SALTING_KEY: &str = "fL@.P47H$P!fmcdc";

pub struct Authenticated;
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NakamaClient<T> {
    /// NAKAMA_USERNAME
    username: String,
    password: String,
    token: Option<String>,
    refresh_token: Option<String>,
    /// NAKAMA_ENDPOINT
    url: String,
    /// NAKAMA_SERVER_KEY_NAME
    server_key_name: String,
    /// NAKAMA_SERVER_KEY
    server_key_value: String,
    _state: PhantomData<T>,
}

impl<T> NakamaClient<T> {
    pub fn try_new() -> Result<NakamaClient<Unauthenticated>, Error> {
        let username = get_env_user();
        let url = get_env_endpoint();
        let server_key_name = get_env_server_key_name();
        let server_key_value = get_env_server_key_value();
        let env_password = get_env_password()?;
        let password = get_passord(&env_password);

        Ok(
            NakamaClient { 
                username, 
                password,
                url, 
                server_key_name, 
                server_key_value,
                _state: PhantomData::<Unauthenticated>,
                token: None,
                refresh_token: None,
                
            }
        )
    }
}

impl NakamaClient<Unauthenticated> {
    pub async fn authenticate(self, http_client: &reqwest::Client) -> Result<NakamaClient<Authenticated>, Error> {
        let request = AuthRequestBody {
            username: self.username.clone(),
            password: self.password.clone(),
        };
        let body = serde_json::to_string(&request)?;
    
        let response: AuthResponseBody  = http_client.post(format!("{}{AUTH_PATH}", self.url))
            .body(body)
            .basic_auth(&self.server_key_name, Some(&self.server_key_value))
            .send()
            .await?
            .json()
            .await?;

        Ok(NakamaClient { 
            username: self.username,
             password: self.password,
             token: Some(response.token),
             refresh_token: response.refresh_token,
             url: self.url,
             server_key_name: self.server_key_name,
             server_key_value: self.server_key_value,
             _state: PhantomData::<Authenticated> 
            })
    }
}

impl NakamaClient<Authenticated> {
    pub async fn get_skill_rating(&self, http_client: &reqwest::Client, _player_id: &str) -> Result<MhthRating, Error> {
        let token = self.token.as_ref().expect("Client is already authenticated");
        let response: HealthcheckResponse  = http_client.post(format!("{}{HEALTHCHECK_PATH}", self.url))
            .bearer_auth(token.strip_prefix("=").unwrap_or(token))
            .send()
            .await?
            .json()
            .await?;
        debug!("helthcheck: {}", response.success);

        Ok(MhthRating::default())
    }
}
