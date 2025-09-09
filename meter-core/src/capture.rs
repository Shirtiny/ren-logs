//! 使用WinDivert进行网络数据包捕获

const BUF_SIZE: usize = 10 * 1024 * 1024; // 10MB缓冲区

use crate::{MeterError, Result};
use async_channel::{Receiver, Sender};
use lazy_static::lazy_static;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use windivert::prelude::*;

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

/// 数据包捕获配置
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub filter: String,
    pub region_file_path: String,
}

/// 捕获的数据包数据
#[derive(Debug, Clone)]
pub struct PacketData {
    pub opcode: u16,
    pub data: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}

/// 在所有TCP端口启动数据包捕获
pub fn start_capture(region_file_path: String) -> Result<Receiver<(u16, Vec<u8>)>> {
    let (tx, rx) = async_channel::unbounded();

    // 尝试不同的过滤器设置
    let filter = "ip and tcp".to_string();
    // 或者尝试: "tcp" 或 "ip"

    let config = CaptureConfig {
        filter: filter.clone(),
        region_file_path,
    };

    log::info!("使用WinDivert过滤器: {}", filter);

    // 启动捕获任务
    task::spawn(async move {
        if let Err(e) = run_capture(config, tx).await {
            log::error!("数据包捕获失败: {:?}", e);
        }
    });

    Ok(rx)
}

