use crate::database::QueryResult;
use crate::utils::sql_formatter::SqlFormatter;
use crate::AppState;
use tauri::State;

/// 执行 SQL 查询
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .execute_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 批量执行 SQL 查询
#[tauri::command]
pub async fn execute_query_batch(
    connection_id: String,
    sqls: Vec<String>,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let mut results = Vec::new();
    
    for sql in sqls {
        let result = execute_query(connection_id.clone(), sql, database.clone(), state.clone()).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// 更新表数据
#[tauri::command]
pub async fn update_table_data(
    connection_id: String,
    database: String,
    table: String,
    column: String,
    value: Option<String>,
    where_clause: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 使用 SqlFormatter 构建 UPDATE SQL 语句
    let sql = SqlFormatter::format_update(
        &db_type,
        &database,
        &table,
        &column,
        value.as_deref(),
        &where_clause,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// 插入表数据
#[tauri::command]
pub async fn insert_table_data(
    connection_id: String,
    database: String,
    table: String,
    data: std::collections::HashMap<String, Option<String>>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let columns: Vec<String> = data.keys().cloned().collect();
    let values: Vec<String> = data.values().map(|v| {
        if let Some(val) = v {
            format!("'{}'", val.replace("'", "''"))
        } else {
            "NULL".to_string()
        }
    }).collect();
    
    // 使用 SqlFormatter 构建 INSERT SQL 语句
    let sql = SqlFormatter::format_insert(
        &db_type,
        &database,
        &table,
        &columns,
        &values,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// 删除表数据
#[tauri::command]
pub async fn delete_table_data(
    connection_id: String,
    database: String,
    table: String,
    where_clause: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 使用 SqlFormatter 构建 DELETE SQL 语句
    let sql = SqlFormatter::format_delete(
        &db_type,
        &database,
        &table,
        &where_clause,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

