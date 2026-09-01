use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub port: u16,
    pub version: String,
    pub local_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHandshakeRequest {
    pub device_id: String,
    pub device_name: String,
    pub pin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHandshakeResponse {
    pub accepted: bool,
    pub device_id: String,
    pub device_name: String,
    pub state_vector_base64: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncExchangeRequest {
    pub device_id: String,
    pub pin: Option<String>,
    pub state_vector_base64: String,
    pub updates_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncExchangeResponse {
    pub success: bool,
    pub updates_base64: String,
    pub summary: SyncStats,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStats {
    pub characters_synced: usize,
    pub chats_synced: usize,
    pub personas_synced: usize,
    pub lorebooks_synced: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub device_id: String,
    pub device_name: String,
    pub address: String, // e.g. "192.168.1.105:24567"
    pub port: u16,
    pub last_seen_epoch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconMessage {
    pub kind: String, // "probe" | "announce"
    pub device_id: String,
    pub device_name: String,
    pub port: u16,
}