/// 内部捕获函数，具有完整的WinDivert实现
async fn run_capture(config: CaptureConfig, tx: Sender<(u16, Vec<u8>)>) -> Result<()> {
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
    let handle = WinDivert::<NetworkLayer>::network(&config.filter, 0, WinDivertFlags::new())
        .map_err(|e| MeterError::WinDivertError(format!("创建WinDivert句柄失败: {}", e)))?;

    log::info!("WinDivert句柄创建成功，过滤器: {}", config.filter);

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

/// 解析以太网头部并返回IP数据包
fn parse_ethernet_header(packet_data: &[u8]) -> Result<&[u8]> {
    if packet_data.len() < 14 {
        return Err(MeterError::ParseError(
            "数据包太小，没有以太网头部".to_string(),
        ));
    }

    // 以太网类型字段（偏移12-13）
    let eth_type = u16::from_be_bytes([packet_data[12], packet_data[13]]);

    // 检查是否为IPv4 (0x0800)
    if eth_type != 0x0800 {
        return Err(MeterError::ParseError("不是IPv4数据包".to_string()));
    }

    Ok(&packet_data[14..])
}

/// 解析IP头部并返回TCP数据包
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

/// 解析TCP头部并返回payload
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

/// 尝试通过小包识别服务器
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

/// 尝试通过登录返回包识别服务器
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

/// 模拟服务器识别（用于测试目的，已注释）
async fn try_simulate_server_identification(src_server: &str) -> Result<()> {
    // 模拟游戏服务器地址
    const SIMULATED_SERVER_IP: &str = "118.195.195.148";

    // 检查是否包含模拟服务器地址
    if src_server.contains(SIMULATED_SERVER_IP) {
        let mut current_server = CURRENT_SERVER.lock().await;
        if *current_server != src_server {
            log::info!("🎯 [模拟] 识别到游戏服务器!");
            log::info!("🏠 服务器地址: {} (模拟)", src_server);
            log::info!("🔍 模拟识别模式: 包含IP {}", SIMULATED_SERVER_IP);

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
        return Ok(());
    }

    Ok(())
}

/// 清空TCP缓存
async fn clear_tcp_cache() {
    let mut cache = TCP_CACHE.lock().await;
    cache.clear();
}

/// 服务器变更时清空数据
fn clear_data_on_server_change() {
    // 这里可以添加清理逻辑
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

/// 格式化字节数组为十六进制字符串
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

/// 处理数据缓冲区，提取完整的数据包
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

/// 停止数据包捕获（占位符 - 实际实现需要句柄管理）
pub fn stop_capture() -> Result<()> {
    log::info!("停止数据包捕获");
    // TODO: 实现正确的捕获停止
    Ok(())
}

/// 重置服务器识别状态（用于重新开始服务器识别）
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

/// 获取捕获统计信息
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

use std::net::Ipv4Addr;

/// Send forged packets to the server (real implementation)
pub async fn send_forged_packets_simple() -> Result<()> {
    log::info!("Starting real packet forging...");

    // Get server connection information
    let current_server = CURRENT_SERVER.lock().await.clone();
    if current_server.is_empty() {
        log::warn!("No server connection available for packet forging");
        return Ok(());
    }

    log::info!("Server connection: {}", current_server);

    // Parse server connection info
    // Format: "client_ip:client_port -> server_ip:server_port"
    let parts: Vec<&str> = current_server.split(" -> ").collect();
    if parts.len() != 2 {
        log::error!("Invalid server connection format: {}", current_server);
        return Err(MeterError::ParseError("Invalid server connection format".to_string()));
    }

    let client_parts: Vec<&str> = parts[0].split(':').collect();
    let server_parts: Vec<&str> = parts[1].split(':').collect();

    if client_parts.len() != 2 || server_parts.len() != 2 {
        log::error!("Invalid IP:port format in connection: {}", current_server);
        return Err(MeterError::ParseError("Invalid IP:port format".to_string()));
    }

    let client_ip: Ipv4Addr = client_parts[0].parse()
        .map_err(|_| MeterError::ParseError("Invalid client IP".to_string()))?;
    let client_port: u16 = client_parts[1].parse()
        .map_err(|_| MeterError::ParseError("Invalid client port".to_string()))?;
    let server_ip: Ipv4Addr = server_parts[0].parse()
        .map_err(|_| MeterError::ParseError("Invalid server IP".to_string()))?;
    let server_port: u16 = server_parts[1].parse()
        .map_err(|_| MeterError::ParseError("Invalid server port".to_string()))?;

    // Initialize forge system and set server connection
    crate::forge::init_forge_system().await?;
    let server_conn = crate::forge::ServerConnection {
        client_ip,
        server_ip,
        client_port,
        server_port,
    };
    crate::forge::set_server_connection(server_conn).await?;

    // Parse the hex data from the user
    let packet1_hex = "00 00 83 75 00 00 00 44 00 02 00 00 00 00 63 33 53 42 00 00 00 00 00 00 00 2d 0a 2c 08 c0 80 ac 02 12 23 12 02 08 68 12 0a 08 72 12 06 a2 c7 a7 c9 92 33 12 03 08 da 03 12 03 08 c8 03 12 07 08 bb 03 12 02 b8 17 1a 00 00 00 00 9b 00 02 00 00 00 00 63 33 53 42 00 00 00 00 00 00 00 2e 0a 82 01 0a 79 08 80 85 84 93 d6 08 12 65 12 03 08 d8 4f 12 03 08 d9 4f 12 03 08 da 4f 12 03 08 e2 4f 12 03 08 e3 4f 12 03 08 e4 4f 12 03 08 ec 4f 12 03 08 ed 4f 12 03 08 ee 4f 12 07 08 c2 58 12 02 99 15 12 07 08 c3 58 12 02 99 15 12 07 08 c6 58 12 02 a6 04 12 03 08 d0 58 12 02 08 68 12 0a 08 72 12 06 a2 c7 a7 c9 92 33 12 06 08 46 12 02 90 03 1a 00 5a 07 12 05 08 02 10 e3 01 28 80 85 84 93 d6 08";
    let packet1_payload = parse_hex_to_bytes_simple(packet1_hex)?;

    let packet2_hex = "00 00 83 c7 28 b5 2f fd 00 58 bc 16 00 a6 26 85 47 d0 2e 27 1d a8 70 f0 00 38 2f 4d 19 bc 08 be 68 b9 f5 fb fb 59 12 72 c7 ac c3 2a 38 a6 92 c0 7a 29 99 73 0f e0 93 c8 a1 3b 08 79 5a 97 51 5e b0 1e 1a 5a 0a 9f 00 2f b8 a3 c1 bd f5 c4 22 d1 b8 5b ee 26 8d ec 2d 03 74 00 75 00 6f 00 b2 47 9e 66 53 e3 f9 98 4d 9e 66 c1 26 13 e3 79 1a 8c bb 16 cf 77 1f e4 69 2c 9e 77 cf 1d 45 e3 f2 b2 62 97 0c 75 d5 50 8e 0d ac dd 2e 42 05 73 74 75 46 2e 2b 16 62 8b b6 5d 99 e2 24 d5 20 ae bc 40 e5 cd 80 b8 55 ee c6 dd ed f6 bb eb 42 83 44 d6 b9 11 a4 00 ce 87 c8 6a 77 51 70 3e 44 56 61 18 be 88 13 68 a4 40 63 8b 60 81 87 83 dd 5d b9 38 bb 1b 5e 8e e7 d0 4b 82 3d 11 1a 25 92 ec d1 58 5d 4a f6 94 9a 92 98 3c 69 6b 43 56 bf 25 4f 1a 2b 81 9b 3d 2a a1 0e c1 f2 a6 93 e3 79 01 41 f2 23 38 18 6e 46 6c 4c 40 79 3e e3 4a 5e e4 a9 89 c9 f3 30 14 f2 22 4b b9 29 cf c7 19 f9 9a d4 ed 84 e7 31 52 c8 d7 a2 3c 0f 4e c9 d7 90 8e 3c 7f 9c 7c cd 08 e6 c5 f3 01 40 c8 33 70 61 b1 c2 f3 ad 16 f2 34 15 9e 0f 2f 31 a8 e2 ef 2a 06 65 b9 3a fd 4f 9e da b9 be 33 aa 2e ff 2e c7 68 9c 7d 42 95 e5 7a 96 61 34 76 c4 30 0c 67 b2 5b b5 7f 3b 96 e1 ef 19 06 69 0c 67 28 fe 8e 62 90 76 31 8f 39 77 33 cf 79 52 6f 23 56 d9 cd 93 b2 3a b0 20 f5 78 ee 04 7d 23 34 3c bd 56 8c 48 d8 e3 f0 e3 09 f4 6a 09 a1 e3 59 e4 94 21 b4 47 5b 42 1e ec 51 8f e7 3b 72 8e a2 71 79 59 99 45 aa 1a ed 3d e6 aa b4 f7 57 48 02 9b 3b 92 18 d6 40 e2 81 0f 8f 19 a6 9d 2b a9 0e 89 ab 9a c5 0d 89 ab aa 91 ca 1f 37 a0 e0 41 58 bd a2 28 4f ea 02 80 e1 ef 18 43 d5 73 bd 67 d8 89 bf 9f 18 a4 fe 2e 33 48 73 6a 74 7d 34 2c c5 df 53 0c 52 13 7f 37 31 48 63 c7 4f 6d f9 6f 39 46 81 aa 74 bd 34 8c fa 3b 2d f1 17 7d a8 61 68 a7 0c 05 33 63 4a 52 58 e9 30 03 19 da 66 0c 11 40 18 4e 9a 81 88 85 55 86 fd 0d 65 45 34 8a 21 17 e3 12 c8 1f b8 4d ad ec d6 74 60 b6 6d 66 61 80 e4 44 14 08 83 e8 2c e4 b9 f1 15 f8 f8 08 b1 69 f1 05 47 75 c8 e2 3b 5e 29 5f 9f 41 dd 07 13 05 bc 6d c1 97 e3 9e f0 71 52 a2 e4 ab 18 53 64 2d 10 21 c3 18 13 3c 8d 57 46 3a 43 fd a2 09 32 ea 8a b1 f0 e6 82 74 11 8c 60 45 25 a6 73 7d 90 48 53 92 1a 97 b2 66 ea 46 7e 5c 61 7b c3 49 e6 66 ae 56 f4 25 26 61 24 f0 2f e7 40 4b 22 08 00 69 94 68 01 f7 dc 2a 53 5a 61 56 34 2b 73 8f ba 7d c6 fa ae 3c be a1 80 e5 cf 9c 1f 01 00 00";
    let packet2_payload = parse_hex_to_bytes_simple(packet2_hex)?;

    log::info!("📤 [真实发送] Packet 1 (0x0600): {} bytes", packet1_payload.len());
    log::info!("📤 [真实发送] Packet 2 (0x0680): {} bytes", packet2_payload.len());

    // Use the real packet sending function
    crate::forge::send_forged_packets().await?;

    log::info!("✅ Successfully sent both forged packets with 100ms interval");

    Ok(())
}

/// Simple hex parser for demonstration
fn parse_hex_to_bytes_simple(hex_str: &str) -> Result<Vec<u8>> {
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
