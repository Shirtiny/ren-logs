use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// Configuration mode
#[derive(Debug, Clone)]
pub enum ConfigMode {
    Standalone,
    Tauri,
}

// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub packet_capture: PacketCaptureConfig,
    pub web_server: WebServerConfig,
    pub data_manager: DataManagerConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketCaptureConfig {
    pub filter: String,
    pub buffer_size: usize,
    pub mtu: usize,
    pub enable_tcp_reassembly: bool,
    pub max_connections: usize,
    pub connection_timeout: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub enable_websocket: bool,
    pub static_files_path: Option<String>,
    pub request_timeout: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataManagerConfig {
    pub cache_file_path: String,
    pub settings_file_path: String,
    pub skill_config_path: Option<String>,
    pub auto_save_interval: u64, // seconds
    pub max_cache_age: u64, // days
    pub enable_persistence: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub enable_file_logging: bool,
    pub log_file_path: Option<String>,
    pub max_log_files: usize,
    pub max_log_size: u64, // MB
    pub enable_console_logging: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            packet_capture: PacketCaptureConfig::default(),
            web_server: WebServerConfig::default(),
            data_manager: DataManagerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for PacketCaptureConfig {
    fn default() -> Self {
        Self {
            filter: "ip and tcp".to_string(),
            buffer_size: 10 * 1024 * 1024, // 10MB
            mtu: 1500,
            enable_tcp_reassembly: true,
            max_connections: 10000,
            connection_timeout: 300, // 5 minutes
        }
    }
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8989,
            enable_cors: true,
            enable_websocket: true,
            static_files_path: Some("public".to_string()),
            request_timeout: 30,
        }
    }
}

impl Default for DataManagerConfig {
    fn default() -> Self {
        Self {
            cache_file_path: "users.json".to_string(),
            settings_file_path: "settings.json".to_string(),
            skill_config_path: Some("tables/skill_names.json".to_string()),
            auto_save_interval: 300, // 5 minutes
            max_cache_age: 30, // 30 days
            enable_persistence: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            enable_file_logging: true,
            log_file_path: Some("logs/meter-core.log".to_string()),
            max_log_files: 5,
            max_log_size: 10, // 10MB
            enable_console_logging: true,
        }
    }
}

impl AppConfig {
    /// Load configuration for standalone application
    pub fn load_for_standalone() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::load_with_mode(ConfigMode::Standalone)
    }

    /// Load configuration for Tauri application
    pub fn load_for_tauri() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::load_with_mode(ConfigMode::Tauri)
    }

    /// Internal method to load configuration based on mode
    fn load_with_mode(mode: ConfigMode) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_paths = match mode {
            ConfigMode::Standalone => vec![
                PathBuf::from("config.json"),
                std::env::current_exe()?
                    .parent()
                    .unwrap_or(&PathBuf::from("."))
                    .join("config.json"),
            ],
            ConfigMode::Tauri => vec![
                std::env::current_exe()?
                    .parent()
                    .unwrap_or(&PathBuf::from("."))
                    .join("config.json"),
                PathBuf::from("../meter-core/config.json"),
            ],
        };

        // Try to load from each path
        for path in config_paths {
            if let Ok(config) = Self::load_from_file(&path) {
                let mut config = config;

                // Load environment variables
                config.load_from_env()?;

                // Validate configuration
                if let Err(errors) = config.validate_with_paths(&mode) {
                    log::warn!("Configuration validation failed: {:?}", errors);
                    // Continue to try next path instead of failing
                    continue;
                }

                log::info!("Loaded configuration from {:?} for {:?}", path, mode);
                return Ok(config);
            }
        }

        // If no config file found, use defaults
        let mut config = Self::default();
        config.load_from_env()?;
        if let Err(errors) = config.validate_with_paths(&mode) {
            log::warn!("Default configuration validation failed: {:?}", errors);
            // For defaults, we'll be more lenient and just log warnings
        }

        log::warn!("No configuration file found, using defaults for {:?}", mode);
        Ok(config)
    }

    /// Load configuration from a specific file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !path.as_ref().exists() {
            return Err(format!("Config file not found: {:?}", path.as_ref()).into());
        }

        let content = fs::read_to_string(&path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = serde_json::to_string_pretty(self)?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content)?;
        log::info!("Saved configuration to {:?}", path.as_ref());
        Ok(())
    }
}

// Command line arguments structure
#[derive(Debug)]
pub struct AppArgs {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub log_level: Option<String>,
    pub config_file: Option<String>,
    pub interface: Option<String>,
    pub verbose: bool,
    pub daemon: bool,
}

