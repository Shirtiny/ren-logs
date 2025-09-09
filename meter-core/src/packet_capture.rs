//! 使用WinDivert进行网络数据包捕获

const BUF_SIZE: usize = 10 * 1024 * 1024; // 10MB缓冲区

use crate::{MeterError, Result};
use crate::utils;
use async_channel::{Receiver, Sender};
use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::task;
use windivert::prelude::*;

// PacketCapture 结构体包装
pub struct PacketCapture {
    filter: String,
}

impl PacketCapture {
    pub fn new(_data_manager: Arc<crate::data_manager::DataManager>) -> Self {
        Self {
            filter: "ip and tcp".to_string(),
        }
    }

    pub async fn start_capture(&mut self) -> Result<()> {
        let rx = start_capture(self.filter.clone())?;
        log::info!("Packet capture started");

        // 这里可以启动一个任务来处理接收到的数据包
        tokio::spawn(async move {
            while let Ok((opcode, data)) = rx.recv().await {
                // 处理接收到的数据包
                log::debug!("Received packet: opcode=0x{:04x}, size={}", opcode, data.len());
            }
        });

        Ok(())
    }

    pub fn update_filter(&mut self, filter: String) {
        self.filter = filter;
        log::info!("Updated packet filter to: {}", self.filter);
    }

    pub fn get_current_filter(&self) -> &str {
        &self.filter
    }
}

