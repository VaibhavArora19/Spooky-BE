use actix_web::{HttpResponse, post, web};
use mongodb::bson::oid::ObjectId;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::{AppState, db::db::Room};

#[derive(Serialize, Deserialize)]
pub struct RoomRequest {
    id: String,
    users: Vec<String>,
    platform: String,
}

#[post("/room/create")]
pub async fn create_new_room(
    req: web::Json<RoomRequest>,
    app_state: web::Data<AppState>,
) -> HttpResponse {
    let room_collection = app_state.db.collection::<Room>("rooms");

    let user_ids: Vec<ObjectId> = req
        .users
        .iter()
        .map(|id_str| ObjectId::parse_str(id_str).unwrap())
        .collect();

    let room_details = Room {
        id: mongodb::bson::oid::ObjectId::parse_str(req.id.clone()).unwrap(),
        users: user_ids,
        platform: serde_json::to_string(&req.platform).unwrap(),
    };

    room_collection
        .insert_one(room_details.clone())
        .await
        .unwrap();

    let serialized_details = serde_json::to_string(&room_details).unwrap();

    app_state
        .redis
        .clone()
        .set::<String, String, ()>(room_details.id.clone().to_string(), serialized_details)
        .await
        .unwrap();

    log::info!("New room created with ID: {}", room_details.id.to_string());

    HttpResponse::Ok().body(req.id.clone())
}
