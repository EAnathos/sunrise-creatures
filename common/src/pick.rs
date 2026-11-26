//! Where the crosshair finds a creature (common/SPECS.md §3.4): its own outline, turned with it, for a creature whose
//! parts reach out of its box (the Magma Golem's arms). A client knows only its template's box, scaled
//! (sunrise-org/sunrise#1134, `mob::scale_client_box`), and even its own box is the same width all round: an outline
//! given here (`outline`) is tried along the player's look as the client picks (`client_pick`), and the creature is
//! picked where the outline is met first, if nothing nearer stands in the way. The server checks the blow against its
//! box with vanilla's 3 blocks of leeway, which an outline up to that far out of it passes.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{ClientHost, ClientPickEvent, CrosshairTarget};

use crate::sync::Locked;

/// A creature's outline (blocks): how far it reaches to each side of its middle, before and behind it, and how high.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub side: f64,
    pub depth: f64,
    pub height: f64,
}

impl Outline {
    /// Its neutral values, for a mod's literal to end with (`..Outline::DEFAULT`).
    pub const DEFAULT: Outline = Outline {
        side: 0.5,
        depth: 0.5,
        height: 1.0,
    };
}

/// A player's reach for entities when the game doesn't say (`minecraft:entity_interaction_range`'s default).
const REACH: f64 = 3.0;

static OUTLINES: Mutex<BTreeMap<&'static str, Outline>> = Mutex::new(BTreeMap::new());

/// From the mod's `register`: creatures of type `kind` are picked by `outline`.
pub fn outline(kind: &'static str, outline: Outline) {
    OUTLINES.locked().insert(kind, outline);
}

/// From the library's `client_pick`: an outlined creature met along the look before what the game found is picked.
pub fn pick(host: &dyn ClientHost, event: &mut ClientPickEvent<'_>) {
    let outlines = OUTLINES.locked().clone();
    if outlines.is_empty() {
        return;
    }
    let Some(camera) = host.camera_entity() else {
        return;
    };
    let (Some(look), Some(level)) = (camera.view_vector(host), camera.level(host)) else {
        return;
    };
    let origin = event.origin;
    let reach = host
        .living(camera)
        .and_then(|l| l.attribute_value(host, "minecraft:entity_interaction_range"))
        .unwrap_or(REACH);
    // What the game found, and how far: only nearer is better.
    let mut best = match *event.result {
        Some(CrosshairTarget::Block { hit, .. }) => distance(origin, hit.point).min(reach),
        Some(CrosshairTarget::Entity { point, .. }) => distance(origin, point).min(reach),
        _ => reach,
    };
    let mut picked = None;
    let around = reach + 4.0;
    let from = origin.map(|v| v - around);
    let to = origin.map(|v| v + around);
    for e in level.entities_in(host, from, to).unwrap_or_default() {
        let Some(outline) = e.kind(host).and_then(|k| outlines.get(k.id(host).as_str()).copied()) else {
            continue;
        };
        let (Some(at), Some(yaw)) = (e.position(host), e.body_yaw(host)) else {
            continue;
        };
        // A young one's outline is as much smaller as its box (its `minecraft:scale` over its client box's).
        let kind = e.kind(host).map(|k| k.id(host)).unwrap_or_default();
        let base = f64::from(crate::mob::client_box_scale(&kind));
        let scale = host
            .living(e)
            .and_then(|l| l.attribute_value(host, "minecraft:scale"))
            .map_or(1.0, |s| s / base);
        let outline = Outline {
            side: outline.side * scale,
            depth: outline.depth * scale,
            height: outline.height * scale,
        };
        if let Some(t) = enter(origin, look, at, yaw, outline)
            && t < best
        {
            best = t;
            picked = Some(e);
        }
    }
    if let Some(entity) = picked {
        let point = [0, 1, 2].map(|k| origin[k] + look[k] * best);
        *event.result = Some(CrosshairTarget::Entity { entity, point });
    }
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// How far along `look` (a unit vector) from `origin` the ray enters `outline`, standing at `at` turned to `yaw`
/// (degrees, the game's: facing `(-sin, cos)`), if it does ahead.
fn enter(origin: [f64; 3], look: [f64; 3], at: [f64; 3], yaw: f32, outline: Outline) -> Option<f64> {
    let (sin, cos) = f64::from(yaw).to_radians().sin_cos();
    // Into its own frame: across its sides, up, ahead of it.
    let local = |v: [f64; 3]| [v[0] * cos + v[2] * sin, v[1], -v[0] * sin + v[2] * cos];
    let o = local([origin[0] - at[0], origin[1] - at[1], origin[2] - at[2]]);
    let d = local(look);
    let low = [-outline.side, 0.0, -outline.depth];
    let high = [outline.side, outline.height, outline.depth];
    let (mut near, mut far) = (0.0f64, f64::MAX);
    for k in 0..3 {
        if d[k].abs() < 1e-9 {
            if o[k] < low[k] || o[k] > high[k] {
                return None;
            }
            continue;
        }
        let (a, b) = ((low[k] - o[k]) / d[k], (high[k] - o[k]) / d[k]);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far).then_some(near)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDE: Outline = Outline {
        side: 2.0,
        depth: 1.0,
        height: 4.0,
    };

    #[test]
    fn its_arms_are_met_to_its_sides_not_before_it() {
        // It faces +z (yaw 0); looking along -z at 1.8 blocks to its side: its arm.
        let t = enter([1.8, 1.0, 5.0], [0.0, 0.0, -1.0], [0.0; 3], 0.0, WIDE);
        assert!((t.unwrap() - 4.0).abs() < 1e-9);
        // Turned a quarter (facing -x), the same look passes 1.8 blocks before its chest, wider than its depth.
        assert_eq!(enter([1.8, 1.0, 5.0], [0.0, 0.0, -1.0], [0.0; 3], 90.0, WIDE), None);
    }

    #[test]
    fn what_is_behind_the_eye_is_not_picked() {
        assert_eq!(enter([0.0, 1.0, 5.0], [0.0, 0.0, 1.0], [0.0; 3], 0.0, WIDE), None);
    }
}
