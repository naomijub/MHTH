use serde::{Deserialize, Serialize};


pub const HEALTHCHECK_PATH: &str = "/v2/console/api/endpoints/rpc/healthcheck";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HealthcheckResponse {
    pub success: bool
}

pub const AUTH_PATH: &str = "/v2/console/authenticate";

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AuthRequestBody {
    pub username: String,
    pub password: String
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AuthResponseBody {
    pub token: String,
    #[serde(rename = "refreshToken", skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}