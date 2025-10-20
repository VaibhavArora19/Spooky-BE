use anyhow::Error;
use lofi_party::{
    actions::{
        add_user::{add_new_user, create_new_user},
        new_lobby::create_new_lobby,
    },
    db::db::{Lobby, connect_to_db},
    redis::connect_to_redis,
    ws_conn::{self, ActionType, handle_connection},
};
use mongodb::bson::oid::ObjectId;
use ws_conn::WebsocketEvent;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    log::info!("Connecting to redis");

    let config = lofi_party::config::Config::get_config();

    log::info!("Connecting to DB...");
    let (db, _, _) = connect_to_db(config.mongodb_url).await?;
    log::info!("Connected to DB successfully");

    log::info!("Connecting to Redis...");
    let mut redis_connection = connect_to_redis(config.redis_url)
        .await
        .map_err(|_| anyhow::Error::msg("Failed to connect to redis"))?;
    log::info!("Connected to Redis successfully");

    log::info!("Starting Lofi Party server...");

    let server = ws_conn::create_websocket_connection().await?;

    while let Ok((stream, addr)) = server.accept().await {
        let message = handle_connection(stream, addr).await?;

        let websocket_event_details: WebsocketEvent =
            serde_json::from_str(message.to_text().unwrap())
                .map_err(|_| anyhow::Error::msg("Failed to parse message into websocket event"))
                .unwrap();

        match websocket_event_details.action {
            ActionType::NewLobbyCreated => {
                match websocket_event_details.payload {
                    ws_conn::EventPayload::CreateLobby(lobby_data) => {
                        let user_ids: Vec<ObjectId> = lobby_data
                            .users
                            .iter()
                            .map(|id_str| ObjectId::parse_str(id_str).unwrap())
                            .collect();

                        let lobby_details = Lobby {
                            id: mongodb::bson::oid::ObjectId::parse_str(lobby_data.lobby_id)
                                    .unwrap(),
                            users: user_ids,
                            platform: serde_json::to_string(&lobby_data.platform).unwrap(),
                        };

                        create_new_lobby(lobby_details, &mut redis_connection, db.clone()).await;

                        //* return success message here */
                    }
                    _ => {
                        log::info!("Unexpected payload");

                        //*Return error here */
                    }
                };
            }
            ActionType::UserJoined => {
                match websocket_event_details.payload {
                    ws_conn::EventPayload::UserJoined(user_data) => {
                        let user_info = add_new_user(
                            ObjectId::parse_str(user_data.lobby_id).unwrap(),
                            ObjectId::parse_str(user_data.user_id).unwrap(),
                            &mut redis_connection,
                            db.clone(),
                        )
                        .await?;

                    //*return user details here */
                    }
                    _ => {
                        log::info!("Unexpected payload");

                        //*Return error here */
                    }
                };
            },
            ActionType::UserCreated => {
                match websocket_event_details.payload {
                    ws_conn::EventPayload::CreateUser(user_data) => {
                        let user = create_new_user(user_data.name, user_data.avatar, &mut redis_connection, db.clone()).await?;

                        //*return user here */
                    }
                    _ => {
                        log::info!("Unexpected payload");

                        //*Return error here */
                    }
                };
            },
            _ => {}
        }
    }
    Ok(())
}
