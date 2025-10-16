use names::Generator;
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisError};

use crate::db::db::User;


pub async fn connect_to_redis(redis_url: String) -> Result<MultiplexedConnection, RedisError> {
    let client = redis::Client::open(redis_url)?;
    let connection = client.get_multiplexed_async_connection().await?;

    Ok(connection)
}

//add name and attach it with a random number to create a username
pub async fn create_user(id: mongodb::bson::oid::ObjectId, user_name: String, user_avatar: String, redis_conn: &mut MultiplexedConnection) -> User {

        let mut name_generator = Generator::with_naming(names::Name::Numbered);
        let username = name_generator.next().unwrap();

        let user = User {
            id: id.clone(),
            username: username.clone(),
            name: user_name,
            avatar: user_avatar,
        };

        redis_conn.set_nx::<String, String, ()>(id.clone().to_string(), serde_json::to_string(&user).unwrap()).await.unwrap();

        user
}