use async_trait::async_trait;
use sqlx::{Column, PgPool, Pool, Postgres, Row};
use std::collections::HashMap;
use std::time::Instant;

use super::traits::*;

/// PostgreSQL 数据库连接
pub struct PostgreSqlDatabase {
    pool: Option<Pool<Postgres>>,
    config: Option<ConnectionConfig>,
}

impl PostgreSqlDatabase {
    pub fn new() -> Self {
        Self {
            pool: None,
            config: None,
        }
    }

    /// 构建连接字符串
    fn build_connection_string(config: &ConnectionConfig) -> String {
        let mut url = format!(
            "postgres://{}:{}@{}:{}",
            config.username, config.password, config.host, config.port
        );

        if let Some(ref database) = config.database {
            url.push_str(&format!("/{}", database));
        } else {
            url.push_str("/postgres"); // 默认数据库
        }

        // SSL 配置
        if config.ssl {
            url.push_str("?sslmode=require");
        } else {
            url.push_str("?sslmode=prefer");
        }

        url
    }

    /// 使用指定的连接池执行查询
    async fn execute_query_with_pool(
        &self,
        pool: &Pool<Postgres>,
        sql: &str,
    ) -> DbResult<QueryResult> {
        let start = Instant::now();

        // 判断是否为查询语句
        let is_select = sql.trim().to_uppercase().starts_with("SELECT")
            || sql.trim().to_uppercase().starts_with("SHOW")
            || sql.trim().to_uppercase().starts_with("EXPLAIN")
            || sql.trim().to_uppercase().starts_with("WITH");

        if is_select {
            // 查询操作
            let rows = sqlx::query(sql)
                .fetch_all(pool)
                .await
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let mut columns = Vec::new();
            let mut result_rows = Vec::new();

            if !rows.is_empty() {
                // 获取列名
                for column in rows[0].columns() {
                    columns.push(column.name().to_string());
                }

                // 转换行数据
                for (row_idx, row) in rows.iter().enumerate() {
                    let mut row_map = HashMap::new();
                    for (idx, column) in row.columns().iter().enumerate() {
                        // 尝试多种数据类型获取，类似于 MySQL 的处理方式
                        let value = if let Ok(s) = row.try_get::<String, _>(idx) {
                            serde_json::Value::String(s)
                        } else if let Ok(i) = row.try_get::<i64, _>(idx) {
                            serde_json::Value::Number(serde_json::Number::from(i))
                        } else if let Ok(i) = row.try_get::<i32, _>(idx) {
                            serde_json::Value::Number(serde_json::Number::from(i as i64))
                        } else if let Ok(i) = row.try_get::<i16, _>(idx) {
                            serde_json::Value::Number(serde_json::Number::from(i as i64))
                        } else if let Ok(f) = row.try_get::<f64, _>(idx) {
                            serde_json::Number::from_f64(f)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        } else if let Ok(f) = row.try_get::<f32, _>(idx) {
                            serde_json::Number::from_f64(f as f64)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        } else if let Ok(b) = row.try_get::<bool, _>(idx) {
                            serde_json::Value::Bool(b)
                        } else if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(idx) {
                            // 处理 bytea 等二进制数据
                            // 尝试转换为 UTF-8 字符串，如果失败则使用 base64 或 hex
                            match String::from_utf8(bytes.clone()) {
                                Ok(s) => serde_json::Value::String(s),
                                Err(_) => {
                                    // 对于非 UTF-8 数据，使用 hex 编码
                                    let hex_string = bytes.iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<String>();
                                    serde_json::Value::String(format!("\\x{}", hex_string))
                                }
                            }
                        } else if let Ok(s) = row.try_get::<chrono::NaiveDateTime, _>(idx) {
                            serde_json::Value::String(s.to_string())
                        } else if let Ok(s) = row.try_get::<chrono::NaiveDate, _>(idx) {
                            serde_json::Value::String(s.to_string())
                        } else if let Ok(s) = row.try_get::<chrono::NaiveTime, _>(idx) {
                            serde_json::Value::String(s.to_string())
                        } else if let Ok(s) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(idx) {
                            serde_json::Value::String(s.to_string())
                        } else if let Ok(json) = row.try_get::<serde_json::Value, _>(idx) {
                            // 处理 JSON/JSONB 类型
                            json
                        } else if let Ok(uuid) = row.try_get::<sqlx::types::Uuid, _>(idx) {
                            // 处理 UUID 类型
                            serde_json::Value::String(uuid.to_string())
                        } else if let Ok(None) = row.try_get::<Option<String>, _>(idx) {
                            // 明确的 NULL 值
                            serde_json::Value::Null
                        } else {
                            // 最后尝试获取原始字符串表示
                            match row.try_get::<Option<String>, _>(idx) {
                                Ok(Some(s)) => serde_json::Value::String(s),
                                _ => serde_json::Value::Null,
                            }
                        };
                        
                        row_map.insert(column.name().to_string(), value.clone());
                        
                        // 打印第一行的数据作为调试
                        if row_idx == 0 {
                            println!("列 {} (类型: {:?}): {:?}", column.name(), column.type_info(), value);
                        }
                    }
                    result_rows.push(row_map);
                }
            }

            Ok(QueryResult {
                columns,
                rows: result_rows,
                affected_rows: rows.len() as u64,
                execution_time_ms: start.elapsed().as_millis(),
            })
        } else {
            // 非查询操作（INSERT, UPDATE, DELETE 等）
            let result = sqlx::query(sql)
                .execute(pool)
                .await
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: result.rows_affected(),
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
    }

    /// 使用指定连接池获取表结构
    async fn get_table_structure_with_pool(
        &self,
        pool: &Pool<Postgres>,
        table: &str,
        schema: Option<&str>,
    ) -> DbResult<Vec<ColumnInfo>> {
        let schema_name = schema.unwrap_or("public");

        let rows = sqlx::query(
            "SELECT
                column_name,
                data_type,
                is_nullable,
                column_default,
                character_maximum_length,
                numeric_precision,
                numeric_scale,
                col_description((table_schema||'.'||table_name)::regclass::oid, ordinal_position) as comment
             FROM information_schema.columns
             WHERE table_schema = $1 AND table_name = $2
             ORDER BY ordinal_position"
        )
        .bind(schema_name)
        .bind(table)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        // 获取主键信息
        let pk_rows = sqlx::query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             WHERE i.indrelid = ($1 || '.' || $2)::regclass AND i.indisprimary"
        )
        .bind(schema_name)
        .bind(table)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let primary_keys: Vec<String> = pk_rows
            .iter()
            .map(|row| row.try_get::<String, _>(0).unwrap_or_default())
            .collect();

        let mut columns = Vec::new();
        for row in rows {
            let is_nullable: String = row.try_get(2).unwrap_or_default();
            let column_name: String = row.try_get(0).unwrap_or_default();
            let column_default: Option<String> = row.try_get(3).ok();
            
            let is_auto_increment = column_default
                .as_ref()
                .map(|s| s.contains("nextval"))
                .unwrap_or(false);

            columns.push(ColumnInfo {
                name: column_name.clone(),
                data_type: row.try_get(1).unwrap_or_default(),
                nullable: is_nullable.to_uppercase() == "YES",
                default_value: column_default,
                is_primary_key: primary_keys.contains(&column_name),
                is_auto_increment,
                comment: row.try_get(7).ok(),
                character_maximum_length: row.try_get(4).ok(),
                numeric_precision: row.try_get(5).ok(),
                numeric_scale: row.try_get(6).ok(),
            });
        }

        Ok(columns)
    }
}

