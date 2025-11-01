use async_trait::async_trait;
use sqlx::{Column, Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use std::time::Instant;

use super::traits::*;

/// SQLite 数据库连接
pub struct SqliteDatabase {
    pool: Option<Pool<Sqlite>>,
    config: Option<ConnectionConfig>,
}

impl SqliteDatabase {
    pub fn new() -> Self {
        Self {
            pool: None,
            config: None,
        }
    }

    /// 构建 SQLite 连接字符串
    fn build_connection_string(config: &ConnectionConfig) -> String {
        // SQLite 使用文件路径或 :memory:
        // 我们使用 database 字段作为文件路径
        if let Some(ref db_path) = config.database {
            if db_path.is_empty() || db_path == ":memory:" {
                "sqlite::memory:".to_string()
            } else {
                format!("sqlite:{}", db_path)
            }
        } else {
            // 如果没有指定数据库，使用主机字段作为路径（为了兼容）
            if config.host.is_empty() || config.host == ":memory:" {
                "sqlite::memory:".to_string()
            } else {
                format!("sqlite:{}", config.host)
            }
        }
    }
}

#[async_trait]
impl DatabaseOperations for SqliteDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let connection_string = Self::build_connection_string(config);
        
        match SqlitePool::connect(&connection_string).await {
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
        
        let pool = SqlitePool::connect(&connection_string)
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

    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let start = Instant::now();

        // SQLite 中数据库是单个文件，忽略 database 参数

        // 判断是否为查询语句
        let is_select = sql.trim().to_uppercase().starts_with("SELECT")
            || sql.trim().to_uppercase().starts_with("PRAGMA")
            || sql.trim().to_uppercase().starts_with("EXPLAIN");

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
        // SQLite 是单文件数据库，固定返回 "main" 作为数据库名称
        // 不显示文件路径，因为用户已经在连接名中看到了
        Ok(vec![DatabaseInfo {
            name: "main".to_string(),
            charset: Some("UTF-8".to_string()),
            collation: None,
        }])
    }

    async fn get_tables(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        let rows = sqlx::query(
            "SELECT 
                name,
                type
             FROM sqlite_master 
             WHERE type IN ('table', 'view') 
               AND name NOT LIKE 'sqlite_%'
             ORDER BY name"
        )
        .fetch_all(pool)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut tables = Vec::new();
        for row in rows {
            let table_name: String = row.try_get(0).unwrap_or_default();
            let table_type: String = row.try_get(1).unwrap_or_default();
            
            // 获取表的行数
            let count_sql = format!("SELECT COUNT(*) as count FROM \"{}\"", table_name);
            let count_row = sqlx::query(&count_sql)
                .fetch_one(pool)
                .await
                .ok();
            let row_count: Option<u64> = count_row
                .and_then(|r| r.try_get::<i64, _>(0).ok())
                .map(|n| n as u64);

            tables.push(TableInfo {
                name: table_name,
                schema: None,
                table_type: table_type.to_uppercase(),
                engine: Some("SQLite".to_string()),
                rows: row_count,
                size_mb: None,
                comment: None,
            });
        }

        Ok(tables)
    }

    async fn get_table_structure(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        // 使用 PRAGMA table_info 获取表结构
        let pragma_sql = format!("PRAGMA table_info(\"{}\")", table);
        let rows = sqlx::query(&pragma_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut columns = Vec::new();
        for row in rows {
            let column_name: String = row.try_get(1).unwrap_or_default();
            let data_type: String = row.try_get(2).unwrap_or_default();
            let not_null: i64 = row.try_get(3).unwrap_or(0);
            let default_value: Option<String> = row.try_get(4).ok();
            let is_pk: i64 = row.try_get(5).unwrap_or(0);

            columns.push(ColumnInfo {
                name: column_name,
                data_type,
                nullable: not_null == 0,
                default_value,
                is_primary_key: is_pk > 0,
                is_auto_increment: false, // SQLite 的 AUTOINCREMENT 需要额外查询
                comment: None,
                character_maximum_length: None,
                numeric_precision: None,
                numeric_scale: None,
            });
        }

        Ok(columns)
    }

    async fn get_indexes(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DbError::ConnectionFailed("未连接到数据库".to_string()))?;

        // 获取索引列表
        let pragma_sql = format!("PRAGMA index_list(\"{}\")", table);
        let rows = sqlx::query(&pragma_sql)
            .fetch_all(pool)
            .await
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut indexes = Vec::new();
        for row in rows {
            let index_name: String = row.try_get(1).unwrap_or_default();
            let is_unique: i64 = row.try_get(2).unwrap_or(0);
            
            // 获取索引的列信息
            let index_info_sql = format!("PRAGMA index_info(\"{}\")", index_name);
            let col_rows = sqlx::query(&index_info_sql)
                .fetch_all(pool)
                .await
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let mut columns = Vec::new();
            for col_row in col_rows {
                let column_name: String = col_row.try_get(2).unwrap_or_default();
                columns.push(column_name);
            }

            indexes.push(IndexInfo {
                name: index_name.clone(),
                columns,
                is_unique: is_unique > 0,
                is_primary: index_name.starts_with("sqlite_autoindex"),
                index_type: "BTREE".to_string(),
            });
        }

        Ok(indexes)
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

