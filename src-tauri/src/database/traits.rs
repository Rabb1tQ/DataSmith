use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub db_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
    pub ssl: bool,
    pub connection_timeout: u64,
    pub pool_size: u32,
}

/// 数据库类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    SQLite,
    MongoDB,
    Redis,
    Elasticsearch,
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub affected_rows: u64,
    pub execution_time_ms: u128,
}

/// 数据库元数据 - 数据库信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

/// 数据库元数据 - 表信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub table_type: String,
    pub engine: Option<String>,
    pub rows: Option<u64>,
    pub size_mb: Option<f64>,
    pub comment: Option<String>,
}

/// 数据库元数据 - 列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub comment: Option<String>,
    pub character_maximum_length: Option<i64>,
    pub numeric_precision: Option<i64>,
    pub numeric_scale: Option<i64>,
}

/// 数据库元数据 - 索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: String,
}

/// 数据库操作结果
pub type DbResult<T> = Result<T, DbError>;

/// 数据库错误
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("查询执行失败: {0}")]
    QueryFailed(String),

    #[error("不支持的数据库类型")]
    UnsupportedDatabase,

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("其他错误: {0}")]
    Other(String),
}

impl Serialize for DbError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 数据库操作 Trait
#[async_trait]
pub trait DatabaseOperations: Send + Sync {
    /// 测试连接
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool>;

    /// 连接数据库
    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()>;

    /// 断开连接
    async fn disconnect(&mut self) -> DbResult<()>;

    /// 执行查询
    async fn execute_query(&self, sql: &str, database: Option<&str>) -> DbResult<QueryResult>;

    /// 获取数据库列表
    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>>;

    /// 获取表列表
    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>>;

    /// 获取表结构
    async fn get_table_structure(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<ColumnInfo>>;

    /// 获取索引信息
    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>>;
    
    /// 获取 Any 引用，用于向下转型
    fn as_any(&self) -> &dyn std::any::Any;
}

