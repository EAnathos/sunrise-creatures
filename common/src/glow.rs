//! Lights around glowing creatures (the fairies' night light, the fireflies' lanterns): coloured light the client
//! draws (`visuals`), seen only, lighting nothing in the world.
//!
//! The game takes at most 8 such lights from all mods together, and only within 32 blocks of the player, so the
//! library hands them out: to the creatures nearest a player. The client's `visuals` hook knows no positions: a mod
//! notes each glowing creature from its server tick (`shine`), the library's `visuals` draws them (single player, as
//! the rest of the series).
use std::time::Instant;

use mod_api::{ColoredLight, ServerHost, VisualsEvent};

use crate::state::EntityMap;

/// A creature's light: its colour (`0xRRGGBB`), reach (blocks) and strength (0 to 1), how high above its feet, and
/// whether it shines at this moment (a firefly's pulse).
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub rgb: u32,
    pub radius: f32,
    pub intensity: f32,
    pub lift: f64,
    pub shown: fn(u32) -> bool,
}

/// The game's budget, for all mods.
const LIGHTS: usize = 8;
/// How far from the player the game draws one.
const REACH: f64 = 32.0;
/// A light not noted again for this long is dropped (its creature unloaded, say).
const KEPT_SECONDS: u64 = 2;

struct Shining {
    at: [f64; 3],
    light: Light,
    /// How far the nearest player was.
    near: f64,
    when: Instant,
}

static SHINING: EntityMap<Shining> = EntityMap::new();

/// From the mod's `entity_tick_post`: creature `entity`, in level `dimension`, shines `light` this tick.
pub fn shine(host: &dyn ServerHost, dimension: &str, entity: u32, light: Light) {
    let Some(at) = host.entity(entity).and_then(|e| e.position(host)) else {
        return;
    };
    let near = crate::mob::nearest_player(host, dimension, at);
    let at = [at[0], at[1] + light.lift, at[2]];
    let when = Instant::now();
    SHINING.locked().insert(entity, Shining { at, light, near, when });
}

/// Creature `entity` no longer shines (it landed, it died).
pub fn dim(entity: u32) {
    SHINING.locked().remove(&entity);
}

/// From the library's `visuals`: the lights of the creatures nearest a player.
pub fn visuals(event: &mut VisualsEvent<'_>) {
    for (at, l) in lit() {
        event.lights.push(ColoredLight::new(at, l.rgb, l.radius, l.intensity));
    }
}

/// The lights to draw now, nearest first, at most `LIGHTS`: copied out of `SHINING`, then asked whether they shine
/// with no lock held (`shown` is a mod's).
fn lit() -> Vec<([f64; 3], Light)> {
    let mut lights: Vec<(u32, f64, [f64; 3], Light)> = {
        let mut shining = SHINING.locked();
        shining.retain(|_, s| s.when.elapsed().as_secs() < KEPT_SECONDS);
        shining
            .iter()
            .filter(|(_, s)| s.near <= REACH)
            .map(|(id, s)| (*id, s.near, s.at, s.light))
            .collect()
    };
    lights.retain(|(id, _, _, l)| (l.shown)(*id));
    lights.sort_by(|a, b| a.1.total_cmp(&b.1));
    lights.into_iter().take(LIGHTS).map(|(_, _, at, l)| (at, l)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `shown` that reads the lights itself, as a mod's may (it would deadlock under `SHINING`'s lock).
    fn reads_the_lights(_: u32) -> bool {
        SHINING.locked().len() < usize::MAX
    }

    fn noted(id: u32, near: f64, shown: fn(u32) -> bool) {
        let light = Light {
            rgb: 0xFFFFFF,
            radius: 4.0,
            intensity: 1.0,
            lift: 0.0,
            shown,
        };
        let s = Shining {
            at: [near, 0.0, 0.0],
            light,
            near,
            when: Instant::now(),
        };
        SHINING.locked().insert(id, s);
    }

    #[test]
    fn the_nearest_shown_lights_are_drawn_and_shown_runs_with_no_lock_held() {
        for id in 0..12 {
            noted(4_000_000 + id, f64::from(id), reads_the_lights);
        }
        noted(4_000_100, 0.5, |_| false);
        noted(4_000_101, REACH + 1.0, |_| true);
        let lit: Vec<f64> = lit()
            .into_iter()
            .filter(|(at, _)| at[0] < 100.0)
            .map(|(at, _)| at[0])
            .collect();
        assert_eq!(lit.len(), LIGHTS, "the game's budget");
        assert!(lit.windows(2).all(|w| w[0] <= w[1]), "nearest first");
        assert!(!lit.contains(&0.5), "not shown at this moment");
        for id in (0..12).chain([100, 101]) {
            dim(4_000_000 + id);
        }
    }
}
