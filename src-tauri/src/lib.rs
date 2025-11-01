pub mod commands;
pub mod database;
pub mod models;
pub mod utils;

use database::ConnectionManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 应用状态
pub struct AppState {
    pub connection_manager: Arc<Mutex<ConnectionManager>>,
}