// 全局状态变量
lazy_static::lazy_static! {
    static ref CURRENT_SERVER: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref SERVER_IDENTIFIED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref TCP_CACHE: Arc<Mutex<BTreeMap<u32, Vec<u8>>>> = Arc::new(Mutex::new(BTreeMap::new()));
    static ref TCP_NEXT_SEQ: Arc<Mutex<i64>> = Arc::new(Mutex::new(-1));
    static ref TCP_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    static ref DATA_BUFFER: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    static ref TCP_LAST_TIME: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    // 调试计数器
    static ref PACKET_COUNTER: AtomicU64 = AtomicU64::new(0);
    static ref FILTERED_PACKETS: AtomicU64 = AtomicU64::new(0);
    // 服务器切换检测计数器
    static ref MISMATCHED_PACKETS: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

// 解析IP头部并返回TCP数据包
fn parse_ip_header(ip_data: &[u8]) -> Result<(&[u8], String, String, u16, u16)> {
    if ip_data.len() < 20 {
        return Err(MeterError::ParseError("IP数据包太小".to_string()));
    }

    // 检查IP版本
    let ip_version = ip_data[0] >> 4;
    if ip_version != 4 {
        return Err(MeterError::ParseError("不是IPv4".to_string()));
    }

    // IP头部长度
    let ip_header_len = ((ip_data[0] & 0x0F) as usize) * 4;
    if ip_data.len() < ip_header_len + 20 {
        return Err(MeterError::ParseError(
            "数据包太小，没有TCP头部".to_string(),
        ));
    }

    // 检查协议
    let protocol = ip_data[9];
    if protocol != 6 {
        return Err(MeterError::ParseError("不是TCP协议".to_string()));
    }

    // 提取源和目的IP地址
    let src_ip = format!(
        "{}.{}.{}.{}",
        ip_data[12], ip_data[13], ip_data[14], ip_data[15]
    );
    let dst_ip = format!(
        "{}.{}.{}.{}",
        ip_data[16], ip_data[17], ip_data[18], ip_data[19]
    );

    Ok((&ip_data[ip_header_len..], src_ip, dst_ip, 0, 0)) // 暂时返回0端口
}

// 解析TCP头部并返回payload
fn parse_tcp_header(tcp_data: &[u8]) -> Result<(&[u8], u16, u16, u32)> {
    if tcp_data.len() < 20 {
        return Err(MeterError::ParseError("TCP数据包太小".to_string()));
    }

    // TCP头部长度
    let tcp_header_len = ((tcp_data[12] >> 4) as usize) * 4;

    // 提取端口
    let src_port = u16::from_be_bytes([tcp_data[0], tcp_data[1]]);
    let dst_port = u16::from_be_bytes([tcp_data[2], tcp_data[3]]);

    // 提取序列号
    let seq_no = u32::from_be_bytes([tcp_data[4], tcp_data[5], tcp_data[6], tcp_data[7]]);

    let payload_offset = tcp_header_len;
    if tcp_data.len() <= payload_offset {
        return Err(MeterError::ParseError("没有TCP payload".to_string()));
    }

    Ok((&tcp_data[payload_offset..], src_port, dst_port, seq_no))
}

// 尝试通过小包识别服务器
async fn try_identify_server_by_small_packet(buf: &[u8], src_server: &str) -> Result<()> {
    if buf.len() <= 10 {
        log::debug!("🔍 小包识别跳过 - 数据包太小: {} bytes", buf.len());
        return Ok(());
    }

    // 检查buf[4] == 0
    if buf[4] != 0 {
        log::debug!("🔍 小包识别跳过 - buf[4] != 0: 0x{:02x}", buf[4]);
        return Ok(());
    }

    let data = &buf[10..];
    if data.is_empty() {
        log::debug!("🔍 小包识别跳过 - 数据部分为空");
        return Ok(());
    }

    log::debug!("🔍 开始小包识别 - 解析数据流，大小: {} bytes", data.len());

    // 解析数据流
    let mut offset = 0;
    while offset + 4 <= data.len() {
        let len_bytes = &data[offset..offset + 4];
        let packet_len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        if packet_len == 0 || offset + 4 + packet_len > data.len() {
            log::debug!(
                "🔍 小包识别结束 - 无效包长度或超出边界 (offset: {}, packet_len: {})",
                offset,
                packet_len
            );
            break;
        }

        let packet_data = &data[offset + 4..offset + 4 + packet_len];
        if packet_data.len() >= 11 {
            // 检查签名 0x00, 0x63, 0x33, 0x53, 0x42, 0x00
            let signature = [0x00, 0x63, 0x33, 0x53, 0x42, 0x00];
            if packet_data[5..5 + signature.len()] == signature {
                // 找到匹配的签名，更新服务器
                let mut current_server = CURRENT_SERVER.lock().await;
                if *current_server != src_server {
                    log::info!("🎯 通过小包识别找到游戏服务器!");
                    log::info!("🏠 服务器地址: {}", src_server);
                    log::info!("🔍 匹配签名: {:02x?} (偏移量: 5)", signature);
                    log::info!("📦 数据包大小: {} bytes", packet_len);
                    log::info!("✅ 服务器识别完成，开始跟踪该连接的数据包");

                    *current_server = src_server.to_string();

                    // 设置服务器已识别状态
                    let mut server_identified = SERVER_IDENTIFIED.lock().await;
                    *server_identified = true;

                    clear_tcp_cache().await;
                    let mut tcp_next_seq = TCP_NEXT_SEQ.lock().await;
                    *tcp_next_seq = -1;
                    clear_data_on_server_change();
                }
                return Ok(());
            }
        }

        offset += 4 + packet_len;
    }

    log::debug!("🔍 小包识别完成 - 未找到匹配的签名");
    Ok(())
}

// 尝试通过登录返回包识别服务器
async fn try_identify_server_by_login_response(buf: &[u8], src_server: &str) -> Result<()> {
    if buf.len() != 0x62 {
        log::debug!(
            "🔍 登录返回包识别跳过 - 数据包大小不匹配: {} bytes (期望: 98 bytes)",
            buf.len()
        );
        return Ok(());
    }

    log::debug!("🔍 开始登录返回包识别 - 数据包大小: {} bytes", buf.len());

    // 签名模式
    let signature = [
        0x00, 0x00, 0x00, 0x62, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x11, 0x45, 0x14, 0x00,
        0x00, 0x00, 0x00, 0x0a, 0x4e, 0x08, 0x01, 0x22, 0x24,
    ];

    // 检查签名匹配
    let signature1_match = buf.len() >= 10 && buf[0..10] == signature[0..10];
    let signature2_match = buf.len() >= 20 && buf[14..20] == signature[14..20];

    log::debug!(
        "🔍 签名匹配检查 - 签名1: {}, 签名2: {}",
        signature1_match,
        signature2_match
    );

    if signature1_match && signature2_match {
        let mut current_server = CURRENT_SERVER.lock().await;
        if *current_server != src_server {
            log::info!("🎯 通过登录返回包识别找到游戏服务器!");
            log::info!("🏠 服务器地址: {}", src_server);
            log::info!("🔍 匹配签名模式: 98字节登录返回包");
            log::info!("📦 数据包大小: {} bytes", buf.len());

            *current_server = src_server.to_string();

            // 设置服务器已识别状态
            let mut server_identified = SERVER_IDENTIFIED.lock().await;
            *server_identified = true;

            clear_tcp_cache().await;
            let mut tcp_next_seq = TCP_NEXT_SEQ.lock().await;
            *tcp_next_seq = -1;
            clear_data_on_server_change();

            log::info!("✅ 服务器识别完成，开始跟踪该连接的数据包");
        }
    } else {
        log::debug!("🔍 登录返回包识别完成 - 签名不匹配");
    }

    Ok(())
}

// 清空TCP缓存
async fn clear_tcp_cache() {
    let mut cache = TCP_CACHE.lock().await;
    cache.clear();
}

// 服务器变更时清空数据
fn clear_data_on_server_change() {
    // 这里可以添加清理逻辑
}

// 处理数据缓冲区，提取完整的数据包
async fn process_data_buffer(
    data_buffer: &mut Vec<u8>,
    tx: &Sender<(u16, Vec<u8>)>,
) -> Result<usize> {
    log::debug!("🔄 进入数据缓冲区处理函数 - 缓冲区大小: {} bytes", data_buffer.len());
    let mut processed_count = 0;

    while data_buffer.len() > 4 {
        let packet_size = u32::from_be_bytes([
            data_buffer[0],
            data_buffer[1],
            data_buffer[2],
            data_buffer[3],
        ]) as usize;

        // 验证包长度是否合理（避免解析错误导致的巨大值）
        if packet_size > 10 * 1024 * 1024 { // 10MB上限
            log::warn!("⚠️ 检测到异常大的数据包长度: {} bytes，可能是解析错误", packet_size);

            // 调试：打印前16个字节的内容，帮助分析数据格式
            if data_buffer.len() >= 16 {
                log::debug!("🔍 前16字节数据: {:02x?}", &data_buffer[0..16]);
            } else {
                log::debug!("🔍 缓冲区数据: {:02x?}", data_buffer);
            }

            data_buffer.clear();
            break;
        }

        log::debug!("🔍 解析数据包长度: {} bytes (缓冲区大小: {} bytes)", packet_size, data_buffer.len());

        if data_buffer.len() < packet_size {
            log::debug!(
                "📊 数据缓冲区等待更多数据 - 需要: {} bytes, 当前: {} bytes",
                packet_size,
                data_buffer.len()
            );
            break;
        }

        if data_buffer.len() >= packet_size {
            let packet = data_buffer[0..packet_size].to_vec();
            *data_buffer = data_buffer[packet_size..].to_vec();

            log::debug!("📦 提取完整数据包 - 大小: {} bytes", packet.len());

            // 发送数据包
            if packet.len() >= 6 {
                let opcode = u16::from_le_bytes([packet[4], packet[5]]);
                let data = packet[6..].to_vec();

                log::debug!("🔍 数据包格式检查通过 - Opcode: 0x{:04x}, 数据大小: {} bytes", opcode, data.len());

                // 记录服务器通信数据包的完整载荷（过滤掉4字节的小包）
                if data.len() > 4 {
                    log::info!(
                        "📤 [服务器通信] Opcode: 0x{:04x} | 载荷大小: {} bytes",
                        opcode,
                        data.len()
                    );
                    if !data.is_empty() {
                        let hex_dump = format_hex_dump(&data);
                        log::info!("📦 载荷数据:\n{}", hex_dump);
                    }
                }

                log::debug!(
                    "📤 发送数据包 - Opcode: 0x{:04x}, 大小: {} bytes",
                    opcode,
                    data.len()
                );

                if let Err(e) = tx.send((opcode, data)).await {
                    log::error!("发送数据包失败: {:?}", e);
                } else {
                    processed_count += 1;
                }
            } else {
                log::debug!("⚠️ 跳过数据包 - 大小不足: {} bytes (需要至少6字节)", packet.len());
            }
        } else if packet_size > 0x0fffff {
            log::warn!("⚠️ 检测到无效数据包长度: {} bytes，清空缓冲区", packet_size);
            data_buffer.clear();
            break;
        }
    }

    Ok(processed_count)
}

// 格式化字节数组为十六进制字符串
fn format_hex_dump(data: &[u8]) -> String {
    let mut result = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = i * 16;
        result.push_str(&format!("{:04x}: ", offset));
        for &byte in chunk {
            result.push_str(&format!("{:02x} ", byte));
        }
        // 补齐到16字节
        if chunk.len() < 16 {
            for _ in 0..(16 - chunk.len()) {
                result.push_str("   ");
            }
        }
        result.push('\n');
    }
    result.trim_end().to_string()
}

