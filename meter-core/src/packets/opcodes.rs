//! Packet opcodes for Lost Ark network protocol

/// Packet operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Pkt {
    // Connection and initialization
    InitEnv = 0x0001,
    InitPC = 0x0002,
    MigrationExecute = 0x0003,

    // Entity management
    NewPC = 0x0004,
    NewNpc = 0x0005,
    NewNpcSummon = 0x0006,
    NewVehicle = 0x0007,
    NewProjectile = 0x0008,
    NewTrap = 0x0009,
    RemoveObject = 0x000A,

    // Combat and skills
    SkillStartNotify = 0x000B,
    SkillDamageNotify = 0x000C,
    SkillDamageAbnormalMoveNotify = 0x000D,
    SkillCastNotify = 0x000E,
    SkillCooldownNotify = 0x000F,
    SkillStageNotify = 0x0010,

    // Status effects
    StatusEffectAddNotify = 0x0011,
    StatusEffectRemoveNotify = 0x0012,
    StatusEffectDurationNotify = 0x0013,
    StatusEffectSyncDataNotify = 0x0014,

    // Party and raid
    PartyInfo = 0x0015,
    PartyLeaveResult = 0x0016,
    PartyStatusEffectAddNotify = 0x0017,
    PartyStatusEffectRemoveNotify = 0x0018,
    PartyStatusEffectResultNotify = 0x0019,
    PartyMemberUpdateMinNotify = 0x001A,
    TroopMemberUpdateMinNotify = 0x001B,

    // Zone and area
    ZoneMemberLoadStatusNotify = 0x001C,
    ZoneObjectUnpublishNotify = 0x001D,
    NewTransit = 0x001E,
    TriggerStartNotify = 0x001F,
    TriggerBossBattleStatus = 0x0020,

    // Combat events
    DeathNotify = 0x0021,
    CounterAttackNotify = 0x0022,
    RaidBegin = 0x0023,
    RaidBossKillNotify = 0x0024,
    RaidResult = 0x0025,

    // Identity and gauges
    IdentityGaugeChangeNotify = 0x0026,
    IdentityStanceChangeNotify = 0x0027,
    ParalyzationStateNotify = 0x0028,

    // Item and equipment
    InitItem = 0x0029,

    // Unknown/Reserved
    Unknown = 0xFFFF,
}

impl Pkt {
    /// Try to convert a u16 value to a Pkt enum
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0001 => Some(Pkt::InitEnv),
            0x0002 => Some(Pkt::InitPC),
            0x0003 => Some(Pkt::MigrationExecute),
            0x0004 => Some(Pkt::NewPC),
            0x0005 => Some(Pkt::NewNpc),
            0x0006 => Some(Pkt::NewNpcSummon),
            0x0007 => Some(Pkt::NewVehicle),
            0x0008 => Some(Pkt::NewProjectile),
            0x0009 => Some(Pkt::NewTrap),
            0x000A => Some(Pkt::RemoveObject),
            0x000B => Some(Pkt::SkillStartNotify),
            0x000C => Some(Pkt::SkillDamageNotify),
            0x000D => Some(Pkt::SkillDamageAbnormalMoveNotify),
            0x000E => Some(Pkt::SkillCastNotify),
            0x000F => Some(Pkt::SkillCooldownNotify),
            0x0010 => Some(Pkt::SkillStageNotify),
            0x0011 => Some(Pkt::StatusEffectAddNotify),
            0x0012 => Some(Pkt::StatusEffectRemoveNotify),
            0x0013 => Some(Pkt::StatusEffectDurationNotify),
            0x0014 => Some(Pkt::StatusEffectSyncDataNotify),
            0x0015 => Some(Pkt::PartyInfo),
            0x0016 => Some(Pkt::PartyLeaveResult),
            0x0017 => Some(Pkt::PartyStatusEffectAddNotify),
            0x0018 => Some(Pkt::PartyStatusEffectRemoveNotify),
            0x0019 => Some(Pkt::PartyStatusEffectResultNotify),
            0x001A => Some(Pkt::PartyMemberUpdateMinNotify),
            0x001B => Some(Pkt::TroopMemberUpdateMinNotify),
            0x001C => Some(Pkt::ZoneMemberLoadStatusNotify),
            0x001D => Some(Pkt::ZoneObjectUnpublishNotify),
            0x001E => Some(Pkt::NewTransit),
            0x001F => Some(Pkt::TriggerStartNotify),
            0x0020 => Some(Pkt::TriggerBossBattleStatus),
            0x0021 => Some(Pkt::DeathNotify),
            0x0022 => Some(Pkt::CounterAttackNotify),
            0x0023 => Some(Pkt::RaidBegin),
            0x0024 => Some(Pkt::RaidBossKillNotify),
            0x0025 => Some(Pkt::RaidResult),
            0x0026 => Some(Pkt::IdentityGaugeChangeNotify),
            0x0027 => Some(Pkt::IdentityStanceChangeNotify),
            0x0028 => Some(Pkt::ParalyzationStateNotify),
            0x0029 => Some(Pkt::InitItem),
            _ => None,
        }
    }

    /// Convert Pkt to u16
    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

