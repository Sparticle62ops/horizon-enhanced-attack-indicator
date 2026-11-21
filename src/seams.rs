//! Tracked seams, engine-stage: the pinned mod API (f00125f / e59da909,
//! api 0.2) has no counterpart for these yet.
//!
//! - No progress redirect: upstream replaces the value vanilla's
//!   `renderCrosshair`/`renderHotbar` read from
//!   `player.getAttackCooldownProgress`. The mod API has no hook at that
//!   read; the port replaces the `minecraft:crosshair` layer instead
//!   (`gui_layer_pre` cancel + the mod layer's draw, the SDK's
//!   replaceLayer pattern) and draws the engine's own crosshair sequence
//!   with the chain's answer. The hotbar-mode sword indicator (attack
//!   indicator option 2) stays the engine's draw: replacing that layer
//!   wholesale would redraw every slot, so the redirect's hotbar half
//!   (`2.0F -> 0.99F`) ships as the value only.
//! - The chain's world inputs have no client wires yet: the local attack
//!   strength scale, the sleep timer, the block breaking progress, the
//!   item cooldown progress of both hands and the held stacks' components
//!   (`CONTAINER`, `BUNDLE_CONTENTS`, translation keys) are `Snapshot`
//!   fields the glue fills from the host where it can (the crosshair
//!   target, the item in use, the attack-indicator preference) and holds
//!   at the vanilla steady state where it cannot. The weapon branch runs
//!   off the snapshot's `weapon_progress`; until its wire lands the chain
//!   answers the other branches live and the weapon bar follows the
//!   snapshot.
//! - `dontShowPlus` (upstream redirects `Entity.isAlive` to false so the
//!   vanilla plus marker never draws) has no separate counterpart in the
//!   engine's crosshair sequence: the layer draws the one `hud/crosshair`
//!   sprite, which is the same single texture the engine draws, so the
//!   suppression is implicit.
//! - `weaponShouldShow` reads `player.getAttackCooldownProgressPerTick() >
//!   5` upstream verbatim; the engine's full-indicator decision reads the
//!   weapon's attack-strength delay (`getCurrentItemAttackStrengthDelay >
//!   MIN_CROSSHAIR_ATTACK_SPEED`). Both are snapshot fields ported as
//!   written.
