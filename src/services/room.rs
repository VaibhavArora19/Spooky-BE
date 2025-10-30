use actix_web::{HttpResponse, post, web};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
    AppState, SyncInfo, actions::Platform, db::db::Room, services::video::set_sync_info,
    ws_conn::VideoAction,
};

#[derive(Serialize, Deserialize)]
pub struct RoomRequest {
    room_id: String,
    users: Vec<String>,
    platform: Platform,
    time: f32,
    updated_at: f64,
    action: VideoAction,
    updated_by: String,
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

    let sync_info = SyncInfo {
        last_action: req.action.clone(),
        time: req.time,
        updated_at: req.updated_at,
        updated_by: req.updated_by.clone(),
    };

    set_sync_info(req.room_id.clone(), sync_info, app_state.room_sync.clone()).await;

    HttpResponse::Ok().body(req.room_id.clone())
}
