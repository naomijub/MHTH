#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Failed to load .env: {0}")]
    DotenvError(#[from] dotenv::Error),
}

#[derive(Debug, Clone)]
pub struct InternalClients {
    pub redis: redis::Client,
    pub http_client: reqwest::Client,
}

impl InternalClients {
    pub fn try_from_env() -> Result<Self, Error> {
        dotenv::dotenv()?;
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let user = std::env::var("REDIS_USER").unwrap_or_else(|_| "root".to_string());
        let password = std::env::var("REDIS_PASSWORD").unwrap_or_else(|_| "password".to_string());
        let redis = match std::env::var("REDIS_URL") {
            Ok(url) => redis::Client::open(format!("redis://{user}:{password}@{url}:{port}"))?,
            Err(_) => redis::Client::open(format!("redis://{user}:{password}@localhost:{port}"))?,
        };
        let http_client = reqwest::Client::new();
        Ok(Self { redis, http_client })
    }

    pub async fn redis(&self) -> Result<redis::aio::MultiplexedConnection, Error> {
        Ok(self.redis.get_multiplexed_tokio_connection().await?)
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_load() {
        unsafe {
            std::env::set_var("REDIS_URL", "redis_mms");
            std::env::set_var("REDIS_PORT", "6379");
            std::env::set_var("REDIS_USER", "redis_mms_admin");
            std::env::set_var("REDIS_PASSWORD", "super_sercure");
        }
        assert!(InternalClients::try_from_env().is_ok())
    }
}
