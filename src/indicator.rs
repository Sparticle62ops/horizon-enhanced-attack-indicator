//! java: EnhancedAttackIndicator.java 1:1 (Minenash, MIT). The progress
//! chain the vanilla crosshair and hotbar attack indicator render from,
//! over a world snapshot whose fields carry the exact reads upstream makes.

use crate::config::{Config, WeaponCoolDownImportance};

/// java: the 2.0F sentinel the mixin maps to the full indicator texture.
pub const FULL: f32 = 2.0;

/// One container entry: java `item.getCount()` / `item.getMaxCount()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerItem {
    pub count: i32,
    pub max_count: i32,
}

/// A held stack view with exactly what the chain reads off it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HandItem {
    /// java: `item.getTranslationKey().contains("pickaxe"/"shovel"/"axe")`.
    pub translation_key: String,
    /// java: `player.getItemCooldownManager().getCooldownProgress(stack, 0)`.
    pub cooldown_progress: f32,
    /// java: `item == Items.CROSSBOW`.
    pub crossbow: bool,
    /// java: `CrossbowItem.isCharged(stack)`.
    pub crossbow_charged: bool,
    /// java: `stack.get(DataComponentTypes.CONTAINER)`, streamed to a list.
    pub container: Option<Vec<ContainerItem>>,
    /// java: `stack.get(DataComponentTypes.BUNDLE_CONTENTS)`, iterated counts summed.
    pub bundle_total: Option<i32>,
}

/// java: the item kinds the ranged-draw branch switches on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveItem {
    Bow,
    Crossbow,
    Trident,
}

/// The world snapshot upstream reads per call. Fields the mod API has no
/// client wires for yet ride the seams (see seams.rs); the glue fills what
/// the host does expose and the defaults hold the vanilla steady state.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// java: the vanilla progress the HUD passes in
    /// (`player.getAttackCooldownProgress(baseTime)`).
    pub weapon_progress: f32,
    /// java: `player.getAttackCooldownProgressPerTick()`.
    pub attack_cooldown_progress_per_tick: f32,
    /// java: `client.targetedEntity instanceof LivingEntity le && le.isAlive()`.
    pub targeted_living_alive: bool,
    /// java: `player.getSleepTimer()`.
    pub sleep_timer: i32,
    /// java: `interactionManager.getBlockBreakingProgress()`.
    pub block_breaking_progress: i32,
    /// java: `player.getActiveItem()`'s item kind.
    pub active_item: Option<ActiveItem>,
    /// java: `stack.getMaxUseTime(player)` of the active stack.
    pub active_max_use_time: i32,
    /// java: `player.getItemUseTime()`.
    pub item_use_time: i32,
    /// java: `player.getItemUseTimeLeft()`.
    pub item_use_time_left: i32,
    /// java: `CrossbowItem.getPullTime(stack, player)` of the active stack.
    pub crossbow_pull_time: i32,
    /// java: the active item holds a FOOD component or is a potion.
    pub active_food_or_potion: bool,
    pub main_hand: HandItem,
    pub off_hand: HandItem,
}

impl Default for Snapshot {
    fn default() -> Self {
        Snapshot {
            weapon_progress: 1.0,
            attack_cooldown_progress_per_tick: 0.0,
            targeted_living_alive: false,
            sleep_timer: 0,
            block_breaking_progress: 0,
            active_item: None,
            active_max_use_time: 0,
            item_use_time: 0,
            item_use_time_left: 0,
            crossbow_pull_time: 1,
            active_food_or_potion: false,
            main_hand: HandItem::default(),
            off_hand: HandItem::default(),
        }
    }
}

