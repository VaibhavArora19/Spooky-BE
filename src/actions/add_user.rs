use mongodb::Database;
use mongodb::bson::doc;
use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::db::db::Lobby;

pub async fn add_new_user(
    lobby_id: mongodb::bson::oid::ObjectId,
    user: mongodb::bson::oid::ObjectId,
    redis_conn: &mut MultiplexedConnection,
    db_conn: Database,
) -> Result<(), anyhow::Error> {
    let lobby_collection = db_conn.collection::<Lobby>("lobbies");

    if let Err(error) = lobby_collection
        .update_one(
            doc! { "id": lobby_id.clone() },
            doc! { "$addToSet": {"users": user.clone() }},
        )
        .await
    {
        log::error!(
            "Failed to insert user into lobby. Failed with error: {:?}",
            error
        );

        return Err(anyhow::Error::msg("Failed to insert user into lobby"));
    };

    let lobby_data: String = redis_conn.get(lobby_id.clone().to_string()).await.unwrap();
    let mut lobby: Lobby = serde_json::from_str(&lobby_data).unwrap();

    lobby.users.push(user.clone());

    let serialized_details = serde_json::to_string(&lobby).unwrap();

    redis_conn
        .set::<String, String, ()>(lobby_id.clone().to_string(), serialized_details)
        .await
        .unwrap();

    log::info!("New user {} added to lobby with ID: {}", user, lobby_id);

    Ok(())
}
