use crate::database::redis::RedisDatabase;
use crate::AppState;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

/// Redis 命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisCommandResult {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u128,
}

/// Redis 键值信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisKeyInfo {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
    pub size: Option<usize>,
    pub encoding: Option<String>,
}

/// Redis 键值详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisKeyDetail {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
    pub value: serde_json::Value,
}

/// 执行 Redis 命令
#[tauri::command]
pub async fn execute_redis_command(
    connection_id: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<RedisCommandResult, String> {
    let start = std::time::Instant::now();
    
    // 解析命令和参数
    let parts: Vec<String> = shell_words::split(&command)
        .map_err(|e| format!("解析命令失败: {}", e))?;
    
    if parts.is_empty() {
        return Ok(RedisCommandResult {
            success: false,
            result: None,
            error: Some("命令不能为空".to_string()),
            execution_time_ms: start.elapsed().as_millis(),
        });
    }
    
    let cmd = parts[0].to_uppercase();
    let args: Vec<String> = parts[1..].to_vec();
    
    let manager = state.connection_manager.lock().await;
    let connections = manager.get_connection(&connection_id).await
        .map_err(|e| e.to_string())?;
    
    let connections_guard = connections.read().await;
    let db = connections_guard.get(&connection_id)
        .ok_or_else(|| "连接不存在".to_string())?;
    
    // 向下转型为 RedisDatabase
    let redis_db = db.as_any()
        .downcast_ref::<RedisDatabase>()
        .ok_or_else(|| "不是 Redis 连接".to_string())?;
    
    match redis_db.execute_command(&cmd, args).await {
        Ok(value) => {
            let json_value = redis_value_to_json(value);
            Ok(RedisCommandResult {
                success: true,
                result: Some(json_value),
                error: None,
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
        Err(e) => {
            Ok(RedisCommandResult {
                success: false,
                result: None,
                error: Some(e.to_string()),
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
    }
}

/// 获取 Redis 服务器信息
#[tauri::command]
pub async fn get_redis_info(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let manager = state.connection_manager.lock().await;
    let connections = manager.get_connection(&connection_id).await
        .map_err(|e| e.to_string())?;
    
    let connections_guard = connections.read().await;
    let db = connections_guard.get(&connection_id)
        .ok_or_else(|| "连接不存在".to_string())?;
    
    let redis_db = db.as_any()
        .downcast_ref::<RedisDatabase>()
        .ok_or_else(|| "不是 Redis 连接".to_string())?;
    
    redis_db.get_server_info().await.map_err(|e| e.to_string())
}

/// 获取键值
#[tauri::command]
pub async fn get_redis_key_value(
    connection_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<RedisKeyDetail, String> {
    let manager = state.connection_manager.lock().await;
    let connections = manager.get_connection(&connection_id).await
        .map_err(|e| e.to_string())?;
    
    let connections_guard = connections.read().await;
    let db = connections_guard.get(&connection_id)
        .ok_or_else(|| "连接不存在".to_string())?;
    
    let redis_db = db.as_any()
        .downcast_ref::<RedisDatabase>()
        .ok_or_else(|| "不是 Redis 连接".to_string())?;
    
    // 获取键类型
    let key_type_value = redis_db.execute_command("TYPE", vec![key.clone()]).await
        .map_err(|e| e.to_string())?;
    let key_type = match key_type_value {
        redis::Value::SimpleString(s) => s,
        _ => "unknown".to_string(),
    };
    
    // 获取 TTL
    let ttl = redis_db.get_key_ttl(&key).await.map_err(|e| e.to_string())?;
    
    // 获取值
    let value = redis_db.get_key_value(&key).await.map_err(|e| e.to_string())?;
    let json_value = redis_value_to_json(value);
    
    Ok(RedisKeyDetail {
        key,
        key_type,
        ttl,
        value: json_value,
    })
}

/// 设置键值
#[tauri::command]
pub async fn set_redis_key_value(
    connection_id: String,
    key: String,
    value: String,
    ttl: Option<u64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    let connections = manager.get_connection(&connection_id).await
        .map_err(|e| e.to_string())?;
    
    let connections_guard = connections.read().await;
    let db = connections_guard.get(&connection_id)
        .ok_or_else(|| "连接不存在".to_string())?;
    
    let redis_db = db.as_any()
        .downcast_ref::<RedisDatabase>()
        .ok_or_else(|| "不是 Redis 连接".to_string())?;
    
    redis_db.set_key_value(&key, &value, ttl).await.map_err(|e| e.to_string())
}

/// 删除键
#[tauri::command]
pub async fn delete_redis_key(
    connection_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    let connections = manager.get_connection(&connection_id).await
        .map_err(|e| e.to_string())?;
    
    let connections_guard = connections.read().await;
    let db = connections_guard.get(&connection_id)
        .ok_or_else(|| "连接不存在".to_string())?;
    
    let redis_db = db.as_any()
        .downcast_ref::<RedisDatabase>()
        .ok_or_else(|| "不是 Redis 连接".to_string())?;
    
    redis_db.delete_key(&key).await.map_err(|e| e.to_string())
}

/// 将 Redis Value 转换为 JSON
fn redis_value_to_json(value: redis::Value) -> serde_json::Value {
    match value {
        redis::Value::Nil => serde_json::Value::Null,
        redis::Value::Int(i) => serde_json::json!(i),
        redis::Value::BulkString(data) => {
            // 尝试将字节转换为字符串
            match String::from_utf8(data.clone()) {
                Ok(s) => serde_json::Value::String(s),
                Err(_) => {
                    // 如果不是有效的 UTF-8，返回 base64 编码
                    serde_json::Value::String(general_purpose::STANDARD.encode(&data))
                }
            }
        }
        redis::Value::Array(values) => {
            let arr: Vec<serde_json::Value> = values.into_iter()
                .map(redis_value_to_json)
                .collect();
            serde_json::Value::Array(arr)
        }
        redis::Value::SimpleString(s) => serde_json::Value::String(s),
        redis::Value::Okay => serde_json::Value::String("OK".to_string()),
        redis::Value::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map.into_iter()
                .map(|(k, v)| {
                    let key = match k {
                        redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                        redis::Value::SimpleString(s) => s,
                        _ => format!("{:?}", k),
                    };
                    (key, redis_value_to_json(v))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        redis::Value::Attribute { data, attributes: _ } => {
            // For attributes, just return the data part
            redis_value_to_json(*data)
        }
        redis::Value::Set(values) => {
            let arr: Vec<serde_json::Value> = values.into_iter()
                .map(redis_value_to_json)
                .collect();
            serde_json::Value::Array(arr)
        }
        redis::Value::Double(f) => serde_json::json!(f),
        redis::Value::Boolean(b) => serde_json::json!(b),
        redis::Value::VerbatimString { format: _, text } => serde_json::Value::String(text),
        redis::Value::BigNumber(n) => serde_json::Value::String(format!("{}", n)),
        redis::Value::Push { kind: _, data } => {
            let arr: Vec<serde_json::Value> = data.into_iter()
                .map(redis_value_to_json)
                .collect();
            serde_json::Value::Array(arr)
        }
        redis::Value::ServerError(err) => {
            serde_json::json!({
                "error": true,
                "message": err.details().unwrap_or("Unknown error"),
                "kind": format!("{:?}", err.kind())
            })
        }
    }
}

