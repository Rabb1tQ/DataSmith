use crate::database::QueryResult;
use crate::AppState;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use tauri::State;

/// 导出为 CSV
#[tauri::command]
pub async fn export_to_csv(
    data: QueryResult,
    file_path: String,
) -> Result<bool, String> {
    let mut file = File::create(&file_path).map_err(|e| e.to_string())?;
    
    // 写入表头
    let header = data.columns.join(",");
    writeln!(file, "{}", header).map_err(|e| e.to_string())?;
    
    // 写入数据行
    for row in &data.rows {
        let values: Vec<String> = data
            .columns
            .iter()
            .map(|col| {
                let val = row.get(col).unwrap_or(&Value::Null);
                match val {
                    Value::String(s) => format!("\"{}\"", s.replace("\"", "\"\"")),
                    Value::Null => String::new(),
                    _ => val.to_string(),
                }
            })
            .collect();
        writeln!(file, "{}", values.join(",")).map_err(|e| e.to_string())?;
    }
    
    Ok(true)
}

/// 导出为 JSON
#[tauri::command]
pub async fn export_to_json(
    data: QueryResult,
    file_path: String,
) -> Result<bool, String> {
    let json = serde_json::to_string_pretty(&data.rows).map_err(|e| e.to_string())?;
    
    let mut file = File::create(&file_path).map_err(|e| e.to_string())?;
    file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    
    Ok(true)
}

/// 导出为 SQL INSERT 语句
#[tauri::command]
pub async fn export_to_sql(
    data: QueryResult,
    table_name: String,
    file_path: String,
) -> Result<bool, String> {
    let mut file = File::create(&file_path).map_err(|e| e.to_string())?;
    
    for row in &data.rows {
        let columns = data.columns.join("`, `");
        let values: Vec<String> = data
            .columns
            .iter()
            .map(|col| {
                let val = row.get(col).unwrap_or(&Value::Null);
                match val {
                    Value::String(s) => format!("'{}'", s.replace("'", "''")),
                    Value::Null => "NULL".to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                    _ => format!("'{}'", val.to_string()),
                }
            })
            .collect();
        
        let insert_stmt = format!(
            "INSERT INTO `{}` (`{}`) VALUES ({});\n",
            table_name,
            columns,
            values.join(", ")
        );
        
        file.write_all(insert_stmt.as_bytes()).map_err(|e| e.to_string())?;
    }
    
    Ok(true)
}

/// 导出表结构为 DDL
#[tauri::command]
pub async fn export_table_ddl(
    connection_id: String,
    database: String,
    table: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.connection_manager.lock().await;

    // 获取表的 CREATE TABLE 语句
    let sql = format!("SHOW CREATE TABLE `{}`.`{}`", database, table);
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| format!("获取表结构失败: {}", e))?;

    // SHOW CREATE TABLE 返回的结果中，第二列是 CREATE TABLE 语句
    if let Some(row) = result.rows.first() {
        if let Some(create_stmt) = row.get("Create Table") {
            if let Some(ddl) = create_stmt.as_str() {
                return Ok(ddl.to_string());
            }
        }
    }

    Err(format!("无法获取表 {}.{} 的结构", database, table))
}