#[async_trait]
impl DatabaseOperations for PostgreSqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let connection_string = Self::build_connection_string(config);
        
        match PgPool::connect(&connection_string).await {
            Ok(pool) => {
                // 测试查询
                let result = sqlx::query("SELECT 1")
                    .fetch_one(&pool)
                    .await;
                
                pool.close().await;
                
                match result {
                    Ok(_) => Ok(true),
                    Err(e) => Err(DbError::ConnectionFailed(e.to_string())),
                }
            }
            Err(e) => Err(DbError::ConnectionFailed(e.to_string())),
        }
    }

    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()> {
        let connection_string = Self::build_connection_string(&config);
        
        let pool = PgPool::connect(&connection_string)
            .await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        
        // 如果已经有连接，先关闭
        if let Some(old_pool) = self.pool.take() {
            old_pool.close().await;
        }
        
        self.pool = Some(pool);
        self.config = Some(config);
        
        Ok(())
    }

    /// 切换到指定数据库（PostgreSQL 需要重新连接）
    async fn switch_database(&mut self, database: &str) -> DbResult<()> {
        if let Some(ref config) = self.config {
            // 创建新的配置，使用指定的数据库
            let mut new_config = config.clone();
            new_config.database = Some(database.to_string());
            
            // 重新连接
            self.connect(new_config).await
        } else {
            Err(DbError::ConnectionFailed("未连接到数据库".to_string()))
        }
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        self.config = None;
        Ok(())
    }

    async fn execute_query(&self, sql: &str, database: Option<&str>) -> DbResult<QueryResult> {
        let start = Instant::now();

        // PostgreSQL 中数据库切换需要重新连接
        // 如果指定了数据库且与当前连接的数据库不同，需要创建临时连接
        let pool = if let Some(db_name) = database {
            if let Some(ref config) = self.config {
                if config.database.as_deref() != Some(db_name) {
                    // 创建临时连接到指定数据库
                    let temp_config = ConnectionConfig {
                        id: format!("temp_{}", config.id),
                        name: config.name.clone(),
                        db_type: config.db_type.clone(),
                        host: config.host.clone(),
                        port: config.port,
                        username: config.username.clone(),
                        password: config.password.clone(),
                        database: Some(db_name.to_string()),
                        ssl: config.ssl,
                        connection_timeout: config.connection_timeout,
                        pool_size: config.pool_size,
                    };
                    
                    let connection_string = Self::build_connection_string(&temp_config);
                    
                    let temp_pool = PgPool::connect(&connection_string)
                        .await
                        .map_err(|e| DbError::ConnectionFailed(format!("连接到数据库 {} 失败: {}", db_name, e)))?;
                    
                    // 使用临时连接执行查询
                    let result = self.execute_query_with_pool(&temp_pool, sql).await;
                    
                    temp_pool.close().await;
                    
                    return result;
                }
            }
            
            // 如果是同一个数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        } else {
            // 没有指定数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        };

        self.execute_query_with_pool(pool, sql).await
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let rows = sqlx::query(
            "SELECT datname, pg_encoding_to_char(encoding) AS encoding, datcollate 
             FROM pg_database 
             WHERE datistemplate = false AND datname NOT IN ('postgres')
             ORDER BY datname"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut databases = Vec::new();
        for row in rows {
            databases.push(DatabaseInfo {
                name: row.try_get(0).unwrap_or_default(),
                charset: row.try_get(1).ok(),
                collation: row.try_get(2).ok(),
            });
        }

        Ok(databases)
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        // PostgreSQL 需要使用指定的数据库连接池来查询表
        // 如果指定了数据库且与当前连接的数据库不同，需要创建临时连接
        let pool = if let Some(db_name) = database {
            // 检查是否需要切换数据库
            if let Some(ref config) = self.config {
                if config.database.as_deref() != Some(db_name) {
                    // 创建临时连接到指定数据库
                    let temp_config = ConnectionConfig {
                        id: format!("temp_{}", config.id),
                        name: config.name.clone(),
                        db_type: config.db_type.clone(),
                        host: config.host.clone(),
                        port: config.port,
                        username: config.username.clone(),
                        password: config.password.clone(),
                        database: Some(db_name.to_string()),
                        ssl: config.ssl,
                        connection_timeout: config.connection_timeout,
                        pool_size: config.pool_size,
                    };
                    
                    let connection_string = Self::build_connection_string(&temp_config);
                    
                    let temp_pool = PgPool::connect(&connection_string)
                        .await
                        .map_err(|e| DbError::ConnectionFailed(format!("连接到数据库 {} 失败: {}", db_name, e)))?;
                    
                    // 使用临时连接查询
                    let rows = sqlx::query(
                        "SELECT 
                            schemaname, 
                            tablename, 
                            'TABLE' as table_type,
                            NULL as engine,
                            NULL as table_rows,
                            pg_total_relation_size(schemaname||'.'||tablename)::bigint / 1024 / 1024 as size_mb,
                            obj_description((schemaname||'.'||tablename)::regclass) as comment
                         FROM pg_tables 
                         WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
                         ORDER BY schemaname, tablename"
                    )
                    .fetch_all(&temp_pool)
                    .await
                    .map_err(|e| DbError::QueryFailed(e.to_string()))?;

                    let mut tables = Vec::new();
                    for row in rows {
                        let schema: String = row.try_get(0).unwrap_or_default();
                        tables.push(TableInfo {
                            name: row.try_get(1).unwrap_or_default(),
                            schema: Some(schema),
                            table_type: row.try_get(2).unwrap_or_default(),
                            engine: row.try_get(3).ok(),
                            rows: None,
                            size_mb: row.try_get(5).ok(),
                            comment: row.try_get(6).ok(),
                        });
                    }
                    
                    temp_pool.close().await;
                    return Ok(tables);
                }
            }
            
            // 如果是同一个数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        } else {
            // 没有指定数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        };

        // 查询表列表
        let rows = sqlx::query(
            "SELECT 
                schemaname, 
                tablename, 
                'TABLE' as table_type,
                NULL as engine,
                NULL as table_rows,
                pg_total_relation_size(schemaname||'.'||tablename)::bigint / 1024 / 1024 as size_mb,
                obj_description((schemaname||'.'||tablename)::regclass) as comment
             FROM pg_tables 
             WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
             ORDER BY schemaname, tablename"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut tables = Vec::new();
        for row in rows {
            let schema: String = row.try_get(0).unwrap_or_default();
            tables.push(TableInfo {
                name: row.try_get(1).unwrap_or_default(),
                schema: Some(schema),
                table_type: row.try_get(2).unwrap_or_default(),
                engine: row.try_get(3).ok(),
                rows: None,
                size_mb: row.try_get(5).ok(),
                comment: row.try_get(6).ok(),
            });
        }

        Ok(tables)
    }

    async fn get_views(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        // PostgreSQL 需要使用指定的数据库连接池来查询视图
        // 如果指定了数据库且与当前连接的数据库不同，需要创建临时连接
        let pool = if let Some(db_name) = database {
            // 检查是否需要切换数据库
            if let Some(ref config) = self.config {
                if config.database.as_deref() != Some(db_name) {
                    // 创建临时连接到指定数据库
                    let temp_config = ConnectionConfig {
                        id: format!("temp_{}", config.id),
                        name: config.name.clone(),
                        db_type: config.db_type.clone(),
                        host: config.host.clone(),
                        port: config.port,
                        username: config.username.clone(),
                        password: config.password.clone(),
                        database: Some(db_name.to_string()),
                        ssl: config.ssl,
                        connection_timeout: config.connection_timeout,
                        pool_size: config.pool_size,
                    };
                    
                    let connection_string = Self::build_connection_string(&temp_config);
                    
                    let temp_pool = PgPool::connect(&connection_string)
                        .await
                        .map_err(|e| DbError::ConnectionFailed(format!("连接到数据库 {} 失败: {}", db_name, e)))?;
                    
                    // 使用临时连接查询
                    let rows = sqlx::query(
                        "SELECT 
                            schemaname, 
                            viewname as tablename, 
                            'VIEW' as table_type,
                            NULL as engine,
                            NULL as table_rows,
                            pg_total_relation_size(schemaname||'.'||viewname)::bigint / 1024 / 1024 as size_mb,
                            obj_description((schemaname||'.'||viewname)::regclass) as comment
                         FROM pg_views 
                         WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
                         ORDER BY schemaname, viewname"
                    )
                    .fetch_all(&temp_pool)
                    .await
                    .map_err(|e| DbError::QueryFailed(e.to_string()))?;

                    let mut views = Vec::new();
                    for row in rows {
                        let schema: String = row.try_get(0).unwrap_or_default();
                        views.push(TableInfo {
                            name: row.try_get(1).unwrap_or_default(),
                            schema: Some(schema),
                            table_type: row.try_get(2).unwrap_or_default(),
                            engine: row.try_get(3).ok(),
                            rows: None,
                            size_mb: row.try_get(5).ok(),
                            comment: row.try_get(6).ok(),
                        });
                    }
                    
                    temp_pool.close().await;
                    return Ok(views);
                }
            }
            
            // 如果是同一个数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        } else {
            // 没有指定数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        };

        // 查询视图列表
        let rows = sqlx::query(
            "SELECT 
                schemaname, 
                viewname as tablename, 
                'VIEW' as table_type,
                NULL as engine,
                NULL as table_rows,
                pg_total_relation_size(schemaname||'.'||viewname)::bigint / 1024 / 1024 as size_mb,
                obj_description((schemaname||'.'||viewname)::regclass) as comment
             FROM pg_views 
             WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
             ORDER BY schemaname, viewname"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut views = Vec::new();
        for row in rows {
            let schema: String = row.try_get(0).unwrap_or_default();
            views.push(TableInfo {
                name: row.try_get(1).unwrap_or_default(),
                schema: Some(schema),
                table_type: row.try_get(2).unwrap_or_default(),
                engine: row.try_get(3).ok(),
                rows: None,
                size_mb: row.try_get(5).ok(),
                comment: row.try_get(6).ok(),
            });
        }

        Ok(views)
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        // 确定使用哪个连接池
        let pool = if let Some(db_name) = database {
            // 检查当前连接的数据库是否与请求的数据库相同
            let current_db = self.config.as_ref()
                .and_then(|c| c.database.as_deref())
                .unwrap_or("postgres");
            
            if current_db == db_name {
                // 同一个数据库，使用当前连接
                self.pool.as_ref()
                    .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
            } else {
                // 不同的数据库，创建临时连接
                let config = self.config.as_ref()
                    .ok_or_else(|| DbError::ConnectionFailed("连接配置不存在".to_string()))?;
                
                let mut temp_config = config.clone();
                temp_config.database = Some(db_name.to_string());
                
                let temp_connection_string = Self::build_connection_string(&temp_config);
                let temp_pool = PgPool::connect(&temp_connection_string).await
                    .map_err(|e| DbError::ConnectionFailed(format!("连接数据库失败: {}", e)))?;
                
                // 使用临时连接执行查询
                let result = self.get_table_structure_with_pool(&temp_pool, table, schema).await;
                temp_pool.close().await;
                return result;
            }
        } else {
            // 没有指定数据库，使用当前连接
            self.pool.as_ref()
                .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?
        };

        self.get_table_structure_with_pool(pool, table, schema).await
    }

    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let schema_name = schema.unwrap_or("public");

        let rows = sqlx::query(
            "SELECT 
                i.relname as index_name,
                a.attname as column_name,
                ix.indisunique,
                ix.indisprimary,
                am.amname as index_type
             FROM pg_class t
             JOIN pg_index ix ON t.oid = ix.indrelid
             JOIN pg_class i ON i.oid = ix.indexrelid
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
             JOIN pg_am am ON i.relam = am.oid
             WHERE t.relname = $1 
               AND t.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = $2)
             ORDER BY i.relname, a.attnum"
        )
        .bind(table)
        .bind(schema_name)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        // 按索引名分组
        let mut index_map: HashMap<String, IndexInfo> = HashMap::new();
        
        for row in rows {
            let index_name: String = row.try_get(0).unwrap_or_default();
            let column_name: String = row.try_get(1).unwrap_or_default();
            let is_unique: bool = row.try_get(2).unwrap_or(false);
            let is_primary: bool = row.try_get(3).unwrap_or(false);
            let index_type: String = row.try_get(4).unwrap_or_default();

            index_map
                .entry(index_name.clone())
                .and_modify(|info| info.columns.push(column_name.clone()))
                .or_insert_with(|| IndexInfo {
                    name: index_name,
                    columns: vec![column_name],
                    is_unique,
                    is_primary,
                    index_type,
                });
        }

        Ok(index_map.into_values().collect())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}