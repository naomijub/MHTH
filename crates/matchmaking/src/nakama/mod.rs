use std::marker::PhantomData;

use skillratings::mhth::MhthRating;
use tracing::{debug, error};

use crate::nakama::{
    endpoints::{
        AuthRequestBody, AuthResponseBody, CreateUserRequestBody, HealthcheckResponse, RpcResponse, AUTH_PATH, HEALTHCHECK_PATH, NEW_USER
    },
    helpers::{
        get_env_endpoint, get_env_password, get_env_server_key_name, get_env_server_key_value,
        get_env_user, get_password,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NakamaClient<T = DefaultNakama> {
    /// NAKAMA_USERNAME
    username: String,
    password: String,
    token: Option<String>,
    /// NAKAMA_HOST
    url: String,
    /// NAKAMA_SERVER_KEY_NAME
    server_key_name: String,
    /// NAKAMA_SERVER_KEY
    server_key_value: String,
    _state: PhantomData<T>,
}

impl NakamaClient<DefaultNakama> {
    pub fn try_new() -> Result<NakamaClient<Unauthenticated>, Error> {
        let username = get_env_user();
        let url = get_env_endpoint();
        let server_key_name = get_env_server_key_name();
        let server_key_value = get_env_server_key_value();
        let env_password = get_env_password()?;
        let password = get_password(&env_password);

        Ok(NakamaClient {
            username,
            password,
            url,
            server_key_name,
            server_key_value,
            _state: PhantomData::<Unauthenticated>,
            token: None,
        })
    }
}

impl NakamaClient<NoUserRegistered> {
    pub async fn register_admin(self, http_client: &reqwest::Client) -> Result<NakamaClient<Unauthenticated>, Error> {
        let new_admin = CreateUserRequestBody::new_admin(self.username.clone(), self.password.clone());
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
            let _ = res.text().await
            // TODO: Remove
            .inspect(|body| debug!("Body Admin: {body:?}"))
            .inspect_err(|err| error!("Body Admin Err: {err:?}"));
        }
        
        Ok(
            NakamaClient {
                username: self.username,
                password: self.password,
                token: self.token,
                url: self.url,
                server_key_name: self.server_key_name,
                server_key_value: self.server_key_value,
                _state: PhantomData::<Unauthenticated>,
            }
        )
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
            _state: PhantomData::<Authenticated>,
        })
    }
}

impl NakamaClient<Authenticated> {
    pub async fn get_skill_rating(
        &self,
        http_client: &reqwest::Client,
        _player_id: &str,
    ) -> Result<MhthRating, Error> {
        let token = self
            .token
            .as_ref()
            .expect("Client is already authenticated");
        let response: RpcResponse<HealthcheckResponse> = http_client
            .request(HEALTHCHECK_PATH.0,format!("{}{}", self.url, HEALTHCHECK_PATH.1))
            .bearer_auth(token)
            .send()
            .await
            .inspect_err(|err| error!("{err:?}"))?
            .json()
            .await
            .inspect_err(|err| error!("{err:?}"))?;
        debug!("helthcheck: {}", response.body.success);

        Ok(MhthRating::default())
    }
}
