use std::sync::Arc;
use chrono::Utc;

use meter_core::{
    data_manager::DataManager,
    packet_capture::PacketCapture,
    web_server::WebServer,
    config::{AppConfig, AppArgs},
    models::*,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse command line arguments
    let args = AppArgs::parse();

    // Load configuration using the new simplified approach
    let config = AppConfig::load_for_standalone().unwrap_or_else(|e| {
        println!("Failed to load configuration: {}, using defaults", e);
        AppConfig::default()
    });

    // Initialize logging - use config file level if command line not specified
    let log_level = args.log_level.as_deref()
        .or_else(|| Some(&config.logging.level))
        .unwrap_or("info");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    log::info!("Starting Meter Core - Star Resonance Damage Counter");

    // Validate configuration
    if let Err(errors) = config.validate() {
        log::error!("Configuration validation failed:");
        for error in errors {
            log::error!("  - {}", error);
        }
        std::process::exit(1);
    }

    log::info!("Configuration loaded successfully");

    // Initialize data manager
    let data_manager = Arc::new(DataManager::new());
    data_manager.initialize().await?;

    log::info!("Data manager initialized");

    // Initialize packet capture
    let packet_capture = PacketCapture::new(data_manager.clone());

    // Initialize web server
    let web_server = WebServer::new(data_manager.clone());

    // Start background tasks
    let data_manager_clone = data_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            if !data_manager_clone.is_paused() {
                data_manager_clone.update_dps();
                data_manager_clone.update_hps();
            }
            data_manager_clone.check_timeout_clear();
        }
    });

    // Start auto-save task
    let data_manager_clone = data_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = data_manager_clone.save_user_cache().await {
                log::error!("Failed to auto-save user cache: {}", e);
            }
        }
    });

    // Start packet capture in a separate task
    let mut packet_capture_handle = packet_capture;
    let capture_task = tokio::spawn(async move {
        if let Err(e) = packet_capture_handle.start_capture().await {
            log::error!("Packet capture failed: {}", e);
        }
    });

    // Start web server
    let mut web_server_handle = web_server;
    let server_task = tokio::spawn(async move {
        if let Err(e) = web_server_handle.start().await {
            log::error!("Web server failed: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received shutdown signal");
        }
        _ = capture_task => {
            log::info!("Packet capture task finished");
        }
        _ = server_task => {
            log::info!("Web server task finished");
        }
    }

    // Graceful shutdown
    log::info!("Shutting down gracefully...");

    // Save final data
    if let Err(e) = data_manager.save_user_cache().await {
        log::error!("Failed to save user cache on shutdown: {}", e);
    }

    if let Err(e) = data_manager.save_settings().await {
        log::error!("Failed to save settings on shutdown: {}", e);
    }

    log::info!("Shutdown complete");
    Ok(())
}

// Re-export error types from lib crate
pub use meter_core::MeterError;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

// Build information
pub fn print_version_info() {
    println!("{} v{}", NAME, VERSION);
    println!("{}", DESCRIPTION);
    println!("Built with Rust {}", std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()));
    println!("Build time: {}", std::env::var("VERGEN_BUILD_TIMESTAMP").unwrap_or_else(|_| "unknown".to_string()));
    println!("Git commit: {}", std::env::var("VERGEN_GIT_SHA").unwrap_or_else(|_| "unknown".to_string()));
}

// System information
pub fn print_system_info() {
    println!("System Information:");
    println!("  OS: {}", std::env::consts::OS);
    println!("  Architecture: {}", std::env::consts::ARCH);
    println!("  CPU cores: {}", num_cpus::get());
    println!("  Memory: {} MB", sys_info::mem_info().unwrap().total / 1024);
}

// Health check
pub async fn health_check(data_manager: &DataManager) -> serde_json::Value {
    use serde_json::json;

    let uptime = Utc::now().signed_duration_since(data_manager.start_time).num_seconds();

    json!({
        "status": "healthy",
        "version": VERSION,
        "uptime_seconds": uptime,
        "users_count": data_manager.users.len(),
        "enemies_count": data_manager.enemies.len(),
        "is_paused": data_manager.is_paused(),
        "timestamp": Utc::now().to_rfc3339()
    })
}

// Test utilities (only compiled in test builds)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let data_manager = Arc::new(DataManager::new());

        // Test adding damage
        data_manager.add_damage(
            12345,
            1001,
            "物理".to_string(),
            1000,
            true,
            false,
            false,
            0,
            67890,
        ).await;

        // Verify damage was recorded
        let user_data = data_manager.get_all_users_data();
        assert!(user_data.contains_key(&12345));
    }

    #[tokio::test]
    async fn test_user_creation() {
        let data_manager = DataManager::new();
        let user = data_manager.get_or_create_user(99999);

        assert_eq!(user.read().uid, 99999);
    }

    #[test]
    fn test_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.web_server.port, 8989);
        assert_eq!(config.web_server.host, "127.0.0.1");
    }
}
