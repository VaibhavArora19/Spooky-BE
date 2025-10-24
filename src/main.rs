use std::{collections::HashMap, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use anyhow::Error;
use lofi_party::{
    actions::add_user::add_new_user, db::db::{connect_to_db, Message}, services::{message::{add_message, broadcast_message}, room::create_new_room, user::create_new_user}, ws_conn::{self, handle_connection, ActionType}, AppState, RoomUserMap
};
use mongodb::{Database, bson::oid::ObjectId};
use tokio::{signal::{self}, sync::RwLock};
use ws_conn::WebsocketEvent;


async fn run_websocket(db: Database, room_users_collection: RoomUserMap) {
    let server = ws_conn::create_websocket_connection().await.unwrap();

    while let Ok((stream, addr)) = server.accept().await {
        let (outgoing, message) = handle_connection(stream, addr).await.unwrap();


        if let Ok(text) = message.to_text() {
            log::info!("Text: {}", text);
            match serde_json::from_str::<WebsocketEvent>(text) {
                Ok(websocket_event_details) => match websocket_event_details.action {
                    ActionType::UserJoined => {
                        if let ws_conn::EventPayload::UserJoined(user_data) =
                            websocket_event_details.payload
                        {
                            let _user_info = add_new_user(
                                user_data.room_id.clone(),
                                ObjectId::parse_str(&user_data.user_id).unwrap(),
                                db.clone(),
                            )
                            .await
                            .unwrap();

                        let mut write_users_connection = room_users_collection.write().await;

                        let room_map = write_users_connection.entry(user_data.room_id).or_insert(HashMap::new());
                        room_map.insert(user_data.user_id, Arc::new(RwLock::new(outgoing)));
                        }
                    },
                    ActionType::Message => {
                        if let ws_conn::EventPayload::ChatMessage(message_data) = websocket_event_details.payload {
                            
                            let message = Message {
                                user_id: ObjectId::parse_str(&message_data.user_id).unwrap(),
                                message: message_data.message
                            };

                            match add_message(db.clone(), message_data.room_id.clone(), message).await {
                                Ok(result) => {
                                    broadcast_message(room_users_collection.clone(), message_data.room_id, result).await;                                

                                },
                                Err(err) => {
                                    log::error!("Failed to add message into the DB. Failed with error: {:?}", err)
                                }
                            }
                        }   
                    }
                    _ => {}
                },
                Err(err) => {
                    log::error!(
                        "Failed to parse websocket event. Failed with error: {:?}",
                        err
                    );

                    continue;
                }
            }
        } else {
            log::error!("Failed to convert message to text");
            continue;
        }
    }
}

async fn run_api(db: Database, http_port: String) -> Result<(), Error> {
    let db_clone = db.clone();

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

    // Spawn Actix HTTP server
    let http_db_clone = db.clone();

    let users_connection: RoomUserMap = Arc::new(RwLock::new(HashMap::new()));

    tokio::select! {
        result = run_api(http_db_clone, config.http_port) => {
            if let Err(e) = result {
                log::error!("API server error: {}", e);
            }
        }
        _ = run_websocket(db, users_connection) => {}
        _ = signal::ctrl_c() => {
            log::info!("Shutdown singal received. Stopping...");
        }

    }

    Ok(())
}
