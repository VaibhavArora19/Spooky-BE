use actix_web::{HttpResponse, post, web};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{AppState, actions::Platform, db::db::Room};

#[derive(Serialize, Deserialize)]
pub struct RoomRequest {
    room_id: String,
    users: Vec<String>,
    platform: Platform,
}

#[post("/room/create")]
pub async fn create_new_room(
    req: web::Json<RoomRequest>,
    app_state: web::Data<AppState>,
) -> HttpResponse {
    let room_collection = app_state.db.collection::<Room>("rooms");

    let id = ObjectId::new();

    let user_ids: Vec<ObjectId> = req
        .users
        .iter()
        .map(|id_str| ObjectId::parse_str(id_str).unwrap())
        .collect();

    let room_details = Room {
        id,
        room_id: req.room_id.clone(),
        users: user_ids,
        messages: Vec::new(),
        platform: serde_json::to_string(&req.platform).unwrap(),
    };

    room_collection
        .insert_one(room_details.clone())
        .await
        .unwrap();

    log::info!(
        "New room created with ID: {}",
        room_details.room_id.to_string()
    );

    HttpResponse::Ok().body(req.room_id.clone())
}
