use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::{watch, RwLock};
use uuid::Uuid;

use super::client::sync_with_peer;
use super::discovery::{get_local_ip_addresses, run_discovery_responder, scan_lan_peers};
use super::protocol::{DiscoveredPeer, SyncDeviceInfo, SyncStats};
use super::server::{get_default_device_name, run_sync_server, ServerContext};
use crate::storage::AppSettings;
use crate::AppState;
pub const DEFAULT_SYNC_PORT: u16 = 24567;

pub struct SyncManager {
    pub device_id: String,
    pub sync_port: u16,
    pub device_name: Arc<RwLock<String>>,
    app_state: Arc<AppState>,
    app_handle: AppHandle,
    shutdown_tx: watch::Sender<bool>,
}

impl SyncManager {
    pub fn new(
        base_dir: PathBuf,
        settings: &AppSettings,
        app_state: Arc<AppState>,
        app_handle: AppHandle,
    ) -> Self {
        let dev_id_file = base_dir.join("device_id.txt");
        let device_id = if let Ok(s) = fs::read_to_string(&dev_id_file) {
            let trimmed = s.trim().to_string();
            if !trimmed.is_empty() {
                trimmed
            } else {
                let id = Uuid::new_v4().to_string();
                let _ = fs::write(&dev_id_file, &id);
                id
            }
        } else {
            let id = Uuid::new_v4().to_string();
            let _ = fs::write(&dev_id_file, &id);
            id
        };

        let initial_name = settings
            .device_name
            .clone()
            .unwrap_or_else(get_default_device_name);

        let sync_port = settings.sync_port.unwrap_or(DEFAULT_SYNC_PORT);
        let (shutdown_tx, _) = watch::channel(false);

        Self {
            device_id,
            sync_port,
            device_name: Arc::new(RwLock::new(initial_name)),
            app_state,
            app_handle,
            shutdown_tx,
        }
    }

    pub fn start(&self) {
        let ctx = ServerContext {
            app_state: Arc::clone(&self.app_state),
            app_handle: self.app_handle.clone(),
            device_id: self.device_id.clone(),
            sync_port: self.sync_port,
        };

        let shutdown_rx_server = self.shutdown_tx.subscribe();
        let port = self.sync_port;
        tauri::async_runtime::spawn(async move {
            if let Err(e) = run_sync_server(ctx, port, shutdown_rx_server).await {
                eprintln!("[SyncManager] Server error: {e}");
            }
        });

        let dev_id = self.device_id.clone();
        let dev_name = Arc::clone(&self.device_name);
        let shutdown_rx_discovery = self.shutdown_tx.subscribe();
        tauri::async_runtime::spawn(async move {
            run_discovery_responder(dev_id, dev_name, port, shutdown_rx_discovery).await;
        });
    }

    pub async fn update_device_name(&self, new_name: String) {
        let mut w = self.device_name.write().await;
        *w = new_name;
    }

    pub async fn scan_peers(&self, timeout_ms: u64) -> Result<Vec<DiscoveredPeer>, String> {
        let name = self.device_name.read().await.clone();
        let dur = Duration::from_millis(timeout_ms.max(500));
        scan_lan_peers(&self.device_id, &name, self.sync_port, dur).await
    }

    pub async fn sync_with(
        &self,
        target_address: &str,
        pin: Option<String>,
    ) -> Result<SyncStats, String> {
        sync_with_peer(
            target_address,
            pin,
            Arc::clone(&self.app_state),
            self.app_handle.clone(),
            self.device_id.clone(),
        )
        .await
    }

    pub async fn get_device_info(&self) -> SyncDeviceInfo {
        let name = self.device_name.read().await.clone();
        SyncDeviceInfo {
            device_id: self.device_id.clone(),
            device_name: name,
            port: self.sync_port,
            version: "1.0.0".to_string(),
            local_ips: get_local_ip_addresses(),
        }
    }
}
