use async_trait::async_trait;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Column, MySql, Pool, Row};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::traits::*;

/// MySQL 数据库连接
pub struct MySqlDatabase {
    pool: Option<Pool<MySql>>,
    config: Option<ConnectionConfig>,
}

impl MySqlDatabase {
    pub fn new() -> Self {
        Self {
            pool: None,
            config: None,
        }
    }

    /// 构建连接字符串
    fn build_connection_string(config: &ConnectionConfig) -> String {
        let mut url = format!(
            "mysql://{}:{}@{}:{}",
            config.username, config.password, config.host, config.port
        );

        if let Some(ref database) = config.database {
            url.push_str(&format!("/{}", database));
        }

        // SSL 配置
        if config.ssl {
            url.push_str("?ssl-mode=REQUIRED");
        }

        url
    }
}

#[async_trait]
impl DatabaseOperations for MySqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let mut connection_string = Self::build_connection_string(config);
        
        // 添加超时参数到连接字符串
        let separator = if connection_string.contains('?') { "&" } else { "?" };
        connection_string.push_str(&format!(
            "{}connect_timeout={}",
            separator,
            config.connection_timeout
        ));
        
        // 配置连接池选项
        let pool_options = MySqlPoolOptions::new()
            .max_connections(1) // 测试时只需要1个连接
            .acquire_timeout(Duration::from_secs(config.connection_timeout as u64))
            .idle_timeout(Some(Duration::from_secs(30)));
        
        match pool_options.connect(&connection_string).await {
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
        let mut connection_string = Self::build_connection_string(&config);
        
        // 添加超时参数到连接字符串
        let separator = if connection_string.contains('?') { "&" } else { "?" };
        connection_string.push_str(&format!(
            "{}connect_timeout={}",
            separator,
            config.connection_timeout
        ));
        
        // 配置连接池选项
        let pool_options = MySqlPoolOptions::new()
            .max_connections(config.pool_size as u32)
            .acquire_timeout(Duration::from_secs(config.connection_timeout as u64))
            .idle_timeout(Some(Duration::from_secs(300))) // 5分钟空闲超时
            .max_lifetime(Some(Duration::from_secs(1800))); // 30分钟最大生命周期
        
        let pool = pool_options.connect(&connection_string)
            .await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        
        self.pool = Some(pool);
        self.config = Some(config);
        
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        self.config = None;
        Ok(())
    }

    async fn execute_query(&self, sql: &str, database: Option<&str>) -> DbResult<QueryResult> {
        use sqlx::Executor;
        
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let start = Instant::now();

        // 调试信息
        println!("执行查询 - 接收到的database参数: {:?}", database);
        println!("执行查询 - 原始SQL: {}", sql);

        // 简单的SQL语句分割：按分号分割并清理注释
        let statements = self.split_sql_statements(sql);
        println!("分割后的SQL语句数量: {}", statements.len());
        
        if statements.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: 0,
                execution_time_ms: 0,
            });
        }
        
        // 执行第一条有效的SQL语句
        let sql_to_execute = &statements[0];
        println!("实际执行的SQL: {}", sql_to_execute);

        // 判断是否为查询语句
        let is_select = sql_to_execute.trim().to_uppercase().starts_with("SELECT")
            || sql_to_execute.trim().to_uppercase().starts_with("SHOW")
            || sql_to_execute.trim().to_uppercase().starts_with("DESCRIBE")
            || sql_to_execute.trim().to_uppercase().starts_with("EXPLAIN");

        if is_select {
            // 如果指定了数据库，获取专用连接并设置数据库上下文
            if let Some(db_name) = database {
                if !db_name.is_empty() {
                    let mut conn = pool.acquire()
                        .await
                        .map_err(|e| DbError::QueryFailed(format!("获取连接失败: {}", e)))?;
                    
                    // 设置数据库上下文
                    let use_sql = format!("USE `{}`", db_name);
                    println!("设置数据库上下文: {}", use_sql);
                    conn.execute(use_sql.as_str())
                        .await
                        .map_err(|e| DbError::QueryFailed(format!("切换数据库失败: {}", e)))?;
                    
                    // 在同一连接上执行用户的原生SQL
                    let rows = sqlx::query(sql_to_execute)
                        .fetch_all(&mut *conn)
                        .await
                        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
                        
                    return self.process_query_result(rows, start);
                }
            }
            
            // 没有指定数据库，直接在池上执行
            let rows = sqlx::query(sql_to_execute)
                .fetch_all(pool)
                .await
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;
                
            self.process_query_result(rows, start)
        } else {
            // 非查询操作（INSERT, UPDATE, DELETE 等）
            if let Some(db_name) = database {
                if !db_name.is_empty() {
                    let mut conn = pool.acquire()
                        .await
                        .map_err(|e| DbError::QueryFailed(format!("获取连接失败: {}", e)))?;
                    
                    // 设置数据库上下文
                    let use_sql = format!("USE `{}`", db_name);
                    println!("设置数据库上下文: {}", use_sql);
                    conn.execute(use_sql.as_str())
                        .await
                        .map_err(|e| DbError::QueryFailed(format!("切换数据库失败: {}", e)))?;
                    
                    // 在同一连接上执行用户的原生SQL
                    let result = sqlx::query(sql_to_execute)
                        .execute(&mut *conn)
                        .await
                        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

                    let duration = start.elapsed();

                    return Ok(QueryResult {
                        columns: vec![],
                        rows: vec![],
                        affected_rows: result.rows_affected(),
                        execution_time_ms: duration.as_millis(),
                    });
                }
            }
            
            // 没有指定数据库，直接执行
            let result = sqlx::query(sql_to_execute)
                .execute(pool)
                .await
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let duration = start.elapsed();

            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: result.rows_affected(),
                execution_time_ms: duration.as_millis(),
            })
        }
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let rows = sqlx::query(
            "SELECT SCHEMA_NAME, DEFAULT_CHARACTER_SET_NAME, DEFAULT_COLLATION_NAME 
             FROM information_schema.SCHEMATA 
             WHERE SCHEMA_NAME NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys')
             ORDER BY SCHEMA_NAME"
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
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let db_name = database.or_else(|| {
            self.config.as_ref().and_then(|c| c.database.as_deref())
        }).ok_or_else(|| DbError::ConfigError("未指定数据库".to_string()))?;

        let rows = sqlx::query(
            "SELECT TABLE_NAME, TABLE_TYPE, ENGINE, TABLE_ROWS, 
                    ROUND(((DATA_LENGTH + INDEX_LENGTH) / 1024 / 1024), 2) AS SIZE_MB,
                    TABLE_COMMENT
             FROM information_schema.TABLES 
             WHERE TABLE_SCHEMA = ?
             ORDER BY TABLE_NAME"
        )
        .bind(db_name)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut tables = Vec::new();
        for row in rows {
            tables.push(TableInfo {
                name: row.try_get(0).unwrap_or_default(),
                schema: Some(db_name.to_string()),
                table_type: row.try_get(1).unwrap_or_default(),
                engine: row.try_get(2).ok(),
                rows: row.try_get(3).ok(),
                size_mb: row.try_get(4).ok(),
                comment: row.try_get(5).ok(),
            });
        }

        Ok(tables)
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let db_name = schema.or_else(|| {
            self.config.as_ref().and_then(|c| c.database.as_deref())
        }).ok_or_else(|| DbError::ConfigError("未指定数据库".to_string()))?;

        let rows = sqlx::query(
            "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT, 
                    COLUMN_KEY, EXTRA, COLUMN_COMMENT, CHARACTER_MAXIMUM_LENGTH,
                    NUMERIC_PRECISION, NUMERIC_SCALE
             FROM information_schema.COLUMNS 
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
             ORDER BY ORDINAL_POSITION"
        )
        .bind(db_name)
        .bind(table)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut columns = Vec::new();
        for row in rows {
            let is_nullable: String = row.try_get(2).unwrap_or_default();
            let column_key: String = row.try_get(4).unwrap_or_default();
            let extra: String = row.try_get(5).unwrap_or_default();

            columns.push(ColumnInfo {
                name: row.try_get(0).unwrap_or_default(),
                data_type: row.try_get(1).unwrap_or_default(),
                nullable: is_nullable.to_uppercase() == "YES",
                default_value: row.try_get(3).ok(),
                is_primary_key: column_key == "PRI",
                is_auto_increment: extra.contains("auto_increment"),
                comment: row.try_get(6).ok(),
                character_maximum_length: row.try_get(7).ok(),
                numeric_precision: row.try_get(8).ok(),
                numeric_scale: row.try_get(9).ok(),
            });
        }

        Ok(columns)
    }

    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let db_name = schema.or_else(|| {
            self.config.as_ref().and_then(|c| c.database.as_deref())
        }).ok_or_else(|| DbError::ConfigError("未指定数据库".to_string()))?;

        let rows = sqlx::query(
            "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE, INDEX_TYPE
             FROM information_schema.STATISTICS 
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
             ORDER BY INDEX_NAME, SEQ_IN_INDEX"
        )
        .bind(db_name)
        .bind(table)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        // 按索引名分组
        let mut index_map: HashMap<String, IndexInfo> = HashMap::new();
        
        for row in rows {
            let index_name: String = row.try_get(0).unwrap_or_default();
            let column_name: String = row.try_get(1).unwrap_or_default();
            let non_unique: i32 = row.try_get(2).unwrap_or(1);
            let index_type: String = row.try_get(3).unwrap_or_default();

            index_map
                .entry(index_name.clone())
                .and_modify(|info| info.columns.push(column_name.clone()))
                .or_insert_with(|| IndexInfo {
                    name: index_name.clone(),
                    columns: vec![column_name],
                    is_unique: non_unique == 0,
                    is_primary: index_name == "PRIMARY",
                    index_type: index_type.clone(),
                });
        }

        Ok(index_map.into_values().collect())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MySqlDatabase {
    /// 分割SQL语句：按分号分割，简单处理注释
    fn split_sql_statements(&self, sql: &str) -> Vec<String> {
        let mut statements = Vec::new();
        
        // 按行处理，移除单行注释
        let lines: Vec<&str> = sql.lines().collect();
        let mut cleaned_lines = Vec::new();
        
        for line in lines {
            let trimmed = line.trim();
            // 跳过空行和以--开头的注释行
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                cleaned_lines.push(line);
            }
        }
        
        // 重新组合成字符串
        let cleaned_sql = cleaned_lines.join("\n");
        
        // 按分号分割语句
        let parts: Vec<&str> = cleaned_sql.split(';').collect();
        
        for part in parts {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                statements.push(trimmed.to_string());
            }
        }
        
        statements
    }

    /// 处理查询结果
    fn process_query_result(&self, rows: Vec<sqlx::mysql::MySqlRow>, start: std::time::Instant) -> DbResult<QueryResult> {
        use std::collections::HashMap;
        
        // 添加调试信息
        println!("查询返回的行数: {}", rows.len());
        
        let mut columns = Vec::new();
        let mut result_rows = Vec::new();

        if !rows.is_empty() {
            // 获取列名
            for column in rows[0].columns() {
                columns.push(column.name().to_string());
            }
            println!("查询返回的列名: {:?}", columns);

            // 转换行数据
            for (row_idx, row) in rows.iter().enumerate() {
                let mut row_map = HashMap::new();
                for (idx, column) in row.columns().iter().enumerate() {
                    // 尝试不同的数据类型获取
                    let value = if let Ok(s) = row.try_get::<String, _>(idx) {
                        serde_json::Value::String(s)
                    } else if let Ok(i) = row.try_get::<i64, _>(idx) {
                        serde_json::Value::Number(serde_json::Number::from(i))
                    } else if let Ok(f) = row.try_get::<f64, _>(idx) {
                        serde_json::Number::from_f64(f)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    } else if let Ok(b) = row.try_get::<bool, _>(idx) {
                        serde_json::Value::Bool(b)
                    } else {
                        // 如果都失败了，尝试获取原始字节并转换为字符串
                        match row.try_get::<Option<Vec<u8>>, _>(idx) {
                            Ok(Some(bytes)) => {
                                match String::from_utf8(bytes) {
                                    Ok(s) => serde_json::Value::String(s),
                                    Err(_) => serde_json::Value::Null,
                                }
                            }
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
        } else {
            println!("查询没有返回任何行数据");
        }

        let duration = start.elapsed();

        Ok(QueryResult {
            columns,
            rows: result_rows,
            affected_rows: rows.len() as u64,
            execution_time_ms: duration.as_millis(),
        })
    }
}

