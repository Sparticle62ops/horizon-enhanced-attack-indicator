//! Exact 1:1 Rust translation of Enhanced Attack Indicator by Minenash
//! (upstream Java, MIT (c) 2021 Minenash,
//! https://github.com/Minenash/Enhanced-Attack-Indicator). Carries the
//! upstream license and credit. Ported for the Horizon client by
//! sparticle.
//!
//! The attack indicator shows more than melee attack cooldowns: the
//! vanilla crosshair/hotbar indicator renders the mod's progress chain
//! instead (sleeping, block breaking, bow/crossbow/trident draw, food and
//! potions, shulker/bundle fullness, item cooldowns, charged crossbow,
//! with the vanilla weapon cooldown at the configured priority). The whole
//! flow rides the mod API: the nine config entries register as settings
//! with the original's en_us lang labels and defaults, and the progress
//! chain replaces the vanilla crosshair layer's attack indicator (the
//! SDK's layer replacement in place of the upstream render-crosshair
//! redirect; see seams.rs for what stays vanilla).

mod config;
mod crosshair;
mod indicator;
mod seams;
#[cfg(test)]
mod tests;

use mod_api::{
    CameraType, ClientHost, ClientSetupEvent, Flow, GuiLayer, GuiLayerPlacement,
    GuiLayerPreEvent, Registrar, RegistrationContext, RegisterGuiLayersEvent,
    SettingChangedEvent, SettingSpec,
};

pub use crate::config::{Config, WeaponCoolDownImportance, MOD_ID};
pub use crate::crosshair::{
    ChainAnswer, SpriteDraw, chain_answer, crosshair_layer_plan, crosshair_plan,
    full_attack_indicator, hotbar_progress, to_gui_draws, WHITE,
};
pub use crate::indicator::{
    ActiveItem, ContainerItem, HandItem, Snapshot, bow_pull_progress, get_progress, weapon_cooldown,
    FULL,
};

