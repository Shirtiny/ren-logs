use crate::data_manager::DataManager;
use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tokio::sync::broadcast;

// Web server configuration
pub struct WebServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8989,
            enable_cors: true,
        }
    }
}

pub struct WebServer {
    config: WebServerConfig,
    data_manager: Arc<DataManager>,
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl WebServer {
    pub fn new(data_manager: Arc<DataManager>) -> Self {
        Self {
            config: WebServerConfig::default(),
            data_manager,
            shutdown_tx: None,
        }
    }

    pub fn with_config(mut self, config: WebServerConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let app = self.create_router();

        let addr = format!("{}:{}", self.config.host, self.config.port);
        log::info!("Starting web server at http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        log::info!("Web server listening on {}", addr);

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.recv().await;
                log::info!("Web server shutting down gracefully");
            })
            .await?;

        Ok(())
    }

    pub fn shutdown(&self) {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }
    }

    fn create_router(&self) -> Router {
        let cors_layer = if self.config.enable_cors {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            CorsLayer::new()
        };

        let data_manager = self.data_manager.clone();
        let data_manager_ws = self.data_manager.clone();
        let data_manager_static = self.data_manager.clone();

        Router::new()
            .route("/api/data", get(get_user_data))
            .route("/api/enemies", get(get_enemy_data))
            .route("/api/clear", get(clear_data))
            .route("/api/pause", get(get_pause_status).post(set_pause_status))
            .route("/api/skill/:uid", get(get_user_skill_data))
            .route("/api/settings", get(get_settings).post(update_settings))
            .route("/api/health", get(health_check))
            .route("/api/history/list", get(list_history_snapshots))
            .route("/api/history/:timestamp", get(get_history_snapshot))
            .route("/ws", get(ws_handler))
            .route("/files/*path", get(serve_static_file))
            .layer(cors_layer)
            .with_state(data_manager)
    }
}

// API handlers
async fn get_user_data(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
) -> Json<Value> {
    let user_data = data_manager.get_all_users_data();
    Json(json!({
        "code": 0,
        "user": user_data
    }))
}

async fn get_enemy_data(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
) -> Json<Value> {
    let enemy_data = data_manager.get_all_enemies_data();
    Json(json!({
        "code": 0,
        "enemy": enemy_data
    }))
}

async fn clear_data(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
) -> Json<Value> {
    data_manager.clear_all();
    log::info!("Statistics have been cleared via API");
    Json(json!({
        "code": 0,
        "msg": "Statistics have been cleared!"
    }))
}

async fn get_pause_status(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
) -> Json<Value> {
    let is_paused = data_manager.is_paused();
    Json(json!({
        "code": 0,
        "paused": is_paused
    }))
}

