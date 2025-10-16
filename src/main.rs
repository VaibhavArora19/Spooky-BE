use anyhow::Error;
use lofi_party::{actions::{add_user::add_new_user, new_lobby::create_new_lobby}, db::db::{connect_to_db, Lobby}, redis::connect_to_redis, ws_conn::{self, handle_connection, ActionType}};
use mongodb::bson::oid::ObjectId;
use ws_conn::WebSocketEvent;

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
    let mut redis_connection = connect_to_redis(config.redis_url).await.map_err(|_| anyhow::Error::msg("Failed to connect to redis"))?;
    log::info!("Connected to Redis successfully");

    log::info!("Starting Lofi Party server...");

    let server = ws_conn::create_websocket_connection().await?;

    while let Ok((stream, addr)) = server.accept().await {
        let message = handle_connection(stream, addr).await?;

        let websocket_event_details: WebSocketEvent = serde_json::from_str(message.to_text().unwrap()).map_err(|_| anyhow::Error::msg("Failed to parse message into websocket event")).unwrap();

        match websocket_event_details.action {
            ActionType::NewLobbyCreated => {
                let lobby_details = Lobby {
                    id: mongodb::bson::oid::ObjectId::parse_str(websocket_event_details.lobby_id).unwrap(),
                    users: vec![ObjectId::parse_str(websocket_event_details.user).unwrap()],
                    platform: serde_json::to_string(&websocket_event_details.platform).unwrap(), 
                };

                create_new_lobby(lobby_details, &mut redis_connection, db.clone()).await;
            },
            ActionType::UserJoined => {
                add_new_user(ObjectId::parse_str(websocket_event_details.lobby_id).unwrap(), ObjectId::parse_str(websocket_event_details.user).unwrap(), &mut redis_connection, db.clone()).await?;
            }
            _ => {}
        }
    }
    Ok(())
}
