use mongodb::Database;
use mongodb::bson::doc;

use crate::db::db::Room;

pub async fn add_new_user(
    room_id: String,
    user: mongodb::bson::oid::ObjectId,
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

    log::info!("New user {} added to room with ID: {}", user, room_id);

    Ok(())
}
