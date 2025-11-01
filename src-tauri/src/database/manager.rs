use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::traits::*;
#[cfg(feature = "mysql")]
use super::mysql::MySqlDatabase;

/// 数据库连接管理器
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>,
    // 存储连接ID到数据库类型的映射
    connection_types: Arc<RwLock<HashMap<String, DatabaseType>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_types: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建数据库实例
    pub async fn create_connection(
        &self,
        config: ConnectionConfig,
    ) -> DbResult<String> {
        let connection_id = config.id.clone();
        
        let mut db: Box<dyn DatabaseOperations> = match config.db_type {
            #[cfg(feature = "mysql")]
            DatabaseType::MySQL => Box::new(MySqlDatabase::new()),
            
            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => {
                Box::new(super::postgresql::PostgreSqlDatabase::new())
            }
            
            #[cfg(feature = "sqlite")]
            DatabaseType::SQLite => {
                Box::new(super::sqlite::SqliteDatabase::new())
            }
            
            #[cfg(feature = "mongodb-support")]
            DatabaseType::MongoDB => {
                Box::new(super::mongodb::MongoDatabase::new())
            }
            
            #[cfg(feature = "redis-support")]
            DatabaseType::Redis => {
                Box::new(super::redis::RedisDatabase::new())
            }
            
            _ => return Err(DbError::UnsupportedDatabase),
        };

        // 保存数据库类型
        let db_type = config.db_type.clone();
        
        // 连接数据库
        db.connect(config).await?;

        // 存储连接
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), db);
        
        // 存储数据库类型
        let mut connection_types = self.connection_types.write().await;
        connection_types.insert(connection_id.clone(), db_type);

        Ok(connection_id)
    }

    /// 获取连接
    pub async fn get_connection(
        &self,
        connection_id: &str,
    ) -> DbResult<Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>> {
        let connections = self.connections.read().await;
        if connections.contains_key(connection_id) {
            Ok(self.connections.clone())
        } else {
            Err(DbError::ConnectionFailed("连接不存在".to_string()))
        }
    }

    /// 测试连接
    pub async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let db: Box<dyn DatabaseOperations> = match config.db_type {
            #[cfg(feature = "mysql")]
            DatabaseType::MySQL => Box::new(MySqlDatabase::new()),
            
            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => {
                Box::new(super::postgresql::PostgreSqlDatabase::new())
            }
            
            #[cfg(feature = "sqlite")]
            DatabaseType::SQLite => {
                Box::new(super::sqlite::SqliteDatabase::new())
            }
            
            #[cfg(feature = "mongodb-support")]
            DatabaseType::MongoDB => {
                Box::new(super::mongodb::MongoDatabase::new())
            }
            
            #[cfg(feature = "redis-support")]
            DatabaseType::Redis => {
                Box::new(super::redis::RedisDatabase::new())
            }
            
            _ => return Err(DbError::UnsupportedDatabase),
        };

        db.test_connection(config).await
    }

    /// 断开连接
    pub async fn disconnect(&self, connection_id: &str) -> DbResult<()> {
        let mut connections = self.connections.write().await;
        if let Some(mut db) = connections.remove(connection_id) {
            db.disconnect().await?;
        }
        
        // 同时删除数据库类型映射
        let mut connection_types = self.connection_types.write().await;
        connection_types.remove(connection_id);
        
        Ok(())
    }

    /// 执行查询
    pub async fn execute_query(
        &self,
        connection_id: &str,
        sql: &str,
        database: Option<&str>,
    ) -> DbResult<QueryResult> {
        let connections = self.connections.read().await;
        let db = connections
            .get(connection_id)
            .ok_or_else(|| DbError::ConnectionFailed("连接不存在".to_string()))?;
        
        db.execute_query(sql, database).await
    }

    /// 获取数据库列表
    pub async fn get_databases(&self, connection_id: &str) -> DbResult<Vec<DatabaseInfo>> {
        let connections = self.connections.read().await;
        let db = connections
            .get(connection_id)
            .ok_or_else(|| DbError::ConnectionFailed("连接不存在".to_string()))?;
        
        db.get_databases().await
    }

    /// 获取表列表
    pub async fn get_tables(
        &self,
        connection_id: &str,
        database: Option<&str>,
    ) -> DbResult<Vec<TableInfo>> {
        let connections = self.connections.read().await;
        let db = connections
            .get(connection_id)
            .ok_or_else(|| DbError::ConnectionFailed("连接不存在".to_string()))?;
        
        db.get_tables(database).await
    }

    /// 获取表结构
    pub async fn get_table_structure(
        &self,
        connection_id: &str,
        table: &str,
        schema: Option<&str>,
    ) -> DbResult<Vec<ColumnInfo>> {
        let connections = self.connections.read().await;
        let db = connections
            .get(connection_id)
            .ok_or_else(|| DbError::ConnectionFailed("连接不存在".to_string()))?;
        
        db.get_table_structure(table, schema).await
    }
    
    /// 获取连接的数据库类型
    pub async fn get_database_type(&self, connection_id: &str) -> DbResult<DatabaseType> {
        let connection_types = self.connection_types.read().await;
        connection_types
            .get(connection_id)
            .cloned()
            .ok_or_else(|| DbError::ConnectionFailed("连接不存在".to_string()))
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

