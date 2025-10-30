use std::{collections::HashMap, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use anyhow::Error;
use lofi_party::{
    AppState, RoomSync, RoomUserMap,
    db::db::{connect_to_db},
    services::{
        room::create_new_room,
        user::create_new_user,
    },
    ws_conn::{self, handle_connection},
};

use mongodb::Database;
use tokio::{
    signal::{self},
    sync::RwLock,
};

async fn run_websocket(db: Database, room_users_collection: RoomUserMap, room_sync: RoomSync) {
    let server = ws_conn::create_websocket_connection().await.unwrap();

    while let Ok((stream, addr)) = server.accept().await {
        handle_connection(stream, addr, db.clone(), room_sync.clone(), room_users_collection.clone()).await;
    }
}

async fn run_api(db: Database, http_port: String, room_sync: RoomSync) -> Result<(), Error> {
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method(),
            )
            .app_data(web::Data::new(AppState {
                db: db.clone(),
                room_sync: room_sync.clone(),
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
    let room_sync: RoomSync = Arc::new(RwLock::new(HashMap::new()));

    tokio::select! {
        result = run_api(http_db_clone, config.http_port, room_sync.clone()) => {
            if let Err(e) = result {
                log::error!("API server error: {}", e);
            }
        }
        _ = run_websocket(db, users_connection, room_sync) => {}
        _ = signal::ctrl_c() => {
            log::info!("Shutdown singal received. Stopping...");
        }

    }

    Ok(())
}