// 重置服务器识别状态（用于重新开始服务器识别）
pub async fn reset_server_identification() {
    let mut server_identified = SERVER_IDENTIFIED.lock().await;
    *server_identified = false;

    let mut current_server = CURRENT_SERVER.lock().await;
    *current_server = String::new();

    // 重置不匹配计数器
    let mut mismatched_packets = MISMATCHED_PACKETS.lock().await;
    *mismatched_packets = 0;

    clear_tcp_cache().await;

    let mut tcp_next_seq = TCP_NEXT_SEQ.lock().await;
    *tcp_next_seq = -1;

    clear_data_on_server_change();

    log::info!("🔄 服务器识别状态已重置，可以重新开始识别游戏服务器");
}

// 获取捕获统计信息
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub packets_captured: u64,
    pub packets_processed: u64,
    pub packets_dropped: u64,
    pub uptime_seconds: u64,
}

pub fn get_capture_stats() -> CaptureStats {
    // TODO: 实现实际的统计跟踪
    CaptureStats {
        packets_captured: 0,
        packets_processed: 0,
        packets_dropped: 0,
        uptime_seconds: 0,
    }
}

/// 数据包捕获配置
#[derive(Debug, Clone)]
pub struct PacketCaptureConfig {
    pub filter: String,
}

