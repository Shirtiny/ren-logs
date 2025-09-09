//! # Meter Core
//!
//! A custom implementation of the meter-core crate for LOA Logs,
//! providing network packet capture and parsing functionality using WinDivert.

pub mod capture;
pub mod decryption;
pub mod forge;
pub mod packets;

// Re-export main interfaces
pub use capture::{start_capture, reset_server_identification, send_forged_packets_simple};
pub use decryption::DamageEncryptionHandler;
pub use packets::{definitions, opcodes, structures};

// Error handling
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MeterError {
    #[error("WinDivert error: {0}")]
    WinDivertError(String),

    #[error("Packet parsing error: {0}")]
    ParseError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Generic error: {0}")]
    GenericError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, MeterError>;

// Common utilities
pub mod utils {
    use std::path::PathBuf;

    /// Get the default WinDivert DLL path
    pub fn get_windivert_dll_path() -> PathBuf {
        // Try to find WinDivert.dll in common locations
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));

        // Check current directory first
        let dll_path = exe_dir.join("WinDivert.dll");
        if dll_path.exists() {
            return dll_path;
        }

        // Check system32
        if let Ok(system32) = std::env::var("SystemRoot") {
            let sys_dll = PathBuf::from(system32).join("System32").join("WinDivert.dll");
            if sys_dll.exists() {
                return sys_dll;
            }
        }

        // Return current directory as fallback
        dll_path
    }

    /// Check if WinDivert driver is installed
    pub fn is_windivert_installed() -> bool {
        // Check for driver files
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));

        exe_dir.join("WinDivert64.sys").exists() || exe_dir.join("WinDivert32.sys").exists()
    }

    /// Check if the current process has administrator privileges
    pub fn is_admin() -> bool {
        #[cfg(windows)]
        {
            use std::mem;
            use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
            use winapi::um::securitybaseapi::GetTokenInformation;
            use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};

            unsafe {
                let mut token = mem::zeroed();
                let mut elevation: TOKEN_ELEVATION = mem::zeroed();
                let mut size = mem::size_of::<TOKEN_ELEVATION>() as u32;

                if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0 {
                    if GetTokenInformation(token, TokenElevation, &mut elevation as *mut _ as *mut _, size, &mut size) != 0 {
                        return elevation.TokenIsElevated != 0;
                    }
                }
            }
            false
        }

        #[cfg(not(windows))]
        {
            // On non-Windows platforms, assume admin privileges
            true
        }
    }
}
