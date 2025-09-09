use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub skills: HashMap<u32, SkillInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub profession: Option<String>,
    pub element: Option<String>,
}

impl SkillConfig {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn get_skill_name(&self, skill_id: u32) -> String {
        self.skills
            .get(&skill_id)
            .map(|skill| skill.name.clone())
            .unwrap_or_else(|| skill_id.to_string())
    }

    pub fn add_skill(&mut self, skill_id: u32, name: String) {
        self.skills.insert(skill_id, SkillInfo {
            id: skill_id,
            name,
            description: None,
            profession: None,
            element: None,
        });
    }

    pub fn load_from_json(&mut self, json_data: &str) -> Result<(), serde_json::Error> {
        let data: serde_json::Value = serde_json::from_str(json_data)?;
        if let Some(skill_names) = data.get("skill_names").and_then(|v| v.as_object()) {
            for (key, value) in skill_names {
                if let Ok(skill_id) = key.parse::<u32>() {
                    if let Some(name) = value.as_str() {
                        self.add_skill(skill_id, name.to_string());
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillInfo {
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            description: None,
            profession: None,
            element: None,
        }
    }
}

// 职业相关的技能映射
pub fn get_sub_profession_by_skill_id(skill_id: u32) -> Option<String> {
    match skill_id {
        1241 => Some("射线".to_string()),
        2307 | 2361 | 55302 => Some("协奏".to_string()),
        20301 => Some("愈合".to_string()),
        1518 | 1541 | 21402 => Some("惩戒".to_string()),
        2306 => Some("狂音".to_string()),
        120901 | 120902 => Some("冰矛".to_string()),
        1714 | 1734 => Some("居合".to_string()),
        44701 | 179906 => Some("月刃".to_string()),
        220112 | 2203622 => Some("鹰弓".to_string()),
        2292 | 1700820 | 1700825 | 1700827 => Some("狼弓".to_string()),
        1419 => Some("空枪".to_string()),
        1405 | 1418 => Some("重装".to_string()),
        2405 => Some("防盾".to_string()),
        2406 => Some("光盾".to_string()),
        199902 => Some("岩盾".to_string()),
        1930 | 1931 | 1934 | 1935 => Some("格挡".to_string()),
        _ => None,
    }
}

pub fn get_profession_name_from_id(profession_id: u32) -> Option<String> {
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
