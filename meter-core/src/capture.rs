//! Network packet capture using WinDivert

use crate::{MeterError, Result};
use async_channel::{Receiver, Sender};
use tokio::task;
use windivert::prelude;


/// Packet capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub port: u16,
    pub filter: String,
    pub region_file_path: String,
}

/// Captured packet data
#[derive(Debug, Clone)]
pub struct PacketData {
    pub opcode: u16,
    pub data: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}

/// Start packet capture on the specified port
pub fn start_capture(port: u16, region_file_path: String) -> Result<Receiver<(u16, Vec<u8>)>> {
    let (tx, rx) = async_channel::unbounded();

    // Create filter for capturing traffic on the specified port
    let filter = format!("tcp.DstPort == {} or tcp.SrcPort == {}", port, port);

    let config = CaptureConfig {
        port,
        filter,
        region_file_path,
    };

    // Spawn capture task
    task::spawn(async move {
        if let Err(e) = run_capture(config, tx).await {
            log::error!("Packet capture failed: {:?}", e);
        }
    });

    Ok(rx)
}

/// Internal capture function with full WinDivert implementation
async fn run_capture(config: CaptureConfig, tx: Sender<(u16, Vec<u8>)>) -> Result<()> {
    // Check if WinDivert is available
    if !crate::utils::is_windivert_installed() {
        return Err(MeterError::WinDivertError(
            "WinDivert driver not found. Please ensure WinDivert64.sys is installed.".to_string()
        ));
    }

    log::info!("Starting packet capture on port {}", config.port);

    // Create WinDivert handle
    let handle = WinDivert::new(&config.filter, Layer::Network, Priority::default(), Flags::default())
        .map_err(|e| MeterError::WinDivertError(format!("Failed to create WinDivert handle: {:?}", e)))?;

    log::info!("WinDivert handle created successfully");

    // Main capture loop
    loop {
        // Receive packet
        let mut packet = match handle.recv() {
            Ok(packet) => packet,
            Err(e) => {
                log::warn!("Failed to receive packet: {:?}", e);
                continue;
            }
        };

        // Process packet
        match process_packet(&packet) {
            Ok(packet_data) => {
                // Send packet data through channel
                if tx.send((packet_data.opcode, packet_data.data)).await.is_err() {
                    log::warn!("Failed to send packet data through channel");
                    break;
                }
            }
            Err(e) => {
                log::debug!("Failed to process packet: {:?}", e);
            }
        }

        // Re-inject packet
        if let Err(e) = handle.send(&packet) {
            log::warn!("Failed to re-inject packet: {:?}", e);
        }
    }

    Ok(())
}

/// Process a captured packet and extract relevant data
fn process_packet(packet: &Packet) -> Result<PacketData> {
    // Extract TCP payload
    let tcp_payload = extract_tcp_payload(packet)?;

    if tcp_payload.is_empty() {
        return Err(MeterError::ParseError("Empty TCP payload".to_string()));
    }

    // Extract opcode from packet data
    // This is a simplified implementation - actual protocol may vary
    if tcp_payload.len() < 2 {
        return Err(MeterError::ParseError("Packet too small for opcode".to_string()));
    }

    let opcode = u16::from_le_bytes([tcp_payload[0], tcp_payload[1]]);
    let data = tcp_payload[2..].to_vec();

    let packet_data = PacketData {
        opcode,
        data,
        timestamp: std::time::SystemTime::now(),
    };

    Ok(packet_data)
}

/// Extract TCP payload from network packet
fn extract_tcp_payload(packet: &Packet) -> Result<&[u8]> {
    // Get network headers
    let network_header = packet
        .network_header()
        .ok_or_else(|| MeterError::ParseError("Missing network header".to_string()))?;

    // Check if it's TCP
    if network_header.protocol() != Protocol::Tcp {
        return Err(MeterError::ParseError("Not a TCP packet".to_string()));
    }

    // Get TCP header
    let tcp_header = packet
        .tcp_header()
        .ok_or_else(|| MeterError::ParseError("Missing TCP header".to_string()))?;

    // Calculate payload offset
    let ip_header_len = (network_header.header_len() as usize) * 4;
    let tcp_header_len = (tcp_header.header_len() as usize) * 4;
    let payload_offset = ip_header_len + tcp_header_len;

    if packet.data().len() < payload_offset {
        return Err(MeterError::ParseError("Packet too small for payload".to_string()));
    }

    Ok(&packet.data()[payload_offset..])
}

/// Stop packet capture (placeholder - actual implementation would need handle management)
pub fn stop_capture() -> Result<()> {
    log::info!("Stopping packet capture");
    // TODO: Implement proper capture stopping
    Ok(())
}

/// Get capture statistics
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub packets_captured: u64,
    pub packets_processed: u64,
    pub packets_dropped: u64,
    pub uptime_seconds: u64,
}

pub fn get_capture_stats() -> CaptureStats {
    // TODO: Implement actual statistics tracking
    CaptureStats {
        packets_captured: 0,
        packets_processed: 0,
        packets_dropped: 0,
        uptime_seconds: 0,
    }
}