async fn set_pause_status(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
    axum::extract::Json(payload): axum::extract::Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if let Some(paused) = payload.get("paused").and_then(|v| v.as_bool()) {
        data_manager.pause(paused);
        log::info!("Statistics {} via API", if paused { "paused" } else { "resumed" });
        Ok(Json(json!({
            "code": 0,
            "msg": format!("Statistics {}!", if paused { "paused" } else { "resumed" }),
            "paused": paused
        })))
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

async fn get_user_skill_data(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
    Path(uid): Path<u32>,
) -> Result<Json<Value>, StatusCode> {
    // Get user data
    let user_data = data_manager.get_all_users_data();
    let user_info = user_data.get(&uid).ok_or(StatusCode::NOT_FOUND)?;

    // Get skill configuration for name mapping
    let skill_config = data_manager.skill_config.read();

    // Build skill statistics from user data
    let mut skill_stats = serde_json::Map::new();

    // Extract skill information from user data if available
    // This is a placeholder - in a real implementation, you would track skill usage
    // and return actual skill statistics with proper name mapping

    let response = json!({
        "code": 0,
        "data": {
            "uid": uid,
            "name": user_info.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown"),
            "profession": user_info.get("profession").and_then(|v| v.as_str()).unwrap_or("Unknown"),
            "skill_count": skill_stats.len(),
            "skills": skill_stats
        }
    });

    Ok(Json(response))
}

async fn get_settings(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
) -> Json<Value> {
    let settings = data_manager.settings.read().clone();
    Json(json!({
        "code": 0,
        "data": settings
    }))
}

async fn update_settings(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
    axum::extract::Json(payload): axum::extract::Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut settings = data_manager.settings.write();

    if let Some(auto_clear_server) = payload.get("auto_clear_on_server_change").and_then(|v| v.as_bool()) {
        settings.auto_clear_on_server_change = auto_clear_server;
    }
    if let Some(auto_clear_timeout) = payload.get("auto_clear_on_timeout").and_then(|v| v.as_bool()) {
        settings.auto_clear_on_timeout = auto_clear_timeout;
    }
    if let Some(only_elite) = payload.get("only_record_elite_dummy").and_then(|v| v.as_bool()) {
        settings.only_record_elite_dummy = only_elite;
    }

    // Save settings asynchronously
    let data_manager_clone = data_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = data_manager_clone.save_settings().await {
            log::error!("Failed to save settings: {}", e);
        }
    });

    Ok(Json(json!({
        "code": 0,
        "data": settings.clone()
    })))
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "code": 0,
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn list_history_snapshots(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
) -> Json<Value> {
    let history_manager = HistoryManager::new(data_manager);

    match history_manager.list_snapshots().await {
        Ok(snapshots) => Json(json!({
            "code": 0,
            "snapshots": snapshots,
            "count": snapshots.len()
        })),
        Err(e) => Json(json!({
            "code": 1,
            "error": format!("Failed to list snapshots: {}", e)
        }))
    }
}

async fn get_history_snapshot(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
    Path(timestamp): Path<i64>,
) -> Json<Value> {
    let history_manager = HistoryManager::new(data_manager);

    match history_manager.load_snapshot(timestamp).await {
        Ok(data) => Json(data),
        Err(e) => Json(json!({
            "code": 1,
            "error": format!("Failed to load snapshot {}: {}", timestamp, e)
        }))
    }
}

async fn ws_handler(
    axum::extract::State(data_manager): axum::extract::State<Arc<DataManager>>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    WebSocketHandler::handle_connection(data_manager, ws).await
}

async fn serve_static_file(
    Path(path): Path<String>,
) -> Result<Vec<u8>, StatusCode> {
    let static_server = StaticFileServer::new("public".to_string());
    static_server.serve_file(&path).await
}

// WebSocket support for real-time updates
pub struct WebSocketHandler {
    data_manager: Arc<DataManager>,
}

impl WebSocketHandler {
    pub fn new(data_manager: Arc<DataManager>) -> Self {
        Self { data_manager }
    }

    pub async fn handle_connection(
        data_manager: Arc<DataManager>,
        ws: axum::extract::ws::WebSocketUpgrade,
    ) -> axum::response::Response {
        ws.on_upgrade(move |socket| Self::handle_socket_static(data_manager, socket))
    }

    async fn handle_socket_static(data_manager: Arc<DataManager>, mut socket: axum::extract::ws::WebSocket) {
        log::info!("WebSocket client connected");

        // Send initial data
        let user_data = data_manager.get_all_users_data();
        let initial_msg = json!({
            "code": 0,
            "user": user_data
        });

        if let Ok(msg) = serde_json::to_string(&initial_msg) {
            if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                log::warn!("Failed to send initial WebSocket message");
                return;
            }
        }

        // Real-time updates loop
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !data_manager.is_paused() {
                        let user_data = data_manager.get_all_users_data();
                        let msg = json!({
                            "code": 0,
                            "user": user_data
                        });

                        if let Ok(msg_str) = serde_json::to_string(&msg) {
                            if socket.send(axum::extract::ws::Message::Text(msg_str)).await.is_err() {
                                log::warn!("Failed to send WebSocket update");
                                break;
                            }
                        }
                    }
                }
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(axum::extract::ws::Message::Close(_))) => {
                            log::info!("WebSocket client disconnected");
                            break;
                        }
                        Some(Ok(_)) => {
                            // Handle other messages if needed
                        }
                        Some(Err(e)) => {
                            log::error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            log::info!("WebSocket connection closed");
                            break;
                        }
                    }
                }
            }
        }
    }
}

// Static file serving (simplified)
pub struct StaticFileServer {
    web_root: String,
}

impl StaticFileServer {
    pub fn new(web_root: String) -> Self {
        Self { web_root }
    }

    pub async fn serve_file(&self, path: &str) -> Result<Vec<u8>, StatusCode> {
        let file_path = format!("{}/{}", self.web_root, path.trim_start_matches('/'));

        match tokio::fs::read(&file_path).await {
            Ok(content) => Ok(content),
            Err(_) => Err(StatusCode::NOT_FOUND),
        }
    }
}

