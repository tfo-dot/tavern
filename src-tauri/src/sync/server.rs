use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use tower_http::cors::CorsLayer;

use super::discovery::get_local_ip_addresses;
use super::protocol::{
    SyncDeviceInfo, SyncExchangeRequest, SyncExchangeResponse, SyncHandshakeRequest,
    SyncHandshakeResponse, SyncStats,
};
use crate::AppState;

#[derive(Clone)]
pub struct ServerContext {
    pub app_state: Arc<AppState>,
    pub app_handle: AppHandle,
    pub device_id: String,
    pub sync_port: u16,
}

pub async fn run_sync_server(
    ctx: ServerContext,
    port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), String> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind sync server on {addr}: {e}"))?;

    let app = Router::new()
        .route("/api/sync/info", get(handle_info))
        .route("/api/sync/handshake", post(handle_handshake))
        .route("/api/sync/exchange", post(handle_exchange))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(ctx);

    println!("[Sync Server] Listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
        })
        .await
        .map_err(|e| format!("Sync server error: {e}"))?;

    Ok(())
}

async fn handle_info(State(ctx): State<ServerContext>) -> impl IntoResponse {
    let settings = ctx.app_state.settings.lock().await;
    let dev_name = settings
        .device_name
        .clone()
        .unwrap_or_else(get_default_device_name);

    let info = SyncDeviceInfo {
        device_id: ctx.device_id.clone(),
        device_name: dev_name,
        port: ctx.sync_port,
        version: "1.0.0".to_string(),
        local_ips: get_local_ip_addresses(),
    };

    Json(info)
}

async fn handle_handshake(
    State(ctx): State<ServerContext>,
    Json(req): Json<SyncHandshakeRequest>,
) -> impl IntoResponse {
    let settings = ctx.app_state.settings.lock().await;
    let dev_name = settings
        .device_name
        .clone()
        .unwrap_or_else(get_default_device_name);

    // Check PIN if configured
    if let Some(expected_pin) = &settings.sync_pin
        && !expected_pin.trim().is_empty()
    {
        let provided = req.pin.unwrap_or_default();
        if provided != *expected_pin {
            return (
                StatusCode::UNAUTHORIZED,
                Json(SyncHandshakeResponse {
                    accepted: false,
                    device_id: ctx.device_id.clone(),
                    device_name: dev_name,
                    state_vector_base64: None,
                    error_message: Some("Invalid PIN".to_string()),
                }),
            );
        }
    }
    drop(settings);

    // Get current state vector
    let sv_b64 = match ctx.app_state.storage.load_crdt_doc() {
        Ok(doc) => {
            let sv = doc.state_vector();
            let encoded = sv.encode();
            Some(BASE64.encode(encoded))
        }
        Err(_) => None,
    };

    (
        StatusCode::OK,
        Json(SyncHandshakeResponse {
            accepted: true,
            device_id: ctx.device_id.clone(),
            device_name: dev_name,
            state_vector_base64: sv_b64,
            error_message: None,
        }),
    )
}

async fn handle_exchange(
    State(ctx): State<ServerContext>,
    Json(req): Json<SyncExchangeRequest>,
) -> impl IntoResponse {
    let settings = ctx.app_state.settings.lock().await;
    if let Some(expected_pin) = &settings.sync_pin
        && !expected_pin.trim().is_empty()
    {
        let provided = req.pin.unwrap_or_default();
        if provided != *expected_pin {
            return (
                StatusCode::UNAUTHORIZED,
                Json(SyncExchangeResponse {
                    success: false,
                    updates_base64: String::new(),
                    summary: SyncStats::default(),
                    error_message: Some("Invalid PIN".to_string()),
                }),
            );
        }
    }
    drop(settings);

    let doc = match ctx.app_state.storage.load_crdt_doc() {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SyncExchangeResponse {
                    success: false,
                    updates_base64: String::new(),
                    summary: SyncStats::default(),
                    error_message: Some(format!("Failed to load local CRDT doc: {e}")),
                }),
            );
        }
    };

    // 1. Import updates sent by client
    if !req.updates_base64.trim().is_empty()
        && let Ok(updates_bytes) = BASE64.decode(req.updates_base64.trim())
        && let Err(e) = doc.import_updates(&updates_bytes)
    {
        eprintln!("[Sync Server] Error importing client updates: {e}");
    }

    // 2. Sync to local disk files
    let summary = match ctx.app_state.storage.sync_disk_from_crdt(&doc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Sync Server] Error syncing disk: {e}");
            SyncStats::default()
        }
    };

    // 3. Save CRDT snapshot
    let _ = ctx.app_state.storage.save_crdt_doc(&doc);

    // 4. Compute reply updates based on client state vector
    let mut reply_updates_b64 = String::new();
    if !req.state_vector_base64.trim().is_empty()
        && let Ok(sv_bytes) = BASE64.decode(req.state_vector_base64.trim())
        && let Ok(client_vv) = loro::VersionVector::decode(&sv_bytes)
        && let Ok(updates_for_client) = doc.export_updates_from(&client_vv)
    {
        reply_updates_b64 = BASE64.encode(updates_for_client);
    }

    // 5. Emit UI event so active chat/characters refresh
    let _ = ctx.app_handle.emit("sync-completed", &summary);

    (
        StatusCode::OK,
        Json(SyncExchangeResponse {
            success: true,
            updates_base64: reply_updates_b64,
            summary,
            error_message: None,
        }),
    )
}

pub fn get_default_device_name() -> String {
    #[cfg(target_os = "android")]
    {
        "Android Device".to_string()
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(host) = std::env::var("HOSTNAME").or_else(|_| std::env::var("HOST"))
            && !host.trim().is_empty()
        {
            return host;
        }
        "Tavern Desktop".to_string()
    }
}
