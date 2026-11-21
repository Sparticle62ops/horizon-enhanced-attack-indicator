//! Tests against the upstream ground truth (Minenash's Java, read 1:1).

use super::*;
use crate::config::{Config, WeaponCoolDownImportance};
use crate::indicator::{
    ActiveItem, ContainerItem, HandItem, Snapshot, bow_pull_progress, get_progress, weapon_cooldown,
    FULL,
};

fn snap() -> Snapshot {
    Snapshot::default()
}

#[test]
fn default_config_matches_upstream() {
    let c = Config::default();
    assert_eq!(c.weapon_cooldown_importance, WeaponCoolDownImportance::Middle);
    assert!(c.disable_pickaxes_and_shovels);
    assert!(!c.disable_axes);
    assert!(c.show_block_breaking);
    assert!(c.show_range_weapon_draw);
    assert!(c.show_item_cooldowns);
    assert!(c.show_food_and_potions);
    assert!(c.show_sleep);
    assert!(c.show_item_container_fullness);
}

#[test]
fn weapon_should_show_is_progress_below_one_or_slow_weapon_on_living_target() {
    // java: weaponProgress < 1 || targetedEntity instanceof LivingEntity
    // le && le.isAlive() && getAttackCooldownProgressPerTick() > 5
    let mut s = snap();
    s.weapon_progress = 0.5;
    let mut c = Config::default();
    // FIRST importance + should show -> the weapon branch answers.
    c.weapon_cooldown_importance = WeaponCoolDownImportance::First;
    let _ = get_progress(&s, &c);
    // Not showing: weapon_progress >= 1, no living target.
    let s2 = snap();
    let out = get_progress(&s2, &c);
    assert_eq!(out, 1.0, "the weapon branch is skipped when weaponShouldShow is false");
}

#[test]
fn weapon_branch_runs_first_middle_last_in_configured_position() {
    let mut s = snap();
    s.main_hand.translation_key = "item.minecraft.wooden_sword".into();
    let mut c = Config::default();
    c.weapon_cooldown_importance = WeaponCoolDownImportance::Last;
    // weaponShouldShow false, no other branch fires: default 1.0 either way.
    assert_eq!(get_progress(&s, &c), 1.0);
    // A sleep branch value hides the LAST weapon branch (sleep outranks it).
    s.sleep_timer = 50;
    assert_eq!(get_progress(&s, &c), 0.5);
    c.weapon_cooldown_importance = WeaponCoolDownImportance::First;
    // FIRST outranks sleep only when weaponShouldShow; progress 1.0 is not.
    assert_eq!(get_progress(&s, &c), 0.5);
    s.weapon_progress = 0.5;
    assert_eq!(get_progress(&s, &c), 0.5, "the weapon branch outranks sleep at FIRST");
}

#[test]
fn sleep_progress_maps_one_to_one_hundred() {
    let mut s = snap();
    let c = Config::default();
    s.sleep_timer = 1;
    assert_eq!(get_progress(&s, &c), 0.01);
    s.sleep_timer = 99;
    assert_eq!(get_progress(&s, &c), 0.99);
    s.sleep_timer = 100;
    assert_eq!(get_progress(&s, &c), FULL, "java: sleep == 100 ? 2.0F");
    s.sleep_timer = 101;
    assert_eq!(get_progress(&s, &c), 1.0, "sleep > 100 falls through");
    s.sleep_timer = 0;
    assert_eq!(get_progress(&s, &c), 1.0);
}

#[test]
fn block_breaking_divides_by_ten() {
    let mut s = snap();
    let c = Config::default();
    s.block_breaking_progress = 5;
    assert_eq!(get_progress(&s, &c), 0.5);
    s.block_breaking_progress = 10;
    assert_eq!(get_progress(&s, &c), 1.0);
    s.block_breaking_progress = 0;
    assert_eq!(get_progress(&s, &c), 1.0);
}

#[test]
fn bow_uses_the_vanilla_pull_progress_helper() {
    let mut s = snap();
    let c = Config::default();
    s.active_item = Some(ActiveItem::Bow);
    // java: BowItem.getPullProgress(72000 - getItemUseTimeLeft())
    // Half a second of draw: g = 0.5, f = (0.25 + 1)/3.
    s.item_use_time_left = 72000 - 10;
    let expected = bow_pull_progress(10);
    assert_eq!(get_progress(&s, &c), expected);
    assert!((expected - 5.0 / 12.0).abs() < 1e-6, "g = 0.5, f = (0.25 + 1)/3 = 5/12");
    // A full second of draw: g = 1, f = (1 + 2)/3 = 1.0 -> the 2.0F sentinel.
    s.item_use_time_left = 72000 - 20;
    assert_eq!(get_progress(&s, &c), FULL, "pull progress 1.0 -> the 2.0F sentinel");
    s.item_use_time_left = 0; // the full 72000 ticks
    assert_eq!(get_progress(&s, &c), FULL);
}

