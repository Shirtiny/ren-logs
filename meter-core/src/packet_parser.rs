use crate::models::*;
use crate::data_manager::DataManager;
use bytes::{Buf, Bytes};
use prost::Message;
use std::collections::HashMap;
use std::sync::Arc;

// Protobuf message definitions (simplified for now)
#[derive(Clone, PartialEq, Message)]
pub struct SyncNearDeltaInfo {
    #[prost(message, repeated, tag = "1")]
    pub delta_infos: Vec<AoiSyncDelta>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SyncToMeDeltaInfo {
    #[prost(message, optional, tag = "1")]
    pub delta_info: Option<AoiSyncToMeDelta>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SyncContainerData {
    #[prost(message, optional, tag = "1")]
    pub v_data: Option<VData>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SyncContainerDirtyData {
    #[prost(message, optional, tag = "1")]
    pub v_data: Option<VData>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SyncNearEntities {
    #[prost(message, repeated, tag = "1")]
    pub appear: Vec<Entity>,
}

#[derive(Clone, PartialEq, Message)]
pub struct AoiSyncDelta {
    #[prost(uint64, optional, tag = "1")]
    pub uuid: Option<u64>,
    #[prost(message, optional, tag = "2")]
    pub attrs: Option<AttrCollection>,
    #[prost(message, optional, tag = "3")]
    pub skill_effects: Option<SkillEffects>,
}

#[derive(Clone, PartialEq, Message)]
pub struct AoiSyncToMeDelta {
    #[prost(message, optional, tag = "1")]
    pub base_delta: Option<AoiSyncDelta>,
}

#[derive(Clone, PartialEq, Message)]
pub struct AttrCollection {
    #[prost(message, repeated, tag = "1")]
    pub attrs: Vec<Attr>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Attr {
    #[prost(uint32, optional, tag = "1")]
    pub id: Option<u32>,
    #[prost(bytes, optional, tag = "2")]
    pub raw_data: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SkillEffects {
    #[prost(message, repeated, tag = "1")]
    pub damages: Vec<SyncDamageInfo>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SyncDamageInfo {
    #[prost(uint32, optional, tag = "1")]
    pub owner_id: Option<u32>,
    #[prost(uint64, optional, tag = "2")]
    pub attacker_uuid: Option<u64>,
    #[prost(uint64, optional, tag = "3")]
    pub target_uuid: Option<u64>,
    #[prost(uint64, optional, tag = "4")]
    pub value: Option<u64>,
    #[prost(uint64, optional, tag = "5")]
    pub lucky_value: Option<u64>,
    #[prost(uint32, optional, tag = "6")]
    pub type_flag: Option<u32>,
    #[prost(bool, optional, tag = "7")]
    pub is_miss: Option<bool>,
    #[prost(uint32, optional, tag = "8")]
    pub damage_source: Option<u32>,
    #[prost(uint32, optional, tag = "9")]
    pub property: Option<u32>,
    #[prost(uint64, optional, tag = "10")]
    pub hp_lessen_value: Option<u64>,
    #[prost(bool, optional, tag = "11")]
    pub is_dead: Option<bool>,
    #[prost(uint64, optional, tag = "12")]
    pub summoner_id: Option<u64>,
    #[prost(uint64, optional, tag = "13")]
    pub top_summoner_id: Option<u64>,
    #[prost(uint32, optional, tag = "14")]
    pub r#type: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct VData {
    #[prost(uint32, optional, tag = "1")]
    pub char_id: Option<u32>,
    #[prost(message, optional, tag = "2")]
    pub role_level: Option<RoleLevel>,
    #[prost(message, optional, tag = "3")]
    pub attr: Option<AttrData>,
    #[prost(message, optional, tag = "4")]
    pub char_base: Option<CharBase>,
    #[prost(message, optional, tag = "5")]
    pub profession_list: Option<ProfessionList>,
    #[prost(bytes, optional, tag = "6")]
    pub buffer: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, Message)]
pub struct RoleLevel {
    #[prost(uint32, optional, tag = "1")]
    pub level: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct AttrData {
    #[prost(uint32, optional, tag = "1")]
    pub cur_hp: Option<u32>,
    #[prost(uint32, optional, tag = "2")]
    pub max_hp: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct CharBase {
    #[prost(string, optional, tag = "1")]
    pub name: Option<String>,
    #[prost(uint32, optional, tag = "2")]
    pub fight_point: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProfessionList {
    #[prost(uint32, optional, tag = "1")]
    pub cur_profession_id: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Entity {
    #[prost(uint64, optional, tag = "1")]
    pub uuid: Option<u64>,
    #[prost(uint32, optional, tag = "2")]
    pub ent_type: Option<u32>,
    #[prost(message, optional, tag = "3")]
    pub attrs: Option<AttrCollection>,
}

// Message type constants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageType {
    Notify = 2,
    Return = 3,
    FrameDown = 6,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotifyMethod {
    SyncNearEntities = 0x00000006,
    SyncContainerData = 0x00000015,
    SyncContainerDirtyData = 0x00000016,
    SyncServerTime = 0x0000002b,
    SyncNearDeltaInfo = 0x0000002d,
    SyncToMeDeltaInfo = 0x0000002e,
}

// Damage type enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EDamageType {
    Damage = 0,
    Heal = 1,
}

// Entity type enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EEntityType {
    EntChar = 1,
    EntMonster = 2,
}

// Attribute type constants
pub const ATTR_NAME: u32 = 0x01;
pub const ATTR_ID: u32 = 0x0a;
pub const ATTR_PROFESSION_ID: u32 = 0xdc;
pub const ATTR_FIGHT_POINT: u32 = 0x272e;
pub const ATTR_LEVEL: u32 = 0x2710;
pub const ATTR_RANK_LEVEL: u32 = 0x274c;
pub const ATTR_CRIT: u32 = 0x2b66;
pub const ATTR_LUCKY: u32 = 0x2b7a;
pub const ATTR_HP: u32 = 0x2c2e;
pub const ATTR_MAX_HP: u32 = 0x2c38;
pub const ATTR_ELEMENT_FLAG: u32 = 0x646d6c;
pub const ATTR_ENERGY_FLAG: u32 = 0x543cd3c6;

pub struct PacketParser {
    data_manager: Arc<DataManager>,
    current_user_uuid: u64,
}

impl PacketParser {
    pub fn new(data_manager: Arc<DataManager>) -> Self {
        Self {
            data_manager,
            current_user_uuid: 0,
        }
    }

    pub async fn process_packet(&mut self, packet_data: &[u8]) {
        if packet_data.len() < 6 {
            log::debug!("Received invalid packet: too short");
            return;
        }

        let mut reader = BinaryReader::new(packet_data);

        // Skip packet size (already handled)
        let _packet_size = reader.read_u32_be();

        let packet_type = reader.read_u16_be();
        let is_compressed = (packet_type & 0x8000) != 0;
        let msg_type_id = packet_type & 0x7fff;

        let mut payload_data = reader.read_remaining();

        // Decompress if needed
        let payload = if is_compressed {
            match zstd::decode_all(payload_data) {
                Ok(data) => data,
                Err(e) => {
                    log::error!("Failed to decompress packet: {}", e);
                    return;
                }
            }
        } else {
            payload_data.to_vec()
        };

        match msg_type_id {
            x if x == MessageType::Notify as u16 => {
                self.process_notify_message(&payload).await;
            }
            x if x == MessageType::Return as u16 => {
                // Handle return messages if needed
                log::debug!("Processing return message");
            }
            x if x == MessageType::FrameDown as u16 => {
                let _server_sequence_id = reader.read_u32_be();
                if !payload.is_empty() {
                    // Recursively process nested frame
                    Box::pin(self.process_packet(&payload)).await;
                }
            }
            _ => {
                log::debug!("Unknown message type: {}", msg_type_id);
            }
        }
    }

    async fn process_notify_message(&mut self, payload: &[u8]) {
        if payload.len() < 12 {
            return;
        }

        let mut reader = BinaryReader::new(payload);
        let service_uuid = reader.read_u64_be();
        let _stub_id = reader.read_u32_be();
        let method_id = reader.read_u32_be();

        // Check if it's our service
        if service_uuid != 0x0000000063335342 {
            log::debug!("Skipping message with service ID: {}", service_uuid);
            return;
        }

        let msg_payload = reader.read_remaining();

        match method_id {
            x if x == NotifyMethod::SyncNearEntities as u32 => {
                self.process_sync_near_entities(&msg_payload).await;
            }
            x if x == NotifyMethod::SyncContainerData as u32 => {
                self.process_sync_container_data(&msg_payload).await;
            }
            x if x == NotifyMethod::SyncContainerDirtyData as u32 => {
                self.process_sync_container_dirty_data(&msg_payload).await;
            }
            x if x == NotifyMethod::SyncToMeDeltaInfo as u32 => {
                self.process_sync_to_me_delta_info(&msg_payload).await;
            }
            x if x == NotifyMethod::SyncNearDeltaInfo as u32 => {
                self.process_sync_near_delta_info(&msg_payload).await;
            }
            _ => {
                log::debug!("Unknown notify method: {}", method_id);
            }
        }
    }

    async fn process_sync_near_entities(&mut self, payload: &[u8]) {
        let sync_near_entities = match SyncNearEntities::decode(payload) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to decode SyncNearEntities: {}", e);
                return;
            }
        };

        for entity in sync_near_entities.appear {
            if let Some(uuid) = entity.uuid {
                let entity_uid = (uuid >> 16) as u32;
                let entity_type = entity.ent_type.unwrap_or(0);

                if let Some(attrs) = entity.attrs {
                    match entity_type {
                        x if x == EEntityType::EntMonster as u32 => {
                            self.process_enemy_attrs(entity_uid, &attrs.attrs).await;
                        }
                        x if x == EEntityType::EntChar as u32 => {
                            self.process_player_attrs(entity_uid, &attrs.attrs).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn process_sync_container_data(&mut self, payload: &[u8]) {
        let sync_container_data = match SyncContainerData::decode(payload) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to decode SyncContainerData: {}", e);
                return;
            }
        };

        if let Some(v_data) = sync_container_data.v_data {
            if let Some(char_id) = v_data.char_id {
                if let Some(role_level) = v_data.role_level {
                    if let Some(level) = role_level.level {
                        self.data_manager.set_user_attr(char_id, "level", level);
                    }
                }

                if let Some(attr) = v_data.attr {
                    if let Some(cur_hp) = attr.cur_hp {
                        self.data_manager.set_user_attr(char_id, "hp", cur_hp);
                    }
                    if let Some(max_hp) = attr.max_hp {
                        self.data_manager.set_user_attr(char_id, "max_hp", max_hp);
                    }
                }

                if let Some(char_base) = v_data.char_base {
                    if let Some(name) = char_base.name {
                        self.data_manager.set_user_name(char_id, name);
                    }
                    if let Some(fight_point) = char_base.fight_point {
                        self.data_manager.set_user_fight_point(char_id, fight_point);
                    }
                }

                if let Some(profession_list) = v_data.profession_list {
                    if let Some(profession_id) = profession_list.cur_profession_id {
                        if let Some(profession_name) = get_profession_name_from_id(profession_id) {
                            self.data_manager.set_user_profession(char_id, profession_name);
                        }
                    }
                }
            }
        }
    }

    async fn process_sync_container_dirty_data(&mut self, payload: &[u8]) {
        if self.current_user_uuid == 0 {
            return;
        }

        let sync_container_dirty_data = match SyncContainerDirtyData::decode(payload) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to decode SyncContainerDirtyData: {}", e);
                return;
            }
        };

        if let Some(v_data) = sync_container_dirty_data.v_data {
            if let Some(buffer) = v_data.buffer {
                self.process_dirty_data_buffer(&buffer).await;
            }
        }
    }

    async fn process_dirty_data_buffer(&mut self, buffer: &[u8]) {
        let mut reader = BinaryReader::new(buffer);

        // Skip identifier check for now
        if buffer.len() < 8 {
            return;
        }

        let field_index = reader.read_u32_le();
        reader.read_u32_le(); // Skip padding

        match field_index {
            2 => { // CharBase
                if buffer.len() < 16 {
                    return;
                }
                let sub_field_index = reader.read_u32_le();
                reader.read_u32_le();

                match sub_field_index {
                    5 => { // Name
                        let name = self.read_string(&mut reader);
                        let user_uid = (self.current_user_uuid >> 16) as u32;
                        self.data_manager.set_user_name(user_uid, name);
                    }
                    35 => { // FightPoint
                        let fight_point = reader.read_u32_le();
                        reader.read_u32_le();
                        let user_uid = (self.current_user_uuid >> 16) as u32;
                        self.data_manager.set_user_fight_point(user_uid, fight_point);
                    }
                    _ => {}
                }
            }
            16 => { // UserFightAttr
                if buffer.len() < 16 {
                    return;
                }
                let sub_field_index = reader.read_u32_le();
                reader.read_u32_le();

                match sub_field_index {
                    1 => { // CurHp
                        let cur_hp = reader.read_u32_le();
                        let user_uid = (self.current_user_uuid >> 16) as u32;
                        self.data_manager.set_user_attr(user_uid, "hp", cur_hp);
                    }
                    2 => { // MaxHp
                        let max_hp = reader.read_u32_le();
                        let user_uid = (self.current_user_uuid >> 16) as u32;
                        self.data_manager.set_user_attr(user_uid, "max_hp", max_hp);
                    }
                    _ => {}
                }
            }
            61 => { // ProfessionList
                if buffer.len() < 16 {
                    return;
                }
                let sub_field_index = reader.read_u32_le();
                reader.read_u32_le();

                match sub_field_index {
                    1 => { // CurProfessionId
                        let profession_id = reader.read_u32_le();
                        reader.read_u32_le();
                        if let Some(profession_name) = get_profession_name_from_id(profession_id) {
                            let user_uid = (self.current_user_uuid >> 16) as u32;
                            self.data_manager.set_user_profession(user_uid, profession_name);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    async fn process_sync_to_me_delta_info(&mut self, payload: &[u8]) {
        let sync_to_me_delta_info = match SyncToMeDeltaInfo::decode(payload) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to decode SyncToMeDeltaInfo: {}", e);
                return;
            }
        };

        if let Some(delta_info) = sync_to_me_delta_info.delta_info {
            if let Some(uuid) = delta_info.base_delta.as_ref().and_then(|d| d.uuid) {
                if self.current_user_uuid != uuid {
                    self.current_user_uuid = uuid;
                    let uid = (uuid >> 16) as u32;
                    log::info!("Got player UUID! UUID: {}, UID: {}", uuid, uid);
                }
            }

            if let Some(base_delta) = delta_info.base_delta {
                self.process_aoi_sync_delta(&base_delta).await;
            }
        }
    }

    async fn process_sync_near_delta_info(&mut self, payload: &[u8]) {
        let sync_near_delta_info = match SyncNearDeltaInfo::decode(payload) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to decode SyncNearDeltaInfo: {}", e);
                return;
            }
        };

        for delta_info in sync_near_delta_info.delta_infos {
            self.process_aoi_sync_delta(&delta_info).await;
        }
    }

    async fn process_aoi_sync_delta(&mut self, aoi_sync_delta: &AoiSyncDelta) {
        let target_uuid = match aoi_sync_delta.uuid {
            Some(uuid) => uuid,
            None => return,
        };

        let is_target_player = is_uuid_player(target_uuid);
        let is_target_monster = is_uuid_monster(target_uuid);
        let target_uid = (target_uuid >> 16) as u32;

        // Process attributes
        if let Some(attrs) = &aoi_sync_delta.attrs {
            if is_target_player {
                self.process_player_attrs(target_uid, &attrs.attrs).await;
            } else if is_target_monster {
                self.process_enemy_attrs(target_uid, &attrs.attrs).await;
            }
        }

        // Process skill effects
        if let Some(skill_effects) = &aoi_sync_delta.skill_effects {
            for damage_info in &skill_effects.damages {
                self.process_damage_info(damage_info, target_uuid, is_target_player).await;
            }
        }
    }

    async fn process_damage_info(&mut self, damage_info: &SyncDamageInfo, target_uuid: u64, is_target_player: bool) {
        let skill_id = damage_info.owner_id.unwrap_or(0);
        if skill_id == 0 {
            return;
        }

        let mut attacker_uuid = damage_info.top_summoner_id
            .or(damage_info.attacker_uuid)
            .unwrap_or(0);
        if attacker_uuid == 0 {
            return;
        }

        let is_attacker_player = is_uuid_player(attacker_uuid);
        let attacker_uid = (attacker_uuid >> 16) as u32;

        let value = damage_info.value.unwrap_or(0);
        let lucky_value = damage_info.lucky_value.unwrap_or(0);
        let damage = if value > 0 { value } else { lucky_value };
        if damage == 0 {
            return;
        }

        let type_flag = damage_info.type_flag.unwrap_or(0);
        let is_crit = (type_flag & 1) == 1;
        let is_cause_lucky = (type_flag & 0b100) == 0b100;
        let is_lucky = lucky_value > 0;

        let is_miss = damage_info.is_miss.unwrap_or(false);
        let is_heal = damage_info.r#type == Some(EDamageType::Heal as u32);
        let is_dead = damage_info.is_dead.unwrap_or(false);
        let hp_lessen_value = damage_info.hp_lessen_value.unwrap_or(0);
        let damage_property = damage_info.property.unwrap_or(0);
        let element = get_damage_element_name(damage_property);

        let target_uid = (target_uuid >> 16) as u32;

        if is_target_player {
            // 玩家目标
            if is_heal {
                // 玩家被治疗
                self.data_manager.add_healing(
                    if is_attacker_player { attacker_uid } else { 0 },
                    skill_id,
                    element.clone(),
                    damage,
                    is_crit,
                    is_lucky,
                    is_cause_lucky,
                    target_uid,
                ).await;
            } else {
                // 玩家受到伤害
                self.data_manager.add_taken_damage(target_uid, damage as u32, is_dead).await;
            }

            if is_dead {
                self.data_manager.set_user_attr(target_uid, "hp", 0);
            }
        } else {
            // 非玩家目标
            if is_heal {
                // 非玩家被治疗
            } else {
                // 非玩家受到伤害
                if is_attacker_player {
                    // 只记录玩家造成的伤害
                    self.data_manager.add_damage(
                        attacker_uid,
                        skill_id,
                        element.clone(),
                        damage,
                        is_crit,
                        is_lucky,
                        is_cause_lucky,
                        hp_lessen_value,
                        target_uid,
                    ).await;
                }
            }

            if is_dead {
                self.data_manager.set_enemy_hp(target_uid, 0);
            }
        }

        // Log damage/healing
        let action_type = if is_heal { "HEAL" } else { "DMG" };
        let attacker_info = if is_attacker_player {
            format!("{}#{}", "Player", attacker_uid)
        } else {
            format!("{}#{}", "Enemy", attacker_uid)
        };

        let target_info = if is_target_player {
            format!("{}#{}", "Player", target_uid)
        } else {
            format!("{}#{}", "Enemy", target_uid)
        };

        let extra = if is_crit || is_lucky || is_cause_lucky {
            let mut flags = Vec::new();
            if is_crit { flags.push("Crit"); }
            if is_lucky { flags.push("Lucky"); }
            if is_cause_lucky { flags.push("CauseLucky"); }
            flags.join("|")
        } else {
            "Normal".to_string()
        };

        log::info!(
            "[{}] SRC: {} TGT: {} ID: {} VAL: {} HPLSN: {} ELEM: {} EXT: {}",
            action_type, attacker_info, target_info, skill_id, damage, hp_lessen_value,
            element, extra
        );
    }

    async fn process_player_attrs(&mut self, player_uid: u32, attrs: &[Attr]) {
        for attr in attrs {
            if let Some(attr_id) = attr.id {
                if let Some(raw_data) = &attr.raw_data {
                    self.process_attr_data(player_uid, attr_id, raw_data, true).await;
                }
            }
        }
    }

    async fn process_enemy_attrs(&mut self, enemy_id: u32, attrs: &[Attr]) {
        for attr in attrs {
            if let Some(attr_id) = attr.id {
                if let Some(raw_data) = &attr.raw_data {
                    self.process_attr_data(enemy_id, attr_id, raw_data, false).await;
                }
            }
        }
    }

    async fn process_attr_data(&mut self, uid: u32, attr_id: u32, raw_data: &[u8], is_player: bool) {
        match attr_id {
            ATTR_NAME => {
                if is_player {
                    if let Ok(name) = String::from_utf8(raw_data.to_vec()) {
                        self.data_manager.set_user_name(uid, name);
                    }
                } else {
                    if let Ok(name) = String::from_utf8(raw_data.to_vec()) {
                        self.data_manager.set_enemy_name(uid, name);
                    }
                }
            }
            ATTR_ID => {
                if !is_player {
                    // For monsters, the ID might be used to look up names
                    let monster_id = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    // You could implement monster name lookup here
                }
            }
            ATTR_PROFESSION_ID => {
                if is_player {
                    let profession_id = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    if let Some(profession_name) = get_profession_name_from_id(profession_id) {
                        self.data_manager.set_user_profession(uid, profession_name);
                    }
                }
            }
            ATTR_FIGHT_POINT => {
                if is_player {
                    let fight_point = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    self.data_manager.set_user_fight_point(uid, fight_point);
                }
            }
            ATTR_LEVEL => {
                if is_player {
                    let level = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    self.data_manager.set_user_attr(uid, "level", level);
                }
            }
            ATTR_HP => {
                if is_player {
                    let hp = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    self.data_manager.set_user_attr(uid, "hp", hp);
                } else {
                    let hp = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    self.data_manager.set_enemy_hp(uid, hp);
                }
            }
            ATTR_MAX_HP => {
                if is_player {
                    let max_hp = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    self.data_manager.set_user_attr(uid, "max_hp", max_hp);
                } else {
                    let max_hp = u32::from_be_bytes(raw_data.try_into().unwrap_or_default());
                    self.data_manager.set_enemy_max_hp(uid, max_hp);
                }
            }
            _ => {
                log::debug!("Unknown attribute ID: {} for {} {}", attr_id, if is_player { "player" } else { "enemy" }, uid);
            }
        }
    }

    fn read_string(&self, reader: &mut BinaryReader) -> String {
        let length = reader.read_u32_le();
        reader.read_u32_le(); // Skip padding
        let string_data = reader.read_bytes(length as usize).to_vec();
        reader.read_u32_le(); // Skip padding
        String::from_utf8_lossy(&string_data).to_string()
    }
}

// Utility functions
fn is_uuid_player(uuid: u64) -> bool {
    (uuid & 0xffff) == 640
}

fn is_uuid_monster(uuid: u64) -> bool {
    (uuid & 0xffff) == 64
}

fn get_damage_element_name(property: u32) -> String {
    match property {
        0 => "⚔️物".to_string(),
        1 => "🔥火".to_string(),
        2 => "❄️冰".to_string(),
        3 => "⚡雷".to_string(),
        4 => "🍀森".to_string(),
        5 => "💨风".to_string(),
        6 => "⛰️岩".to_string(),
        7 => "🌟光".to_string(),
        8 => "🌑暗".to_string(),
        _ => "⚔️物".to_string(),
    }
}

fn get_profession_name_from_id(profession_id: u32) -> Option<String> {
    match profession_id {
        1 => Some("雷影剑士".to_string()),
        2 => Some("冰魔导师".to_string()),
        3 => Some("涤罪恶火·战斧".to_string()),
        4 => Some("青岚骑士".to_string()),
        5 => Some("森语者".to_string()),
        8 => Some("雷霆一闪·手炮".to_string()),
        9 => Some("巨刃守护者".to_string()),
        10 => Some("暗灵祈舞·仪刀/仪仗".to_string()),
        11 => Some("神射手".to_string()),
        12 => Some("神盾骑士".to_string()),
        13 => Some("灵魂乐手".to_string()),
        _ => None,
    }
}

// Binary reader helper
pub struct BinaryReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn read_u64_be(&mut self) -> u64 {
        let value = u64::from_be_bytes(self.data[self.position..self.position + 8].try_into().unwrap());
        self.position += 8;
        value
    }

    pub fn read_u32_be(&mut self) -> u32 {
        let value = u32::from_be_bytes(self.data[self.position..self.position + 4].try_into().unwrap());
        self.position += 4;
        value
    }

    pub fn read_u32_le(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.data[self.position..self.position + 4].try_into().unwrap());
        self.position += 4;
        value
    }

    pub fn read_u16_be(&mut self) -> u16 {
        let value = u16::from_be_bytes(self.data[self.position..self.position + 2].try_into().unwrap());
        self.position += 2;
        value
    }

    pub fn read_bytes(&mut self, length: usize) -> &[u8] {
        let start = self.position;
        self.position += length;
        &self.data[start..self.position]
    }

    pub fn read_remaining(&mut self) -> &[u8] {
        let start = self.position;
        self.position = self.data.len();
        &self.data[start..]
    }
}
