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
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let start = Instant::now();

        // PostgreSQL 中数据库切换需要重新连接，这里我们暂时忽略 database 参数
        // 因为 PostgreSQL 连接是基于特定数据库的
        // 注意：在 PostgreSQL 中，如果需要切换数据库，需要重新建立连接
        let _ = database; // 忽略 database 参数

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
                for row in &rows {
                    let mut row_map = HashMap::new();
                    for (idx, column) in row.columns().iter().enumerate() {
                        let value: Option<String> = row.try_get(idx).ok();
                        row_map.insert(
                            column.name().to_string(),
                            serde_json::Value::String(value.unwrap_or_default()),
                        );
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

    async fn get_tables(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

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

    async fn get_table_structure(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

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