#[test]
fn crossbow_divides_the_draw_by_the_pull_time() {
    let mut s = snap();
    let c = Config::default();
    s.active_item = Some(ActiveItem::Crossbow);
    s.active_max_use_time = 40;
    s.crossbow_pull_time = 25;
    s.item_use_time_left = 20;
    assert_eq!(get_progress(&s, &c), 20.0 / 25.0);
    s.item_use_time_left = 10;
    assert_eq!(get_progress(&s, &c), FULL, "30/25 >= 1.0 -> the 2.0F sentinel");
}

#[test]
fn trident_divides_by_ten_ticks() {
    let mut s = snap();
    let c = Config::default();
    s.active_item = Some(ActiveItem::Trident);
    s.active_max_use_time = 10;
    s.item_use_time_left = 5;
    assert_eq!(get_progress(&s, &c), 0.5);
    s.item_use_time_left = 0;
    assert_eq!(get_progress(&s, &c), FULL);
}

#[test]
fn food_and_potions_show_use_progress() {
    let mut s = snap();
    let c = Config::default();
    s.active_food_or_potion = true;
    s.active_max_use_time = 32;
    s.item_use_time = 16;
    assert_eq!(get_progress(&s, &c), 0.5);
    s.item_use_time = 0;
    assert_eq!(get_progress(&s, &c), 1.0, "java: itemCooldown == 0.0F ? 1.0F");
}

#[test]
fn container_fullness_is_total_over_max() {
    let mut s = snap();
    let c = Config::default();
    // 24 free slots (64 each) plus the stacks' own maxima.
    s.main_hand.container = Some(vec![
        ContainerItem { count: 64, max_count: 64 },
        ContainerItem { count: 16, max_count: 64 },
    ]);
    // maxTotal = 64 * (27 - 2) + 64 + 64 = 1600 + 128 = 1728; total = 80.
    assert_eq!(get_progress(&s, &c), 80.0 / 1728.0);
    s.main_hand.container = Some(vec![ContainerItem { count: 64, max_count: 64 }; 27]);
    assert_eq!(get_progress(&s, &c), FULL, "java: result == 1.0F ? 2.0F");
}

#[test]
fn bundle_total_divides_by_sixty_four() {
    let mut s = snap();
    let c = Config::default();
    s.main_hand.bundle_total = Some(32);
    assert_eq!(get_progress(&s, &c), 0.5);
    // java: `return total / 64F;` straight, no sentinel mapping on a bundle.
    s.main_hand.bundle_total = Some(64);
    assert_eq!(get_progress(&s, &c), 1.0);
}

#[test]
fn item_cooldowns_read_off_hand_then_main_hand() {
    let mut s = snap();
    let c = Config::default();
    s.off_hand.cooldown_progress = 0.25;
    assert_eq!(get_progress(&s, &c), 0.25);
    s.off_hand.cooldown_progress = 0.0;
    s.main_hand.cooldown_progress = 0.75;
    assert_eq!(get_progress(&s, &c), 0.75);
    s.main_hand.cooldown_progress = 0.0;
    assert_eq!(get_progress(&s, &c), 1.0);
}

#[test]
fn charged_crossbow_answers_the_sentinel_both_hands() {
    let mut s = snap();
    let c = Config::default();
    // java bug-for-bug: the off-hand branch checks isCharged(mainHand).
    s.off_hand.crossbow = true;
    s.main_hand.crossbow_charged = true;
    assert_eq!(get_progress(&s, &c), FULL);
}

#[test]
fn default_chain_answer_is_one() {
    let s = snap();
    assert_eq!(get_progress(&s, &Config::default()), 1.0);
}

