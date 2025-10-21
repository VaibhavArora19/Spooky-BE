use redis::{Client, RedisError, aio::MultiplexedConnection};

pub async fn connect_to_redis(
    redis_url: String,
) -> Result<(Client, MultiplexedConnection), RedisError> {
    let client = redis::Client::open(redis_url)?;
    let connection = client.get_multiplexed_async_connection().await?;

    Ok((client, connection))
}
