pub mod models;
pub mod data_manager;
pub mod packet_parser;
pub mod packet_capture;
pub mod web_server;
pub mod config;

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::collections::HashMap;
use chrono::Utc;
use log::{info, warn, error};
use tokio::task::JoinHandle;

// Error types
#[derive(Debug)]
pub enum MeterError {
    Io(std::io::Error),
    Parse(String),
    ParseError(String),
    Config(String),
    Network(String),
    WebServer(String),
    PacketCapture(String),
    WinDivertError(String),
    Json(serde_json::Error),
    Other(String),
}

impl std::fmt::Display for MeterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeterError::Io(e) => write!(f, "IO error: {}", e),
            MeterError::Parse(s) => write!(f, "Parse error: {}", s),
            MeterError::ParseError(s) => write!(f, "Parse error: {}", s),
            MeterError::Config(s) => write!(f, "Config error: {}", s),
            MeterError::Network(s) => write!(f, "Network error: {}", s),
            MeterError::WebServer(s) => write!(f, "Web server error: {}", s),
            MeterError::PacketCapture(s) => write!(f, "Packet capture error: {}", s),
            MeterError::WinDivertError(s) => write!(f, "WinDivert error: {}", s),
            MeterError::Json(e) => write!(f, "JSON error: {}", e),
            MeterError::Other(s) => write!(f, "Other error: {}", s),
        }
    }
}

impl std::error::Error for MeterError {}

impl From<std::io::Error> for MeterError {
    fn from(e: std::io::Error) -> Self {
        MeterError::Io(e)
    }
}

impl From<serde_json::Error> for MeterError {
    fn from(e: serde_json::Error) -> Self {
        MeterError::Json(e)
    }
}

impl From<&str> for MeterError {
    fn from(s: &str) -> Self {
        MeterError::Other(s.to_string())
    }
}

impl From<String> for MeterError {
    fn from(s: String) -> Self {
        MeterError::Other(s)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for MeterError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        MeterError::Other(e.to_string())
    }
}

// Result type alias
pub type Result<T> = std::result::Result<T, MeterError>;

// Utility functions
pub mod utils {
    use std::path::Path;

    pub fn is_windivert_installed() -> bool {
        // Check for WinDivert DLLs in the current directory
        let current_dir = std::env::current_dir().unwrap_or_default();
        let dll_path = current_dir.join("WinDivert.dll");

        if dll_path.exists() {
            log::debug!("Found WinDivert.dll at: {:?}", dll_path);
            return true;
        }

        // Check in system directories
        if let Ok(system32) = std::env::var("SystemRoot") {
            let system32_path = Path::new(&system32).join("System32").join("WinDivert.dll");
            if system32_path.exists() {
                log::debug!("Found WinDivert.dll at: {:?}", system32_path);
                return true;
            }
        }

        log::warn!("WinDivert.dll not found");
        false
    }

    pub fn is_admin() -> bool {
        // Simple check for administrator privileges
        // In a real implementation, this would use Windows API to check privileges
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("net")
                .args(&["session"])
                .output()
            {
                return output.status.success();
            }
        }

        // For non-Windows or fallback, assume admin
        #[cfg(not(target_os = "windows"))]
        {
            return true;
        }

        #[cfg(target_os = "windows")]
        {
            // Fallback for Windows
            return true;
        }
    }
}

use data_manager::DataManager;
use packet_capture::PacketCapture;
use web_server::WebServer;
use config::{AppConfig, AppArgs};

pub struct MeterCore {
    data_manager: Arc<DataManager>,
    packet_capture: Option<PacketCapture>,
    web_server: Option<WebServer>,
    tasks: Vec<JoinHandle<()>>,
    config: AppConfig,
}

