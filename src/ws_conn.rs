use futures_util::stream::{SplitSink, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

use crate::config;

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
    pub time: u32,
    pub updated_at: u64,
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
) -> Result<(SplitSink<WebSocketStream<TcpStream>, Message>, Message), anyhow::Error> {
    println!("Incoming TCP connection from: {:?}", addr);

    // This handles the HTTP WebSocket upgrade automatically
    let ws_stream = accept_async(raw_stream).await.map_err(|e| {
        log::error!("WebSocket handshake error for {:?}: {:?}", addr, e);
        anyhow::Error::msg(format!("Error during the websocket handshake: {}", e))
    })?;

    log::info!("WebSocket connection established: {:?}", addr);

    let (outgoing, mut incoming) = ws_stream.split();

    let broadcast_message_option = incoming.next().await;

    if broadcast_message_option.is_none() {
        log::error!("No message received from client: {:?}", addr);
        return Err(anyhow::Error::msg("No message received from client"));
    }

    let broadcast_message = broadcast_message_option.unwrap().map_err(|e| {
        log::error!(
            "Failed to get broadcasted message. Failed with error: {:?}",
            e
        );
        anyhow::Error::msg("Failed to get broadcasted message")
    })?;

    log::info!("Received a message from {}: {:?}", addr, broadcast_message);

    Ok((outgoing, broadcast_message))
}
