use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use mongodb::Database;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::{tungstenite::Message as TokioMessage};

use crate::actions::add_user::add_new_user;
use crate::db::db::Message;
use crate::services::message::{add_message, broadcast_message};
use crate::services::video::set_sync_info;
use crate::{RoomSync, RoomUserMap, config, ws_conn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Play,
    Pause,
    Skip,
    UserJoined,
    UserLeft,
    Unknown,
    Message,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VideoAction {
    Play,
    Pause,
    Skip,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncInfo {
    pub last_action: VideoAction,
    pub time: f32,
    pub updated_at: f64,
    pub updated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketEvent {
    pub action: ActionType,
    pub room_id: Option<String>,
    pub user_id: Option<String>,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    UserJoined(UserJoinData),
    UserLeft,
    VideoAction(SyncInfo),
    ChatMessage(MessageData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserJoinData {
    pub user_id: String,
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub user_id: String,
    pub room_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebsocketResponseType {
    VideoAction,
    Message,
    UserJoined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketResponse<T> {
    pub response_type: WebsocketResponseType,
    pub data: T,
}

impl FromStr for ActionType {
    type Err = ();

    fn from_str(input: &str) -> Result<ActionType, Self::Err> {
        match input {
            "play" => Ok(ActionType::Play),
            "pause" => Ok(ActionType::Pause),
            "skip" => Ok(ActionType::Skip),
            "user_joined" => Ok(ActionType::UserJoined),
            "user_left" => Ok(ActionType::UserLeft),
            "message" => Ok(ActionType::Message),
            _ => Ok(ActionType::Unknown),
        }
    }
}

pub async fn create_websocket_connection() -> Result<TcpListener, anyhow::Error> {
    let port = config::Config::get_config().ws_port;

    log::info!("Starting WebSocket server on port: {}", port);

    let server = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| {
            log::error!(
                "Error creating a TCP Connection. Failed with error: {:?}",
                e
            );
            anyhow::Error::msg("Failed to connect to server")
        })?;

    Ok(server)
}

pub async fn handle_connection(
    raw_stream: TcpStream,
    addr: SocketAddr,
    db: Database,
    room_sync: RoomSync,
    room_users_collection: RoomUserMap,
) {
    println!("Incoming TCP connection from: {:?}", addr);

    // This handles the HTTP WebSocket upgrade automatically
    let ws_stream = accept_async(raw_stream)
        .await
        .map_err(|e| {
            log::error!("WebSocket handshake error for {:?}: {:?}", addr, e);
            anyhow::Error::msg(format!("Error during the websocket handshake: {}", e))
        })
        .expect("Failed to accept raw stream");

    log::info!("WebSocket connection established: {:?}", addr);

    let (outgoing, mut incoming) = ws_stream.split();
    let outgoing = Arc::new(RwLock::new(outgoing));

    while let Some(broadcast_message_option) = incoming.next().await {
        let message = broadcast_message_option
            .map_err(|e| {
                log::error!(
                    "Failed to get broadcasted message. Failed with error: {:?}",
                    e
                );
                anyhow::Error::msg("Failed to get broadcasted message")
            })
            .unwrap();

        log::info!("Received a message from {}: {:?}", addr, message);

        match message {
            TokioMessage::Text(text) => {
                let text = text.as_str();
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

                                let sync_status_write = room_sync.write().await;
                                let sync_status = sync_status_write.get(&user_data.room_id);

                                if let Some(status) = sync_status {
                                    let mut write_outgoing = outgoing.write().await;

                                    if let Err(error) = write_outgoing
                                        .send(TokioMessage::Text(
                                            serde_json::to_string(&WebsocketResponse {
                                                response_type:
                                                    ws_conn::WebsocketResponseType::UserJoined,
                                                data: status,
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await
                                    {
                                        log::error!(
                                            "Failed to sync updates to the joined user. Failed with error: {:?}",
                                            error
                                        );
                                    }

                                    let mut write_users_connection =
                                        room_users_collection.write().await;

                                    let room_map = write_users_connection
                                        .entry(user_data.room_id.clone())
                                        .or_insert(HashMap::new());
                                    room_map
                                        .insert(user_data.user_id, outgoing.clone());
                                }
                            }
                        }
                        ActionType::Message => {
                            if let ws_conn::EventPayload::ChatMessage(message_data) =
                                websocket_event_details.payload
                            {
                                let message = Message {
                                    user_id: ObjectId::parse_str(&message_data.user_id).unwrap(),
                                    message: message_data.message,
                                };

                                match add_message(db.clone(), message_data.room_id.clone(), message)
                                    .await
                                {
                                    Ok(result) => {
                                        broadcast_message(
                                            room_users_collection.clone(),
                                            message_data.room_id,
                                            tokio_tungstenite::tungstenite::Message::Text(
                                                serde_json::to_string(&WebsocketResponse {
                                                    response_type:
                                                        ws_conn::WebsocketResponseType::Message,
                                                    data: &result,
                                                })
                                                .unwrap()
                                                .into(),
                                            ),
                                        )
                                        .await;
                                    }
                                    Err(err) => {
                                        log::error!(
                                            "Failed to add message into the DB. Failed with error: {:?}",
                                            err
                                        )
                                    }
                                }
                            }
                        }
                        ActionType::Play | ActionType::Pause | ActionType::Skip => {
                            if let ws_conn::EventPayload::VideoAction(video_action_data) =
                                websocket_event_details.payload
                            {
                                let room_id =  match websocket_event_details.room_id {
                                    Some(id) => id,
                                    None => {
                                        log::error!("Error parsing room id.c");

                                        continue;
                                    }
                                };
                                let sync_info = SyncInfo {
                                    last_action: video_action_data.last_action,
                                    time: video_action_data.time,
                                    updated_at: video_action_data.updated_at,
                                    updated_by: video_action_data.updated_by,
                                };

                                set_sync_info(
                                    room_id.clone(),
                                    sync_info.clone(),
                                    room_sync.clone(),
                                )
                                .await;

                                broadcast_message(
                                    room_users_collection.clone(),
                                    room_id,
                                    TokioMessage::Text(
                                        serde_json::to_string(&WebsocketResponse {
                                            response_type:
                                                ws_conn::WebsocketResponseType::VideoAction,
                                            data: &sync_info.clone(),
                                        })
                                        .unwrap()
                                        .into(),
                                    ),
                                )
                                .await;
                            }
                        }
                        _ => {}
                    },
                    Err(err) => {
                        log::error!(
                            "Failed to parse websocket event. Failed with error: {:?}",
                            err
                        );
                    }
                }
            }
            TokioMessage::Close(close) => {
                log::info!("Connection closed: {:?}", close.unwrap())
            }
            _ => {
                log::info!("Something went wrong");
            }
        }
    }
}
