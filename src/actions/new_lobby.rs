use mongodb::Database;
use redis::{aio::MultiplexedConnection, AsyncCommands};

use crate::db::db::Lobby;

//when a new lobby is created keep them at both places in redis and in db as well
//store the updatedAt data in redis as well
pub async fn create_new_lobby(lobby_details: Lobby, redis_conn: &mut MultiplexedConnection, db_conn: Database) {
    let lobby_collection = db_conn.collection("lobbies");

    lobby_collection.insert_one(lobby_details.clone()).await.unwrap();

    let serialized_details = serde_json::to_string(&lobby_details).unwrap();

    redis_conn.set::<String, String, ()>(lobby_details.id.clone().to_string(), serialized_details).await.unwrap();

    log::info!("New lobby created with ID: {}", lobby_details.id);
}