impl std::fmt::Display for Pkt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Pkt::InitEnv => "InitEnv",
            Pkt::InitPC => "InitPC",
            Pkt::MigrationExecute => "MigrationExecute",
            Pkt::NewPC => "NewPC",
            Pkt::NewNpc => "NewNpc",
            Pkt::NewNpcSummon => "NewNpcSummon",
            Pkt::NewVehicle => "NewVehicle",
            Pkt::NewProjectile => "NewProjectile",
            Pkt::NewTrap => "NewTrap",
            Pkt::RemoveObject => "RemoveObject",
            Pkt::SkillStartNotify => "SkillStartNotify",
            Pkt::SkillDamageNotify => "SkillDamageNotify",
            Pkt::SkillDamageAbnormalMoveNotify => "SkillDamageAbnormalMoveNotify",
            Pkt::SkillCastNotify => "SkillCastNotify",
            Pkt::SkillCooldownNotify => "SkillCooldownNotify",
            Pkt::SkillStageNotify => "SkillStageNotify",
            Pkt::StatusEffectAddNotify => "StatusEffectAddNotify",
            Pkt::StatusEffectRemoveNotify => "StatusEffectRemoveNotify",
            Pkt::StatusEffectDurationNotify => "StatusEffectDurationNotify",
            Pkt::StatusEffectSyncDataNotify => "StatusEffectSyncDataNotify",
            Pkt::PartyInfo => "PartyInfo",
            Pkt::PartyLeaveResult => "PartyLeaveResult",
            Pkt::PartyStatusEffectAddNotify => "PartyStatusEffectAddNotify",
            Pkt::PartyStatusEffectRemoveNotify => "PartyStatusEffectRemoveNotify",
            Pkt::PartyStatusEffectResultNotify => "PartyStatusEffectResultNotify",
            Pkt::PartyMemberUpdateMinNotify => "PartyMemberUpdateMinNotify",
            Pkt::TroopMemberUpdateMinNotify => "TroopMemberUpdateMinNotify",
            Pkt::ZoneMemberLoadStatusNotify => "ZoneMemberLoadStatusNotify",
            Pkt::ZoneObjectUnpublishNotify => "ZoneObjectUnpublishNotify",
            Pkt::NewTransit => "NewTransit",
            Pkt::TriggerStartNotify => "TriggerStartNotify",
            Pkt::TriggerBossBattleStatus => "TriggerBossBattleStatus",
            Pkt::DeathNotify => "DeathNotify",
            Pkt::CounterAttackNotify => "CounterAttackNotify",
            Pkt::RaidBegin => "RaidBegin",
            Pkt::RaidBossKillNotify => "RaidBossKillNotify",
            Pkt::RaidResult => "RaidResult",
            Pkt::IdentityGaugeChangeNotify => "IdentityGaugeChangeNotify",
            Pkt::IdentityStanceChangeNotify => "IdentityStanceChangeNotify",
            Pkt::ParalyzationStateNotify => "ParalyzationStateNotify",
            Pkt::InitItem => "InitItem",
            Pkt::Unknown => "Unknown",
        };
        write!(f, "{}", name)
    }
}
