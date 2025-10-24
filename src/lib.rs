use std::{collections::HashMap, sync::Arc};

use futures_util::stream::SplitSink;
use mongodb::Database;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub mod actions;
pub mod config;
pub mod db;
pub mod services;
pub mod ws_conn;

pub type Tx = Arc<tokio::sync::RwLock<SplitSink<WebSocketStream<TcpStream>, Message>>>;
pub type RoomUserMap = Arc<tokio::sync::RwLock<HashMap<String, HashMap<String, Tx>>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}