/// java: `EnhancedAttackIndicator.getProgress(float)`, the whole priority
/// chain in the original's order with the original's float comparisons.
pub fn get_progress(snapshot: &Snapshot, config: &Config) -> f32 {
    let weapon_should_show = snapshot.weapon_progress < 1.0
        || snapshot.targeted_living_alive
            && snapshot.attack_cooldown_progress_per_tick > 5.0;

    if config.weapon_cooldown_importance == WeaponCoolDownImportance::First
        && weapon_should_show
    {
        return weapon_cooldown(&snapshot.main_hand, config, snapshot.weapon_progress);
    }

    if config.show_sleep {
        let sleep = snapshot.sleep_timer;
        if sleep > 0 && sleep <= 100 {
            return if sleep == 100 { FULL } else { sleep as f32 / 100.0 };
        }
    }

    if config.show_block_breaking {
        let breaking_progress = snapshot.block_breaking_progress;
        if breaking_progress > 0 {
            return breaking_progress as f32 / 10.0;
        }
    }

    if config.show_range_weapon_draw {
        if let Some(item) = snapshot.active_item {
            match item {
                ActiveItem::Bow => {
                    let progress = bow_pull_progress(72000 - snapshot.item_use_time_left);
                    return if progress == 1.0 { FULL } else { progress };
                }
                ActiveItem::Crossbow => {
                    let progress = (snapshot.active_max_use_time - snapshot.item_use_time_left)
                        as f32
                        / snapshot.crossbow_pull_time as f32;
                    return if progress >= 1.0 { FULL } else { progress };
                }
                ActiveItem::Trident => {
                    let progress = (snapshot.active_max_use_time - snapshot.item_use_time_left)
                        as f32
                        / 10.0;
                    return if progress >= 1.0 { FULL } else { progress };
                }
            }
        }
    }

    if config.show_food_and_potions && snapshot.active_food_or_potion {
        let item_cooldown = snapshot.item_use_time as f32 / snapshot.active_max_use_time as f32;
        return if item_cooldown == 0.0 { 1.0 } else { item_cooldown };
    }

    if config.show_item_container_fullness {
        if let Some(items) = &snapshot.main_hand.container {
            let mut total = 0;
            let mut max_total = 64 * (27 - items.len() as i32);
            for item in items {
                total += item.count;
                max_total += item.max_count;
            }
            let result = total as f32 / max_total as f32;
            return if result == 1.0 { FULL } else { result };
        }
        if let Some(total) = snapshot.main_hand.bundle_total {
            return total as f32 / 64.0;
        }
    }

    if config.weapon_cooldown_importance == WeaponCoolDownImportance::Middle
        && weapon_should_show
    {
        return weapon_cooldown(&snapshot.main_hand, config, snapshot.weapon_progress);
    }

    if config.show_item_cooldowns {
        let cooldown = snapshot.off_hand.cooldown_progress;
        if cooldown != 0.0 {
            return cooldown;
        }
        let cooldown = snapshot.main_hand.cooldown_progress;
        if cooldown != 0.0 {
            return cooldown;
        }
    }

    if config.show_range_weapon_draw
        && (snapshot.main_hand.crossbow && snapshot.main_hand.crossbow_charged
            || snapshot.off_hand.crossbow && snapshot.main_hand.crossbow_charged)
    {
        return FULL;
    }

    if config.weapon_cooldown_importance == WeaponCoolDownImportance::Last
        && weapon_should_show
    {
        return weapon_cooldown(&snapshot.main_hand, config, snapshot.weapon_progress);
    }

    1.0
}

/// java: `weaponCooldown(Item, float)`, translation-key matching verbatim.
pub fn weapon_cooldown(item: &HandItem, config: &Config, weapon_progress: f32) -> f32 {
    if config.disable_pickaxes_and_shovels
        && (item.translation_key.contains("pickaxe") || item.translation_key.contains("shovel"))
    {
        return 1.0;
    }
    if config.disable_axes
        && item.translation_key.contains("axe")
        && !item.translation_key.contains("pickaxe")
    {
        return 1.0;
    }
    if weapon_progress == 1.0 { FULL } else { weapon_progress }
}

/// java: `BowItem.getPullProgress(int)`, the vanilla helper the mod calls.
pub fn bow_pull_progress(use_ticks: i32) -> f32 {
    let g = use_ticks as f32 / 20.0;
    let f = (g * g + g * 2.0) / 3.0;
    if f > 1.0 { 1.0 } else { f }
}
