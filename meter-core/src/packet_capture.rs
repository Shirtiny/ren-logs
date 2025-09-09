use crate::data_manager::DataManager;
use crate::packet_parser::PacketParser;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use windivert::prelude::*;

// Global state variables for stability
lazy_static::lazy_static! {
    static ref CURRENT_SERVER: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref SERVER_IDENTIFIED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref TCP_CACHE: Arc<Mutex<HashMap<u32, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref TCP_NEXT_SEQ: Arc<Mutex<i64>> = Arc::new(Mutex::new(-1));
    static ref TCP_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    static ref DATA_BUFFER: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    static ref TCP_LAST_TIME: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    static ref IP_FRAGMENT_CACHE: Arc<Mutex<HashMap<String, FragmentCache>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref PACKET_COUNTER: AtomicU64 = AtomicU64::new(0);
    static ref FILTERED_PACKETS: AtomicU64 = AtomicU64::new(0);
    static ref MISMATCHED_PACKETS: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

// IP fragment cache structure
#[derive(Debug)]
struct FragmentCache {
    fragments: Vec<Vec<u8>>,
    timestamp: u64,
}

// Packet capture configuration
pub struct PacketCaptureConfig {
    pub filter: String,
    pub buffer_size: usize,
    pub mtu: usize,
    pub fragment_timeout: Duration,
    pub connection_timeout: Duration,
}

impl Default for PacketCaptureConfig {
    fn default() -> Self {
        Self {
            filter: "ip and tcp".to_string(),
            buffer_size: 10 * 1024 * 1024, // 10MB
            mtu: 65535, // Increased from 1500 to 65535 to handle maximum Ethernet frame size
            fragment_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(300),
        }
    }
}

pub struct PacketCapture {
    config: PacketCaptureConfig,
    data_manager: Arc<DataManager>,
    packet_parser: PacketParser,
    start_time: u64,
}

impl PacketCapture {
    pub fn new(data_manager: Arc<DataManager>) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            config: PacketCaptureConfig::default(),
            data_manager: data_manager.clone(),
            packet_parser: PacketParser::new(data_manager),
            start_time,
        }
    }

    pub fn with_config(mut self, config: PacketCaptureConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn start_capture(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!(
            "Starting packet capture with filter: {}",
            self.config.filter
        );

        // Create WinDivert handle for network layer
        let handle =
            WinDivert::<NetworkLayer>::network(&self.config.filter, 0, WinDivertFlags::new())
                .map_err(|e| format!("Failed to create WinDivert handle: {}", e))?;

        log::info!("WinDivert handle opened successfully");

        // Create channels for packet processing
        let (packet_tx, mut packet_rx) = mpsc::channel::<Bytes>(1000);

        // Spawn packet processing task
        let data_manager = self.data_manager.clone();
        tokio::spawn(async move {
            while let Some(packet_data) = packet_rx.recv().await {
                let mut parser = PacketParser::new(data_manager.clone());
                parser.process_packet(&packet_data).await;
            }
        });

        // Spawn cleanup tasks
        self.spawn_cleanup_tasks();

        // Main packet capture loop
        let mut packet_buffer = vec![0u8; self.config.mtu];

        loop {
            // Receive packet
            match handle.recv(Some(&mut packet_buffer)) {
                Ok(packet) => {
                    let packet_count = PACKET_COUNTER.fetch_add(1, Ordering::SeqCst);

                    // Process the captured packet
                    if let Err(e) = self
                        .process_packet(&packet.data, &packet_tx, packet_count)
                        .await
                    {
                        log::warn!("Failed to process packet #{}: {:?}", packet_count, e);
                    }

                    // Re-inject packet back to network
                    if let Err(e) = handle.send(&packet) {
                        log::warn!("Failed to re-inject packet: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to receive packet: {:?}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn process_packet(
        &self,
        packet_data: &[u8],
        packet_tx: &mpsc::Sender<Bytes>,
        packet_count: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse IP packet (WinDivert NetworkLayer returns IP packets directly)
        let ip_data = packet_data;

        // Handle IP fragmentation
        let tcp_data = self.handle_ip_fragmentation(ip_data, packet_count).await?;
        if tcp_data.is_none() {
            return Ok(()); // Fragment not complete yet
        }
        let tcp_data = tcp_data.unwrap();

        // Extract TCP payload
        let payload = self.extract_tcp_payload(&tcp_data, packet_count)?;
        if payload.is_none() {
            return Ok(()); // No payload or invalid packet
        }
        let payload = payload.unwrap();

        // Process TCP stream reassembly
        self.process_tcp_stream(&payload, packet_tx, packet_count, ip_data)
            .await?;

        Ok(())
    }

    async fn handle_ip_fragmentation(
        &self,
        ip_data: &[u8],
        packet_count: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        if ip_data.len() < 20 {
            let filtered = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
            log::debug!(
                "Filtered packet #{}: too short for IP header (filtered: {})",
                packet_count,
                filtered
            );
            return Ok(None);
        }

        // Check IP version
        let ip_version = ip_data[0] >> 4;
        if ip_version != 4 {
            let filtered = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
            log::debug!(
                "Filtered packet #{}: not IPv4 (filtered: {})",
                packet_count,
                filtered
            );
            return Ok(None);
        }

        // Check protocol
        let protocol = ip_data[9];
        if protocol != 6 {
            // TCP
            let filtered = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
            log::debug!(
                "Filtered packet #{}: not TCP (filtered: {})",
                packet_count,
                filtered
            );
            return Ok(None);
        }

        // Check for fragmentation
        let flags = ip_data[6];
        let is_fragment = (flags & 0x1) != 0; // More fragments bit
        let fragment_offset = ((ip_data[6] & 0x1F) as u16) << 8 | ip_data[7] as u16;

        if !is_fragment && fragment_offset == 0 {
            // Not fragmented, return TCP data directly
            let ip_header_len = ((ip_data[0] & 0x0F) as usize) * 4;
            if ip_data.len() < ip_header_len {
                return Ok(None);
            }
            return Ok(Some(ip_data[ip_header_len..].to_vec()));
        }

        // Handle IP fragmentation
        let id = u16::from_be_bytes([ip_data[4], ip_data[5]]);
        let src_ip = format!(
            "{}.{}.{}.{}",
            ip_data[12], ip_data[13], ip_data[14], ip_data[15]
        );
        let dst_ip = format!(
            "{}.{}.{}.{}",
            ip_data[16], ip_data[17], ip_data[18], ip_data[19]
        );

        let key = format!("{}-{}-{}", id, src_ip, dst_ip);

        let mut fragment_cache = IP_FRAGMENT_CACHE.lock().await;

        if !fragment_cache.contains_key(&key) {
            fragment_cache.insert(
                key.clone(),
                FragmentCache {
                    fragments: Vec::new(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
            );
        }

        let cache = fragment_cache.get_mut(&key).unwrap();
        cache.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Add fragment
        cache.fragments.push(ip_data.to_vec());

        if is_fragment {
            // More fragments coming, wait for them
            return Ok(None);
        }

        // Last fragment received, reassemble
        let fragments = &cache.fragments;
        if fragments.is_empty() {
            fragment_cache.remove(&key);
            return Ok(None);
        }

        // Reassemble fragments based on offset
        let reassembled = self.reassemble_fragments(fragments, packet_count)?;
        fragment_cache.remove(&key);

        Ok(Some(reassembled))
    }

    fn reassemble_fragments(
        &self,
        fragments: &[Vec<u8>],
        packet_count: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if fragments.is_empty() {
            return Err("No fragments to reassemble".into());
        }

        // Find total length and collect fragments with offsets
        let mut fragment_data = Vec::new();
        let mut total_length = 0;

        for fragment in fragments {
            if fragment.len() < 20 {
                continue;
            }

            let ip_header_len = ((fragment[0] & 0x0F) as usize) * 4;
            let total_len = u16::from_be_bytes([fragment[2], fragment[3]]) as usize;
            let data_len = total_len - ip_header_len;
            let flags = fragment[6];
            let fragment_offset = ((flags & 0x1F) as u16) << 8 | fragment[7] as u16;
            let data_offset = fragment_offset as usize * 8;

            let payload_start = ip_header_len;
            let payload_end = std::cmp::min(fragment.len(), payload_start + data_len);
            let payload = &fragment[payload_start..payload_end];

            fragment_data.push((data_offset, payload.to_vec()));

            let end_offset = data_offset + payload.len();
            if end_offset > total_length {
                total_length = end_offset;
            }
        }

        // Sort by offset
        fragment_data.sort_by_key(|(offset, _)| *offset);

        // Reassemble
        let mut result = vec![0u8; total_length];
        for (offset, data) in fragment_data {
            if offset + data.len() <= result.len() {
                result[offset..offset + data.len()].copy_from_slice(&data);
            }
        }

        log::debug!(
            "Reassembled {} fragments into {} bytes for packet #{}",
            fragments.len(),
            result.len(),
            packet_count
        );

        Ok(result)
    }

    fn extract_tcp_payload(
        &self,
        tcp_data: &[u8],
        packet_count: u64,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        if tcp_data.len() < 20 {
            let filtered = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
            log::debug!(
                "Filtered TCP packet #{}: header too short (filtered: {})",
                packet_count,
                filtered
            );
            return Ok(None);
        }

        let tcp_header_len = ((tcp_data[12] >> 4) as usize) * 4;

        if tcp_data.len() <= tcp_header_len {
            // No payload
            return Ok(None);
        }

        let payload = &tcp_data[tcp_header_len..];

        // Validate payload length (prevent buffer overflow)
        if payload.len() > self.config.buffer_size {
            log::warn!(
                "Packet #{} payload too large: {} bytes",
                packet_count,
                payload.len()
            );
            return Ok(None);
        }

        Ok(Some(payload.to_vec()))
    }

    async fn process_tcp_stream(
        &self,
        payload: &[u8],
        packet_tx: &mpsc::Sender<Bytes>,
        packet_count: u64,
        ip_data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _lock = TCP_LOCK.lock().await;

        let server_identified = *SERVER_IDENTIFIED.lock().await;
        let current_server = CURRENT_SERVER.lock().await.clone();

        // Server identification logic
        if !server_identified {
            if self
                .try_identify_server(payload, packet_count, ip_data)
                .await?
            {
                // Server identified, clear caches
                self.clear_tcp_cache().await;
                *TCP_NEXT_SEQ.lock().await = -1;
                log::info!(
                    "Server identified and caches cleared for packet #{}",
                    packet_count
                );
            }
            return Ok(());
        }

        // Process packets from identified server
        if current_server.is_empty() {
            return Ok(());
        }

        // TCP sequence number validation
        let mut tcp_next_seq = TCP_NEXT_SEQ.lock().await;
        if *tcp_next_seq == -1 {
            // Initialize sequence number - extract from TCP header
            let ip_header_len = ((ip_data[0] & 0x0F) as usize) * 4;
            if ip_data.len() >= ip_header_len + 20 {
                let tcp_start = ip_header_len;
                let seq_num = u32::from_be_bytes([
                    ip_data[tcp_start + 4],
                    ip_data[tcp_start + 5],
                    ip_data[tcp_start + 6],
                    ip_data[tcp_start + 7],
                ]);
                *tcp_next_seq = seq_num as i64;
                log::debug!(
                    "Initialized TCP sequence tracking for packet #{}: seq={}",
                    packet_count,
                    seq_num
                );
            }
            return Ok(());
        }

        // Add payload to TCP cache for reassembly
        let mut tcp_cache = TCP_CACHE.lock().await;
        let seq_key = packet_count as u32; // Simplified sequence key
        tcp_cache.insert(seq_key, payload.to_vec());

        // Process available packets in order
        let mut processed_packets = 0;
        let mut keys: Vec<u32> = tcp_cache.keys().cloned().collect();
        keys.sort();

        for seq in keys {
            if let Some(cached_data) = tcp_cache.remove(&seq) {
                log::debug!(
                    "Processing cached TCP packet seq {} for packet #{}",
                    seq,
                    packet_count
                );

                // Send to processing task
                let payload_bytes = Bytes::copy_from_slice(&cached_data);
                if packet_tx.send(payload_bytes).await.is_err() {
                    log::warn!(
                        "Failed to send TCP packet to processing task for packet #{}",
                        packet_count
                    );
                } else {
                    processed_packets += 1;
                }
            }
        }

        if processed_packets > 0 {
            *TCP_LAST_TIME.lock().await = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            log::debug!(
                "Processed {} TCP packets for packet #{}",
                processed_packets,
                packet_count
            );
        }

        Ok(())
    }

    async fn try_identify_server(
        &self,
        payload: &[u8],
        packet_count: u64,
        ip_data: &[u8],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if payload.len() < 10 {
            return Ok(false);
        }

        // Extract source IP and port from IP packet header
        let src_ip = format!(
            "{}.{}.{}.{}",
            ip_data[12], ip_data[13], ip_data[14], ip_data[15]
        );
        let dst_ip = format!(
            "{}.{}.{}.{}",
            ip_data[16], ip_data[17], ip_data[18], ip_data[19]
        );

        // Extract ports from TCP header (after IP header)
        let ip_header_len = ((ip_data[0] & 0x0F) as usize) * 4;
        if ip_data.len() < ip_header_len + 4 {
            return Ok(false);
        }

        let tcp_start = ip_header_len;
        let src_port = u16::from_be_bytes([ip_data[tcp_start], ip_data[tcp_start + 1]]);
        let dst_port = u16::from_be_bytes([ip_data[tcp_start + 2], ip_data[tcp_start + 3]]);

        log::debug!("📦 Payload length: {} bytes", payload.len());
        log::debug!(
            "🌐 Connection: {}:{} -> {}:{}, Payload length: {} bytes",
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            payload.len()
        );

        // Check for game server signature
        if payload[4] == 0 && payload.len() >= 10 {
            let data = &payload[10..];
            if data.len() >= 11 {
                // Check for game protocol signature: 0x00, 0x63, 0x33, 0x53, 0x42, 0x00
                let signature = [0x00, 0x63, 0x33, 0x53, 0x42, 0x00];
                if data.len() >= signature.len() && data[5..5 + signature.len()] == signature {
                    // Found game server signature - use source address as server
                    let server_addr = format!("{}:{}", src_ip, src_port);
                    let mut current_server = CURRENT_SERVER.lock().await;
                    *current_server = server_addr.clone();

                    let mut server_identified = SERVER_IDENTIFIED.lock().await;
                    *server_identified = true;

                    log::info!(
                        "🎯 Game server identified via signature for packet #{}: {}",
                        packet_count,
                        server_addr
                    );
                    return Ok(true);
                }
            }
        }

        // Check for login response signature
        if payload.len() == 0x62 {
            // 98 bytes
            let signature1 = [0x00, 0x00, 0x00, 0x62, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01];
            let signature2 = [
                0x00, 0x11, 0x45, 0x14, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x4e, 0x08, 0x01, 0x22, 0x24,
            ];

            if payload.len() >= 10
                && payload[0..10] == signature1
                && payload.len() >= 24
                && payload[14..24] == signature2[0..10]
            {
                // Found login response - use source address as server
                let server_addr = format!("{}:{}", src_ip, src_port);
                let mut current_server = CURRENT_SERVER.lock().await;
                *current_server = server_addr.clone();

                let mut server_identified = SERVER_IDENTIFIED.lock().await;
                *server_identified = true;

                log::info!(
                    "🎯 Game server identified via login response for packet #{}: {}",
                    packet_count,
                    server_addr
                );
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn clear_tcp_cache(&self) {
        let mut tcp_cache = TCP_CACHE.lock().await;
        tcp_cache.clear();
        let mut data_buffer = DATA_BUFFER.lock().await;
        data_buffer.clear();
        log::debug!("TCP cache cleared");
    }

    fn spawn_cleanup_tasks(&self) {
        let fragment_timeout = self.config.fragment_timeout;
        let connection_timeout = self.config.connection_timeout;

        // Cleanup expired IP fragments
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let mut fragment_cache = IP_FRAGMENT_CACHE.lock().await;
                let mut cleared = 0;
                fragment_cache.retain(|_, cache| {
                    if now - cache.timestamp > fragment_timeout.as_secs() {
                        cleared += 1;
                        false
                    } else {
                        true
                    }
                });

                if cleared > 0 {
                    log::debug!("Cleaned up {} expired IP fragment caches", cleared);
                }
            }
        });

        // Cleanup stale TCP connections
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let tcp_last_time = *TCP_LAST_TIME.lock().await;
                if tcp_last_time > 0 && now - tcp_last_time > connection_timeout.as_secs() {
                    log::warn!("TCP connection timeout detected, clearing caches");
                    let mut current_server = CURRENT_SERVER.lock().await;
                    *current_server = String::new();
                    let mut server_identified = SERVER_IDENTIFIED.lock().await;
                    *server_identified = false;
                    let mut tcp_next_seq = TCP_NEXT_SEQ.lock().await;
                    *tcp_next_seq = -1;
                    let mut mismatched_packets = MISMATCHED_PACKETS.lock().await;
                    *mismatched_packets = 0;
                }
            }
        });
    }

    pub fn update_filter(&mut self, filter: String) {
        self.config.filter = filter;
        log::info!("Updated packet filter to: {}", self.config.filter);
    }

    pub fn get_current_filter(&self) -> &str {
        &self.config.filter
    }

    pub async fn get_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert(
            "packets_captured".to_string(),
            PACKET_COUNTER.load(Ordering::SeqCst),
        );
        stats.insert(
            "packets_filtered".to_string(),
            FILTERED_PACKETS.load(Ordering::SeqCst),
        );
        stats.insert(
            "mismatched_packets".to_string(),
            *MISMATCHED_PACKETS.lock().await as u64,
        );

        let tcp_cache = TCP_CACHE.lock().await;
        stats.insert("tcp_cache_size".to_string(), tcp_cache.len() as u64);

        let fragment_cache = IP_FRAGMENT_CACHE.lock().await;
        stats.insert(
            "fragment_cache_size".to_string(),
            fragment_cache.len() as u64,
        );

        stats
    }
}

// Utility functions for network interface management
pub fn list_network_interfaces() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // This is a simplified implementation
    // In a real implementation, you would enumerate network adapters
    let interfaces = vec![
        "\\Device\\NPF_{1C8323DC-4E7D-4D2A-B1D2-5B6C7D8E9F0A}".to_string(),
        "\\Device\\NPF_{2D943EDC-5F8E-5E3B-C2E3-6C7D8E9F0A1B}".to_string(),
    ];

    Ok(interfaces)
}

pub fn find_default_interface() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Simplified implementation - would normally detect the default interface
    // by checking routing table or using Windows API
    let interfaces = list_network_interfaces()?;
    interfaces
        .into_iter()
        .next()
        .ok_or_else(|| "No network interfaces found".into())
}

// TCP packet processing utilities
pub struct TcpPacketInfo {
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
    pub src_port: u16,
    pub dst_port: u16,
    pub sequence_number: u32,
    pub ack_number: u32,
    pub flags: u8,
    pub window_size: u16,
    pub payload_offset: usize,
    pub payload: Vec<u8>,
}

impl TcpPacketInfo {
    pub fn parse(packet_data: &[u8]) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if packet_data.len() < 40 {
            // Minimum IP + TCP header
            return Err("Packet too short".into());
        }

        // Parse IP header (simplified)
        let ip_header_len = ((packet_data[0] & 0x0F) * 4) as usize;
        let src_ip = [
            packet_data[12],
            packet_data[13],
            packet_data[14],
            packet_data[15],
        ];
        let dst_ip = [
            packet_data[16],
            packet_data[17],
            packet_data[18],
            packet_data[19],
        ];

        if packet_data.len() < ip_header_len + 20 {
            return Err("TCP header incomplete".into());
        }

        let tcp_start = ip_header_len;
        let src_port = u16::from_be_bytes([packet_data[tcp_start], packet_data[tcp_start + 1]]);
        let dst_port = u16::from_be_bytes([packet_data[tcp_start + 2], packet_data[tcp_start + 3]]);
        let sequence_number = u32::from_be_bytes([
            packet_data[tcp_start + 4],
            packet_data[tcp_start + 5],
            packet_data[tcp_start + 6],
            packet_data[tcp_start + 7],
        ]);
        let ack_number = u32::from_be_bytes([
            packet_data[tcp_start + 8],
            packet_data[tcp_start + 9],
            packet_data[tcp_start + 10],
            packet_data[tcp_start + 11],
        ]);

        let tcp_header_len = ((packet_data[tcp_start + 12] >> 4) * 4) as usize;
        let flags = packet_data[tcp_start + 13];
        let window_size =
            u16::from_be_bytes([packet_data[tcp_start + 14], packet_data[tcp_start + 15]]);

        let payload_offset = tcp_start + tcp_header_len;
        let payload = if payload_offset < packet_data.len() {
            packet_data[payload_offset..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            sequence_number,
            ack_number,
            flags,
            window_size,
            payload_offset,
            payload,
        })
    }

    pub fn has_payload(&self) -> bool {
        !self.payload.is_empty()
    }

    pub fn is_syn(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    pub fn is_ack(&self) -> bool {
        (self.flags & 0x10) != 0
    }

    pub fn is_fin(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    pub fn is_rst(&self) -> bool {
        (self.flags & 0x04) != 0
    }
}

// TCP connection state tracking
#[derive(Debug, Clone)]
pub struct TcpConnection {
    pub client_ip: [u8; 4],
    pub server_ip: [u8; 4],
    pub client_port: u16,
    pub server_port: u16,
    pub state: TcpState,
    pub next_seq_client: u32,
    pub next_seq_server: u32,
    pub last_activity: std::time::Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

pub struct TcpConnectionTracker {
    connections: std::collections::HashMap<ConnectionKey, TcpConnection>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ConnectionKey {
    client_ip: [u8; 4],
    server_ip: [u8; 4],
    client_port: u16,
    server_port: u16,
}

impl TcpConnectionTracker {
    pub fn new() -> Self {
        Self {
            connections: std::collections::HashMap::new(),
        }
    }

    pub fn process_packet(&mut self, packet_info: &TcpPacketInfo) -> Option<&TcpConnection> {
        let key = ConnectionKey {
            client_ip: packet_info.src_ip,
            server_ip: packet_info.dst_ip,
            client_port: packet_info.src_port,
            server_port: packet_info.dst_port,
        };

        let connection = self
            .connections
            .entry(key)
            .or_insert_with(|| TcpConnection {
                client_ip: packet_info.src_ip,
                server_ip: packet_info.dst_ip,
                client_port: packet_info.src_port,
                server_port: packet_info.dst_port,
                state: TcpState::Closed,
                next_seq_client: 0,
                next_seq_server: 0,
                last_activity: std::time::Instant::now(),
            });

        // Update connection state based on TCP flags and sequence numbers
        match connection.state {
            TcpState::Closed => {
                if packet_info.is_syn() {
                    connection.state = TcpState::SynSent;
                    connection.next_seq_client = packet_info.sequence_number + 1;
                }
            }
            TcpState::SynSent => {
                if packet_info.is_syn() && packet_info.is_ack() {
                    connection.state = TcpState::Established;
                    connection.next_seq_server = packet_info.sequence_number + 1;
                }
            }
            TcpState::Established => {
                connection.last_activity = std::time::Instant::now();
                // Update sequence numbers for data tracking
                if packet_info.src_port == connection.client_port {
                    connection.next_seq_client =
                        packet_info.sequence_number + packet_info.payload.len() as u32;
                } else {
                    connection.next_seq_server =
                        packet_info.sequence_number + packet_info.payload.len() as u32;
                }
            }
            _ => {
                // Handle other states as needed
            }
        }

        Some(connection)
    }

    pub fn cleanup_stale_connections(&mut self, max_age: std::time::Duration) {
        self.connections
            .retain(|_, conn| conn.last_activity.elapsed() < max_age);
    }

    pub fn get_connection(&self, key: &ConnectionKey) -> Option<&TcpConnection> {
        self.connections.get(key)
    }
}
