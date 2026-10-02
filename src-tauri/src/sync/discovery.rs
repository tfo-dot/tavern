use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::watch;

use super::protocol::{BeaconMessage, DiscoveredPeer};

pub const DISCOVERY_PORT: u16 = 24568;

/// Retrieves list of local non-loopback IPv4 addresses.
pub fn get_local_ip_addresses() -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(my_local_ip) = local_ip_address::local_ip() {
        ips.push(my_local_ip.to_string());
    }
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in interfaces {
            if !ip.is_loopback() && ip.is_ipv4() {
                let s = ip.to_string();
                if !ips.contains(&s) {
                    ips.push(s);
                }
            }
        }
    }
    if ips.is_empty() {
        ips.push("127.0.0.1".to_string());
    }
    ips
}

/// Runs background UDP responder that answers discovery probes from other devices on the LAN.
pub async fn run_discovery_responder(
    device_id: String,
    device_name: Arc<tokio::sync::RwLock<String>>,
    sync_port: u16,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let bind_addr = format!("0.0.0.0:{DISCOVERY_PORT}");
    let socket = match UdpSocket::bind(&bind_addr).await {
        Ok(s) => {
            let _ = s.set_broadcast(true);
            s
        }
        Err(e) => {
            eprintln!("[Sync Discovery] Failed to bind responder on {bind_addr}: {e}");
            return;
        }
    };

    let mut buf = [0u8; 2048];
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
            res = socket.recv_from(&mut buf) => {
                match res {
                    Ok((len, peer_addr)) => {
                        if let Ok(msg) = serde_json::from_slice::<BeaconMessage>(&buf[..len]) && msg.kind == "probe" && msg.device_id != device_id {
                                let cur_name = device_name.read().await.clone();
                                let announce = BeaconMessage {
                                    kind: "announce".to_string(),
                                    device_id: device_id.clone(),
                                    device_name: cur_name,
                                    port: sync_port,
                                };
                                if let Ok(reply_bytes) = serde_json::to_vec(&announce) {
                                    let _ = socket.send_to(&reply_bytes, peer_addr).await;
                                }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Sync Discovery] Recv error: {e}");
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

/// Scans local network by sending UDP broadcast probes and collecting announce replies.
pub async fn scan_lan_peers(
    local_device_id: &str,
    local_device_name: &str,
    local_sync_port: u16,
    timeout_duration: Duration,
) -> Result<Vec<DiscoveredPeer>, String> {
    // Bind ephemeral UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind client UDP socket: {e}"))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("Failed to enable broadcast: {e}"))?;

    let probe = BeaconMessage {
        kind: "probe".to_string(),
        device_id: local_device_id.to_string(),
        device_name: local_device_name.to_string(),
        port: local_sync_port,
    };
    let probe_bytes = serde_json::to_vec(&probe).map_err(|e| e.to_string())?;

    // Send broadcast to LAN broadcast address and multicast
    let targets = [
        format!("255.255.255.255:{DISCOVERY_PORT}"),
        format!("224.0.0.1:{DISCOVERY_PORT}"),
    ];
    for target in &targets {
        let _ = socket.send_to(&probe_bytes, target).await;
    }

    let mut peers_map: HashMap<String, DiscoveredPeer> = HashMap::new();
    let mut buf = [0u8; 2048];
    let start = tokio::time::Instant::now();

    while start.elapsed() < timeout_duration {
        let remaining = timeout_duration.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, peer_addr))) => {
                if let Ok(msg) = serde_json::from_slice::<BeaconMessage>(&buf[..len])
                    && (msg.kind == "announce" || msg.kind == "probe")
                    && msg.device_id != local_device_id
                {
                    let ip_str = peer_addr.ip().to_string();
                    let target_addr = format!("{}:{}", ip_str, msg.port);
                    let epoch_now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    peers_map.insert(
                        msg.device_id.clone(),
                        DiscoveredPeer {
                            device_id: msg.device_id,
                            device_name: msg.device_name,
                            address: target_addr,
                            port: msg.port,
                            last_seen_epoch_ms: epoch_now,
                        },
                    );
                }
            }
            _ => break, // Timeout elapsed
        }
    }

    let mut peers: Vec<DiscoveredPeer> = peers_map.into_values().collect();
    peers.sort_by(|a, b| a.device_name.cmp(&b.device_name));
    Ok(peers)
}