/// The nine config entries as settings: (key, label, kind, default), the
/// labels from the original's en_us lang file, the defaults from Config.
pub const SETTINGS: [(&str, &str, SettingKind, SettingDefault); 9] = [
    (
        "weapon_cooldown_importance",
        "Attack Cooldown Priority",
        SettingKind::Choice(&["First", "Middle", "Last"]),
        SettingDefault::Choice(1),
    ),
    (
        "disable_pickaxes_and_shovels",
        "Disable Pickaxe's & Shovel's Attack Cooldown",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
    (
        "disable_axes",
        "Disable Axe's Attack Cooldown",
        SettingKind::Toggle,
        SettingDefault::Toggle(false),
    ),
    (
        "show_block_breaking",
        "Show Block Breaking Progress",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
    (
        "show_range_weapon_draw",
        "Show Ranged Weapon Draw",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
    (
        "show_item_cooldowns",
        "Show Item Cooldowns",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
    (
        "show_food_and_potions",
        "Show Food & Potions",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
    (
        "show_sleep",
        "Show Sleep Progress",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
    (
        "show_item_container_fullness",
        "Show Shulker Fullness",
        SettingKind::Toggle,
        SettingDefault::Toggle(true),
    ),
];

pub enum SettingKind {
    Toggle,
    Choice(&'static [&'static str]),
}

pub enum SettingDefault {
    Toggle(bool),
    Choice(usize),
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// java: Config's MidnightConfig entries as settings, same defaults and
    /// lang labels.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        for (key, label, kind, default) in SETTINGS {
            let _ = match (kind, default) {
                (SettingKind::Toggle, SettingDefault::Toggle(on)) => {
                    registrar.register_setting(SettingSpec::toggle(key, label, on))
                }
                (SettingKind::Choice(options), SettingDefault::Choice(index)) => {
                    registrar.register_setting(SettingSpec::choice(
                        key,
                        label,
                        options.iter().copied(),
                        index,
                    ))
                }
                _ => unreachable!("every setting pairs its kind with its default"),
            };
        }
    }

    /// java: `Config.init(...)`, the initial model from the host's reads.
    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        sync_from_host(host);
    }

    /// The player changed a setting on the page; re-sync the model.
    fn setting_changed(&self, host: &mut dyn ClientHost, _event: &mut SettingChangedEvent) {
        sync_from_host(host);
    }

    /// java: the mixin's redirect expressed as the SDK's layer replacement:
    /// the crosshair layer is cancelled and this mod's own layer, placed
    /// right above it, draws the engine's crosshair sequence with the
    /// chain's answer.
    fn register_gui_layers(
        &self,
        _context: &RegistrationContext,
        event: &mut RegisterGuiLayersEvent<'_>,
    ) {
        event.layers.push(GuiLayer {
            id: LAYER_ID.to_string(),
            placement: GuiLayerPlacement::Above(VANILLA_CROSSHAIR.to_string()),
        });
    }

    /// The replaceLayer cancel: the vanilla crosshair layer (crosshair and
    /// its attack indicator) is hidden this frame; the mod's layer draws it.
    fn gui_layer_pre(&self, _host: &dyn ClientHost, event: &mut GuiLayerPreEvent<'_>) -> Flow {
        if event.layer == VANILLA_CROSSHAIR {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// The replaced draw: the engine's crosshair sequence with the chain's
    /// answer substituted for the vanilla scale, in the vanilla attack
    /// indicator's CROSSHAIR mode only (OFF and HOTBAR draw no crosshair
    /// indicator), first person only, as vanilla's own pass does.
    fn gui_layer_draw(&self, host: &dyn ClientHost, event: &mut mod_api::GuiLayerDrawEvent<'_>) {
        if event.layer != LAYER_ID {
            return;
        }
        if !matches!(host.camera_type(), CameraType::FirstPerson) {
            return;
        }
        let config = config_from_host(host);
        let snapshot = glue_snapshot(host);
        let progress = get_progress(&snapshot, &config);
        let answer = chain_answer(progress);
        let attack = if host.gui_options().attack_indicator == 1 {
            Some(answer)
        } else {
            None
        };
        let [w, h] = event.draw.size();
        let plan = crosshair_layer_plan(
            w as i32,
            h as i32,
            attack,
            DEFAULT_INDICATOR_DELAY,
            Some(snapshot.targeted_living_alive),
        );
        let _ = event.draw.gui_paint(&plan);
    }
}

/// java/glue: the config the host's typed reads give.
pub fn config_from_host(host: &dyn ClientHost) -> Config {
    let mut config = Config::default();
    if let Some(index) = host.setting_choice("weapon_cooldown_importance") {
        config.weapon_cooldown_importance = WeaponCoolDownImportance::from_index(index);
    }
    for (key, _, kind, _) in SETTINGS {
        if matches!(kind, SettingKind::Toggle) {
            if let Some(on) = host.setting_bool(key) {
                set_toggle(&mut config, key, on);
            }
        }
    }
    config
}

fn set_toggle(config: &mut Config, key: &str, on: bool) {
    match key {
        "disable_pickaxes_and_shovels" => config.disable_pickaxes_and_shovels = on,
        "disable_axes" => config.disable_axes = on,
        "show_block_breaking" => config.show_block_breaking = on,
        "show_range_weapon_draw" => config.show_range_weapon_draw = on,
        "show_item_cooldowns" => config.show_item_cooldowns = on,
        "show_food_and_potions" => config.show_food_and_potions = on,
        "show_sleep" => config.show_sleep = on,
        "show_item_container_fullness" => config.show_item_container_fullness = on,
        _ => {}
    }
}

/// java/glue: the initial model sync from the host's setting reads.
fn sync_from_host(host: &dyn ClientHost) {
    let _ = config_from_host(host);
}

/// The replaced layer this mod registers: id `<mod id>:crosshair`, placed
/// right above the vanilla crosshair layer (the SDK's replaceLayer
/// pattern), so it draws in the crosshair layer's place once
/// [`VANILLA_CROSSHAIR`] is cancelled.
pub const LAYER_ID: &str = "enhanced_attack_indicator:crosshair";

/// The vanilla crosshair layer (the crosshair and its attack indicator).
pub const VANILLA_CROSSHAIR: &str = "minecraft:crosshair";

/// java/glue: the world snapshot the client host can fill live; the
/// fields with no host wires yet hold the vanilla steady state (seams.rs).
fn glue_snapshot(host: &dyn ClientHost) -> Snapshot {
    let mut snapshot = Snapshot::default();
    // java: `client.targetedEntity instanceof LivingEntity le && le.isAlive()`
    // - the crosshair read carries the hit but no living/alive flag, so an
    // entity hit stands in for it (seams.rs).
    if let Some(target) = host.crosshair_target() {
        snapshot.targeted_living_alive = matches!(target, mod_api::CrosshairTarget::Entity { .. });
    }
    // java: the item in use, its used ticks and its remaining ticks.
    if let Some(use_) = host.local_item_use() {
        snapshot.item_use_time = use_.ticks_used;
        snapshot.item_use_time_left = use_.ticks_remaining;
        snapshot.active_max_use_time = use_.ticks_used + use_.ticks_remaining;
    }
    snapshot
}

/// The chain's weapon-strength delay for the full-indicator decision
/// (java: `getCurrentItemAttackStrengthDelay`), no client wire yet: the
/// vanilla default attack speed's 20/4 = 5.0, which the vanilla
/// `delay > 5.0` check reads as not slower than minimum (seams.rs).
pub const DEFAULT_INDICATOR_DELAY: f32 = 5.0;
