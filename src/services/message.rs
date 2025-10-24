use futures_util::SinkExt;
use mongodb::{
    bson::{doc, to_bson}, Database
};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message as TokioMessage;

use crate::db::db::{Message, Room, User};

use crate::RoomUserMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMessageResponse {
    pub user_id: String,
    pub username: String,
    pub name: String,
    pub avatar: String,
    pub message: String
}

pub async fn add_message(db: Database, room_id: String, message: Message) -> Result<AddMessageResponse, anyhow::Error> {
    let room_collection = db.collection::<Room>("rooms");
    let user_collection =  db.collection::<User>("users");

    match room_collection
        .update_one(
            doc! {"room_id": room_id},
            doc! {"$push": {"messages": to_bson(&message.clone()).unwrap() }},
            
        )
        .await
    {
        Ok(_) => {
            match user_collection.find_one(doc! {"_id": message.user_id}).await {
                Ok(result) => {
                    if let Some(user) = result {
                        Ok(AddMessageResponse {
                            user_id: message.user_id.to_string(),
                            username: user.username,
                            name: user.name,
                            avatar: user.avatar,
                            message: message.message
                        })
                    } else {
                        Err(anyhow::Error::msg("Failed to fetch user info"))
                    }
                },
                Err(err) => {
                    log::error!("Failed to fetch user info. Failed with error: {:?}", err);

                    Err(anyhow::Error::msg("Failed to fetch user info"))
                }
            }
        }
        Err(err) => {
            log::error!(
                "Failed to insert message into room collection. Failed with error: {:?}",
                err
            );

            Err(anyhow::Error::msg("Failed to insert message into room collection"))
        }
    }

}

pub async fn broadcast_message(room_users_collection: RoomUserMap, room_id: String, message: AddMessageResponse) {
    let read = room_users_collection.read().await;

    if let Some(users) = read.get(&room_id) {
        let broadcast_message = TokioMessage::Text(serde_json::to_string(&message).unwrap().into());

        for (user_id, tx) in users {
            let mut  write_tx = tx.write().await;

            if let Err(error) = write_tx.send(broadcast_message.clone()).await {
                log::error!("Failed to send message to user: {:}. Failed with error: {:?}", user_id, error)
            }
        }
    }
}