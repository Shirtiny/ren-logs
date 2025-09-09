use std::path::Path;
use std::fs;

fn main() {
    // Copy config.json to the build directory for development
    let source_config = "../meter-core/config.json";
    let target_config = "config.json";

    if Path::new(source_config).exists() {
        if let Err(e) = fs::copy(source_config, target_config) {
            println!("Warning: Failed to copy config.json: {}", e);
        } else {
            println!("Copied config.json to build directory");
        }
    } else {
        println!("Warning: Source config.json not found at {}", source_config);
    }

    tauri_build::build()
}
