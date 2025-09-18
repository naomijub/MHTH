#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Redis error: {0}")]
    RedisError(#[from]redis::RedisError),
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
        let redis = match std::env::var("REDIS_URL") {
            Ok(url) => redis::Client::open(url)?,
            Err(_) => redis::Client::open("redis://localhost")?,
        };
        let http_client = reqwest::Client::new();
        Ok(Self { redis, http_client })
    }

    pub async fn redis(&self) -> Result<redis::aio::MultiplexedConnection, Error> {
        Ok(self.redis.get_multiplexed_tokio_connection().await?)
    }
}