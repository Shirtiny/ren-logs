use crate::app;

use anyhow::Result;
use chrono::Utc;
use hashbrown::HashMap;
use log::{info, warn};
use meter_core::packets::definitions::*;
use meter_core::packets::opcodes::Pkt;
use meter_core::start_capture;
use reqwest::Client;
use serde_json::json;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Listener, Manager, Window, Wry};
use uuid::Uuid;

pub fn start(app: AppHandle) -> Result<()> {
    let region_file_path = app::path::data_dir(&app).join("current_region");

    let rx = match start_capture(region_file_path.display().to_string()) {
        Ok(rx) => rx,
        Err(e) => {
            warn!("Error starting capture: {e}");
            return Ok(());
        }
    };

    while let Ok((op, data)) = rx.recv_blocking() {
        match op {
            _ => {}
        }
    }

    Ok(())
}

fn debug_print(args: std::fmt::Arguments<'_>) {
    #[cfg(debug_assertions)]
    {
        info!("{}", args);
    }
}
