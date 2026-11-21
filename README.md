# Horizon Enhanced Attack Indicator

An exact 1:1 Rust translation of [Enhanced Attack Indicator](https://github.com/Minenash/Enhanced-Attack-Indicator)
by Minenash (upstream Java, MIT (c) 2021 Minenash). Carries the upstream
license and credit. Ported for the Horizon client by sparticle.

The attack indicator shows more than melee attack cooldowns: the vanilla
crosshair/hotbar indicator renders the mod's progress chain instead —
sleeping, block breaking, bow/crossbow/trident draw, food and potions,
shulker/bundle fullness, item cooldowns, charged crossbow, with the vanilla
weapon cooldown at the configured priority (First/Middle/Last).

## Build

```
cargo test --release   # 18 tests, green
```

Pinned to Horizon `f00125f61f794aaa3d4e950d301f353b35b6ee3d` and
horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, mod api 0.2.

## How the render rides the mod API

Upstream rides a redirect inside vanilla `InGameHud.renderCrosshair` (the
progress the vanilla crosshair path reads is replaced by
`EnhancedAttackIndicator.getProgress`, the plus marker is suppressed, and
the full indicator texture is drawn again at the mixin's TAIL when the
chain answers the 2.0F sentinel). The mod API has no redirect at that
read, so the port uses the SDK's layer replacement instead:
`register_gui_layers` places `enhanced_attack_indicator:crosshair` right
above the vanilla `minecraft:crosshair` layer, `gui_layer_pre` cancels the
vanilla layer, and `gui_layer_draw` draws the engine's own crosshair
sequence (`horizon-client-render/src/hud_layout.rs`, `crosshair_draws` +
`full_attack_indicator`, ported verbatim) with the chain's answer
substituted for the vanilla scale. Same sprites, same coordinates, same
inverting CROSSHAIR pipeline. The indicator rides the vanilla
attack-indicator preference's CROSSHAIR mode only; the hotbar-mode sword
indicator stays the engine's draw (the value the redirect would feed ships
as `hotbar_progress`).

## Known seams (engine-stage; tracked)

- The chain's world inputs have no client wires yet: the local attack
  strength scale, the sleep timer, the block breaking progress, the item
  cooldown progress of both hands and the held stacks' components
  (`CONTAINER`, `BUNDLE_CONTENTS`, translation keys) are `Snapshot` fields
  the glue fills from the host where it can (the crosshair target, the item
  in use, the attack-indicator preference) and holds at the vanilla steady
  state where it cannot. Until those wires land the weapon-cooldown branch
  follows the snapshot rather than the live cooldown.
- The item in use carries no item identity on the host surface, so the
  ranged-draw and food/potion branches stay snapshot-held.
- `dontShowPlus` (upstream redirects `Entity.isAlive` to false) has no
  separate counterpart: the engine's crosshair sequence draws the one
  `hud/crosshair` sprite, so the suppression is implicit.
- `weaponShouldShow` reads `getAttackCooldownProgressPerTick() > 5`
  upstream verbatim; the engine's full-indicator decision reads the
  weapon's attack-strength delay. Both are snapshot fields ported as
  written.

Licence: MIT (LICENSE carries the upstream text).
