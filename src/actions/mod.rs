use serde::{Deserialize, Serialize};

pub mod add_user;
pub mod new_lobby;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Netflix,
}
