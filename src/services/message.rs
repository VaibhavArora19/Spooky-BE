use mongodb::{
    bson::{doc, to_bson}, Database
};
use serde::{Deserialize, Serialize};

use crate::db::db::{Message, Room, User};

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
