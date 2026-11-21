//! java: config/Config.java 1:1 (Minenash's MidnightConfig entries; the
//! entries register as settings with the original's en_us lang labels and
//! the same defaults; labels from
//! assets/enhanced_attack_indicator/lang/en_us.json).

pub const MOD_ID: &str = "enhanced_attack_indicator";

/// java: Config.WeaponCoolDownImportance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponCoolDownImportance {
    First,
    Middle,
    Last,
}

impl WeaponCoolDownImportance {
    /// java: the enum's ordinal order, the lang labels First/Middle/Last.
    pub const NAMES: [&'static str; 3] = ["First", "Middle", "Last"];

    pub fn from_index(index: u32) -> WeaponCoolDownImportance {
        match index {
            0 => WeaponCoolDownImportance::First,
            2 => WeaponCoolDownImportance::Last,
            _ => WeaponCoolDownImportance::Middle,
        }
    }

    pub fn index(self) -> usize {
        match self {
            WeaponCoolDownImportance::First => 0,
            WeaponCoolDownImportance::Middle => 1,
            WeaponCoolDownImportance::Last => 2,
        }
    }
}

/// java: Config (MidnightConfig entries, same defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub weapon_cooldown_importance: WeaponCoolDownImportance,
    pub disable_pickaxes_and_shovels: bool,
    pub disable_axes: bool,
    pub show_block_breaking: bool,
    pub show_range_weapon_draw: bool,
    pub show_item_cooldowns: bool,
    pub show_food_and_potions: bool,
    pub show_sleep: bool,
    pub show_item_container_fullness: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            weapon_cooldown_importance: WeaponCoolDownImportance::Middle,
            disable_pickaxes_and_shovels: true,
            disable_axes: false,
            show_block_breaking: true,
            show_range_weapon_draw: true,
            show_item_cooldowns: true,
            show_food_and_potions: true,
            show_sleep: true,
            show_item_container_fullness: true,
        }
    }
}
