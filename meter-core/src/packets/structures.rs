//! Common data structures used in Lost Ark packets

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Status effect data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffectData {
    pub status_effect_id: u32,
    pub instance_id: u32,
    pub source_id: u32,
    pub target_id: u32,
    pub value: Vec<u8>,
    pub expiration_delay: f32,
}

/// Skill cooldown data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCooldownStruct {
    pub skill_id: u32,
    pub current_cooldown: f32,
    pub skill_cooldown_stack_data: Option<SkillCooldownStackData>,
}

/// Skill cooldown stack data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCooldownStackData {
    pub current_stack_cooldown: Option<f32>,
    pub has_stacks: bool,
}

/// PC (Player Character) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PCStruct {
    pub character_id: u64,
    pub name: String,
    pub class_id: u32,
    pub level: u32,
    pub gear_level: f32,
    pub stat_pairs: Vec<StatPair>,
}

/// NPC structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcStruct {
    pub npc_id: u32,
    pub name: String,
    pub level: u32,
    pub stat_pairs: Vec<StatPair>,
}

/// Stat pair for entity stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatPair {
    pub stat_type: u32,
    pub value: f32,
}

/// Projectile info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileInfo {
    pub projectile_id: u32,
    pub owner_id: u32,
    pub skill_id: u32,
    pub skill_effect_id: Option<u32>,
}

/// Trap structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrapStruct {
    pub object_id: u32,
    pub owner_id: u32,
    pub skill_id: u32,
}

/// Skill move option data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMoveOptionData {
    pub down_time: Option<f32>,
    pub move_time: Option<f32>,
    pub stand_up_time: Option<f32>,
}

/// Tripod index for skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripodIndex {
    pub first: u8,
    pub second: u8,
    pub third: u8,
}

/// Tripod level for skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripodLevel {
    pub first: u8,
    pub second: u8,
    pub third: u8,
}

/// Skill option data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOptionData {
    pub tripod_index: Option<TripodIndex>,
    pub tripod_level: Option<TripodLevel>,
}

/// Party instance info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyInstanceInfo {
    pub raid_instance_id: u32,
    pub party_instance_id: u32,
    pub character_id: u64,
    pub character_name: String,
}

/// Status effect target type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StatusEffectTargetType {
    Local = 0,
    Party = 1,
}

/// Status effect type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StatusEffectType {
    Unknown = 0,
    Shield = 1,
    HardCrowdControl = 2,
}

/// Status effect buff type flags
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatusEffectBuffTypeFlags(u32);

impl StatusEffectBuffTypeFlags {
    pub const DMG: u32 = 0x0001;
    pub const HEAL: u32 = 0x0002;
    pub const SHIELD: u32 = 0x0004;

    pub fn bits(&self) -> u32 {
        self.0
    }

    pub fn has_dmg(&self) -> bool {
        self.0 & Self::DMG != 0
    }

    pub fn has_heal(&self) -> bool {
        self.0 & Self::HEAL != 0
    }

    pub fn has_shield(&self) -> bool {
        self.0 & Self::SHIELD != 0
    }
}

/// Status effect target
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StatusEffectTarget {
    Self_ = 0,
    Party = 1,
    Enemy = 2,
}

/// Status effect details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffectDetails {
    pub status_effect_id: u32,
    pub instance_id: u32,
    pub source_id: u32,
    pub target_id: u32,
    pub value: u64,
    pub expiration_delay: f32,
    pub timestamp: DateTime<Utc>,
    pub target_type: StatusEffectTargetType,
    pub status_effect_type: StatusEffectType,
}

/// Item data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemData {
    pub item_id: u32,
    pub item_level: u32,
    pub storage_type: u32,
}

/// Zone key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneKey {
    pub zone_id: u32,
    pub zone_level: u32,
}

/// Transit structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitStruct {
    pub zone_instance_id: u32,
}

/// Signal info for triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSignalInfo {
    pub signal: u32,
    pub trigger_signal_type: u32,
}

/// Boss battle status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossBattleStatus {
    pub boss_id: u32,
    pub status: u32,
}

/// Identity gauge data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityGaugeData {
    pub gauge1: f32,
    pub gauge2: f32,
    pub gauge3: f32,
}

/// Paralyzation (stagger) state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParalyzationState {
    pub object_id: u32,
    pub paralyzation_point: f32,
    pub paralyzation_max_point: f32,
}
