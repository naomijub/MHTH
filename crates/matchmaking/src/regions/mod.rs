use redis::{AsyncTypedCommands, RedisError, aio::MultiplexedConnection};

pub const REGIONS_KEY: &str = "match:regions";

pub async fn set_regions(
    conn: MultiplexedConnection,
    regions: &[String],
) -> Result<(), RedisError> {
    let mut conn = conn.clone();

    let encode = bitcode::encode(regions);
    conn.set(REGIONS_KEY, encode).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use testcontainers::{
        ContainerAsync, GenericImage, ImageExt,
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
    };

    use super::*;

    #[tokio::test]
    async fn set_multiple_regions() {
        let container = create_redis(6379).await;
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let client = redis_client(host.to_string(), port).await;
        let conn = client.get_multiplexed_async_connection().await.unwrap();
        let regions = &[
            "CAN".to_string(),
            "US".to_string(),
            "SOUTH_AMERICA".to_string(),
        ];

        set_regions(conn.clone(), regions).await.unwrap();

        let encoded = conn.clone().get(REGIONS_KEY).await.unwrap().unwrap();
        container.pause().await.unwrap();

        let decoded: Vec<String> = bitcode::decode(encoded.as_bytes()).unwrap();

        assert_eq!(decoded, regions);
    }

    async fn redis_client(host: String, port: u16) -> redis::Client {
        redis::Client::open(format!("redis://{host}:{port}")).unwrap()
    }

    async fn create_redis(port: u16) -> ContainerAsync<GenericImage> {
        GenericImage::new("redis", "8.2.1-bookworm")
            .with_exposed_port(port.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_network("bridge")
            .with_env_var("REDIS_PASSWORD", "super-secret-password")
            .with_env_var("REDIS_USER", "redis_mms_admin")
            .start()
            .await
            .expect("Failed to start Redis")
    }
}
