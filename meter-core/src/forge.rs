//! Packet forging module for sending custom packets to the game server

use crate::{MeterError, Result};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::Mutex;
use windivert::prelude::*;
use std::net::Ipv4Addr;

// Global state for packet forging
lazy_static::lazy_static! {
    static ref FORGE_HANDLE: Arc<Mutex<Option<WinDivert<NetworkLayer>>>> = Arc::new(Mutex::new(None));
    static ref SERVER_CONNECTION: Arc<Mutex<Option<ServerConnection>>> = Arc::new(Mutex::new(None));
}

/// Server connection information
#[derive(Debug, Clone)]
pub struct ServerConnection {
    pub client_ip: Ipv4Addr,
    pub server_ip: Ipv4Addr,
    pub client_port: u16,
    pub server_port: u16,
}

/// Initialize the forging system
pub async fn init_forge_system() -> Result<()> {
    // Create a WinDivert handle for outbound packets
    let filter = "outbound and ip and tcp".to_string();
    let handle = WinDivert::<NetworkLayer>::network(&filter, 0, WinDivertFlags::new())
        .map_err(|e| MeterError::WinDivertError(format!("Failed to create forge handle: {}", e)))?;

    log::info!("Packet forging system initialized with filter: {}", filter);

    let mut forge_handle = FORGE_HANDLE.lock().await;
    *forge_handle = Some(handle);

    Ok(())
}

/// Set the server connection information
pub async fn set_server_connection(conn: ServerConnection) -> Result<()> {
    let mut server_conn = SERVER_CONNECTION.lock().await;
    *server_conn = Some(conn.clone());
    log::info!("Server connection set: {}:{} -> {}:{}",
               conn.client_ip, conn.client_port, conn.server_ip, conn.server_port);
    Ok(())
}

/// Parse hex string into byte vector
pub fn parse_hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    let hex_str = hex_str.replace(" ", "").replace("\n", "");
    let mut bytes = Vec::new();

    for i in (0..hex_str.len()).step_by(2) {
        let byte_str = &hex_str[i..i+2];
        let byte = u8::from_str_radix(byte_str, 16)
            .map_err(|_| MeterError::ParseError(format!("Invalid hex byte: {}", byte_str)))?;
        bytes.push(byte);
    }

    Ok(bytes)
}

/// Construct a game protocol packet
pub fn construct_game_packet(opcode: u16, payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::new();

    // Add 4-byte length (opcode + payload)
    let length = 2 + payload.len() as u32;
    packet.extend_from_slice(&length.to_be_bytes());

    // Add 2-byte opcode
    packet.extend_from_slice(&opcode.to_be_bytes());

    // Add payload
    packet.extend_from_slice(payload);

    packet
}

