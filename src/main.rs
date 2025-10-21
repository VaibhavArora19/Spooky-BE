use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use anyhow::Error;
use lofi_party::{
    actions::add_user::add_new_user,
    db::db::connect_to_db,
    redis::connect_to_redis,
    services::{lobby::create_new_lobby, user::create_new_user},
    ws_conn::{self, handle_connection, ActionType}, AppState,
};
use mongodb::{bson::oid::ObjectId};
use ws_conn::WebsocketEvent;

#[actix_web::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = lofi_party::config::Config::get_config();

    let (db, _, _) = connect_to_db(config.mongodb_url).await?;
    let (_, mut redis_connection) = connect_to_redis(config.redis_url).await?;

    // Spawn Actix HTTP server
    let http_db_clone = db.clone();
    let redis_connection_clone = redis_connection.clone();

    let http_server = tokio::spawn(async move {
        let db_clone = http_db_clone.clone();
        let redis_clone = redis_connection_clone.clone();

        HttpServer::new(move || {
            App::new()
            .wrap(Cors::default().allow_any_origin().allow_any_header().allow_any_method())
                .app_data(web::Data::new(AppState {
                    db: db_clone.clone(),
                    redis: redis_clone.clone(),
                }))
                .service(create_new_user)
                .service(create_new_lobby)
        })
        .bind(("localhost", config.http_port.parse::<u16>().unwrap()))
        .unwrap()
        .run()
        .await
        .unwrap();
    });

    // Spawn WebSocket server concurrently
    let ws_server = tokio::spawn(async move {
        let server = ws_conn::create_websocket_connection().await.unwrap();

        while let Ok((stream, addr)) = server.accept().await {
            let (_, message) = handle_connection(stream, addr).await.unwrap();
            let websocket_event_details: WebsocketEvent =
                serde_json::from_str(message.to_text().unwrap()).unwrap();

            if let ActionType::UserJoined = websocket_event_details.action {
                if let ws_conn::EventPayload::UserJoined(user_data) =
                    websocket_event_details.payload
                {
                    let _user_info = add_new_user(
                        ObjectId::parse_str(&user_data.lobby_id).unwrap(),
                        ObjectId::parse_str(&user_data.user_id).unwrap(),
                        &mut redis_connection,
                        db.clone(),
                    )
                    .await
                    .unwrap();
                }
            }
        }
    });

    // Wait for both to run (this will block until either task exits)
    tokio::try_join!(http_server, ws_server)?;

    Ok(())
}
