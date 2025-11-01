use serde::{Deserialize, Serialize};

/// 连接配置（用于存储和传输）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConnection {
    pub id: String,
    pub name: String,
    pub db_type: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// 加密后的密码
    pub encrypted_password: Option<String>,
    pub database: Option<String>,
    pub ssl: bool,
    pub connection_timeout: u64,
    pub pool_size: u32,
    pub group: Option<String>,
    pub color: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 连接测试结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub version: Option<String>,
    pub ping_time_ms: u128,
}

/// 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

