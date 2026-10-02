use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::protocol::{
    SyncExchangeRequest, SyncExchangeResponse, SyncHandshakeRequest, SyncHandshakeResponse,
    SyncStats,
};
use super::server::get_default_device_name;
use crate::AppState;

pub async fn sync_with_peer(
    target_address: &str,
    pin: Option<String>,
    app_state: Arc<AppState>,
    app_handle: AppHandle,
    local_device_id: String,
) -> Result<SyncStats, String> {
    let mut base_url = target_address.trim().to_string();
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        base_url = format!("http://{base_url}");
    }
    let base_url = base_url.trim_end_matches('/');

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let local_device_name = {
        let settings = app_state.settings.lock().await;
        settings
            .device_name
            .clone()
            .unwrap_or_else(get_default_device_name)
    };

    // Step 1: Handshake
    let handshake_url = format!("{base_url}/api/sync/handshake");
    let handshake_req = SyncHandshakeRequest {
        device_id: local_device_id.clone(),
        device_name: local_device_name,
        pin: pin.clone(),
    };

    let handshake_res = client
        .post(&handshake_url)
        .json(&handshake_req)
        .send()
        .await
        .map_err(|e| format!("Connection failed to {handshake_url}: {e}"))?;

    if !handshake_res.status().is_success() {
        if handshake_res.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err("Authentication failed: Incorrect PIN".to_string());
        }
        return Err(format!(
            "Handshake failed with status: {}",
            handshake_res.status()
        ));
    }

    let handshake_data: SyncHandshakeResponse = handshake_res
        .json()
        .await
        .map_err(|e| format!("Invalid handshake response: {e}"))?;

    if !handshake_data.accepted {
        return Err(handshake_data
            .error_message
            .unwrap_or_else(|| "Handshake rejected by remote peer".to_string()));
    }

    // Step 2: Load local CRDT doc and compute updates to send
    let doc = app_state.storage.load_crdt_doc()?;
    let local_sv = doc.state_vector();
    let local_sv_encoded = local_sv.encode();
    let local_sv_b64 = BASE64.encode(local_sv_encoded);

    let mut outgoing_updates_b64 = String::new();
    if let Some(target_sv_str) = &handshake_data.state_vector_base64
        && let Ok(target_sv_bytes) = BASE64.decode(target_sv_str.trim())
        && let Ok(target_vv) = loro::VersionVector::decode(&target_sv_bytes)
        && let Ok(updates) = doc.export_updates_from(&target_vv)
    {
        outgoing_updates_b64 = BASE64.encode(updates);
    }
    if outgoing_updates_b64.is_empty()
        && let Ok(snapshot) = doc.export_snapshot()
    {
        outgoing_updates_b64 = BASE64.encode(snapshot);
    }

    // Step 3: Send exchange request
    let exchange_url = format!("{base_url}/api/sync/exchange");
    let exchange_req = SyncExchangeRequest {
        device_id: local_device_id,
        pin,
        state_vector_base64: local_sv_b64,
        updates_base64: outgoing_updates_b64,
    };

    let exchange_res = client
        .post(&exchange_url)
        .json(&exchange_req)
        .send()
        .await
        .map_err(|e| format!("Sync exchange failed at {exchange_url}: {e}"))?;

    if !exchange_res.status().is_success() {
        return Err(format!(
            "Sync exchange HTTP error: {}",
            exchange_res.status()
        ));
    }

    let exchange_data: SyncExchangeResponse = exchange_res
        .json()
        .await
        .map_err(|e| format!("Invalid sync exchange response: {e}"))?;

    if !exchange_data.success {
        return Err(exchange_data
            .error_message
            .unwrap_or_else(|| "Sync exchange failed on remote device".to_string()));
    }

    // Step 4: Import remote updates into local CRDT doc
    if !exchange_data.updates_base64.trim().is_empty()
        && let Ok(remote_updates_bytes) = BASE64.decode(exchange_data.updates_base64.trim())
    {
        doc.import_updates(&remote_updates_bytes)?;
    }

    // Step 5: Sync to local disk and persist
    let stats = app_state.storage.sync_disk_from_crdt(&doc)?;
    let _ = app_state.storage.save_crdt_doc(&doc);

    // Step 6: Notify UI
    let _ = app_handle.emit("sync-completed", &stats);

    Ok(stats)
}