// History data management
pub struct HistoryManager {
    data_manager: Arc<DataManager>,
    history_dir: String,
}

impl HistoryManager {
    pub fn new(data_manager: Arc<DataManager>) -> Self {
        Self {
            data_manager,
            history_dir: "logs".to_string(),
        }
    }

    pub fn with_history_dir(mut self, dir: String) -> Self {
        self.history_dir = dir;
        self
    }

    pub async fn save_snapshot(&self, timestamp: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;
        use tokio::fs as async_fs;

        // Create logs directory if it doesn't exist
        if !fs::metadata(&self.history_dir).is_ok() {
            fs::create_dir_all(&self.history_dir)?;
        }

        // Create timestamp directory
        let timestamp_dir = format!("{}/{}", self.history_dir, timestamp);
        if !fs::metadata(&timestamp_dir).is_ok() {
            fs::create_dir_all(&timestamp_dir)?;
        }

        // Save user data
        let users_file = format!("{}/users.json", timestamp_dir);
        let user_data = self.data_manager.get_all_users_data();
        let users_content = serde_json::to_string_pretty(&user_data)?;
        async_fs::write(&users_file, users_content).await?;

        // Save enemy data
        let enemies_file = format!("{}/enemies.json", timestamp_dir);
        let enemy_data = self.data_manager.get_all_enemies_data();
        let enemies_content = serde_json::to_string_pretty(&enemy_data)?;
        async_fs::write(&enemies_file, enemies_content).await?;

        // Save summary
        let summary_file = format!("{}/summary.json", timestamp_dir);
        let summary = json!({
            "timestamp": timestamp,
            "user_count": user_data.len(),
            "enemy_count": enemy_data.len(),
            "total_users": user_data.keys().collect::<Vec<_>>(),
            "total_enemies": enemy_data.keys().collect::<Vec<_>>()
        });
        let summary_content = serde_json::to_string_pretty(&summary)?;
        async_fs::write(&summary_file, summary_content).await?;

        log::info!("History snapshot saved for timestamp: {}", timestamp);
        Ok(())
    }

    pub async fn load_snapshot(&self, timestamp: i64) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        use tokio::fs;

        let users_file = format!("{}/{}/users.json", self.history_dir, timestamp);
        let enemies_file = format!("{}/{}/enemies.json", self.history_dir, timestamp);
        let summary_file = format!("{}/{}/summary.json", self.history_dir, timestamp);

        // Load user data
        let user_content = fs::read_to_string(&users_file).await?;
        let user_data: serde_json::Value = serde_json::from_str(&user_content)?;

        // Load enemy data
        let enemy_content = fs::read_to_string(&enemies_file).await?;
        let enemy_data: serde_json::Value = serde_json::from_str(&enemy_content)?;

        // Load summary
        let summary_content = fs::read_to_string(&summary_file).await?;
        let summary: serde_json::Value = serde_json::from_str(&summary_content)?;

        Ok(json!({
            "code": 0,
            "timestamp": timestamp,
            "users": user_data,
            "enemies": enemy_data,
            "summary": summary
        }))
    }

    pub async fn list_snapshots(&self) -> Result<Vec<i64>, Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;

        let mut snapshots = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.history_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            if let Some(dir_name) = entry.file_name().to_str() {
                                if let Ok(timestamp) = dir_name.parse::<i64>() {
                                    snapshots.push(timestamp);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.cmp(a));

        Ok(snapshots)
    }

    pub async fn get_all_user_data(&self, timestamp: i64) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        use tokio::fs;

        let users_file = format!("{}/{}/users.json", self.history_dir, timestamp);
        let content = fs::read_to_string(&users_file).await?;
        let data: serde_json::Value = serde_json::from_str(&content)?;

        Ok(json!({
            "code": 0,
            "timestamp": timestamp,
            "user": data
        }))
    }

    pub async fn cleanup_old_snapshots(&self, keep_days: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let cutoff = now - (keep_days * 24 * 60 * 60);

        if let Ok(entries) = fs::read_dir(&self.history_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified()?.duration_since(UNIX_EPOCH) {
                            let modified_secs = modified.as_secs() as i64;
                            if modified_secs < cutoff {
                                if let Err(e) = fs::remove_dir_all(entry.path()) {
                                    log::warn!("Failed to remove old snapshot: {:?}", e);
                                } else {
                                    log::info!("Removed old snapshot: {:?}", entry.file_name());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
