use mongodb::Database;
use mongodb::bson::doc;
use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::db::db::Room;

pub async fn add_new_user(
    room_id: String,
    user: mongodb::bson::oid::ObjectId,
    redis_conn: &mut MultiplexedConnection,
    db_conn: Database,
) -> Result<(), anyhow::Error> {
    let room_collection = db_conn.collection::<Room>("rooms");

    if let Err(error) = room_collection
        .update_one(
            doc! { "room_id": room_id.clone() },
            doc! { "$addToSet": {"users": user.clone() }},
        )
        .await
    {
        log::error!(
            "Failed to insert user into room. Failed with error: {:?}",
            error
        );

        return Err(anyhow::Error::msg("Failed to insert user into room"));
    };

    let room_data: String = redis_conn.get(room_id.clone().to_string()).await.unwrap();
    let mut room: Room = serde_json::from_str(&room_data).unwrap();

    room.users.push(user.clone());

    let serialized_details = serde_json::to_string(&room).unwrap();

    redis_conn
        .set::<String, String, ()>(room_id.clone().to_string(), serialized_details)
        .await
        .unwrap();

    log::info!("New user {} added to room with ID: {}", user, room_id);

    Ok(())
}