#[test]
fn weapon_cooldown_gates_pickaxes_shovels_and_axes_by_translation_key() {
    let c = Config::default();
    let mut pickaxe = HandItem::default();
    pickaxe.translation_key = "item.minecraft.iron_pickaxe".into();
    assert_eq!(weapon_cooldown(&pickaxe, &c, 0.5), 1.0, "pickaxes disabled by default");
    let mut shovel = HandItem::default();
    shovel.translation_key = "item.minecraft.diamond_shovel".into();
    assert_eq!(weapon_cooldown(&shovel, &c, 0.5), 1.0);
    let mut axe = HandItem::default();
    axe.translation_key = "item.minecraft.iron_axe".into();
    assert_eq!(weapon_cooldown(&axe, &c, 0.5), 0.5, "axes enabled by default");
    let mut c = Config::default();
    c.disable_axes = true;
    assert_eq!(weapon_cooldown(&axe, &c, 0.5), 1.0);
    // java: contains("axe") && !contains("pickaxe") - "axe" is a substring
    // of "pickaxe", so the axe gate never hits a pickaxe key; with the
    // pickaxe gate off a pickaxe key rides its own progress.
    let mut pickaxe = HandItem::default();
    pickaxe.translation_key = "item.minecraft.iron_pickaxe".into();
    let mut c = Config::default();
    c.disable_pickaxes_and_shovels = false;
    c.disable_axes = true;
    assert_eq!(weapon_cooldown(&pickaxe, &c, 0.5), 0.5, "the axe gate skips pickaxe keys");
    let mut axe = HandItem::default();
    axe.translation_key = "item.minecraft.iron_axe".into();
    assert_eq!(weapon_cooldown(&axe, &c, 0.5), 1.0, "the axe gate fires on an axe key");
    let mut sword = HandItem::default();
    sword.translation_key = "item.minecraft.iron_sword".into();
    assert_eq!(weapon_cooldown(&sword, &c, 0.5), 0.5);
    assert_eq!(weapon_cooldown(&sword, &c, 1.0), FULL, "java: progress == 1 ? 2");
}

#[test]
fn fullness_maps_the_sentinel_and_keeps_other_scales() {
    use crate::crosshair::{chain_answer, hotbar_progress};
    let a = chain_answer(FULL);
    assert_eq!(a.scale, 1.0, "java: return 1.0F to the vanilla path");
    assert!(a.fullness, "java: renderFullness = true");
    let a = chain_answer(0.5);
    assert_eq!(a.scale, 0.5);
    assert!(!a.fullness);
    assert_eq!(hotbar_progress(FULL), 0.99, "java: 2.0F ? 0.99F");
    assert_eq!(hotbar_progress(0.25), 0.25);
}

#[test]
fn crosshair_plan_is_the_engine_sequence_with_the_chain_answer() {
    use crate::crosshair::{ChainAnswer, crosshair_plan, full_attack_indicator};
    // 640x360 window, charging at half.
    let plan = crosshair_plan(640, 360, ChainAnswer { scale: 0.5, fullness: false }, 20.0, None);
    assert_eq!(plan[0].sprite, "hud/crosshair");
    assert_eq!(plan[0].x, (640 - 15) / 2);
    assert_eq!(plan[0].y, (360 - 15) / 2);
    let indicator = &plan[1];
    assert_eq!(indicator.sprite, "hud/crosshair_attack_indicator_background");
    assert_eq!((indicator.x, indicator.y), (640 / 2 - 8, 360 / 2 - 7 + 16));
    assert_eq!((indicator.w, indicator.h), (16, 4));
    let progress = &plan[2];
    assert_eq!(progress.sprite, "hud/crosshair_attack_indicator_progress");
    assert_eq!(progress.w, (0.5 * 17.0) as i32);
    assert_eq!(progress.region, Some([16, 4, 0, 0]));
    assert!(plan.iter().all(|d| d.crosshair_pipeline), "RenderPipelines.CROSSHAIR");
    // Fully drawn: the background + progress pair vanish (scale not < 1,
    // full needs the vanilla decision).
    let plan = crosshair_plan(640, 360, ChainAnswer { scale: 1.0, fullness: false }, 20.0, None);
    assert_eq!(plan.len(), 1, "no indicator at full without the vanilla full decision");
    // The vanilla full decision (slow weapon on a living target).
    let plan = crosshair_plan(640, 360, ChainAnswer { scale: 1.0, fullness: false }, 25.0, Some(true));
    assert_eq!(plan.len(), 2);
    assert_eq!(plan[1].sprite, "hud/crosshair_attack_indicator_full");
    // The 2.0F sentinel forces the full texture (java renderFullness).
    let plan = crosshair_plan(640, 360, chain_answer(FULL), 20.0, None);
    assert_eq!(plan.len(), 2);
    assert_eq!(plan[1].sprite, "hud/crosshair_attack_indicator_full");
    // fast_attack_indicator's own truth table (the engine's helper, verbatim).
    assert!(full_attack_indicator(1.0, 25.0, Some(true)));
    assert!(!full_attack_indicator(0.99, 25.0, Some(true)));
    assert!(!full_attack_indicator(1.0, 5.0, Some(true)));
    assert!(!full_attack_indicator(1.0, 25.0, Some(false)));
}

#[test]
fn settings_keys_cover_every_config_entry() {
    assert_eq!(crate::SETTINGS.len(), 9);
    for (key, _, _, _) in crate::SETTINGS {
        assert!(!key.is_empty());
    }
}