/// Calculate IP checksum
fn calculate_ip_checksum(ip_header: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for i in (0..ip_header.len()).step_by(2) {
        if i + 1 < ip_header.len() {
            let word = ((ip_header[i] as u32) << 8) | (ip_header[i + 1] as u32);
            sum += word;
        } else {
            let word = (ip_header[i] as u32) << 8;
            sum += word;
        }
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

/// Calculate TCP checksum
fn calculate_tcp_checksum(ip_header: &[u8], tcp_segment: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // Pseudo-header: src_ip, dst_ip, protocol, tcp_length
    let src_ip = &ip_header[12..16];
    let dst_ip = &ip_header[16..20];

    for i in (0..4).step_by(2) {
        let word = ((src_ip[i] as u32) << 8) | src_ip[i + 1] as u32;
        sum += word;
    }
    for i in (0..4).step_by(2) {
        let word = ((dst_ip[i] as u32) << 8) | dst_ip[i + 1] as u32;
        sum += word;
    }

    sum += 6; // TCP protocol number
    sum += tcp_segment.len() as u32;

    // TCP segment
    for i in (0..tcp_segment.len()).step_by(2) {
        if i + 1 < tcp_segment.len() {
            let word = ((tcp_segment[i] as u32) << 8) | (tcp_segment[i + 1] as u32);
            sum += word;
        } else {
            let word = (tcp_segment[i] as u32) << 8;
            sum += word;
        }
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

/// Construct IP header
fn construct_ip_header(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, total_length: u16) -> Vec<u8> {
    let mut header = vec![0u8; 20];

    header[0] = 0x45; // Version 4, header length 20 bytes
    header[1] = 0x00; // TOS
    header[2..4].copy_from_slice(&total_length.to_be_bytes()); // Total length
    header[4..6].copy_from_slice(&[0x00, 0x01]); // ID
    header[6..8].copy_from_slice(&[0x00, 0x00]); // Flags/Fragment offset
    header[8] = 64; // TTL
    header[9] = 6; // Protocol (TCP)
    // Checksum will be calculated later
    header[10..12].copy_from_slice(&[0x00, 0x00]);
    header[12..16].copy_from_slice(&src_ip.octets()); // Source IP
    header[16..20].copy_from_slice(&dst_ip.octets()); // Destination IP

    let checksum = calculate_ip_checksum(&header);
    header[10..12].copy_from_slice(&checksum.to_be_bytes());

    header
}

/// Construct TCP header
fn construct_tcp_header(src_port: u16, dst_port: u16, seq_num: u32, ack_num: u32, payload_len: usize) -> Vec<u8> {
    let mut header = vec![0u8; 20];

    header[0..2].copy_from_slice(&src_port.to_be_bytes()); // Source port
    header[2..4].copy_from_slice(&dst_port.to_be_bytes()); // Destination port
    header[4..8].copy_from_slice(&seq_num.to_be_bytes()); // Sequence number
    header[8..12].copy_from_slice(&ack_num.to_be_bytes()); // Acknowledgment number
    header[12] = 0x50; // Data offset (5 * 4 = 20 bytes)
    header[13] = 0x18; // Flags: PSH + ACK
    header[14..16].copy_from_slice(&[0x00, 0x00]); // Window size (placeholder)
    // Checksum will be calculated later
    header[16..18].copy_from_slice(&[0x00, 0x00]);
    header[18..20].copy_from_slice(&[0x00, 0x00]); // Urgent pointer

    header
}

/// Send a forged packet
pub async fn send_forged_packet(opcode: u16, payload: &[u8]) -> Result<()> {
    let server_conn = SERVER_CONNECTION.lock().await;
    let conn = server_conn.as_ref()
        .ok_or_else(|| MeterError::GenericError(anyhow::anyhow!("No server connection configured")))?
        .clone();

    let forge_handle = FORGE_HANDLE.lock().await;
    let handle = forge_handle.as_ref()
        .ok_or_else(|| MeterError::GenericError(anyhow::anyhow!("Forge system not initialized")))?;

    // Construct game protocol packet
    let game_packet = construct_game_packet(opcode, payload);

    // Construct IP header
    let ip_total_length = 20 + 20 + game_packet.len(); // IP header + TCP header + payload
    let ip_header = construct_ip_header(conn.client_ip, conn.server_ip, ip_total_length as u16);

    // Construct TCP header (simplified - using placeholder sequence numbers)
    let seq_num = 1000; // Placeholder
    let ack_num = 2000; // Placeholder
    let mut tcp_header = construct_tcp_header(conn.client_port, conn.server_port, seq_num, ack_num, game_packet.len());

    // Calculate TCP checksum
    let tcp_checksum = calculate_tcp_checksum(&ip_header, &tcp_header);
    tcp_header[16..18].copy_from_slice(&tcp_checksum.to_be_bytes());

    // Combine headers and payload
    let mut packet_data = Vec::new();
    packet_data.extend_from_slice(&ip_header);
    packet_data.extend_from_slice(&tcp_header);
    packet_data.extend_from_slice(&game_packet);

    // Create a simple packet structure that WinDivert can understand
    // We'll use the existing pattern from the capture system
    let packet = WinDivertPacket {
        data: packet_data.into(),
        address: unsafe { windivert::address::WinDivertAddress::<windivert::layer::NetworkLayer>::new() },
    };

    // Send the packet
    handle.send(&packet)
        .map_err(|e| MeterError::GenericError(anyhow::anyhow!("Failed to send packet: {}", e)))?;

    log::info!("Sent forged packet - Opcode: 0x{:04x}, Size: {} bytes", opcode, game_packet.len());

    Ok(())
}

/// Send the two specific packets with 100ms delay
pub async fn send_forged_packets() -> Result<()> {
    // Packet 1: Opcode 0x0600, 227 bytes payload
    let packet1_hex = "00 00 83 75 00 00 00 44 00 02 00 00 00 00 63 33 53 42 00 00 00 00 00 00 00 2d 0a 2c 08 c0 80 ac 02 12 23 12 02 08 68 12 0a 08 72 12 06 a2 c7 a7 c9 92 33 12 03 08 da 03 12 03 08 c8 03 12 07 08 bb 03 12 02 b8 17 1a 00 00 00 00 9b 00 02 00 00 00 00 63 33 53 42 00 00 00 00 00 00 00 2e 0a 82 01 0a 79 08 80 85 84 93 d6 08 12 65 12 03 08 d8 4f 12 03 08 d9 4f 12 03 08 da 4f 12 03 08 e2 4f 12 03 08 e3 4f 12 03 08 e4 4f 12 03 08 ec 4f 12 03 08 ed 4f 12 03 08 ee 4f 12 07 08 c2 58 12 02 99 15 12 07 08 c3 58 12 02 99 15 12 07 08 c6 58 12 02 a6 04 12 03 08 d0 58 12 02 08 68 12 0a 08 72 12 06 a2 c7 a7 c9 92 33 12 06 08 46 12 02 90 03 1a 00 5a 07 12 05 08 02 10 e3 01 28 80 85 84 93 d6 08";
    let packet1_payload = parse_hex_to_bytes(packet1_hex)?;

    // Packet 2: Opcode 0x0680, 743 bytes payload
    let packet2_hex = "00 00 83 c7 28 b5 2f fd 00 58 bc 16 00 a6 26 85 47 d0 2e 27 1d a8 70 f0 00 38 2f 4d 19 bc 08 be 68 b9 f5 fb fb 59 12 72 c7 ac c3 2a 38 a6 92 c0 7a 29 99 73 0f e0 93 c8 a1 3b 08 79 5a 97 51 5e b0 1e 1a 5a 0a 9f 00 2f b8 a3 c1 bd f5 c4 22 d1 b8 5b ee 26 8d ec 2d 03 74 00 75 00 6f 00 b2 47 9e 66 53 e3 f9 98 4d 9e 66 c1 26 13 e3 79 1a 8c bb 16 cf 77 1f e4 69 2c 9e 77 cf 1d 45 e3 f2 b2 62 97 0c 75 d5 50 8e 0d ac dd 2e 42 05 73 74 75 46 2e 2b 16 62 8b b6 5d 99 e2 24 d5 20 ae bc 40 e5 cd 80 b8 55 ee c6 dd ed f6 bb eb 42 83 44 d6 b9 11 a4 00 ce 87 c8 6a 77 51 70 3e 44 56 61 18 be 88 13 68 a4 40 63 8b 60 81 87 83 dd 5d b9 38 bb 1b 5e 8e e7 d0 4b 82 3d 11 1a 25 92 ec d1 58 5d 4a f6 94 9a 92 98 3c 69 6b 43 56 bf 25 4f 1a 2b 81 9b 3d 2a a1 0e c1 f2 a6 93 e3 79 01 41 f2 23 38 18 6e 46 6c 4c 40 79 3e e3 4a 5e e4 a9 89 c9 f3 30 14 f2 22 4b b9 29 cf c7 19 f9 9a d4 ed 84 e7 31 52 c8 d7 a2 3c 0f 4e c9 d7 90 8e 3c 7f 9c 7c cd 08 e6 c5 f3 01 40 c8 33 70 61 b1 c2 f3 ad 16 f2 34 15 9e 0f 2f 31 a8 e2 ef 2a 06 65 b9 3a fd 4f 9e da b9 be 33 aa 2e ff 2e c7 68 9c 7d 42 95 e5 7a 96 61 34 76 c4 30 0c 67 b2 5b b5 7f 3b 96 e1 ef 19 06 69 0c 67 28 fe 8e 62 90 76 31 8f 39 77 33 cf 79 52 6f 23 56 d9 cd 93 b2 3a b0 20 f5 78 ee 04 7d 23 34 3c bd 56 8c 48 d8 e3 f0 e3 09 f4 6a 09 a1 e3 59 e4 94 21 b4 47 5b 42 1e ec 51 8f e7 3b 72 8e a2 71 79 59 99 45 aa 1a ed 3d e6 aa b4 f7 57 48 02 9b 3b 92 18 d6 40 e2 81 0f 8f 19 a6 9d 2b a9 0e 89 ab 9a c5 0d 89 ab aa 91 ca 1f 37 a0 e0 41 58 bd a2 28 4f ea 02 80 e1 ef 18 43 d5 73 bd 67 d8 89 bf 9f 18 a4 fe 2e 33 48 73 6a 74 7d 34 2c c5 df 53 0c 52 13 7f 37 31 48 63 c7 4f 6d f9 6f 39 46 81 aa 74 bd 34 8c fa 3b 2d f1 17 7d a8 61 68 a7 0c 05 33 63 4a 52 58 e9 30 03 19 da 66 0c 11 40 18 4e 9a 81 88 85 55 86 fd 0d 65 45 34 8a 21 17 e3 12 c8 1f b8 4d ad ec d6 74 60 b6 6d 66 61 80 e4 44 14 08 83 e8 2c e4 b9 f1 15 f8 f8 08 b1 69 f1 05 47 75 c8 e2 3b 5e 29 5f 9f 41 dd 07 13 05 bc 6d c1 97 e3 9e f0 71 52 a2 e4 ab 18 53 64 2d 10 21 c3 18 13 3c 8d 57 46 3a 43 fd a2 09 32 ea 8a b1 f0 e6 82 74 11 8c 60 45 25 a6 73 7d 90 48 53 92 1a 97 b2 66 ea 46 7e 5c 61 7b c3 49 e6 66 ae 56 f4 25 26 61 24 f0 2f e7 40 4b 22 08 00 69 94 68 01 f7 dc 2a 53 5a 61 56 34 2b 73 8f ba 7d c6 fa ae 3c be a1 80 e5 cf 9c 1f 01 00 00";
    let packet2_payload = parse_hex_to_bytes(packet2_hex)?;

    // Send first packet
    send_forged_packet(0x0600, &packet1_payload).await?;
    log::info!("Sent first forged packet (0x0600) - {} bytes", packet1_payload.len());

    // Wait 100ms
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Send second packet
    send_forged_packet(0x0680, &packet2_payload).await?;
    log::info!("Sent second forged packet (0x0680) - {} bytes", packet2_payload.len());

    log::info!("Successfully sent both forged packets with 100ms interval");

    Ok(())
}

/// Clean up the forging system
pub async fn cleanup_forge_system() -> Result<()> {
    let mut forge_handle = FORGE_HANDLE.lock().await;
    *forge_handle = None;

    let mut server_conn = SERVER_CONNECTION.lock().await;
    *server_conn = None;

    log::info!("Packet forging system cleaned up");
    Ok(())
}
