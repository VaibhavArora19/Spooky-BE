use anyhow::Error;
use mongodb::{Client, Collection, Database, IndexModel, options::IndexOptions};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    pub id: Option<mongodb::bson::oid::ObjectId>,
    pub username: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub user_id: mongodb::bson::oid::ObjectId,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Room {
    #[serde(rename = "_id")]
    pub id: mongodb::bson::oid::ObjectId,
    pub room_id: String,
    pub users: Vec<mongodb::bson::oid::ObjectId>,
    pub messages: Vec<Message>,
    pub platform: String,
}

pub async fn connect_to_db(
    mongodb_url: String,
) -> Result<(Database, Collection<User>, Collection<Room>), Error> {
    let client = Client::with_uri_str(mongodb_url.as_str()).await?;

    let db = client.database("lofi-party");

    let users = db.collection::<User>("users");
    let rooms = db.collection::<Room>("rooms");

    let options = IndexOptions::builder().unique(true).build();

    let user_model = IndexModel::builder()
        .keys(mongodb::bson::doc! { "username": 1 })
        .options(options.clone())
        .build();

    let room_model = IndexModel::builder()
        .keys(mongodb::bson::doc! { "room_id": 1 })
        .options(options)
        .build();

    if let Err(err) = users.create_index(user_model).await {
        log::error!("Failed to create index on user. Failed with err: {:?}", err);

        return Err(anyhow::Error::msg("Failed to create index on user"));
    };

    if let Err(err) = rooms.create_index(room_model).await {
        log::error!("Failed to create index on room. Failed with err: {:?}", err);

        return Err(anyhow::Error::msg("Failed to create index on room"));
    };

    Ok((db, users, rooms))
}
