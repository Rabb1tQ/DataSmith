use async_trait::async_trait;
use sqlx::{Column, PgPool, Pool, Postgres, Row};
use std::collections::HashMap;
use std::time::Instant;
use url::Url;

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
        let mut url = Url::parse(&format!("postgres://{}:{}/", config.host, config.port))
            .expect("Invalid PostgreSQL connection URL");
        
        url.set_username(&config.username).unwrap();
        url.set_password(Some(&config.password)).unwrap();

        // 检查数据库名称是否存在且不为空字符串
        if let Some(ref database) = config.database {
            if !database.trim().is_empty() {
                url.set_path(database);
            } else {
                url.set_path("postgres"); // 空字符串时使用默认数据库
            }
        } else {
            url.set_path("postgres"); // 默认数据库
        }

        // SSL 配置
        if config.ssl {
            url.query_pairs_mut().append_pair("sslmode", "require");
        } else {
            url.query_pairs_mut().append_pair("sslmode", "prefer");
        }

        url.to_string()
    }

    /// 使用指定的连接池执行查询
    async fn execute_query_with_pool(
        &self,
        pool: &Pool<Postgres>,
        sql: &str,
    ) -> DbResult<QueryResult> {
        let start = Instant::now();

        // 智能分割SQL语句
        let statements = self.split_sql_statements(sql);
        println!("PostgreSQL 分割后的SQL语句数量: {}", statements.len());
        
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
            return self.execute_single_statement_pg(pool, &statements[0], start).await;
        }
        
        // 多条语句：依次执行
        let mut total_affected_rows: u64 = 0;
        let mut last_query_result: Option<QueryResult> = None;
        
        for (idx, stmt) in statements.iter().enumerate() {
            println!("执行第 {} 条SQL: {}", idx + 1, stmt);
            
            // 判断是否为查询语句
            let is_select = stmt.trim().to_uppercase().starts_with("SELECT")
                || stmt.trim().to_uppercase().starts_with("SHOW")
                || stmt.trim().to_uppercase().starts_with("EXPLAIN")
                || stmt.trim().to_uppercase().starts_with("WITH");
            
            if is_select {
                // 查询语句
                let rows = sqlx::query(stmt)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("语句 {} 执行失败: {}", idx + 1, e)))?;
                
                // 保存最后一个查询结果
                last_query_result = Some(self.process_query_result_pg(rows, start)?);
            } else {
                // 非查询语句
                let result = sqlx::query(stmt)
                    .execute(pool)
                    .await
                    .map_err(|e| DbError::QueryFailed(format!("语句 {} 执行失败: {}", idx + 1, e)))?;
                
                total_affected_rows += result.rows_affected();
            }
        }
        
        // 返回结果：如果有查询结果则返回，否则返回累积的影响行数
        if let Some(query_result) = last_query_result {
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
    
    /// 执行单条SQL语句（PostgreSQL）
    async fn execute_single_statement_pg(
        &self,
        pool: &Pool<Postgres>,
        sql: &str,
        start: std::time::Instant,
    ) -> DbResult<QueryResult> {
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

            self.process_query_result_pg(rows, start)
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
    
    /// 处理查询结果（PostgreSQL）
    fn process_query_result_pg(
        &self,
        rows: Vec<sqlx::postgres::PgRow>,
        start: std::time::Instant,
    ) -> DbResult<QueryResult> {
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
                    // 尝试多种数据类型获取
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
                        match String::from_utf8(bytes.clone()) {
                            Ok(s) => serde_json::Value::String(s),
                            Err(_) => {
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
                        json
                    } else if let Ok(uuid) = row.try_get::<sqlx::types::Uuid, _>(idx) {
                        serde_json::Value::String(uuid.to_string())
                    } else if let Ok(None) = row.try_get::<Option<String>, _>(idx) {
                        serde_json::Value::Null
                    } else {
                        match row.try_get::<Option<String>, _>(idx) {
                            Ok(Some(s)) => serde_json::Value::String(s),
                            _ => serde_json::Value::Null,
                        }
                    };
                    
                    row_map.insert(column.name().to_string(), value.clone());
                    
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
    }

    /// 使用指定连接池获取表结构
    async fn get_table_structure_with_pool(
        &self,
        pool: &Pool<Postgres>,
        table: &str,
        schema: Option<&str>,
    ) -> DbResult<Vec<ColumnInfo>> {
        let schema_name = schema.unwrap_or("public");

        // 使用 pg_attribute 和 pg_class 获取列信息
        // PostgreSQL 表名大小写敏感：未加引号的标识符会被转为小写存储
        // 所以需要同时尝试原始名称和小写名称
        let table_lower = table.to_lowercase();
        
        // 先尝试精确匹配，再尝试小写匹配
        let rows = sqlx::query(
            "SELECT
                a.attname as column_name,
                pg_catalog.format_type(a.atttypid, a.atttypmod) as data_type,
                NOT a.attnotnull as is_nullable,
                pg_get_expr(d.adbin, d.adrelid) as column_default,
                CASE
                    WHEN t.typname IN ('varchar', 'char', 'bpchar') THEN a.atttypmod - 4
                    ELSE NULL
                END as character_maximum_length,
                CASE
                    WHEN t.typname IN ('numeric', 'decimal') THEN ((a.atttypmod - 4) >> 16) & 65535
                    ELSE NULL
                END as numeric_precision,
                CASE
                    WHEN t.typname IN ('numeric', 'decimal') THEN (a.atttypmod - 4) & 65535
                    ELSE NULL
                END as numeric_scale,
                col_description(c.oid, a.attnum) as comment
             FROM pg_catalog.pg_attribute a
             JOIN pg_catalog.pg_class c ON a.attrelid = c.oid
             JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
             LEFT JOIN pg_catalog.pg_type t ON a.atttypid = t.oid
             LEFT JOIN pg_catalog.pg_attrdef d ON (a.attrelid, a.attnum) = (d.adrelid, d.adnum)
             WHERE n.nspname = $1
               AND (c.relname = $2 OR c.relname = $3)
               AND a.attnum > 0
               AND NOT a.attisdropped
             ORDER BY a.attnum"
        )
        .bind(schema_name)
        .bind(table)
        .bind(&table_lower)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        // 获取主键信息 - 同时尝试原始名称和小写名称
        let pk_rows = sqlx::query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             JOIN pg_class c ON c.oid = i.indrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = $1 AND (c.relname = $2 OR c.relname = $3) AND i.indisprimary"
        )
        .bind(schema_name)
        .bind(table)
        .bind(&table_lower)
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let primary_keys: Vec<String> = pk_rows
            .iter()
            .map(|row| row.try_get::<String, _>(0).unwrap_or_default())
            .collect();

        let mut columns = Vec::new();
        for row in rows {
            let column_name: String = row.try_get(0).unwrap_or_default();
            let data_type: String = row.try_get(1).unwrap_or_default();
            let is_nullable: bool = row.try_get(2).unwrap_or(true);
            let column_default: Option<String> = row.try_get(3).ok();

            let is_auto_increment = column_default
                .as_ref()
                .map(|s| s.contains("nextval"))
                .unwrap_or(false);

            columns.push(ColumnInfo {
                name: column_name.clone(),
                data_type,
                nullable: is_nullable,
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
             WHERE datistemplate = false
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
                    // 使用 pg_class 和 pg_namespace 获取原始表名（保持大小写）
                    let rows = sqlx::query(
                        "SELECT
                            n.nspname as schemaname,
                            c.relname as tablename,
                            'TABLE' as table_type,
                            NULL as engine,
                            NULL as table_rows,
                            pg_total_relation_size(c.oid)::bigint / 1024 / 1024 as size_mb,
                            obj_description(c.oid) as comment
                         FROM pg_class c
                         JOIN pg_namespace n ON n.oid = c.relnamespace
                         WHERE c.relkind = 'r'
                           AND n.nspname NOT IN ('pg_catalog', 'information_schema')
                         ORDER BY n.nspname, c.relname"
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
        // 使用 pg_class 和 pg_namespace 获取原始表名（保持大小写）
        let rows = sqlx::query(
            "SELECT
                n.nspname as schemaname,
                c.relname as tablename,
                'TABLE' as table_type,
                NULL as engine,
                NULL as table_rows,
                pg_total_relation_size(c.oid)::bigint / 1024 / 1024 as size_mb,
                obj_description(c.oid) as comment
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relkind = 'r'
               AND n.nspname NOT IN ('pg_catalog', 'information_schema')
             ORDER BY n.nspname, c.relname"
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
                    // 使用 pg_class 和 pg_namespace 获取原始视图名（保持大小写）
                    let rows = sqlx::query(
                        "SELECT
                            n.nspname as schemaname,
                            c.relname as viewname,
                            'VIEW' as table_type,
                            NULL as engine,
                            NULL as table_rows,
                            pg_total_relation_size(c.oid)::bigint / 1024 / 1024 as size_mb,
                            obj_description(c.oid) as comment
                         FROM pg_class c
                         JOIN pg_namespace n ON n.oid = c.relnamespace
                         WHERE c.relkind = 'v'
                           AND n.nspname NOT IN ('pg_catalog', 'information_schema')
                         ORDER BY n.nspname, c.relname"
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
        // 使用 pg_class 和 pg_namespace 获取原始视图名（保持大小写）
        let rows = sqlx::query(
            "SELECT
                n.nspname as schemaname,
                c.relname as viewname,
                'VIEW' as table_type,
                NULL as engine,
                NULL as table_rows,
                pg_total_relation_size(c.oid)::bigint / 1024 / 1024 as size_mb,
                obj_description(c.oid) as comment
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relkind = 'v'
               AND n.nspname NOT IN ('pg_catalog', 'information_schema')
             ORDER BY n.nspname, c.relname"
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
    
    async fn get_table_options(&self, table: &str, schema: Option<&str>) -> DbResult<TableOptions> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;
        let schema_name = schema.unwrap_or("public");

        let row = sqlx::query(
            "SELECT obj_description(c.oid) as comment
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relname = $1 AND n.nspname = $2 AND c.relkind = 'r'"
        )
        .bind(table)
        .bind(schema_name)
        .fetch_optional(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let comment = row.and_then(|r| r.try_get("comment").ok());

        Ok(TableOptions {
            engine: None,
            charset: None,
            collation: None,
            comment,
            auto_increment: None,
        })
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl PostgreSqlDatabase {
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
                // 双引号标识符（PostgreSQL中双引号用于标识符）
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
                // 美元符号引用字符串（PostgreSQL特有）
                '$' => {
                    // 检查是否是美元符号引用
                    let mut tag = String::new();
                    tag.push('$');
                    
                    // 读取标签
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            tag.push(c);
                            chars.next();
                        } else if c == '$' {
                            tag.push('$');
                            chars.next();
                            break;
                        } else {
                            break;
                        }
                    }
                    
                    current_statement.push_str(&tag);
                    
                    // 如果标签以$结尾，查找结束标签
                    if tag.ends_with('$') && tag.len() > 1 {
                        let end_tag = tag.clone();
                        let mut content = String::new();
                        
                        while let Some(c) = chars.next() {
                            content.push(c);
                            if content.ends_with(&end_tag) {
                                break;
                            }
                        }
                        
                        current_statement.push_str(&content);
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
}