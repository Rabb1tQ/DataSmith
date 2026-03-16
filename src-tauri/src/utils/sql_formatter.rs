use crate::database::DatabaseType;

/// SQL 格式化工具，用于适配不同数据库的 SQL 语法
pub struct SqlFormatter;

impl SqlFormatter {
    /// 格式化 SELECT 语句
    #[allow(dead_code)]
    pub fn format_select(
        db_type: &DatabaseType,
        database: &str,
        table: &str,
        schema: Option<&str>,
        columns: &str,
        where_clause: Option<&str>,
        limit: Option<u64>,
    ) -> String {
        let table_ref = Self::format_table_ref(db_type, database, table, schema);
        let mut sql = format!("SELECT {} FROM {}", columns, table_ref);
        
        if let Some(where_cond) = where_clause {
            sql.push_str(&format!(" WHERE {}", where_cond));
        }
        
        if let Some(lim) = limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
        
        sql
    }
    
    /// 格式化 UPDATE 语句
    pub fn format_update(
        db_type: &DatabaseType,
        database: &str,
        table: &str,
        schema: Option<&str>,
        column: &str,
        value: Option<&str>,
        where_clause: &str,
    ) -> String {
        let table_ref = Self::format_table_ref(db_type, database, table, schema);
        let column_ref = Self::quote_identifier(db_type, column);
        let value_str = if let Some(v) = value {
            format!("'{}'", v.replace("'", "''"))
        } else {
            "NULL".to_string()
        };
        
        format!(
            "UPDATE {} SET {} = {} WHERE {}",
            table_ref, column_ref, value_str, where_clause
        )
    }
    
    /// 格式化 INSERT 语句
    pub fn format_insert(
        db_type: &DatabaseType,
        database: &str,
        table: &str,
        schema: Option<&str>,
        columns: &[String],
        values: &[String],
    ) -> String {
        let table_ref = Self::format_table_ref(db_type, database, table, schema);
        let quoted_columns: Vec<String> = columns
            .iter()
            .map(|col| Self::quote_identifier(db_type, col))
            .collect();
        
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_ref,
            quoted_columns.join(", "),
            values.join(", ")
        )
    }
    
    /// 格式化 DELETE 语句
    pub fn format_delete(
        db_type: &DatabaseType,
        database: &str,
        table: &str,
        schema: Option<&str>,
        where_clause: &str,
    ) -> String {
        let table_ref = Self::format_table_ref(db_type, database, table, schema);
        format!("DELETE FROM {} WHERE {}", table_ref, where_clause)
    }
    
    /// 格式化表引用（database.table 或 table）
    fn format_table_ref(db_type: &DatabaseType, database: &str, table: &str, schema: Option<&str>) -> String {
        match db_type {
            DatabaseType::SQLite => {
                // SQLite 不使用 database.table 格式，只使用表名
                Self::quote_identifier(db_type, table)
            }
            DatabaseType::MySQL => {
                // MySQL 使用 `database`.`table`
                format!(
                    "{}.{}",
                    Self::quote_identifier(db_type, database),
                    Self::quote_identifier(db_type, table)
                )
            }
            DatabaseType::PostgreSQL => {
                // PostgreSQL 使用 schema.table 格式
                // 如果提供了 schema 参数，使用 schema.table
                // 否则默认使用 public schema
                let schema_name = schema.unwrap_or("public");
                format!(
                    "{}.{}",
                    Self::quote_identifier(db_type, schema_name),
                    Self::quote_identifier(db_type, table)
                )
            }
            // 为未来的数据库类型预留
            _ => {
                // 默认使用 MySQL 风格
                format!(
                    "{}.{}",
                    Self::quote_identifier(db_type, database),
                    Self::quote_identifier(db_type, table)
                )
            }
        }
    }
    
    /// 根据数据库类型引用标识符（列名、表名等）
    pub fn quote_identifier(db_type: &DatabaseType, identifier: &str) -> String {
        match db_type {
            DatabaseType::SQLite => {
                // SQLite 使用双引号
                format!("\"{}\"", identifier)
            }
            DatabaseType::MySQL => {
                // MySQL 使用反引号
                format!("`{}`", identifier)
            }
            DatabaseType::PostgreSQL => {
                // PostgreSQL 使用双引号
                format!("\"{}\"", identifier)
            }
            // 为未来的数据库类型预留
            _ => {
                // 默认使用反引号
                format!("`{}`", identifier)
            }
        }
    }
}

