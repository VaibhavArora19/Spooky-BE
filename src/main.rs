use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use anyhow::Error;
use futures_util::SinkExt;
use lofi_party::{
    actions::add_user::add_new_user, db::db::{connect_to_db, Message}, services::{message::add_message, room::create_new_room, user::create_new_user}, ws_conn::{self, handle_connection, ActionType}, AppState
};
use mongodb::{Database, bson::oid::ObjectId};
use tokio::signal::{self};
use tokio_tungstenite::tungstenite::Message as TokioMessage;
use ws_conn::WebsocketEvent;

async fn run_websocket(db: Database) {
    let server = ws_conn::create_websocket_connection().await.unwrap();

    while let Ok((stream, addr)) = server.accept().await {
        let (mut outgoing, message) = handle_connection(stream, addr).await.unwrap();


        if let Ok(text) = message.to_text() {
            log::info!("Text: {}", text);
            match serde_json::from_str::<WebsocketEvent>(text) {
                Ok(websocket_event_details) => match websocket_event_details.action {
                    ActionType::UserJoined => {
                        if let ws_conn::EventPayload::UserJoined(user_data) =
                            websocket_event_details.payload
                        {
                            let _user_info = add_new_user(
                                user_data.room_id,
                                ObjectId::parse_str(&user_data.user_id).unwrap(),
                                db.clone(),
                            )
                            .await
                            .unwrap();
                        }
                    },
                    ActionType::Message => {
                        if let ws_conn::EventPayload::ChatMessage(message_data) = websocket_event_details.payload {
                            
                            let message = Message {
                                user_id: ObjectId::parse_str(&message_data.user_id).unwrap(),
                                message: message_data.message
                            };

                            match add_message(db.clone(), message_data.room_id, message).await {
                                Ok(result) => {
                                    if let Err(error) = outgoing.send(TokioMessage::Text(serde_json::to_string(&result).unwrap().into())).await {
                                        log::error!("Failed to send the response back. Failed with error: {:?}", error);
                                    }
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

    tokio::select! {
        result = run_api(http_db_clone, config.http_port) => {
            if let Err(e) = result {
                log::error!("API server error: {}", e);
            }
        }
        _ = run_websocket(db) => {}
        _ = signal::ctrl_c() => {
            log::info!("Shutdown singal received. Stopping...");
        }

    }

    Ok(())
}
