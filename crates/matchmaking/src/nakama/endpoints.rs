use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, DeserializeOwned},
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RpcResponse<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
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
    let s = String::deserialize(deserializer)?;

    // Step 2: parse the string into the desired type
    serde_json::from_str(&s).map_err(de::Error::custom)
}

pub const HEALTHCHECK_PATH: (reqwest::Method, &str) = (
    reqwest::Method::POST,
    "/v2/console/api/endpoints/rpc/healthcheck",
);

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

impl Default for AuthResponseBody {
    fn default() -> Self {
        Self {
            token: "token".to_string(),
        }
    }
}

pub const NEW_USER: (reqwest::Method, &str) = (reqwest::Method::POST, "/v2/console/user");

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CreateUserRequestBody {
    username: String,
    password: String,
    email: String,
    role: String,
    newsletter_subscription: bool,
}

impl Default for CreateUserRequestBody {
    fn default() -> Self {
        Self {
            username: Default::default(),
            password: Default::default(),
            email: "nakama.admin@mhth.net".to_string(),
            role: "USER_ROLE_ADMIN".to_string(),
            newsletter_subscription: false,
        }
    }
}

impl CreateUserRequestBody {
    pub fn new_admin(username: String, password: String) -> Self {
        Self {
            username,
            password,
            ..Default::default()
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

    #[test]
    pub fn new_admin() {
        let admin =
            CreateUserRequestBody::new_admin("username".to_string(), "password".to_string());
        let default = CreateUserRequestBody::default();

        assert_eq!(admin.username, "username");
        assert_eq!(admin.password, "password");
        assert_eq!(admin.email, default.email);
        assert_eq!(admin.role, default.role);
        assert!(!admin.newsletter_subscription && !default.newsletter_subscription);
    }
}
