use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uid: u32,
    pub name: String,
    pub profession: String,
    pub sub_profession: String,
    pub fight_point: u32,
    pub level: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub damage_stats: DamageStats,
    pub healing_stats: HealingStats,
    pub taken_damage: u32,
    pub dead_count: u32,
    pub skill_usage: HashMap<u32, SkillStats>,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageStats {
    pub total_damage: u64,
    pub normal_damage: u64,
    pub critical_damage: u64,
    pub lucky_damage: u64,
    pub crit_lucky_damage: u64,
    pub hp_lessen: u64,
    pub normal_count: u32,
    pub critical_count: u32,
    pub lucky_count: u32,
    pub total_count: u32,
    pub dps: f64,
    pub dps_max: f64,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingStats {
    pub total_healing: u64,
    pub normal_healing: u64,
    pub critical_healing: u64,
    pub lucky_healing: u64,
    pub crit_lucky_healing: u64,
    pub normal_count: u32,
    pub critical_count: u32,
    pub lucky_count: u32,
    pub total_count: u32,
    pub hps: f64,
    pub hps_max: f64,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStats {
    pub skill_id: u32,
    pub display_name: String,
    pub skill_type: String, // "damage" or "healing"
    pub element: String,
    pub total_damage: u64,
    pub total_count: u32,
    pub crit_count: u32,
    pub lucky_count: u32,
    pub crit_rate: f64,
    pub lucky_rate: f64,
    pub damage_breakdown: DamageBreakdown,
    pub count_breakdown: CountBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageBreakdown {
    pub normal: u64,
    pub critical: u64,
    pub lucky: u64,
    pub crit_lucky: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountBreakdown {
    pub normal: u32,
    pub critical: u32,
    pub lucky: u32,
    pub total: u32,
}

impl Default for User {
    fn default() -> Self {
        Self {
            uid: 0,
            name: String::new(),
            profession: "未知".to_string(),
            sub_profession: String::new(),
            fight_point: 0,
            level: 0,
            hp: 0,
            max_hp: 0,
            damage_stats: DamageStats::default(),
            healing_stats: HealingStats::default(),
            taken_damage: 0,
            dead_count: 0,
            skill_usage: HashMap::new(),
            last_update: Utc::now(),
        }
    }
}

impl Default for DamageStats {
    fn default() -> Self {
        Self {
            total_damage: 0,
            normal_damage: 0,
            critical_damage: 0,
            lucky_damage: 0,
            crit_lucky_damage: 0,
            hp_lessen: 0,
            normal_count: 0,
            critical_count: 0,
            lucky_count: 0,
            total_count: 0,
            dps: 0.0,
            dps_max: 0.0,
            time_range: None,
        }
    }
}

impl Default for HealingStats {
    fn default() -> Self {
        Self {
            total_healing: 0,
            normal_healing: 0,
            critical_healing: 0,
            lucky_healing: 0,
            crit_lucky_healing: 0,
            normal_count: 0,
            critical_count: 0,
            lucky_count: 0,
            total_count: 0,
            hps: 0.0,
            hps_max: 0.0,
            time_range: None,
        }
    }
}

impl User {
    pub fn new(uid: u32) -> Self {
        Self {
            uid,
            ..Default::default()
        }
    }

    pub fn add_damage(&mut self, skill_id: u32, element: String, damage: u64, is_crit: bool, is_lucky: bool, is_cause_lucky: bool, hp_lessen: u64) {
        let now = Utc::now();

        // 更新总体伤害统计
        if is_crit && is_lucky {
            self.damage_stats.crit_lucky_damage += damage;
        } else if is_crit {
            self.damage_stats.critical_damage += damage;
        } else if is_lucky {
            self.damage_stats.lucky_damage += damage;
        } else {
            self.damage_stats.normal_damage += damage;
        }
        self.damage_stats.total_damage += damage;
        self.damage_stats.hp_lessen += hp_lessen;

        // 更新次数统计
        if is_crit {
            self.damage_stats.critical_count += 1;
        }
        if is_lucky {
            self.damage_stats.lucky_count += 1;
        }
        if !is_crit && !is_lucky {
            self.damage_stats.normal_count += 1;
        }
        self.damage_stats.total_count += 1;

        // 更新时间范围
        match self.damage_stats.time_range {
            Some((start, _)) => {
                self.damage_stats.time_range = Some((start, now));
            }
            None => {
                self.damage_stats.time_range = Some((now, now));
            }
        }

        // 更新技能使用统计
        let skill_key = skill_id;
        if !self.skill_usage.contains_key(&skill_key) {
            self.skill_usage.insert(skill_key, SkillStats {
                skill_id,
                display_name: skill_id.to_string(), // 暂时使用skill_id作为名称
                skill_type: "damage".to_string(),
                element,
                total_damage: 0,
                total_count: 0,
                crit_count: 0,
                lucky_count: 0,
                crit_rate: 0.0,
                lucky_rate: 0.0,
                damage_breakdown: DamageBreakdown::default(),
                count_breakdown: CountBreakdown::default(),
            });
        }

        if let Some(skill_stat) = self.skill_usage.get_mut(&skill_key) {
            skill_stat.total_damage += damage;
            skill_stat.total_count += 1;
            if is_crit {
                skill_stat.crit_count += 1;
            }
            if is_cause_lucky {
                skill_stat.lucky_count += 1;
            }

            // 更新技能伤害细分
            if is_crit && is_cause_lucky {
                skill_stat.damage_breakdown.crit_lucky += damage;
            } else if is_crit {
                skill_stat.damage_breakdown.critical += damage;
            } else if is_cause_lucky {
                skill_stat.damage_breakdown.lucky += damage;
            } else {
                skill_stat.damage_breakdown.normal += damage;
            }
            skill_stat.damage_breakdown.total += damage;

            // 更新技能次数细分
            if is_crit {
                skill_stat.count_breakdown.critical += 1;
            }
            if is_cause_lucky {
                skill_stat.count_breakdown.lucky += 1;
            }
            if !is_crit && !is_cause_lucky {
                skill_stat.count_breakdown.normal += 1;
            }
            skill_stat.count_breakdown.total += 1;

            // 更新命中率
            skill_stat.crit_rate = if skill_stat.total_count > 0 {
                skill_stat.crit_count as f64 / skill_stat.total_count as f64
            } else {
                0.0
            };
            skill_stat.lucky_rate = if skill_stat.total_count > 0 {
                skill_stat.lucky_count as f64 / skill_stat.total_count as f64
            } else {
                0.0
            };
        }

        self.last_update = now;
    }

    pub fn add_healing(&mut self, skill_id: u32, element: String, healing: u64, is_crit: bool, is_lucky: bool, is_cause_lucky: bool) {
        let now = Utc::now();
        let skill_key = skill_id + 1000000000; // 区分治疗技能

        // 更新总体治疗统计
        if is_crit && is_lucky {
            self.healing_stats.crit_lucky_healing += healing;
        } else if is_crit {
            self.healing_stats.critical_healing += healing;
        } else if is_lucky {
            self.healing_stats.lucky_healing += healing;
        } else {
            self.healing_stats.normal_healing += healing;
        }
        self.healing_stats.total_healing += healing;

        // 更新次数统计
        if is_crit {
            self.healing_stats.critical_count += 1;
        }
        if is_lucky {
            self.healing_stats.lucky_count += 1;
        }
        if !is_crit && !is_lucky {
            self.healing_stats.normal_count += 1;
        }
        self.healing_stats.total_count += 1;

        // 更新时间范围
        match self.healing_stats.time_range {
            Some((start, _)) => {
                self.healing_stats.time_range = Some((start, now));
            }
            None => {
                self.healing_stats.time_range = Some((now, now));
            }
        }

        // 更新技能使用统计
        if !self.skill_usage.contains_key(&skill_key) {
            self.skill_usage.insert(skill_key, SkillStats {
                skill_id,
                display_name: skill_id.to_string(),
                skill_type: "healing".to_string(),
                element,
                total_damage: 0,
                total_count: 0,
                crit_count: 0,
                lucky_count: 0,
                crit_rate: 0.0,
                lucky_rate: 0.0,
                damage_breakdown: DamageBreakdown::default(),
                count_breakdown: CountBreakdown::default(),
            });
        }

        if let Some(skill_stat) = self.skill_usage.get_mut(&skill_key) {
            skill_stat.total_damage += healing;
            skill_stat.total_count += 1;
            if is_crit {
                skill_stat.crit_count += 1;
            }
            if is_cause_lucky {
                skill_stat.lucky_count += 1;
            }

            // 更新技能治疗细分（复用damage_breakdown字段）
            if is_crit && is_cause_lucky {
                skill_stat.damage_breakdown.crit_lucky += healing;
            } else if is_crit {
                skill_stat.damage_breakdown.critical += healing;
            } else if is_cause_lucky {
                skill_stat.damage_breakdown.lucky += healing;
            } else {
                skill_stat.damage_breakdown.normal += healing;
            }
            skill_stat.damage_breakdown.total += healing;

            // 更新技能次数细分
            if is_crit {
                skill_stat.count_breakdown.critical += 1;
            }
            if is_cause_lucky {
                skill_stat.count_breakdown.lucky += 1;
            }
            if !is_crit && !is_cause_lucky {
                skill_stat.count_breakdown.normal += 1;
            }
            skill_stat.count_breakdown.total += 1;

            // 更新命中率
            skill_stat.crit_rate = if skill_stat.total_count > 0 {
                skill_stat.crit_count as f64 / skill_stat.total_count as f64
            } else {
                0.0
            };
            skill_stat.lucky_rate = if skill_stat.total_count > 0 {
                skill_stat.lucky_count as f64 / skill_stat.total_count as f64
            } else {
                0.0
            };
        }

        self.last_update = now;
    }

    pub fn add_taken_damage(&mut self, damage: u32, is_dead: bool) {
        self.taken_damage += damage as u32;
        if is_dead {
            self.dead_count += 1;
        }
    }

    pub fn update_dps(&mut self) {
        if let Some((start, end)) = self.damage_stats.time_range {
            let duration_ms = (end - start).num_milliseconds() as f64;
            if duration_ms > 0.0 {
                let dps = (self.damage_stats.total_damage as f64 / duration_ms) * 1000.0;
                if !dps.is_finite() {
                    return;
                }
                self.damage_stats.dps = dps;
                if dps > self.damage_stats.dps_max {
                    self.damage_stats.dps_max = dps;
                }
            }
        }
    }

    pub fn update_hps(&mut self) {
        if let Some((start, end)) = self.healing_stats.time_range {
            let duration_ms = (end - start).num_milliseconds() as f64;
            if duration_ms > 0.0 {
                let hps = (self.healing_stats.total_healing as f64 / duration_ms) * 1000.0;
                if !hps.is_finite() {
                    return;
                }
                self.healing_stats.hps = hps;
                if hps > self.healing_stats.hps_max {
                    self.healing_stats.hps_max = hps;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.damage_stats = DamageStats::default();
        self.healing_stats = HealingStats::default();
        self.taken_damage = 0;
        self.skill_usage.clear();
        self.fight_point = 0;
        self.last_update = Utc::now();
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_profession(&mut self, profession: String) {
        if profession != self.profession {
            self.sub_profession.clear();
        }
        self.profession = profession;
    }

    pub fn set_sub_profession(&mut self, sub_profession: String) {
        self.sub_profession = sub_profession;
    }

    pub fn set_fight_point(&mut self, fight_point: u32) {
        self.fight_point = fight_point;
    }

    pub fn set_attr(&mut self, key: &str, value: u32) {
        match key {
            "hp" => self.hp = value,
            "max_hp" => self.max_hp = value,
            "level" => self.level = value,
            _ => {}
        }
    }
}

impl Default for DamageBreakdown {
    fn default() -> Self {
        Self {
            normal: 0,
            critical: 0,
            lucky: 0,
            crit_lucky: 0,
            total: 0,
        }
    }
}

impl Default for CountBreakdown {
    fn default() -> Self {
        Self {
            normal: 0,
            critical: 0,
            lucky: 0,
            total: 0,
        }
    }
}