/// 捕获的数据包数据
#[derive(Debug, Clone)]
pub struct PacketData {
    pub opcode: u16,
    pub data: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}

/// 在所有TCP端口启动数据包捕获
pub fn start_capture(filter: String) -> Result<Receiver<(u16, Vec<u8>)>> {
    let (tx, rx) = async_channel::unbounded();

    log::info!("使用WinDivert过滤器: {}", filter);

    // 启动捕获任务
    task::spawn(async move {
        if let Err(e) = run_capture(filter, tx).await {
            log::error!("数据包捕获失败: {:?}", e);
        }
    });

    Ok(rx)
}

/// 内部捕获函数，具有完整的WinDivert实现
async fn run_capture(filter: String, tx: Sender<(u16, Vec<u8>)>) -> Result<()> {
    // 检查WinDivert是否可用
    if !crate::utils::is_windivert_installed() {
        return Err(MeterError::WinDivertError(
            "未找到WinDivert驱动。请确保WinDivert64.sys已安装到应用程序目录。".to_string(),
        ));
    }

    // 检查管理员权限
    if !crate::utils::is_admin() {
        log::warn!("WinDivert需要管理员权限，但当前进程没有管理员权限");
        return Err(MeterError::WinDivertError(
            "WinDivert需要管理员权限。请以管理员身份运行应用程序。".to_string(),
        ));
    }

    log::info!("开始捕获所有TCP端口的数据包");

    // 创建网络层的WinDivert句柄
    let handle = WinDivert::<NetworkLayer>::network(&filter, 0, WinDivertFlags::new())
        .map_err(|e| MeterError::WinDivertError(format!("创建WinDivert句柄失败: {}", e)))?;

    log::info!("WinDivert句柄创建成功，过滤器: {}", filter);

    loop {
        let mut buffer = vec![0u8; BUF_SIZE]; // 10MB缓冲区，用于容纳大型网络数据包

        // 接收数据包
        match handle.recv(Some(&mut buffer[..])) {
            Ok(packet) => {
                // 处理捕获的数据包
                if let Err(e) = process_packet(&packet.data, &tx).await {
                    log::warn!("处理数据包失败: {:?}", e);
                }

                // 将数据包重新注入网络栈
                if let Err(e) = handle.send(&packet) {
                    log::warn!("重新注入数据包失败: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("接收数据包失败: {:?}", e);
                // 小延迟以防止错误时忙等待
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // 检查是否应该停止（生产环境中会通过关闭信号控制）
        // 现在将无限运行直到任务被取消
    }
}

/// 处理捕获的数据包并提取相关数据
async fn process_packet(packet_data: &[u8], tx: &Sender<(u16, Vec<u8>)>) -> Result<()> {
    // 数据包计数器
    let packet_count = PACKET_COUNTER.fetch_add(1, Ordering::SeqCst);

    // WinDivert NetworkLayer 返回的是IP数据包，直接解析IP头部
    // 不需要解析以太网头部
    let ip_data = packet_data;

    // 解析IP头部
    let (tcp_data, src_ip, dst_ip, _, _) = match parse_ip_header(ip_data) {
        Ok(result) => {
            // 排除本地回环地址的数据包
            if result.2 == "127.0.0.1" {
                return Ok(());
            }

            // 成功解析第一个非本地IP数据包时记录一次
            static FIRST_SUCCESS: AtomicU64 = AtomicU64::new(0);
            if FIRST_SUCCESS.fetch_add(1, Ordering::SeqCst) == 0 {
                log::info!("🎉 开始捕获网络数据包");
            }
            result
        }
        Err(e) => {
            let filtered_count = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
            // log::debug!(
            //     "❌ 跳过非TCP数据包 #{}: {} (总过滤: {})",
            //     packet_count,
            //     e,
            //     filtered_count
            // );
            return Ok(());
        }
    };

    // 解析TCP头部
    let (payload, src_port, dst_port, seq_no) = match parse_tcp_header(tcp_data) {
        Ok(result) => result,
        Err(e) => {
            let filtered_count = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
            // log::debug!(
            //     "❌ 跳过无payload数据包 #{}: {} (总过滤: {})",
            //     packet_count,
            //     e,
            //     filtered_count
            // );
            return Ok(());
        }
    };

    let src_server = format!("{}:{} -> {}:{}", src_ip, src_port, dst_ip, dst_port);

    // 获取TCP锁
    let _lock = TCP_LOCK.lock().await;

    // 检查服务器是否已经识别
    let server_identified = SERVER_IDENTIFIED.lock().await.clone();
    // log::debug!("🔍 服务器识别状态: {}", server_identified);

    let mut current_server = CURRENT_SERVER.lock().await;
    if *current_server != src_server {
        if !server_identified {
            // 服务器未识别，记录数据包并尝试识别
            log::debug!(
                "📦 #{}: {}:{} -> {}:{} | 序列号: {} | Payload: {} bytes",
                packet_count,
                src_ip,
                src_port,
                dst_ip,
                dst_port,
                seq_no,
                payload.len()
            );

            // 尝试识别
            drop(current_server); // 释放锁

            if let Err(e) = try_identify_server_by_small_packet(payload, &src_server).await {
                log::warn!("小包识别失败: {:?}", e);
            }

            if let Err(e) = try_identify_server_by_login_response(payload, &src_server).await {
                log::warn!("登录返回包识别失败: {:?}", e);
            }

            // 尝试模拟服务器识别（用于调试）
            // if let Err(e) = try_simulate_server_identification(&src_server).await {
            //     log::warn!("模拟识别失败: {:?}", e);
            // }

            // 重新获取锁
            let current_server = CURRENT_SERVER.lock().await;
            if *current_server != src_server {
                // 识别失败，跳过该数据包
                let filtered_count = FILTERED_PACKETS.fetch_add(1, Ordering::SeqCst);
                // log::debug!(
                //     "❌ 跳过未识别服务器数据包 #{}: {} (总过滤: {})",
                //     packet_count,
                //     src_server,
                //     filtered_count
                // );
                drop(current_server);
                drop(_lock);
                return Ok(());
            }
        } else {
            // 服务器已识别，检查是否是已识别的服务器（双向匹配）
            let reverse_server = format!("{}:{} -> {}:{}", dst_ip, dst_port, src_ip, src_port);
            if *current_server != src_server && *current_server != reverse_server {
                // 不是已识别的服务器，增加不匹配计数器
                let mut mismatched_packets = MISMATCHED_PACKETS.lock().await;
                *mismatched_packets += 1;

                log::debug!(
                    "⚠️ 检测到非目标服务器数据包 #{}: {} (当前服务器: {}, 不匹配计数: {})",
                    packet_count,
                    src_server,
                    *current_server,
                    *mismatched_packets
                );

                // 如果连续不匹配数据包数量超过阈值，触发服务器切换
                const SWITCH_THRESHOLD: u32 = 5;
                if *mismatched_packets >= SWITCH_THRESHOLD {
                    log::warn!("🔄 检测到服务器切换！连续{}个数据包来自不同服务器", SWITCH_THRESHOLD);
                    log::warn!("🔄 当前服务器: {}", *current_server);
                    log::warn!("🔄 新服务器地址: {}", src_server);

                    // 重置服务器识别状态
                    drop(current_server); // 释放锁
                    drop(mismatched_packets); // 释放锁

                    reset_server_identification().await;

                    log::info!("🔄 服务器切换处理完成，等待新数据包重新识别");

                    drop(_lock);
                    return Ok(());
                } else {
                    drop(current_server);
                    drop(mismatched_packets);
                    drop(_lock);
                    return Ok(());
                }
            } else {
                // 是已识别的服务器，重置不匹配计数器
                let mut mismatched_packets = MISMATCHED_PACKETS.lock().await;
                if *mismatched_packets > 0 {
                    log::debug!("✅ 服务器匹配，重置不匹配计数器 (之前: {})", *mismatched_packets);
                    *mismatched_packets = 0;
                }

                // 记录数据包
                log::debug!(
                    "📦 #{}: {}:{} -> {}:{} | 序列号: {} | Payload: {} bytes",
                    packet_count,
                    src_ip,
                    src_port,
                    dst_ip,
                    dst_port,
                    seq_no,
                    payload.len()
                );
            }
        }
    } else {
        // 是已识别的服务器，记录数据包
        log::debug!(
            "📦 #{}: {}:{} -> {}:{} | 序列号: {} | Payload: {} bytes",
            packet_count,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            seq_no,
            payload.len()
        );
    }

    // 处理识别的服务器数据包 - 简化TCP重组逻辑
    let mut tcp_cache = TCP_CACHE.lock().await;

    // 对于识别的服务器，简单地将所有数据包加入缓存，不进行严格的序列号检查
    // 因为双向通信的序列号是独立的
    tcp_cache.insert(seq_no, payload.to_vec());

    // 立即处理缓存中的数据包（简化逻辑）
    let mut data_buffer = DATA_BUFFER.lock().await;
    let mut processed_packets = 0;

    // 按序列号顺序处理所有缓存的数据包
    let mut seq_keys: Vec<u32> = tcp_cache.keys().cloned().collect();
    seq_keys.sort();

    for seq in seq_keys {
        if let Some(cached_data) = tcp_cache.remove(&seq) {
            let cached_len = cached_data.len() as u32;
            log::debug!(
                "🔄 处理缓存数据包 - 序列号: {}, 大小: {} bytes",
                seq,
                cached_len
            );

            let buffer_before = data_buffer.len();
            if data_buffer.is_empty() {
                *data_buffer = cached_data;
            } else {
                data_buffer.extend_from_slice(&cached_data);
            }
            let buffer_after = data_buffer.len();
            log::debug!(
                "📊 数据缓冲区更新 - 之前: {} bytes, 之后: {} bytes",
                buffer_before,
                buffer_after
            );

            // 处理数据缓冲区
            let packets_from_buffer = process_data_buffer(&mut data_buffer, tx).await?;
            processed_packets += packets_from_buffer;
        }
    }

    if processed_packets > 0 {
        log::debug!("📤 已处理并发送 {} 个数据包到通道", processed_packets);
    }

    Ok(())
}
