use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageRecord {
    pub attacker_uid: u32,
    pub target_uid: u32,
    pub skill_id: u32,
    pub element: String,
    pub damage: u64,
    pub hp_lessen: u64,
    pub is_crit: bool,
    pub is_lucky: bool,
    pub is_cause_lucky: bool,
    pub is_miss: bool,
    pub damage_source: DamageSource,
    pub damage_property: DamageProperty,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingRecord {
    pub healer_uid: u32,
    pub target_uid: u32,
    pub skill_id: u32,
    pub element: String,
    pub healing: u64,
    pub is_crit: bool,
    pub is_lucky: bool,
    pub is_cause_lucky: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DamageSource {
    Skill,
    Bullet,
    Buff,
    Fall,
    FakeBullet,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DamageProperty {
    General,
    Fire,
    Water,
    Electricity,
    Wood,
    Wind,
    Rock,
    Light,
    Dark,
}

impl DamageRecord {
    pub fn new(
        attacker_uid: u32,
        target_uid: u32,
        skill_id: u32,
        element: String,
        damage: u64,
        hp_lessen: u64,
        is_crit: bool,
        is_lucky: bool,
        is_cause_lucky: bool,
        is_miss: bool,
        damage_source: DamageSource,
        damage_property: DamageProperty,
    ) -> Self {
        Self {
            attacker_uid,
            target_uid,
            skill_id,
            element,
            damage,
            hp_lessen,
            is_crit,
            is_lucky,
            is_cause_lucky,
            is_miss,
            damage_source,
            damage_property,
            timestamp: Utc::now(),
        }
    }

    pub fn get_element_emoji(&self) -> String {
        match self.damage_property {
            DamageProperty::General => "⚔️物",
            DamageProperty::Fire => "🔥火",
            DamageProperty::Water => "❄️冰",
            DamageProperty::Electricity => "⚡雷",
            DamageProperty::Wood => "🍀森",
            DamageProperty::Wind => "💨风",
            DamageProperty::Rock => "⛰️岩",
            DamageProperty::Light => "🌟光",
            DamageProperty::Dark => "🌑暗",
        }.to_string()
    }
}

impl HealingRecord {
    pub fn new(
        healer_uid: u32,
        target_uid: u32,
        skill_id: u32,
        element: String,
        healing: u64,
        is_crit: bool,
        is_lucky: bool,
        is_cause_lucky: bool,
    ) -> Self {
        Self {
            healer_uid,
            target_uid,
            skill_id,
            element,
            healing,
            is_crit,
            is_lucky,
            is_cause_lucky,
            timestamp: Utc::now(),
        }
    }
}

impl Default for DamageSource {
    fn default() -> Self {
        DamageSource::Skill
    }
}

impl Default for DamageProperty {
    fn default() -> Self {
        DamageProperty::General
    }
}
