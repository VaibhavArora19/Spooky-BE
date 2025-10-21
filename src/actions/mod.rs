use serde::{Deserialize, Serialize};

pub mod add_user;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Netflix,
}
