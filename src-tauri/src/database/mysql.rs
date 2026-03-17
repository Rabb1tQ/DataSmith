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

        // 检查数据库名称是否存在且不为空字符串
        if let Some(ref database) = config.database {
            if !database.trim().is_empty() {
                url.push_str(&format!("/{}", database));
            }
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

        // 智能分割SQL语句
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
        
        // 如果只有一条语句，直接执行
        if statements.len() == 1 {
            return self.execute_single_statement(pool, &statements[0], database, start).await;
        }
        
        // 多条语句：在同一连接上依次执行
        let mut conn = pool.acquire()
            .await
            .map_err(|e| DbError::QueryFailed(format!("获取连接失败: {}", e)))?;
        
        // 如果指定了数据库，先切换数据库上下文
        if let Some(db_name) = database {
            if !db_name.is_empty() {
                let use_sql = format!("USE `{}`", db_name);
                println!("设置数据库上下文: {}", use_sql);
                conn.execute(use_sql.as_str())
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("切换数据库失败: {}", e)))?;
            }
        }
        
        // 执行所有语句，累积结果
        let mut total_affected_rows: u64 = 0;
        let mut last_query_result: Option<QueryResult> = None;
        
        for (idx, stmt) in statements.iter().enumerate() {
            println!("执行第 {} 条SQL: {}", idx + 1, stmt);
            
            // 判断是否为查询语句
            let is_select = stmt.trim().to_uppercase().starts_with("SELECT")
                || stmt.trim().to_uppercase().starts_with("SHOW")
                || stmt.trim().to_uppercase().starts_with("DESCRIBE")
                || stmt.trim().to_uppercase().starts_with("EXPLAIN");
            
            if is_select {
                // 查询语句
                let rows = sqlx::query(stmt)
                    .fetch_all(&mut *conn)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("语句 {} 执行失败: {}", idx + 1, e)))?;
                
                // 保存最后一个查询结果
                last_query_result = Some(self.process_query_result_with_start(rows, start)?);
            } else {
                // 非查询语句
                let result = sqlx::query(stmt)
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("语句 {} 执行失败: {}", idx + 1, e)))?;
                
                total_affected_rows += result.rows_affected();
            }
        }
        
        // 返回结果：如果有查询结果则返回，否则返回累积的影响行数
        if let Some(query_result) = last_query_result {
            // 合并影响行数
            let mut result = query_result;
            result.affected_rows += total_affected_rows;
            Ok(result)
        } else {
            let duration = start.elapsed();
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: total_affected_rows,
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
             ORDER BY SCHEMA_NAME"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut databases = Vec::new();
        for row in rows {
            let name_bytes: Vec<u8> = row
                .try_get("SCHEMA_NAME")
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let name = String::from_utf8_lossy(&name_bytes).into_owned();
            let charset: Option<String> =
                row.try_get("DEFAULT_CHARACTER_SET_NAME").ok();

            let collation: Option<String> =
                row.try_get("DEFAULT_COLLATION_NAME").ok();

            databases.push(DatabaseInfo {
                name,
                charset,
                collation,
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
            let name_bytes: Vec<u8> = row.try_get("TABLE_NAME")
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;
            let name = String::from_utf8_lossy(&name_bytes).into_owned();

            let table_type_bytes: Vec<u8> = row.try_get("TABLE_TYPE")
                .unwrap_or_else(|_| Vec::new());
            let table_type = String::from_utf8_lossy(&table_type_bytes).into_owned();

            tables.push(TableInfo {
                name,
                schema: Some(db_name.to_string()),
                table_type,
                engine: row.try_get("ENGINE").ok(),
                rows: row.try_get("TABLE_ROWS").ok(),
                size_mb: row.try_get("SIZE_MB").ok(),
                comment: row.try_get::<Vec<u8>, _>("TABLE_COMMENT").ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned()),
            });
        }

        Ok(tables)
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>, _database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
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
            let name_bytes: Vec<u8> = row.try_get("COLUMN_NAME")
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;
            let name = String::from_utf8_lossy(&name_bytes).into_owned();

            let data_type_bytes: Vec<u8> = row.try_get("DATA_TYPE")
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;
            let data_type = String::from_utf8_lossy(&data_type_bytes).into_owned();

            let is_nullable_bytes: Vec<u8> = row.try_get("IS_NULLABLE")
                .unwrap_or_else(|_| Vec::new());
            let is_nullable = String::from_utf8_lossy(&is_nullable_bytes).into_owned();

            let column_key_bytes: Vec<u8> = row.try_get("COLUMN_KEY")
                .unwrap_or_else(|_| Vec::new());
            let column_key = String::from_utf8_lossy(&column_key_bytes).into_owned();

            let extra_bytes: Vec<u8> = row.try_get("EXTRA")
                .unwrap_or_else(|_| Vec::new());
            let extra = String::from_utf8_lossy(&extra_bytes).into_owned();

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: is_nullable.to_uppercase() == "YES",
                default_value: row.try_get::<Vec<u8>, _>("COLUMN_DEFAULT").ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned()),
                is_primary_key: column_key == "PRI",
                is_auto_increment: extra.contains("auto_increment"),
                comment: row.try_get::<Vec<u8>, _>("COLUMN_COMMENT").ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned()),
                character_maximum_length: row.try_get("CHARACTER_MAXIMUM_LENGTH").ok(),
                numeric_precision: row.try_get("NUMERIC_PRECISION").ok(),
                numeric_scale: row.try_get("NUMERIC_SCALE").ok(),
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
            let index_name_bytes: Vec<u8> = row.try_get("INDEX_NAME")
                .unwrap_or_else(|_| Vec::new());
            let index_name = String::from_utf8_lossy(&index_name_bytes).into_owned();

            let column_name_bytes: Vec<u8> = row.try_get("COLUMN_NAME")
                .unwrap_or_else(|_| Vec::new());
            let column_name = String::from_utf8_lossy(&column_name_bytes).into_owned();

            let non_unique: i32 = row.try_get("NON_UNIQUE").unwrap_or(1);

            let index_type_bytes: Vec<u8> = row.try_get("INDEX_TYPE")
                .unwrap_or_else(|_| Vec::new());
            let index_type = String::from_utf8_lossy(&index_type_bytes).into_owned();

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
    /// 分割SQL语句：智能处理字符串、注释中的分号
    fn split_sql_statements(&self, sql: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut chars = sql.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                // 单行注释 --
                '-' => {
                    if chars.peek() == Some(&'-') {
                        // 跳过直到行尾
                        chars.next(); // 消耗第二个 -
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                chars.next();
                                break;
                            }
                            chars.next();
                        }
                    } else {
                        current_statement.push(ch);
                    }
                }
                // 多行注释 /* */
                '/' => {
                    if chars.peek() == Some(&'*') {
                        chars.next(); // 消耗 *
                        while let Some(c) = chars.next() {
                            if c == '*' && chars.peek() == Some(&'/') {
                                chars.next(); // 消耗 /
                                break;
                            }
                        }
                    } else {
                        current_statement.push(ch);
                    }
                }
                // 单引号字符串
                '\'' => {
                    current_statement.push(ch);
                    while let Some(c) = chars.next() {
                        current_statement.push(c);
                        if c == '\'' {
                            // 检查是否是转义的单引号 ''
                            if chars.peek() == Some(&'\'') {
                                current_statement.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                    }
                }
                // 双引号字符串
                '"' => {
                    current_statement.push(ch);
                    while let Some(c) = chars.next() {
                        current_statement.push(c);
                        if c == '"' {
                            // 检查是否是转义的双引号 ""
                            if chars.peek() == Some(&'"') {
                                current_statement.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                    }
                }
                // 反引号标识符
                '`' => {
                    current_statement.push(ch);
                    while let Some(c) = chars.next() {
                        current_statement.push(c);
                        if c == '`' {
                            break;
                        }
                    }
                }
                // 分号 - 语句结束
                ';' => {
                    let trimmed = current_statement.trim();
                    if !trimmed.is_empty() {
                        statements.push(trimmed.to_string());
                    }
                    current_statement.clear();
                }
                // 其他字符
                _ => {
                    current_statement.push(ch);
                }
            }
        }
        
        // 处理最后一条语句（没有分号结尾的情况）
        let trimmed = current_statement.trim();
        if !trimmed.is_empty() {
            statements.push(trimmed.to_string());
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

    /// 处理查询结果（带自定义开始时间）
    fn process_query_result_with_start(&self, rows: Vec<sqlx::mysql::MySqlRow>, start: std::time::Instant) -> DbResult<QueryResult> {
        self.process_query_result(rows, start)
    }

    /// 执行单条SQL语句
    async fn execute_single_statement(
        &self,
        pool: &sqlx::Pool<sqlx::MySql>,
        sql: &str,
        database: Option<&str>,
        start: std::time::Instant,
    ) -> DbResult<QueryResult> {
        use sqlx::Executor;
        
        println!("执行单条SQL: {}", sql);

        // 判断是否为查询语句
        let is_select = sql.trim().to_uppercase().starts_with("SELECT")
            || sql.trim().to_uppercase().starts_with("SHOW")
            || sql.trim().to_uppercase().starts_with("DESCRIBE")
            || sql.trim().to_uppercase().starts_with("EXPLAIN");

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
                    let rows = sqlx::query(sql)
                        .fetch_all(&mut *conn)
                        .await
                        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
                        
                    return self.process_query_result(rows, start);
                }
            }
            
            // 没有指定数据库，直接在池上执行
            let rows = sqlx::query(sql)
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
                    let result = sqlx::query(sql)
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
            let result = sqlx::query(sql)
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
}
