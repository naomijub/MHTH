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
