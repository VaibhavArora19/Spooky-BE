use actix_web::{HttpResponse, post, web};
use mongodb::{bson::oid::ObjectId};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::{db::db::Lobby, AppState};

#[derive(Serialize, Deserialize)]
pub struct LobbyRequest {
    id: String,
    users: Vec<String>,
    platform: String,
}

#[post("/lobby/create")]
pub async fn create_new_lobby(
    req: web::Json<LobbyRequest>,
    app_state: web::Data<AppState>
) -> HttpResponse {
    let lobby_collection = app_state.db.collection::<Lobby>("lobbies");

    let user_ids: Vec<ObjectId> = req
        .users
        .iter()
        .map(|id_str| ObjectId::parse_str(id_str).unwrap())
        .collect();

    let lobby_details = Lobby {
        id: mongodb::bson::oid::ObjectId::parse_str(req.id.clone()).unwrap(),
        users: user_ids,
        platform: serde_json::to_string(&req.platform).unwrap(),
    };

    lobby_collection
        .insert_one(lobby_details.clone())
        .await
        .unwrap();

    let serialized_details = serde_json::to_string(&lobby_details).unwrap();

    app_state.redis.clone()
        .set::<String, String, ()>(lobby_details.id.clone().to_string(), serialized_details)
        .await
        .unwrap();

    log::info!(
        "New lobby created with ID: {}",
        lobby_details.id.to_string()
    );

    HttpResponse::Ok().body(req.id.clone())
}
