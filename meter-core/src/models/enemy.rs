use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub id: u32,
    pub name: String,
    pub hp: u32,
    pub max_hp: u32,
    pub last_update: DateTime<Utc>,
}

impl Enemy {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            name: format!("Enemy_{}", id),
            hp: 0,
            max_hp: 0,
            last_update: Utc::now(),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.last_update = Utc::now();
    }

    pub fn set_hp(&mut self, hp: u32) {
        self.hp = hp;
        self.last_update = Utc::now();
    }

    pub fn set_max_hp(&mut self, max_hp: u32) {
        self.max_hp = max_hp;
        self.last_update = Utc::now();
    }

    pub fn is_dead(&self) -> bool {
        self.hp == 0
    }
}

impl Default for Enemy {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            hp: 0,
            max_hp: 0,
            last_update: Utc::now(),
        }
    }
}
