use crate::database::{QueryResult, BatchQueryResult, StatementResult};
use crate::database::{SqlSplitter, SqlDialect, is_query_statement, truncate_sql};
use crate::utils::sql_formatter::SqlFormatter;
use crate::AppState;
use tauri::State;
use std::time::Instant;

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

/// 执行 SQL 脚本（返回每条语句的详细执行结果）
/// 参考 DBeaver 的脚本执行方式
#[tauri::command]
pub async fn execute_sql_script(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<BatchQueryResult, String> {
    let total_start = Instant::now();
    
    // 使用SQL分割器分割脚本
    let mut splitter = SqlSplitter::new(SqlDialect::mysql());
    let statements = splitter.split(&sql);
    
    // 调试日志：打印分割后的语句数量
    println!("SQL分割完成: 共 {} 条语句", statements.len());
    
    if statements.is_empty() {
        return Ok(BatchQueryResult {
            statements: vec![],
            total_time_ms: 0,
            success_count: 0,
            failed_count: 0,
            total_affected_rows: 0,
        });
    }
    
    let manager = state.connection_manager.lock().await;
    let mut results: Vec<StatementResult> = Vec::new();
    let mut success_count = 0usize;
    let mut failed_count = 0usize;
    let mut total_affected_rows: u64 = 0;
    
    for (idx, stmt) in statements.iter().enumerate() {
        let stmt_start = Instant::now();
        let sql_preview = truncate_sql(stmt, 100);
        
        // 执行单条语句
        let result = manager
            .execute_query(&connection_id, stmt, database.as_deref())
            .await;
        
        let stmt_time = stmt_start.elapsed().as_millis();
        
        match result {
            Ok(query_result) => {
                success_count += 1;
                total_affected_rows += query_result.affected_rows;
                
                let is_query = is_query_statement(stmt);
                
                results.push(StatementResult {
                    sql: sql_preview,
                    success: true,
                    error: None,
                    affected_rows: query_result.affected_rows,
                    execution_time_ms: stmt_time,
                    is_query,
                    columns: query_result.columns,
                    rows: query_result.rows,
                });
            }
            Err(e) => {
                failed_count += 1;
                
                results.push(StatementResult {
                    sql: sql_preview,
                    success: false,
                    error: Some(e.to_string()),
                    affected_rows: 0,
                    execution_time_ms: stmt_time,
                    is_query: false,
                    columns: vec![],
                    rows: vec![],
                });
                
                // 参考 DBeaver：继续执行下一条语句，而不是中断
                println!("语句 {} 执行失败: {}，继续执行下一条", idx + 1, e);
            }
        }
    }
    
    let total_time = total_start.elapsed().as_millis();
    
    Ok(BatchQueryResult {
        statements: results,
        total_time_ms: total_time,
        success_count,
        failed_count,
        total_affected_rows,
    })
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
    schema: Option<String>,
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
        schema.as_deref(),
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
    schema: Option<String>,
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
        schema.as_deref(),
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
    schema: Option<String>,
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
        schema.as_deref(),
        &where_clause,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

