use mongodb::bson::doc;
use mongodb::{Database, bson::oid::ObjectId};
use names::Generator;
use redis::{AsyncCommands, aio::MultiplexedConnection};

use crate::db::db::{Lobby, User};

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

pub async fn create_new_user(
    user_name: String,
    user_avatar: String,
    redis_conn: &mut MultiplexedConnection,
    db_conn: Database,
) -> Result<User, anyhow::Error> {
    let mut name_generator = Generator::with_naming(names::Name::Numbered);
    let username = name_generator.next().unwrap();

    let user_collection = db_conn.collection::<User>("users");

    let user_id = ObjectId::new();

    let user = User {
        id: Some(user_id.clone()),
        username: username.clone(),
        name: user_name.clone(),
        avatar: user_avatar.clone(),
    };

    match user_collection.insert_one(user.clone()).await {
        Ok(_) => {
            redis_conn
                .set_nx::<String, String, ()>(
                    user_id.clone().to_string(),
                    serde_json::to_string(&user).unwrap(),
                )
                .await
                .unwrap();

            Ok(User {
                id: Some(user_id),
                username: username,
                name: user_name,
                avatar: user_avatar,
            })
        }
        Err(error) => {
            log::error!(
                "Failed to insert user into the DB. Failed with error: {:?}",
                error
            );
            Err(anyhow::Error::msg("Failed to insert user into the DB"))
        }
    }
}