impl AppArgs {
    pub fn parse() -> Self {
        // Simple argument parsing - in practice, you'd use clap or similar
        let args: Vec<String> = std::env::args().collect();

        let mut host = None;
        let mut port = None;
        let mut log_level = None;
        let mut config_file = None;
        let mut interface = None;
        let mut verbose = false;
        let mut daemon = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--host" | "-h" => {
                    if i + 1 < args.len() {
                        host = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--port" | "-p" => {
                    if i + 1 < args.len() {
                        if let Ok(p) = args[i + 1].parse::<u16>() {
                            port = Some(p);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--log-level" | "-l" => {
                    if i + 1 < args.len() {
                        log_level = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--config" | "-c" => {
                    if i + 1 < args.len() {
                        config_file = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--interface" | "-i" => {
                    if i + 1 < args.len() {
                        interface = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--verbose" | "-v" => {
                    verbose = true;
                    i += 1;
                }
                "--daemon" | "-d" => {
                    daemon = true;
                    i += 1;
                }
                "--help" => {
                    Self::print_help();
                    std::process::exit(0);
                }
                _ => {
                    i += 1;
                }
            }
        }

        Self {
            host,
            port,
            log_level,
            config_file,
            interface,
            verbose,
            daemon,
        }
    }

    fn print_help() {
        println!("Meter Core - Star Resonance Damage Counter");
        println!();
        println!("USAGE:");
        println!("    meter-core [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --host <HOST>              Web server host (default: 127.0.0.1)");
        println!("    -p, --port <PORT>              Web server port (default: 8989)");
        println!("    -l, --log-level <LEVEL>        Log level (trace, debug, info, warn, error)");
        println!("    -c, --config <FILE>            Configuration file path (default: config.json)");
        println!("    -i, --interface <INTERFACE>    Network interface for packet capture");
        println!("    -v, --verbose                  Enable verbose logging");
        println!("    -d, --daemon                   Run as daemon");
        println!("        --help                     Print this help message");
        println!();
        println!("CONFIGURATION:");
        println!("    Create a config.json file to customize settings. Copy from config.example.json");
        println!("    Log level can be set in config file under 'logging.level'");
        println!("    Priority: Command line > Config file > Environment variables > Defaults");
        println!();
        println!("EXAMPLES:");
        println!("    meter-core --port 8080 --log-level debug");
        println!("    meter-core --config my-config.json");
        println!("    cp config.example.json config.json && meter-core");
    }
}

// Configuration validation
impl AppConfig {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate web server config
        if self.web_server.port == 0 {
            errors.push("Web server port cannot be 0".to_string());
        }

        // Validate packet capture config
        if self.packet_capture.buffer_size == 0 {
            errors.push("Packet capture buffer size cannot be 0".to_string());
        }

        // Validate logging config
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            errors.push(format!("Invalid log level: {}. Valid levels are: {}", self.logging.level, valid_levels.join(", ")));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// Environment variable loading
impl AppConfig {
    pub fn load_from_env(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Load configuration from environment variables
        if let Ok(host) = std::env::var("METER_CORE_HOST") {
            self.web_server.host = host;
        }

        if let Ok(port) = std::env::var("METER_CORE_PORT") {
            if let Ok(port) = port.parse::<u16>() {
                self.web_server.port = port;
            }
        }

        if let Ok(log_level) = std::env::var("METER_CORE_LOG_LEVEL") {
            self.logging.level = log_level;
        }

        if let Ok(interface) = std::env::var("METER_CORE_INTERFACE") {
            self.packet_capture.filter = format!("ip and tcp and {}", interface);
        }

        Ok(())
    }
}

// Enhanced validation with path checking
impl AppConfig {
    pub fn validate_with_paths(&self, mode: &ConfigMode) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Basic validation
        if let Err(basic_errors) = self.validate() {
            errors.extend(basic_errors);
        }

        // Path validation based on mode
        match mode {
            ConfigMode::Standalone => {
                // For standalone mode, validate relative paths
                if let Some(log_path) = &self.logging.log_file_path {
                    let log_dir = Path::new(log_path).parent();
                    if let Some(dir) = log_dir {
                        if !dir.exists() {
                            errors.push(format!("Log directory does not exist: {:?}", dir));
                        }
                    }
                }

                if let Some(static_path) = &self.web_server.static_files_path {
                    if !Path::new(static_path).exists() {
                        errors.push(format!("Static files directory does not exist: {}", static_path));
                    }
                }

                if let Some(skill_path) = &self.data_manager.skill_config_path {
                    if !Path::new(skill_path).exists() {
                        errors.push(format!("Skill config file does not exist: {}", skill_path));
                    }
                }
            }
            ConfigMode::Tauri => {
                // For Tauri mode, paths are relative to executable directory
                // Less strict validation as paths might be resolved at runtime
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// Configuration file watching (for hot reload)
pub struct ConfigWatcher {
    config_path: String,
    last_modified: std::time::SystemTime,
}

impl ConfigWatcher {
    pub fn new(config_path: String) -> Self {
        Self {
            config_path,
            last_modified: std::time::SystemTime::UNIX_EPOCH,
        }
    }

    pub fn check_for_changes(&mut self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let metadata = fs::metadata(&self.config_path)?;
        let modified = metadata.modified()?;

        if modified > self.last_modified {
            self.last_modified = modified;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// Default configuration file content
pub fn create_default_config_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = AppConfig::default();
    config.save_to_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.web_server.port, 8989);
        assert_eq!(config.web_server.host, "127.0.0.1");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();

        // Test valid config
        assert!(config.validate().is_ok());

        // Test invalid port
        config.web_server.port = 0;
        assert!(config.validate().is_err());

        // Reset for next test
        config.web_server.port = 8989;

        // Test invalid log level
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_modes() {
        // Test that we can create configs for different modes
        let standalone_config = AppConfig::default();
        let tauri_config = AppConfig::default();

        // Both should have same default values
        assert_eq!(standalone_config.web_server.port, tauri_config.web_server.port);
        assert_eq!(standalone_config.logging.level, tauri_config.logging.level);
    }
}
