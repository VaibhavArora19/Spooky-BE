use mongodb::Database;

pub mod actions;
pub mod config;
pub mod db;
pub mod services;
pub mod ws_conn;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}