impl MeterCore {
    /// Create a new MeterCore instance for standalone use
    pub async fn new() -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_config_mode(false).await
    }

    /// Create a new MeterCore instance with Tauri configuration
    pub async fn new_with_config() -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_config_mode(true).await
    }

    /// Internal method to create MeterCore with configuration mode
    async fn new_with_config_mode(use_tauri_config: bool) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Parse command line arguments
        let args = AppArgs::parse();

        // Load configuration based on mode (needed for logging setup)
        let config = if use_tauri_config {
            AppConfig::load_for_tauri().unwrap_or_else(|e| {
                eprintln!("Failed to load Tauri configuration: {}, using defaults", e);
                AppConfig::default()
            })
        } else {
            AppConfig::load_for_standalone().unwrap_or_else(|e| {
                eprintln!("Failed to load standalone configuration: {}, using defaults", e);
                AppConfig::default()
            })
        };

        // Initialize logging (only if not already initialized)
        let log_level = args.log_level.as_deref()
            .unwrap_or(&config.logging.level);
        if let Err(_) = env_logger::try_init_from_env(env_logger::Env::default().default_filter_or(log_level)) {
            // Logger already initialized, skip
        }

        info!("Starting Meter Core - Star Resonance Damage Counter");

        // Validate configuration
        if let Err(errors) = config.validate() {
            error!("Configuration validation failed:");
            for error in errors {
                error!("  - {}", error);
            }
            return Err("Configuration validation failed".into());
        }

        info!("Configuration loaded successfully");

        // Initialize data manager
        let data_manager = Arc::new(DataManager::new());
        data_manager.initialize().await?;

        info!("Data manager initialized");

        Ok(MeterCore {
            data_manager,
            packet_capture: None,
            web_server: None,
            tasks: Vec::new(),
            config,
        })
    }

    pub async fn start(&mut self) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Initialize packet capture
        let packet_capture = PacketCapture::new(self.data_manager.clone());
        self.packet_capture = Some(packet_capture);

        // Initialize web server
        let web_server = WebServer::new(self.data_manager.clone());
        self.web_server = Some(web_server);

        // Start background tasks
        let data_manager_clone = self.data_manager.clone();
        let update_task = tokio::spawn(async move {
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
        self.tasks.push(update_task);

        // Start auto-save task
        let data_manager_clone = self.data_manager.clone();
        let save_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                if let Err(e) = data_manager_clone.save_user_cache().await {
                    error!("Failed to auto-save user cache: {}", e);
                }
            }
        });
        self.tasks.push(save_task);

        // Start packet capture
        if let Some(mut packet_capture) = self.packet_capture.take() {
            let capture_task = tokio::spawn(async move {
                if let Err(e) = packet_capture.start_capture().await {
                    error!("Packet capture failed: {}", e);
                }
            });
            self.tasks.push(capture_task);
        }

        // Start web server
        if let Some(mut web_server) = self.web_server.take() {
            let server_task = tokio::spawn(async move {
                if let Err(e) = web_server.start().await {
                    error!("Web server failed: {}", e);
                }
            });
            self.tasks.push(server_task);
        }

        info!("Meter Core started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping Meter Core...");

        // Stop all tasks
        for task in &self.tasks {
            task.abort();
        }
        self.tasks.clear();

        // Stop packet capture (this will handle WinDivert cleanup)
        if let Some(ref mut packet_capture) = self.packet_capture {
            // Note: PacketCapture should implement a stop method
            // For now, we'll log the intent
            warn!("Packet capture stop not implemented yet - WinDivert cleanup needed");
        }

        // Save final data
        if let Err(e) = self.data_manager.save_user_cache().await {
            error!("Failed to save user cache on shutdown: {}", e);
        }

        if let Err(e) = self.data_manager.save_settings().await {
            error!("Failed to save settings on shutdown: {}", e);
        }

        info!("Meter Core stopped successfully");
        Ok(())
    }

    pub fn get_data_manager(&self) -> Arc<DataManager> {
        self.data_manager.clone()
    }

    pub fn is_running(&self) -> bool {
        !self.tasks.is_empty()
    }
}

// Re-export for convenience
pub use models::*;
