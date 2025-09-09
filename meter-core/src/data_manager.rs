use crate::models::*;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserCache {
    pub uid: u32,
    pub name: String,
    pub profession: String,
    pub fight_point: u32,
    pub max_hp: u32,
}

#[derive(Debug)]
pub struct DataManager {
    pub users: DashMap<u32, Arc<RwLock<User>>>,
    pub enemies: DashMap<u32, Arc<RwLock<Enemy>>>,
    pub skill_config: Arc<RwLock<SkillConfig>>,
    pub settings: Arc<RwLock<GlobalSettings>>,
    pub cache_file_path: String,
    pub settings_file_path: String,
    pub start_time: DateTime<Utc>,
    pub is_paused: Arc<RwLock<bool>>,
    pub last_log_time: Arc<RwLock<DateTime<Utc>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    pub auto_clear_on_server_change: bool,
    pub auto_clear_on_timeout: bool,
    pub only_record_elite_dummy: bool,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            auto_clear_on_server_change: true,
            auto_clear_on_timeout: false,
            only_record_elite_dummy: false,
        }
    }
}

impl DataManager {
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
            enemies: DashMap::new(),
            skill_config: Arc::new(RwLock::new(SkillConfig::new())),
            settings: Arc::new(RwLock::new(GlobalSettings::default())),
            cache_file_path: "users.json".to_string(),
            settings_file_path: "settings.json".to_string(),
            start_time: Utc::now(),
            is_paused: Arc::new(RwLock::new(false)),
            last_log_time: Arc::new(RwLock::new(Utc::now())),
        }
    }

    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.load_user_cache().await?;
        self.load_settings().await?;
        self.load_skill_config().await?;
        Ok(())
    }

    async fn load_user_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !Path::new(&self.cache_file_path).exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.cache_file_path)?;
        let cache_data: HashMap<String, UserCache> = serde_json::from_str(&content)?;
        let entry_count = cache_data.len();

        for (uid_str, cache) in cache_data {
            if let Ok(uid) = uid_str.parse::<u32>() {
                let user = User::new(uid);
                let user = Arc::new(RwLock::new(user));

                {
                    let mut user_write = user.write();
                    user_write.set_name(cache.name);
                    user_write.set_profession(cache.profession);
                    user_write.set_fight_point(cache.fight_point);
                    user_write.set_attr("max_hp", cache.max_hp);
                }

                self.users.insert(uid, user);
            }
        }

        log::info!("Loaded {} user cache entries", entry_count);
        Ok(())
    }

    async fn load_settings(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !Path::new(&self.settings_file_path).exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.settings_file_path)?;
        let settings: GlobalSettings = serde_json::from_str(&content)?;
        *self.settings.write() = settings;

        Ok(())
    }

    async fn load_skill_config(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Try to load skill names from tables/skill_names.json
        let skill_file_path = "tables/skill_names.json";
        if Path::new(skill_file_path).exists() {
            let content = fs::read_to_string(skill_file_path)?;
            let mut skill_config = self.skill_config.write();
            skill_config.load_from_json(&content)?;
            log::info!("Loaded skill configuration from {}", skill_file_path);
        }

        Ok(())
    }

    pub async fn save_user_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cache_data = HashMap::new();

        for entry in self.users.iter() {
            let uid = *entry.key();
            let user = entry.value().read();

            let cache = UserCache {
                uid,
                name: user.name.clone(),
                profession: user.profession.clone(),
                fight_point: user.fight_point,
                max_hp: user.max_hp,
            };

            cache_data.insert(uid.to_string(), cache);
        }

        let content = serde_json::to_string_pretty(&cache_data)?;
        fs::write(&self.cache_file_path, content)?;

        log::debug!("Saved {} user cache entries", cache_data.len());
        Ok(())
    }

    pub async fn save_settings(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let settings = self.settings.read();
        let content = serde_json::to_string_pretty(&*settings)?;
        fs::write(&self.settings_file_path, content)?;
        Ok(())
    }

    pub fn get_or_create_user(&self, uid: u32) -> Arc<RwLock<User>> {
        self.users
            .entry(uid)
            .or_insert_with(|| Arc::new(RwLock::new(User::new(uid))))
            .clone()
    }

    pub fn get_or_create_enemy(&self, id: u32) -> Arc<RwLock<Enemy>> {
        self.enemies
            .entry(id)
            .or_insert_with(|| Arc::new(RwLock::new(Enemy::new(id))))
            .clone()
    }

    pub async fn add_damage(
        &self,
        uid: u32,
        skill_id: u32,
        element: String,
        damage: u64,
        is_crit: bool,
        is_lucky: bool,
        is_cause_lucky: bool,
        hp_lessen: u64,
        target_uid: u32,
    ) {
        if *self.is_paused.read() {
            return;
        }

        if self.settings.read().only_record_elite_dummy && target_uid != 75 {
            return;
        }

        let user = self.get_or_create_user(uid);
        {
            let mut user_write = user.write();
            user_write.add_damage(skill_id, element, damage, is_crit, is_lucky, is_cause_lucky, hp_lessen);

            // Set sub profession based on skill
            if let Some(sub_profession) = get_sub_profession_by_skill_id(skill_id) {
                user_write.set_sub_profession(sub_profession);
            }
        }

        *self.last_log_time.write() = Utc::now();
    }

    pub async fn add_healing(
        &self,
        uid: u32,
        skill_id: u32,
        element: String,
        healing: u64,
        is_crit: bool,
        is_lucky: bool,
        is_cause_lucky: bool,
        target_uid: u32,
    ) {
        if *self.is_paused.read() {
            return;
        }

        if uid == 0 {
            return; // Skip healing from unknown source
        }

        let user = self.get_or_create_user(uid);
        {
            let mut user_write = user.write();
            user_write.add_healing(skill_id, element, healing, is_crit, is_lucky, is_cause_lucky);

            // Set sub profession based on skill
            if let Some(sub_profession) = get_sub_profession_by_skill_id(skill_id) {
                user_write.set_sub_profession(sub_profession);
            }
        }

        *self.last_log_time.write() = Utc::now();
    }

    pub async fn add_taken_damage(&self, uid: u32, damage: u32, is_dead: bool) {
        if *self.is_paused.read() {
            return;
        }

        let user = self.get_or_create_user(uid);
        {
            let mut user_write = user.write();
            user_write.add_taken_damage(damage, is_dead);
        }

        *self.last_log_time.write() = Utc::now();
    }

    pub fn set_user_name(&self, uid: u32, name: String) {
        let user = self.get_or_create_user(uid);
        user.write().set_name(name);
    }

    pub fn set_user_profession(&self, uid: u32, profession: String) {
        let user = self.get_or_create_user(uid);
        user.write().set_profession(profession);
    }

    pub fn set_user_fight_point(&self, uid: u32, fight_point: u32) {
        let user = self.get_or_create_user(uid);
        user.write().set_fight_point(fight_point);
    }

    pub fn set_user_attr(&self, uid: u32, key: &str, value: u32) {
        let user = self.get_or_create_user(uid);
        user.write().set_attr(key, value);
    }

    pub fn set_enemy_name(&self, id: u32, name: String) {
        let enemy = self.get_or_create_enemy(id);
        enemy.write().set_name(name);
    }

    pub fn set_enemy_hp(&self, id: u32, hp: u32) {
        let enemy = self.get_or_create_enemy(id);
        enemy.write().set_hp(hp);
    }

    pub fn set_enemy_max_hp(&self, id: u32, max_hp: u32) {
        let enemy = self.get_or_create_enemy(id);
        enemy.write().set_max_hp(max_hp);
    }

    pub fn update_dps(&self) {
        for user_entry in self.users.iter() {
            user_entry.value().write().update_dps();
        }
    }

    pub fn update_hps(&self) {
        for user_entry in self.users.iter() {
            user_entry.value().write().update_hps();
        }
    }

    pub fn get_all_users_data(&self) -> HashMap<u32, serde_json::Value> {
        let mut result = HashMap::new();

        for entry in self.users.iter() {
            let uid = *entry.key();
            let user = entry.value().read();

            let summary = serde_json::json!({
                "name": user.name,
                "profession": format!("{}{}", user.profession, user.sub_profession),
                "realtime_dps": user.damage_stats.dps,
                "realtime_dps_max": user.damage_stats.dps_max,
                "total_dps": user.damage_stats.dps,
                "total_damage": {
                    "normal": user.damage_stats.normal_damage,
                    "critical": user.damage_stats.critical_damage,
                    "lucky": user.damage_stats.lucky_damage,
                    "crit_lucky": user.damage_stats.crit_lucky_damage,
                    "total": user.damage_stats.total_damage
                },
                "total_count": {
                    "normal": user.damage_stats.normal_count,
                    "critical": user.damage_stats.critical_count,
                    "lucky": user.damage_stats.lucky_count,
                    "total": user.damage_stats.total_count
                },
                "realtime_hps": user.healing_stats.hps,
                "realtime_hps_max": user.healing_stats.hps_max,
                "total_hps": user.healing_stats.hps,
                "total_healing": {
                    "normal": user.healing_stats.normal_healing,
                    "critical": user.healing_stats.critical_healing,
                    "lucky": user.healing_stats.lucky_healing,
                    "crit_lucky": user.healing_stats.crit_lucky_healing,
                    "total": user.healing_stats.total_healing
                },
                "taken_damage": user.taken_damage,
                "fight_point": user.fight_point,
                "hp": user.hp,
                "max_hp": user.max_hp,
                "dead_count": user.dead_count
            });

            result.insert(uid, summary);
        }

        result
    }

    pub fn get_all_enemies_data(&self) -> HashMap<u32, serde_json::Value> {
        let mut result = HashMap::new();

        for entry in self.enemies.iter() {
            let id = *entry.key();
            let enemy = entry.value().read();

            let data = serde_json::json!({
                "name": enemy.name,
                "hp": enemy.hp,
                "max_hp": enemy.max_hp
            });

            result.insert(id, data);
        }

        result
    }

    pub fn clear_all(&self) {
        // Clear all users
        for user_entry in self.users.iter() {
            user_entry.value().write().reset();
        }

        // Clear all enemies
        self.enemies.clear();
    }

    pub fn pause(&self, paused: bool) {
        *self.is_paused.write() = paused;
    }

    pub fn is_paused(&self) -> bool {
        *self.is_paused.read()
    }

    pub fn check_timeout_clear(&self) {
        if !self.settings.read().auto_clear_on_timeout {
            return;
        }

        let last_log = *self.last_log_time.read();
        let now = Utc::now();
        let timeout_duration = Duration::seconds(15);

        if now.signed_duration_since(last_log) > timeout_duration {
            self.clear_all();
            log::info!("Statistics cleared due to timeout");
        }
    }
}
