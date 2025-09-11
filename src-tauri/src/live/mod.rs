use crate::app;

use anyhow::Result;

use log::{error, info, warn};
use meter_core::MeterCore;

use std::sync::Arc;

use std::time::Duration;
use tauri::AppHandle;

static METER_CORE_INSTANCE: std::sync::OnceLock<Arc<tokio::sync::Mutex<Option<MeterCore>>>> =
    std::sync::OnceLock::new();

pub async fn start_with_retry(_app: AppHandle, max_retries: u32) -> Result<()> {
    let instance = METER_CORE_INSTANCE.get_or_init(|| Arc::new(tokio::sync::Mutex::new(None)));

    for attempt in 1..=max_retries {
        info!(
            "Attempting to start Meter Core (attempt {}/{})",
            attempt, max_retries
        );

        // 使用 Tauri 模式的配置加载
        match MeterCore::new_with_config().await {
            Ok(mut meter_core) => match meter_core.start().await {
                Ok(_) => {
                    *instance.lock().await = Some(meter_core);
                    info!("Meter Core started successfully");
                    return Ok(());
                }
                Err(e) => {
                    error!("Failed to start Meter Core (attempt {}): {}", attempt, e);
                    if attempt == max_retries {
                        return Err(anyhow::anyhow!(
                            "Failed to start Meter Core after {} attempts",
                            max_retries
                        ));
                    }
                }
            },
            Err(e) => {
                error!("Failed to create Meter Core (attempt {}): {}", attempt, e);
                if attempt == max_retries {
                    return Err(anyhow::anyhow!(
                        "Failed to create Meter Core after {} attempts",
                        max_retries
                    ));
                }
            }
        }

        // Wait 5 seconds before retry
        if attempt < max_retries {
            warn!("Retrying in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    Ok(())
}

pub async fn stop() -> Result<()> {
    let instance = METER_CORE_INSTANCE.get_or_init(|| Arc::new(tokio::sync::Mutex::new(None)));

    if let Some(mut meter_core) = instance.lock().await.take() {
        if let Err(e) = meter_core.stop().await {
            error!("Error stopping meter core: {}", e);
            return Err(anyhow::anyhow!("Error stopping meter core: {}", e));
        }
        info!("Meter Core stopped successfully");

        #[cfg(target_os = "windows")]
        {
            info!("Performing WinDivert cleanup...");
            stop_driver();
            remove_driver();
            warn!("WinDivert cleanup logged");
        }
    } else {
        warn!("Meter Core instance not found, nothing to stop");
    }

    Ok(())
}

pub async fn start_async(app: AppHandle) -> Result<()> {
    info!("Starting Meter Core asynchronously...");
    start_with_retry(app, 3).await
}

pub async fn start_sync(app: AppHandle) -> Result<()> {
    info!("Starting Meter Core synchronously...");

    // Only start if not already running
    let instance = METER_CORE_INSTANCE.get_or_init(|| Arc::new(tokio::sync::Mutex::new(None)));

    if instance.lock().await.is_some() {
        info!("Meter Core already running, skipping...");
        return Ok(());
    }

    start_with_retry(app, 3).await
}

pub fn start(app: AppHandle) -> Result<()> {
    // Keep the original synchronous interface for backward compatibility
    // The actual async work will be done by the caller
    info!("Meter Core start requested - use start_sync for actual startup");
    Ok(())
}

fn remove_driver() {
    #[cfg(target_os = "windows")]
    {
        use app::compat::Command;
        let status = Command::new("sc").args(["delete", "windivert"]).status();

        status.expect("unable to delete driver");
    }
}

fn stop_driver() {
    #[cfg(target_os = "windows")]
    {
        use app::compat::Command;
        let status = Command::new("sc").args(["stop", "windivert"]).status();

        if status.is_ok_and(|status| status.success()) {
            info!("stopped driver");
        } else {
            warn!("could not execute command to stop driver");
        }
    }
}
