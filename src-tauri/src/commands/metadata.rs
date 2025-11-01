use crate::database::{ColumnInfo, DatabaseInfo, TableInfo, QueryResult};
use crate::AppState;
use tauri::State;

/// 获取数据库列表
#[tauri::command]
pub async fn get_databases(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<DatabaseInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_databases(&connection_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取表列表
#[tauri::command]
pub async fn get_tables(
    connection_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_tables(&connection_id, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 获取表结构
#[tauri::command]
pub async fn get_table_structure(
    connection_id: String,
    table: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ColumnInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_table_structure(&connection_id, &table, schema.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 查看表数据
#[tauri::command]
pub async fn view_table_data(
    connection_id: String,
    table: String,
    database: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let limit_clause = match limit {
        Some(l) => format!(" LIMIT {}", l),
        None => " LIMIT 1000".to_string(),
    };
    
    let sql = format!("SELECT * FROM `{}`.`{}`{}", database, table, limit_clause);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 清空表数据
#[tauri::command]
pub async fn truncate_table(
    connection_id: String,
    table: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("TRUNCATE TABLE `{}`.`{}`", database, table);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除表
#[tauri::command]
pub async fn drop_table(
    connection_id: String,
    table: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("DROP TABLE `{}`.`{}`", database, table);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取视图列表
#[tauri::command]
pub async fn get_views(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT TABLE_NAME, TABLE_TYPE, NULL as ENGINE, NULL as TABLE_ROWS, 
                NULL as SIZE_MB, TABLE_COMMENT
         FROM information_schema.TABLES 
         WHERE TABLE_SCHEMA = '{}' AND TABLE_TYPE = 'VIEW'
         ORDER BY TABLE_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    let mut views = Vec::new();
    for row in result.rows {
        if let Some(name) = row.get("TABLE_NAME") {
            if let serde_json::Value::String(table_name) = name {
                views.push(TableInfo {
                    name: table_name.clone(),
                    schema: Some(database.clone()),
                    table_type: "VIEW".to_string(),
                    engine: None,
                    rows: None,
                    size_mb: None,
                    comment: row.get("TABLE_COMMENT")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }
    
    Ok(views)
}

/// 获取存储过程列表
#[tauri::command]
pub async fn get_procedures(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT ROUTINE_NAME, ROUTINE_TYPE, CREATED, LAST_ALTERED, ROUTINE_COMMENT
         FROM information_schema.ROUTINES 
         WHERE ROUTINE_SCHEMA = '{}' AND ROUTINE_TYPE = 'PROCEDURE'
         ORDER BY ROUTINE_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取函数列表
#[tauri::command]
pub async fn get_functions(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT ROUTINE_NAME, ROUTINE_TYPE, CREATED, LAST_ALTERED, ROUTINE_COMMENT
         FROM information_schema.ROUTINES 
         WHERE ROUTINE_SCHEMA = '{}' AND ROUTINE_TYPE = 'FUNCTION'
         ORDER BY ROUTINE_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取触发器列表
#[tauri::command]
pub async fn get_triggers(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT TRIGGER_NAME, EVENT_MANIPULATION, EVENT_OBJECT_TABLE, 
                ACTION_TIMING, CREATED
         FROM information_schema.TRIGGERS 
         WHERE TRIGGER_SCHEMA = '{}'
         ORDER BY TRIGGER_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取事件列表
#[tauri::command]
pub async fn get_events(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT EVENT_NAME, STATUS, EVENT_TYPE, EXECUTE_AT, 
                INTERVAL_VALUE, INTERVAL_FIELD, CREATED, LAST_ALTERED
         FROM information_schema.EVENTS 
         WHERE EVENT_SCHEMA = '{}'
         ORDER BY EVENT_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 删除视图
#[tauri::command]
pub async fn drop_view(
    connection_id: String,
    view: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("DROP VIEW `{}`.`{}`", database, view);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取视图定义
#[tauri::command]
pub async fn get_view_definition(
    connection_id: String,
    view: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT VIEW_DEFINITION FROM information_schema.VIEWS 
         WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'",
        database.replace("'", "''"),
        view.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(row) = result.rows.first() {
        if let Some(definition) = row.get("VIEW_DEFINITION") {
            if let serde_json::Value::String(def) = definition {
                return Ok(def.clone());
            }
        }
    }
    
    Err("未找到视图定义".to_string())
}

/// 删除存储过程
#[tauri::command]
pub async fn drop_procedure(
    connection_id: String,
    procedure: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("DROP PROCEDURE `{}`.`{}`", database, procedure);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除函数
#[tauri::command]
pub async fn drop_function(
    connection_id: String,
    function: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("DROP FUNCTION `{}`.`{}`", database, function);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除触发器
#[tauri::command]
pub async fn drop_trigger(
    connection_id: String,
    trigger: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("DROP TRIGGER `{}`.`{}`", database, trigger);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除事件
#[tauri::command]
pub async fn drop_event(
    connection_id: String,
    event: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("DROP EVENT `{}`.`{}`", database, event);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取表索引
#[tauri::command]
pub async fn get_table_indexes(
    connection_id: String,
    database: String,
    table: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT DISTINCT INDEX_NAME as index_name, COLUMN_NAME as column_name, 
                INDEX_TYPE as index_type, NON_UNIQUE as non_unique
         FROM information_schema.STATISTICS 
         WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'
         ORDER BY INDEX_NAME, SEQ_IN_INDEX",
        database.replace("'", "''"),
        table.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取表外键
#[tauri::command]
pub async fn get_table_foreign_keys(
    connection_id: String,
    database: String,
    table: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT CONSTRAINT_NAME as constraint_name, COLUMN_NAME as column_name,
                REFERENCED_TABLE_NAME as referenced_table_name,
                REFERENCED_COLUMN_NAME as referenced_column_name
         FROM information_schema.KEY_COLUMN_USAGE 
         WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'
               AND REFERENCED_TABLE_NAME IS NOT NULL
         ORDER BY CONSTRAINT_NAME",
        database.replace("'", "''"),
        table.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取创建表的DDL语句
#[tauri::command]
pub async fn get_create_table_ddl(
    connection_id: String,
    database: String,
    table: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!("SHOW CREATE TABLE `{}`.`{}`", database, table);
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(row) = result.rows.first() {
        // MySQL 返回的列名通常是 "Create Table"
        for (key, value) in row {
            if key.to_lowercase().contains("create") {
                if let serde_json::Value::String(ddl) = value {
                    return Ok(ddl.clone());
                }
            }
        }
    }
    
    Err("未找到DDL语句".to_string())
}

/// 自动补全数据结构
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AutoCompleteData {
    pub databases: Vec<String>,
    pub tables: Vec<TableSuggestion>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableSuggestion {
    pub name: String,
    pub database: String,
    pub columns: Vec<ColumnSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnSuggestion {
    pub name: String,
    pub data_type: String,
}

/// 获取自动补全数据
#[tauri::command]
pub async fn get_autocomplete_data(
    connection_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<AutoCompleteData, String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库列表
    let databases_info = manager
        .get_databases(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let databases: Vec<String> = databases_info.iter().map(|db| db.name.clone()).collect();
    
    // 获取表和列信息
    let mut tables = Vec::new();
    
    // 如果指定了数据库，只获取该数据库的表
    // 否则获取所有数据库的表
    let target_databases: Vec<String> = if let Some(db) = database {
        vec![db]
    } else {
        databases.clone()
    };
    
    for db_name in target_databases.iter() {
        // 获取该数据库的所有表
        let tables_info = manager
            .get_tables(&connection_id, Some(db_name))
            .await
            .unwrap_or_default();
        
        for table_info in tables_info {
            // 获取表的列信息
            let columns_info = manager
                .get_table_structure(&connection_id, &table_info.name, Some(db_name))
                .await
                .unwrap_or_default();
            
            let columns: Vec<ColumnSuggestion> = columns_info
                .iter()
                .map(|col| ColumnSuggestion {
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                })
                .collect();
            
            tables.push(TableSuggestion {
                name: table_info.name,
                database: db_name.clone(),
                columns,
            });
        }
    }
    
    // SQL 关键字列表
    let keywords = vec![
        "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP",
        "TABLE", "DATABASE", "INDEX", "VIEW", "PROCEDURE", "FUNCTION", "TRIGGER",
        "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "ON", "AS", "AND", "OR", "NOT",
        "IN", "BETWEEN", "LIKE", "IS", "NULL", "ORDER", "BY", "GROUP", "HAVING",
        "LIMIT", "OFFSET", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN",
        "ASC", "DESC", "SET", "VALUES", "INTO", "DEFAULT", "PRIMARY", "KEY",
        "FOREIGN", "REFERENCES", "UNIQUE", "CHECK", "CONSTRAINT", "CASCADE",
        "AUTO_INCREMENT", "UNSIGNED", "ZEROFILL", "BINARY", "COLLATE", "CHARSET",
        "ENGINE", "COMMENT", "IF", "EXISTS", "TEMPORARY", "TRUNCATE",
        "RENAME", "MODIFY", "CHANGE", "ADD", "COLUMN", "AFTER", "FIRST",
        "UNION", "ALL", "CASE", "WHEN", "THEN", "ELSE", "END",
        "CAST", "CONVERT", "SUBSTRING", "CONCAT", "LENGTH", "TRIM",
        "UPPER", "LOWER", "REPLACE", "DATE", "TIME", "TIMESTAMP", "NOW",
        "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND",
        "INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "DECIMAL", "NUMERIC",
        "FLOAT", "DOUBLE", "REAL", "VARCHAR", "CHAR", "TEXT", "BLOB",
        "DATE", "DATETIME", "TIMESTAMP", "TIME", "YEAR", "BOOLEAN", "BOOL",
        "GRANT", "REVOKE", "COMMIT", "ROLLBACK", "SAVEPOINT", "START", "TRANSACTION",
        "BEGIN", "USE", "SHOW", "DESCRIBE", "DESC", "EXPLAIN",
    ].iter().map(|s| s.to_string()).collect();
    
    Ok(AutoCompleteData {
        databases,
        tables,
        keywords,
    })
}