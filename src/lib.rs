use std::{collections::HashMap, sync::Arc};

use futures_util::stream::SplitSink;
use mongodb::Database;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

use crate::ws_conn::SyncInfo;

pub mod actions;
pub mod config;
pub mod db;
pub mod services;
pub mod ws_conn;

pub type Tx = Arc<tokio::sync::RwLock<SplitSink<WebSocketStream<TcpStream>, Message>>>;
pub type RoomUserMap = Arc<tokio::sync::RwLock<HashMap<String, HashMap<String, Tx>>>>; //room -> user -> WebsocketStream
pub type RoomSync = Arc<tokio::sync::RwLock<HashMap<String, SyncInfo>>>; //should also delete ones which are last updated around a day ago

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub room_sync: RoomSync,
}
