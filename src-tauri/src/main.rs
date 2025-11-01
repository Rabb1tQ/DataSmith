// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod models;
mod utils;

use database::ConnectionManager;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// 应用状态
pub struct AppState {
    pub connection_manager: Arc<Mutex<ConnectionManager>>,
}

fn main() {
    // 初始化加密系统
    if let Err(e) = utils::crypto::initialize_master_key() {
        eprintln!("警告: 密钥初始化失败: {}", e);
    }

    // 初始化连接管理器
    let connection_manager = Arc::new(Mutex::new(ConnectionManager::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState { connection_manager })
        .invoke_handler(tauri::generate_handler![
            commands::connection::test_connection,
            commands::connection::save_connection,
            commands::connection::update_connection,
            commands::connection::get_connections,
            commands::connection::delete_connection,
            commands::connection::create_connection,
            commands::connection::disconnect_database,
            commands::query::execute_query,
            commands::query::execute_query_batch,
            commands::query::update_table_data,
            commands::query::insert_table_data,
            commands::query::delete_table_data,
            commands::metadata::get_databases,
            commands::metadata::get_tables,
            commands::metadata::get_table_structure,
            commands::metadata::view_table_data,
            commands::metadata::truncate_table,
            commands::metadata::drop_table,
            commands::metadata::get_views,
            commands::metadata::get_procedures,
            commands::metadata::get_functions,
            commands::metadata::get_triggers,
            commands::metadata::get_events,
            commands::metadata::drop_view,
            commands::metadata::get_view_definition,
            commands::metadata::drop_procedure,
            commands::metadata::drop_function,
            commands::metadata::drop_trigger,
            commands::metadata::drop_event,
            commands::metadata::get_table_indexes,
            commands::metadata::get_table_foreign_keys,
            commands::metadata::get_create_table_ddl,
            commands::metadata::get_autocomplete_data,
            commands::export::export_to_csv,
            commands::export::export_to_json,
            commands::export::export_to_sql,
            commands::export::export_table_ddl,
            commands::utils::read_file,
            commands::utils::write_file,
            commands::redis::execute_redis_command,
            commands::redis::get_redis_info,
            commands::redis::get_redis_key_value,
            commands::redis::set_redis_key_value,
            commands::redis::delete_redis_key,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

