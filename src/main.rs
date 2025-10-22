use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use anyhow::Error;
use lofi_party::{
    actions::add_user::add_new_user, db::db::connect_to_db, redis::connect_to_redis, services::{room::create_new_room, user::create_new_user}, ws_conn::{self, handle_connection, ActionType}, AppState
};
use mongodb::{bson::oid::ObjectId, Database};
use redis::aio::MultiplexedConnection;
use tokio::signal::{self};
use ws_conn::WebsocketEvent;


async fn run_websocket(mut redis_connection: MultiplexedConnection, db: Database) {
        let server = ws_conn::create_websocket_connection().await.unwrap();

        while let Ok((stream, addr)) = server.accept().await {
            let (_, message) = handle_connection(stream, addr).await.unwrap();
            // let websocket_event_details: WebsocketEvent =
            //     serde_json::from_str(message.to_text().expect("Failed to convert message to text")).expect("Failed to parse message");

            if let Ok(text) = message.to_text() {
                log::info!("Text: {}", text);
                match serde_json::from_str::<WebsocketEvent>(text) {
                    Ok(websocket_event_details) => {
                        if let ActionType::UserJoined = websocket_event_details.action {
                            if let ws_conn::EventPayload::UserJoined(user_data) =
                                websocket_event_details.payload
                            {
                                let _user_info = add_new_user(
                                    user_data.room_id,
                                    ObjectId::parse_str(&user_data.user_id).unwrap(),
                                    &mut redis_connection,
                                    db.clone(),
                                )
                                .await
                                .unwrap();
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to parse websocket event. Failed with error: {:?}", err);

                        continue;
                    }
                }
            } else {
                log::error!("Failed to convert message to text");
                continue;
            }
        }

}

async fn run_api(redis_connection: MultiplexedConnection, db: Database, http_port: String) -> Result<(), Error> {
        let db_clone = db.clone();
        let redis_clone = redis_connection.clone();

        HttpServer::new(move || {
            App::new()
                .wrap(
                    Cors::default()
                        .allow_any_origin()
                        .allow_any_header()
                        .allow_any_method(),
                )
                .app_data(web::Data::new(AppState {
                    db: db_clone.clone(),
                    redis: redis_clone.clone(),
                }))
                .service(create_new_user)
                .service(create_new_room)
        })
        .bind(("localhost", http_port.parse::<u16>().unwrap()))
        .unwrap()
        .run()
        .await
        .unwrap();

        Ok(())
    }

#[actix_web::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = lofi_party::config::Config::get_config();

    let (db, _, _) = connect_to_db(config.mongodb_url).await?;
    let (_, redis_connection) = connect_to_redis(config.redis_url).await?;

    // Spawn Actix HTTP server
    let http_db_clone = db.clone();
    let redis_connection_clone = redis_connection.clone();

    tokio::select! {
        result = run_api(redis_connection_clone, http_db_clone, config.http_port) => {
            if let Err(e) = result {
                log::error!("API server error: {}", e);
            }
        }
        _ = run_websocket(redis_connection, db) => {}
        _ = signal::ctrl_c() => {
            log::info!("Shutdown singal received. Stopping...");
        }

    }

    Ok(())
}
