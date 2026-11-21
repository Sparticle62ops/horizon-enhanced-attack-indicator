//! The render integration: the mod's progress drawn where the vanilla
//! attack indicator shows, by replacing the crosshair layer.
//!
//! Upstream rides a redirect inside vanilla `InGameHud.renderCrosshair`
//! (mixin/InGameHudMixin.java): the progress the vanilla crosshair path
//! reads is replaced by `EnhancedAttackIndicator.getProgress`, the vanilla
//! plus marker is suppressed, and the full indicator texture is drawn again
//! at the mixin's TAIL when the chain answers the 2.0F sentinel. The mod
//! API has no redirect; the same effect is the SDK's layer replacement:
//! `gui_layer_pre` cancels the `minecraft:crosshair` layer and the mod's
//! draw fires the layer's own draw, which is the engine's own crosshair
//! sequence (`horizon-client-render/src/hud_layout.rs`, `crosshair_draws`)
//! with the chain's answer substituted for the vanilla scale. Same sprites,
//! same coordinates, same inverting CROSSHAIR blend.

use crate::indicator::FULL;
use mod_api::client::hud_layout::{Blend, GuiDraw};

/// One draw of the crosshair plan, the engine's `GuiDraw::Sprite` shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteDraw {
    pub sprite: &'static str,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// `(spriteW, spriteH, u, v)` for the partial overload, java 1:1.
    pub region: Option<[i32; 4]>,
    /// `RenderPipelines.CROSSHAIR` inverts the pixels under the sprite.
    pub crosshair_pipeline: bool,
}

pub const WHITE: u32 = 0xFFFF_FFFF;

/// The engine's `full_attack_indicator` (`Hud.extractCrosshair`'s full
/// attack indicator): at full strength on a living, alive crosshair pick
/// entity with a weapon slower than `MIN_CROSSHAIR_ATTACK_SPEED` (`delay`
/// is `getCurrentItemAttackStrengthDelay`). Ported verbatim from the
/// engine's hud_layout.rs so the replacement draws vanilla's own decision.
pub fn full_attack_indicator(scale: f32, delay: f32, target: Option<bool>) -> bool {
    scale >= 1.0 && delay > 5.0 && target == Some(true)
}

/// The chain's answer as the crosshair path sees it: the java mixin maps
/// the 2.0F sentinel to `renderFullness` and returns 1.0F to the vanilla
/// path, so a sentinel renders as scale 1.0 with the full texture forced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainAnswer {
    /// The mapped scale (sentinel mapped to 1.0, java: `return 1.0F`).
    pub scale: f32,
    /// java: `renderFullness`, set when the chain answers the 2.0F sentinel.
    pub fullness: bool,
}

/// java: the redirect `setBarProgress`: `getProgress(vanilla)` in, the
/// sentinel mapped to `renderFullness` + 1.0F out.
pub fn chain_answer(progress: f32) -> ChainAnswer {
    if progress >= FULL {
        ChainAnswer { scale: 1.0, fullness: true }
    } else {
        ChainAnswer { scale: progress, fullness: false }
    }
}

/// The replaced crosshair layer's draw, the engine's `crosshair_draws`
/// sequence verbatim with `attack` = the mapped chain answer: the crosshair
/// sprite always; at scale < 1 the background bar and the progress region
/// (`(scale * 17.0) as i32` of 16); at full the full indicator texture,
/// where the full decision is the vanilla one OR the mixin's sentinel
/// fullness. `delay` and `target` are the snapshot inputs of
/// [`full_attack_indicator`].
pub fn crosshair_plan(
    w: i32,
    h: i32,
    answer: ChainAnswer,
    delay: f32,
    target: Option<bool>,
) -> Vec<SpriteDraw> {
    let mut out = vec![SpriteDraw {
        sprite: "hud/crosshair",
        x: (w - 15) / 2,
        y: (h - 15) / 2,
        w: 15,
        h: 15,
        region: None,
        crosshair_pipeline: true,
    }];
    let (x, y) = (w / 2 - 8, h / 2 - 7 + 16);
    let full = answer.fullness || full_attack_indicator(answer.scale, delay, target);
    if full {
        out.push(SpriteDraw {
            sprite: "hud/crosshair_attack_indicator_full",
            x,
            y,
            w: 16,
            h: 16,
            region: None,
            crosshair_pipeline: true,
        });
    } else if answer.scale < 1.0 {
        let progress = (answer.scale * 17.0) as i32;
        out.push(SpriteDraw {
            sprite: "hud/crosshair_attack_indicator_background",
            x,
            y,
            w: 16,
            h: 4,
            region: None,
            crosshair_pipeline: true,
        });
        out.push(SpriteDraw {
            sprite: "hud/crosshair_attack_indicator_progress",
            x,
            y,
            w: progress,
            h: 4,
            region: Some([16, 4, 0, 0]),
            crosshair_pipeline: true,
        });
    }
    out
}

/// java: the hotbar redirect `setHotBarProgress`: the chain's answer for
/// the hotbar's sword-use read, the sentinel mapped to 0.99F so the sword
/// slot shows a nearly-full charge. The hotbar slot's own draw stays the
/// engine's (seams.rs); this is the value it would be fed.
pub fn hotbar_progress(progress: f32) -> f32 {
    if progress >= FULL { 0.99 } else { progress }
}

/// The plan as the engine's own `GuiDraw` draws, what `DrawContext::gui_paint`
/// takes: every sprite through the inverting `CROSSHAIR` pipeline, white.
pub fn to_gui_draws(plan: &[SpriteDraw]) -> Vec<GuiDraw> {
    plan.iter()
        .map(|d| GuiDraw::Sprite {
            sprite: std::borrow::Cow::Borrowed(d.sprite),
            x: d.x,
            y: d.y,
            w: d.w,
            h: d.h,
            region: d.region,
            color: WHITE,
            blend: Blend::Crosshair,
        })
        .collect()
}

/// The crosshair layer's full replacement: the crosshair sprite always, the
/// indicator rides `attack` (the mapped chain answer) only when the vanilla
/// attack-indicator preference is CROSSHAIR (1); OFF (0) and HOTBAR (2)
/// draw no crosshair indicator, java 1:1 of vanilla's own switch.
pub fn crosshair_layer_plan(
    w: i32,
    h: i32,
    attack: Option<ChainAnswer>,
    delay: f32,
    target: Option<bool>,
) -> Vec<GuiDraw> {
    match attack {
        Some(answer) => to_gui_draws(&crosshair_plan(w, h, answer, delay, target)),
        None => to_gui_draws(&crosshair_plan(w, h, ChainAnswer { scale: 1.0, fullness: false }, delay, target)
            .into_iter()
            .take(1)
            .collect::<Vec<_>>()),
    }
}
