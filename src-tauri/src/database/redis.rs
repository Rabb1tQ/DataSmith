use super::traits::*;
use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, Client};

/// Redis 数据库连接
pub struct RedisDatabase {
    config: Option<ConnectionConfig>,
    connection: Option<MultiplexedConnection>,
}

impl RedisDatabase {
    pub fn new() -> Self {
        Self { 
            config: None,
            connection: None,
        }
    }

    /// 构建 Redis 连接 URL
    fn build_connection_url(config: &ConnectionConfig) -> String {
        let password = if !config.password.is_empty() {
            if !config.username.is_empty() {
                // Redis 6.0+ ACL 格式: username:password@
                format!("{}:{}@", config.username, config.password)
            } else {
                // 传统密码格式: :password@
                format!(":{}@", config.password)
            }
        } else {
            String::new()
        };

        let protocol = if config.ssl { "rediss" } else { "redis" };
        let database = config.database.as_ref()
            .and_then(|db| db.parse::<u8>().ok())
            .map(|db| format!("/{}", db))
            .unwrap_or_default();

        format!("{}://{}{}:{}{}", protocol, password, config.host, config.port, database)
    }
}

#[async_trait]
impl DatabaseOperations for RedisDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let url = Self::build_connection_url(config);
        
        // 创建客户端
        let client = Client::open(url)
            .map_err(|e| DbError::ConnectionFailed(format!("创建 Redis 客户端失败: {}", e)))?;
        
        // 获取连接
        let mut conn = client.get_multiplexed_tokio_connection()
            .await
            .map_err(|e| DbError::ConnectionFailed(format!("连接 Redis 失败: {}", e)))?;
        
        // 发送 PING 命令测试连接
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::ConnectionFailed(format!("PING 命令失败: {}", e)))?;
        
        Ok(true)
    }

    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()> {
        let url = Self::build_connection_url(&config);
        
        // 创建客户端
        let client = Client::open(url)
            .map_err(|e| DbError::ConnectionFailed(format!("创建 Redis 客户端失败: {}", e)))?;
        
        // 获取连接
        let conn = client.get_multiplexed_tokio_connection()
            .await
            .map_err(|e| DbError::ConnectionFailed(format!("连接 Redis 失败: {}", e)))?;
        
        self.connection = Some(conn);
        self.config = Some(config);
        
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.connection = None;
        self.config = None;
        Ok(())
    }

    async fn execute_query(&self, _sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        Err(DbError::Other("Redis 不支持 SQL 查询，请使用 Redis 命令".to_string()))
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        // Redis 默认有 16 个数据库 (DB0-DB15)
        let databases: Vec<DatabaseInfo> = (0..16)
            .map(|i| DatabaseInfo {
                name: format!("db{}", i),
                charset: None,
                collation: None,
            })
            .collect();
        
        Ok(databases)
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        // 克隆连接以供使用（MultiplexedConnection 实现了 Clone）
        let mut conn = conn.clone();
        
        // 如果指定了数据库，先切换到对应的数据库
        if let Some(db_str) = database {
            if let Ok(db_num) = db_str.parse::<i64>() {
                let _: String = redis::cmd("SELECT")
                    .arg(db_num)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("切换数据库失败: {}", e)))?;
            }
        }
        
        // 使用 SCAN 命令获取键列表（更安全，不会阻塞）
        // 这里我们获取键的统计信息，按模式分组
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("*")
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取键列表失败: {}", e)))?;
        
        // 按键的前缀分组统计
        let mut key_groups: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        
        for key in &keys {
            // 提取键的前缀（以 : 分隔）
            let prefix = if let Some(pos) = key.find(':') {
                &key[..pos]
            } else {
                "(无前缀)"
            };
            
            *key_groups.entry(prefix.to_string()).or_insert(0) += 1;
        }
        
        // 转换为 TableInfo 格式
        let mut tables: Vec<TableInfo> = key_groups
            .into_iter()
            .map(|(prefix, count)| TableInfo {
                name: if prefix == "(无前缀)" {
                    format!("{} ({} 个键)", prefix, count)
                } else {
                    format!("{}:* ({} 个键)", prefix, count)
                },
                schema: database.map(|s| s.to_string()),
                table_type: "KEY_GROUP".to_string(),
                engine: Some("Redis".to_string()),
                rows: Some(count),
                size_mb: None,
                comment: Some(format!("键前缀分组，共 {} 个键", count)),
            })
            .collect();
        
        // 按键数量降序排序
        tables.sort_by(|a, b| b.rows.unwrap_or(0).cmp(&a.rows.unwrap_or(0)));
        
        // 如果键总数不多（比如少于 20 个），也可以直接显示所有键
        if keys.len() <= 20 && keys.len() > 0 {
            tables.clear();
            for key in keys {
                // 获取键的类型
                let key_type: String = redis::cmd("TYPE")
                    .arg(&key)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or_else(|_| "unknown".to_string());
                
                tables.push(TableInfo {
                    name: key.clone(),
                    schema: database.map(|s| s.to_string()),
                    table_type: key_type.to_uppercase(),
                    engine: Some("Redis".to_string()),
                    rows: Some(1),
                    size_mb: None,
                    comment: None,
                });
            }
        }
        
        Ok(tables)
    }

    async fn get_table_structure(&self, _table: &str, _schema: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        // Redis 键值存储没有表结构
        Ok(vec![])
    }

    async fn get_indexes(&self, _table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        // Redis 没有索引概念
        Ok(vec![])
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl RedisDatabase {
    /// 执行 Redis 命令
    pub async fn execute_command(&self, command: &str, args: Vec<String>) -> DbResult<redis::Value> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        let mut conn = conn.clone();
        
        let mut cmd = redis::cmd(command);
        for arg in args {
            cmd.arg(arg);
        }
        
        cmd.query_async(&mut conn)
            .await
            .map_err(|e| DbError::QueryFailed(format!("执行命令失败: {}", e)))
    }
    
    /// 获取 Redis 服务器信息
    pub async fn get_server_info(&self) -> DbResult<std::collections::HashMap<String, String>> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        let mut conn = conn.clone();
        
        let info: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取服务器信息失败: {}", e)))?;
        
        // 解析 INFO 命令的输出
        let mut result = std::collections::HashMap::new();
        
        for line in info.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once(':') {
                result.insert(key.to_string(), value.to_string());
            }
        }
        
        Ok(result)
    }
    
    /// 获取键的值（根据类型返回不同格式）
    pub async fn get_key_value(&self, key: &str) -> DbResult<redis::Value> {
        use redis::AsyncCommands;
        
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        let mut conn = conn.clone();
        
        // 先获取键的类型
        let key_type: String = redis::cmd("TYPE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取键类型失败: {}", e)))?;
        
        match key_type.as_str() {
            "string" => {
                let value: String = conn.get(key)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("获取字符串值失败: {}", e)))?;
                Ok(redis::Value::BulkString(value.into_bytes()))
            }
            "list" => {
                let values: Vec<String> = conn.lrange(key, 0, -1)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("获取列表值失败: {}", e)))?;
                Ok(redis::Value::Array(values.into_iter().map(|v| redis::Value::BulkString(v.into_bytes())).collect()))
            }
            "set" => {
                let values: Vec<String> = conn.smembers(key)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("获取集合值失败: {}", e)))?;
                Ok(redis::Value::Array(values.into_iter().map(|v| redis::Value::BulkString(v.into_bytes())).collect()))
            }
            "zset" => {
                let values: Vec<(String, f64)> = conn.zrange_withscores(key, 0, -1)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("获取有序集合值失败: {}", e)))?;
                
                let formatted: Vec<redis::Value> = values.into_iter()
                    .map(|(member, score)| {
                        redis::Value::Array(vec![
                            redis::Value::BulkString(member.into_bytes()),
                            redis::Value::BulkString(score.to_string().into_bytes())
                        ])
                    })
                    .collect();
                
                Ok(redis::Value::Array(formatted))
            }
            "hash" => {
                let values: std::collections::HashMap<String, String> = conn.hgetall(key)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("获取哈希值失败: {}", e)))?;
                
                let formatted: Vec<redis::Value> = values.into_iter()
                    .flat_map(|(k, v)| vec![
                        redis::Value::BulkString(k.into_bytes()),
                        redis::Value::BulkString(v.into_bytes())
                    ])
                    .collect();
                
                Ok(redis::Value::Array(formatted))
            }
            _ => Ok(redis::Value::Nil)
        }
    }
    
    /// 设置键值
    pub async fn set_key_value(&self, key: &str, value: &str, ttl: Option<u64>) -> DbResult<()> {
        use redis::AsyncCommands;
        
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        let mut conn = conn.clone();
        
        if let Some(ttl_seconds) = ttl {
            conn.set_ex::<_, _, ()>(key, value, ttl_seconds)
                .await
                .map_err(|e| DbError::QueryFailed(format!("设置键值失败: {}", e)))?;
        } else {
            conn.set::<_, _, ()>(key, value)
                .await
                .map_err(|e| DbError::QueryFailed(format!("设置键值失败: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// 删除键
    pub async fn delete_key(&self, key: &str) -> DbResult<()> {
        use redis::AsyncCommands;
        
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        let mut conn = conn.clone();
        
        conn.del::<_, ()>(key)
            .await
            .map_err(|e| DbError::QueryFailed(format!("删除键失败: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取键的 TTL
    pub async fn get_key_ttl(&self, key: &str) -> DbResult<i64> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 Redis".to_string()))?;
        
        let mut conn = conn.clone();
        
        redis::cmd("TTL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取 TTL 失败: {}", e)))
    }
}

