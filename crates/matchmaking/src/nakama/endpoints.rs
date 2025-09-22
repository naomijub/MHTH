use serde::{de::{self, DeserializeOwned}, Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RpcResponse<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize {
    #[serde(deserialize_with = "de_from_str")]
    pub body: T,
    pub error_message: String,
}

fn de_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    // Step 1: get the raw string
    let s = String::deserialize(deserializer)?.clone();

    // Step 2: parse the string into the desired type
    serde_json::from_str(&s).map_err(de::Error::custom)
}


pub const HEALTHCHECK_PATH: (reqwest::Method, &str) = (reqwest::Method::POST, "/v2/console/api/endpoints/rpc/healthcheck");

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HealthcheckResponse {
    pub success: bool,
}

pub const AUTH_PATH: (reqwest::Method, &str) = (reqwest::Method::POST, "/v2/console/authenticate");

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AuthRequestBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AuthResponseBody {
    pub token: String,
}

pub const NEW_USER: (reqwest::Method, &str) = (reqwest::Method::POST, "/v2/console/user");

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CreateUserRequestBody {
  username: String,
  password: String,
  email: String,
  role: String,
  newsletter_subscription: bool
}

impl Default for CreateUserRequestBody {
    fn default() -> Self {
        Self { 
            username: Default::default(), 
            password: Default::default(), 
            email: "nakama.admin@mhth.net".to_string(), 
            role: "USER_ROLE_ADMIN".to_string(), 
            newsletter_subscription: false
        }
    }
}

impl CreateUserRequestBody {
    pub fn new_admin(username: String, password: String) -> Self {
        Self {
            username, password, ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deser_rpc_response_as_healthcheck() {
        let rpc = "{\"body\": \"{\\\"success\\\": true}\", \"error_message\": \"error\"}";

        let resp: RpcResponse<HealthcheckResponse> = serde_json::from_str(rpc).unwrap();

        assert!(resp.body.success);
    }
}