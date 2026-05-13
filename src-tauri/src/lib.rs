pub mod models;
pub mod hooks;
pub mod utils;
pub mod tasks;

use tauri::{AppHandle, Emitter};
use colored::*;
use dotenv::dotenv;
use std::sync::{Arc, Mutex};
use rig::providers::openai::Client as OpenAiClient;
use rmcp::ServiceExt;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
use serde_json::json;
use std::env;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use uuid::Uuid;

use crate::tasks::{spawn_producer_task, spawn_consumer_task};

#[tauri::command]
async fn start_swarm(app: AppHandle) -> Result<(), String> {
    let neonia_key = env::var("NEONIA_API_KEY").expect("NEONIA_API_KEY must be set");
    let neonia_mcp_url = env::var("NEONIA_MCP_URL").unwrap_or_else(|_| "https://mcp.neonia.io/mcp".to_string());
    let openrouter_key = env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY must be set");
    
    let swarm_id = format!("YC-SWRM-{}", Uuid::new_v4());
    let queue_name = format!("manufacturing_{}", Uuid::new_v4());

    println!("{}", "\n[SYSTEM] Initializing Neonia MCP Gateway & Swarm...".bold().cyan());
    println!("{}", format!("Swarm ID generated: {}", swarm_id).dimmed());
    println!("{}", format!("Using unique queue: {}", queue_name).dimmed());
    println!("{}", "==================================================".bold());

    let _ = app.emit("swarm-started", json!({ "swarm_id": swarm_id, "queue_name": queue_name }));

    let layout_state: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));

    let url = format!("{}?tools=neo_util_svg_generator,neo_util_svg_layout_validator,neo_util_pathfinder,neo_util_rng", neonia_mcp_url);
    let mut config = rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url);
    config.auth_header = Some(format!("Bearer {}", neonia_key));
    let transport = rmcp::transport::StreamableHttpClientTransport::from_config(config);
    let client_info = ClientInfo::new(ClientCapabilities::default(), Implementation::new("rig-swarm-demo", "0.1.0"));
    
    let mcp_client = std::sync::Arc::new(client_info.serve(transport).await.map_err(|e| e.to_string())?);
    let all_tools = mcp_client.list_tools(Default::default()).await.map_err(|e| e.to_string())?.tools;

    let openrouter_client = OpenAiClient::builder()
        .api_key(openrouter_key.as_str())
        .base_url("https://openrouter.ai/api/v1")
        .build()
        .expect("Failed to build OpenAI client");

    let worker_peer = mcp_client.peer().to_owned();

    // Start Producer
    spawn_producer_task(app.clone(), mcp_client.clone(), queue_name.clone());

    // Start Consumer
    spawn_consumer_task(
        app.clone(),
        queue_name,
        neonia_key,
        neonia_mcp_url,
        layout_state,
        mcp_client,
        worker_peer,
        openrouter_client,
        all_tools
    );

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenv().ok();
    
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let subscriber = FmtSubscriber::builder().with_env_filter(EnvFilter::new(env_filter)).finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_swarm])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
