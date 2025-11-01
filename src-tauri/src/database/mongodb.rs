use async_trait::async_trait;
use mongodb::{Client, options::ClientOptions};
use super::traits::*;

/// MongoDB 数据库连接
pub struct MongoDatabase {
    client: Option<Client>,
    config: Option<ConnectionConfig>,
}

impl MongoDatabase {
    pub fn new() -> Self {
        Self { 
            client: None,
            config: None,
        }
    }

    /// 构建 MongoDB 连接 URI
    fn build_connection_uri(config: &ConnectionConfig) -> String {
        let auth_part = if !config.username.is_empty() {
            if !config.password.is_empty() {
                format!("{}:{}@", config.username, config.password)
            } else {
                format!("{}@", config.username)
            }
        } else {
            String::new()
        };

        let protocol = if config.ssl { "mongodb+srv" } else { "mongodb" };
        
        let database_part = config.database.as_ref()
            .map(|db| format!("/{}", db))
            .unwrap_or_default();

        format!("{}://{}{}:{}{}", protocol, auth_part, config.host, config.port, database_part)
    }
}

#[async_trait]
impl DatabaseOperations for MongoDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let uri = Self::build_connection_uri(config);
        
        // 解析连接选项
        let mut client_options = ClientOptions::parse(&uri)
            .await
            .map_err(|e| DbError::ConnectionFailed(format!("解析 MongoDB URI 失败: {}", e)))?;
        
        // 设置连接超时
        client_options.connect_timeout = Some(std::time::Duration::from_secs(config.connection_timeout));
        client_options.server_selection_timeout = Some(std::time::Duration::from_secs(config.connection_timeout));
        
        // 创建客户端
        let client = Client::with_options(client_options)
            .map_err(|e| DbError::ConnectionFailed(format!("创建 MongoDB 客户端失败: {}", e)))?;
        
        // 测试连接 - 执行 ping 命令
        let admin_db = client.database("admin");
        admin_db
            .run_command(mongodb::bson::doc! { "ping": 1 }, None)
            .await
            .map_err(|e| DbError::ConnectionFailed(format!("连接 MongoDB 失败: {}", e)))?;
        
        Ok(true)
    }

    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()> {
        let uri = Self::build_connection_uri(&config);
        
        // 解析连接选项
        let mut client_options = ClientOptions::parse(&uri)
            .await
            .map_err(|e| DbError::ConnectionFailed(format!("解析 MongoDB URI 失败: {}", e)))?;
        
        // 设置连接超时
        client_options.connect_timeout = Some(std::time::Duration::from_secs(config.connection_timeout));
        client_options.server_selection_timeout = Some(std::time::Duration::from_secs(config.connection_timeout));
        
        // 创建客户端
        let client = Client::with_options(client_options)
            .map_err(|e| DbError::ConnectionFailed(format!("创建 MongoDB 客户端失败: {}", e)))?;
        
        self.client = Some(client);
        self.config = Some(config);
        
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.client = None;
        self.config = None;
        Ok(())
    }

    async fn execute_query(&self, _sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        Err(DbError::Other("MongoDB 不支持 SQL 查询，请使用 MongoDB Shell 命令或查询语法".to_string()))
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 MongoDB".to_string()))?;
        
        // 列出所有数据库
        let db_names = client
            .list_database_names(None, None)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取数据库列表失败: {}", e)))?;
        
        let databases = db_names
            .into_iter()
            .map(|name| DatabaseInfo {
                name,
                charset: None,
                collation: None,
            })
            .collect();
        
        Ok(databases)
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 MongoDB".to_string()))?;
        
        let db_name = database.or_else(|| {
            self.config.as_ref().and_then(|c| c.database.as_deref())
        }).ok_or_else(|| DbError::ConfigError("未指定数据库".to_string()))?;
        
        let db = client.database(db_name);
        
        // 获取所有集合名称
        let collection_names = db
            .list_collection_names(None)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取集合列表失败: {}", e)))?;
        
        let mut tables = Vec::new();
        for name in collection_names {
            // 获取集合统计信息
            let collection = db.collection::<mongodb::bson::Document>(&name);
            let count = collection
                .estimated_document_count(None)
                .await
                .ok();
            
            tables.push(TableInfo {
                name,
                schema: Some(db_name.to_string()),
                table_type: "COLLECTION".to_string(),
                engine: Some("MongoDB".to_string()),
                rows: count,
                size_mb: None, // MongoDB 需要额外的权限来获取大小
                comment: None,
            });
        }
        
        Ok(tables)
    }

    async fn get_table_structure(&self, _table: &str, _schema: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        // MongoDB 是无模式的文档数据库
        // 我们可以返回一个说明，或者采样一些文档来推断结构
        Ok(vec![])
    }

    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到 MongoDB".to_string()))?;
        
        let db_name = schema.or_else(|| {
            self.config.as_ref().and_then(|c| c.database.as_deref())
        }).ok_or_else(|| DbError::ConfigError("未指定数据库".to_string()))?;
        
        let db = client.database(db_name);
        let collection = db.collection::<mongodb::bson::Document>(table);
        
        // 获取索引信息
        let mut cursor = collection
            .list_indexes(None)
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取索引列表失败: {}", e)))?;
        
        use futures::stream::StreamExt;
        let mut indexes = Vec::new();
        
        while let Some(result) = cursor.next().await {
            match result {
                Ok(index) => {
                    // 索引名称在 index.options.name 中
                    let name = index.options
                        .as_ref()
                        .and_then(|opts| opts.name.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    let is_unique = index.options
                        .as_ref()
                        .and_then(|opts| opts.unique)
                        .unwrap_or(false);
                    
                    // 提取索引字段
                    let mut columns = Vec::new();
                    for (key, _) in &index.keys {
                        columns.push(key.clone());
                    }
                    
                    indexes.push(IndexInfo {
                        name: name.clone(),
                        columns,
                        is_unique,
                        is_primary: name == "_id_",
                        index_type: "BTREE".to_string(),
                    });
                }
                Err(e) => {
                    return Err(DbError::QueryFailed(format!("读取索引信息失败: {}", e)));
                }
            }
        }
        
        Ok(indexes)
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

