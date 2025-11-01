use crate::database::{ConnectionConfig, DatabaseType};
use crate::models::{ConnectionTestResult, StoredConnection};
use crate::utils::crypto;
use crate::AppState;
use serde_json::{json, Value};
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

/// 将 StoredConnection 转换为 ConnectionConfig
fn stored_to_config(stored: &StoredConnection) -> Result<ConnectionConfig, String> {
    // 解密密码
    let password = if let Some(ref encrypted) = stored.encrypted_password {
        crypto::decrypt_password(encrypted)?
    } else {
        String::new()
    };
    
    Ok(stored_to_config_with_password(stored, &password))
}

/// 将 StoredConnection 转换为 ConnectionConfig（带明文密码）
fn stored_to_config_with_password(stored: &StoredConnection, password: &str) -> ConnectionConfig {
    let db_type = match stored.db_type.as_str() {
        "mysql" => DatabaseType::MySQL,
        "postgresql" => DatabaseType::PostgreSQL,
        "sqlite" => DatabaseType::SQLite,
        "mongodb" => DatabaseType::MongoDB,
        "redis" => DatabaseType::Redis,
        _ => DatabaseType::MySQL,
    };

    ConnectionConfig {
        id: stored.id.clone(),
        name: stored.name.clone(),
        db_type,
        host: stored.host.clone(),
        port: stored.port,
        username: stored.username.clone(),
        password: password.to_string(),
        database: stored.database.clone(),
        ssl: stored.ssl,
        connection_timeout: stored.connection_timeout,
        pool_size: stored.pool_size,
    }
}

/// 测试数据库连接
#[tauri::command]
pub async fn test_connection(
    config: Value,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, String> {
    let start = std::time::Instant::now();

    // 解析配置
    let conn_config: ConnectionConfig =
        serde_json::from_value(config).map_err(|e| e.to_string())?;

    // 使用连接管理器测试连接
    let manager = state.connection_manager.lock().await;
    let result = manager.test_connection(&conn_config).await;

    match result {
        Ok(_) => Ok(ConnectionTestResult {
            success: true,
            message: "连接成功".to_string(),
            version: Some("8.0.35".to_string()), // TODO: 获取实际版本
            ping_time_ms: start.elapsed().as_millis(),
        }),
        Err(e) => Ok(ConnectionTestResult {
            success: false,
            message: e.to_string(),
            version: None,
            ping_time_ms: start.elapsed().as_millis(),
        }),
    }
}

/// 保存连接配置
#[tauri::command]
pub async fn save_connection(
    app: AppHandle,
    mut connection: StoredConnection,
    password: Option<String>,
) -> Result<StoredConnection, String> {
    let store = app
        .store("connections.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    // 如果提供了密码，加密并保存
    if let Some(pwd) = password {
        if !pwd.is_empty() {
            connection.encrypted_password = Some(crypto::encrypt_password(&pwd)?);
        }
    }

    // 保存连接信息
    store.set(connection.id.clone(), json!(connection));
    
    store.save().map_err(|e| e.to_string())?;

    Ok(connection)
}

/// 更新连接配置
#[tauri::command]
pub async fn update_connection(
    app: AppHandle,
    mut connection: StoredConnection,
    password: Option<String>,
) -> Result<StoredConnection, String> {
    let store = app
        .store("connections.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    // 检查连接是否存在
    if !store.has(connection.id.clone()) {
        return Err("连接配置不存在".to_string());
    }

    // 如果提供了密码，加密并保存
    if let Some(pwd) = password {
        if !pwd.is_empty() {
            connection.encrypted_password = Some(crypto::encrypt_password(&pwd)?);
        }
    } else {
        // 如果没有提供新密码，保留原有密码
        if let Some(existing_value) = store.get(connection.id.clone()) {
            if let Ok(existing_conn) = serde_json::from_value::<StoredConnection>(existing_value) {
                connection.encrypted_password = existing_conn.encrypted_password;
            }
        }
    }

    // 更新连接信息
    store.set(connection.id.clone(), json!(connection));
    
    store.save().map_err(|e| e.to_string())?;

    Ok(connection)
}

/// 获取所有连接配置
#[tauri::command]
pub async fn get_connections(app: AppHandle) -> Result<Vec<StoredConnection>, String> {
    let store = app
        .store("connections.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    let mut connections = Vec::new();

    // 遍历所有键值对
    for (_, value) in store.entries() {
        if let Ok(conn) = serde_json::from_value::<StoredConnection>(value.clone()) {
            connections.push(conn);
        }
    }

    Ok(connections)
}

/// 删除连接配置
#[tauri::command]
pub async fn delete_connection(app: AppHandle, id: String) -> Result<bool, String> {
    let store = app
        .store("connections.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;

    store.delete(id);
    store.save().map_err(|e| e.to_string())?;

    Ok(true)
}

/// 创建数据库连接
#[tauri::command]
pub async fn create_connection(
    app: AppHandle,
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 从存储中获取连接配置
    let store = app
        .store("connections.json")
        .map_err(|e| format!("Failed to get store: {}", e))?;
    
    let stored_value = store
        .get(connection_id.clone())
        .ok_or("连接配置不存在")?;
    
    let stored_conn: StoredConnection = serde_json::from_value(stored_value)
        .map_err(|e| format!("解析连接配置失败: {}", e))?;
    
    // 转换为 ConnectionConfig
    let config = stored_to_config(&stored_conn)?;
    
    // 使用连接管理器创建连接
    let manager = state.connection_manager.lock().await;
    manager.create_connection(config).await.map_err(|e| e.to_string())?;
    
    Ok(())
}

/// 断开数据库连接
#[tauri::command]
pub async fn disconnect_database(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 使用连接管理器断开连接
    let manager = state.connection_manager.lock().await;
    manager.disconnect(&connection_id).await.map_err(|e| e.to_string())?;
    
    Ok(())
}

