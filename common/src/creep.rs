//! Creeping (SPECS §3.16): a small creature crawling by hand over the tops of blocks and up their sides, as a beetle or
//! a stick insect does: the ladybugs between flowers, the phasmids over the canopy and up the trunks. Unlike the small
//! crawlers' goal (`crawl`, walked by the engine's path finding), it moves itself, so it climbs.
//!
//! A mod gives a `Creeping` and adds `Creep::new(&HOW)` to its behaviours. A crawl picks a block top of its area (or a
//! point around where it is, for one with none), creeps straight toward it, and climbs while its target is above: up
//! the block in its way, or, under its target, up any side next to it (a trunk), through leaves as an insect goes
//! through foliage; then over the top. It pauses between crawls; while it pauses it doesn't claim, so the creature's
//! other behaviours (the small fliers' rest and flights) go on.
//!
//! A tree climber (`Creep::climber`: the phasmids) holds on to the side it is on, its belly to the bark, while it pauses or
//! keeps still, rather than dropping; under a target well above it, it first goes to the nearest trunk. Its renderer
//! draws it upright while it clings (`clinging`).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::Host;

use crate::body::{Behaviour, Body, Vec3, World};
use crate::sync::Locked;

/// How a creature creeps.
pub struct Creeping {
    /// Its speed on a top and up a side (blocks a tick); one that doesn't climb (a dung beetle) has 0 up a side and
    /// gives up at a wall.
    pub speed: f64,
    pub climb: f64,
    /// How long a crawl and a pause last (ticks).
    pub crawl: (i32, i32),
    pub pause: (i32, i32),
    /// How far around where it is it creeps when its area is empty (blocks).
    pub wander: f64,
    /// The blocks it may creep onto (their tops), from its mod (`creature::territory`'s area of a settled one, the
    /// leaves of a canopy); empty for anywhere around it. Asked at each crawl's start.
    pub area: fn(&dyn Host, u32) -> Vec<[i32; 3]>,
    /// Whether it may creep now (by day, not playing dead...).
    pub may: fn(&Body, &World<'_>) -> bool,
    /// The sound it now and then makes as it moves (a rustle of leaves), if any: one chance in `RUSTLE_ONE_IN` a tick.
    pub rustle: Option<&'static str>,
}

/// How far from the bark a clinging creature's middle is held (blocks): a small creature's half width.
const CLING_GAP: f64 = 0.3;
/// How far it looks for a trunk to climb (blocks across), and how far above it a target sends it there (blocks).
const TRUNK_SEARCH: i32 = 4;
const WELL_ABOVE: f64 = 1.5;

/// The creatures clinging to a side now, and the side's direction (x, z): for their drawing (single player, as the
/// rest of the series: the behaviours and the renderer share the mob's id).
static CLINGING: Mutex<BTreeMap<u32, [i32; 2]>> = Mutex::new(BTreeMap::new());

/// Whether creature `id` clings to a side now (its renderer draws it upright, belly to the bark).
pub fn clinging(id: u32) -> bool {
    CLINGING.locked().contains_key(&id)
}

/// It no longer clings (it let go, or it is gone).
pub fn let_go(id: u32) {
    CLINGING.locked().remove(&id);
}

/// The side next to `cell` (a block it can't walk through, at its feet), preferring the way it faces.
pub fn side_beside(cx: &World<'_>, cell: [i32; 3], yaw: f32) -> Option<[i32; 2]> {
    side_of(|p| !passable(cx, p), cell, yaw)
}

/// `side_beside`, the blocks' solidity told by `solid`.
fn side_of(solid: impl Fn([i32; 3]) -> bool, cell: [i32; 3], yaw: f32) -> Option<[i32; 2]> {
    let (s, c) = f64::from(yaw).to_radians().sin_cos();
    let facing = [(-s).round() as i32, c.round() as i32];
    std::iter::once(facing)
        .chain([[1, 0], [-1, 0], [0, 1], [0, -1]])
        .filter(|d| d[0].abs() + d[1].abs() == 1)
        .find(|d| solid([cell[0] + d[0], cell[1], cell[2] + d[1]]))
}

/// Holds `m` on the side beside it, if there is one and it isn't on the ground: still, its belly to the bark (its
/// middle `CLING_GAP` from it), facing it. Whether it holds on; one that clung and has no side any more (its bark
/// broken) is let go, to fall.
pub fn hold_on(m: &mut Body, cx: &World<'_>) -> bool {
    hold(m, |p| !passable(cx, p))
}

/// `hold_on`, the blocks' solidity told by `solid`.
fn hold(m: &mut Body, solid: impl Fn([i32; 3]) -> bool) -> bool {
    let cell = [
        m.position.x.floor() as i32,
        (m.position.y + 0.05).floor() as i32,
        m.position.z.floor() as i32,
    ];
    let side = (!m.on_ground).then(|| side_of(solid, cell, m.yaw)).flatten();
    let Some(d) = side else {
        if CLINGING.locked().remove(&m.id).is_some() {
            m.no_gravity = false;
        }
        return false;
    };
    m.no_gravity = true;
    m.velocity = Vec3::default();
    m.fall_distance = 0.0;
    // Against the bark: the face of the side is the cell's edge that way.
    if d[0] != 0 {
        m.position.x = f64::from(cell[0]) + if d[0] > 0 { 1.0 - CLING_GAP } else { CLING_GAP };
    } else {
        m.position.z = f64::from(cell[2]) + if d[1] > 0 { 1.0 - CLING_GAP } else { CLING_GAP };
    }
    let yaw = face(d);
    m.yaw = yaw;
    m.body_yaw = yaw;
    m.head_yaw = yaw;
    CLINGING.locked().insert(m.id, d);
    true
}

/// The yaw facing direction `d` (x, z).
fn face(d: [i32; 2]) -> f32 {
    (-f64::from(d[0])).atan2(f64::from(d[1])).to_degrees() as f32
}

/// The nearest trunk (a log) within `TRUNK_SEARCH` blocks across of `cell`, at its feet or a block up.
fn trunk_near(cx: &World<'_>, cell: [i32; 3]) -> Option<[f64; 3]> {
    trunk_in(|p| cx.block(p), cell)
}

/// `trunk_near`, the blocks named by `block`.
fn trunk_in(block: impl Fn([i32; 3]) -> String, cell: [i32; 3]) -> Option<[f64; 3]> {
    let mut best: Option<(i32, [i32; 3])> = None;
    for dx in -TRUNK_SEARCH..=TRUNK_SEARCH {
        for dz in -TRUNK_SEARCH..=TRUNK_SEARCH {
            for dy in 0..=1 {
                let p = [cell[0] + dx, cell[1] + dy, cell[2] + dz];
                let d = dx * dx + dz * dz;
                if best.is_none_or(|b| d < b.0) && is_trunk(&block(p)) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|(_, p)| [f64::from(p[0]) + 0.5, f64::from(cell[1]), f64::from(p[2]) + 0.5])
}

fn is_trunk(name: &str) -> bool {
    name.ends_with("_log") || name.ends_with("_wood") || name.ends_with("_stem")
}

/// How often a creeping creature's sound plays while it moves (one chance in so many ticks).
const RUSTLE_ONE_IN: i32 = 60;

/// Creeping, as a `Creeping` says.
pub struct Creep {
    how: &'static Creeping,
    target: Option<[f64; 3]>,
    ticks: i32,
    pausing: bool,
    area: Vec<[i32; 3]>,
    climbing: bool,
    /// The trunk it goes to first (a tree climber's, under a target well above it).
    trunk: Option<[f64; 3]>,
    /// A tree climber (`climber`).
    clings: bool,
}

impl std::fmt::Debug for Creep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Creep")
            .field("target", &self.target)
            .field("climbing", &self.climbing)
            .finish()
    }
}

impl Creep {
    pub fn new(how: &'static Creeping) -> Self {
        Self {
            how,
            target: None,
            ticks: 0,
            pausing: true,
            area: Vec::new(),
            climbing: false,
            trunk: None,
            clings: false,
        }
    }

    /// Creeping as `how` says, as a tree climber (the phasmids): it holds on to a side (a trunk) when it stops there,
    /// and goes to a trunk to climb to a target well above it. A constructor rather than a field of `Creeping`, which
    /// the mods build as a literal: a new field there would break the mods already built against the library.
    pub fn climber(how: &'static Creeping) -> Self {
        Self {
            clings: true,
            ..Self::new(how)
        }
    }

    /// Stopped (paused, keeping still, or another behaviour runs): a tree climber on a side holds on there, the others
    /// are let go to the ground.
    fn halt(&mut self, m: &mut Body, cx: &World<'_>) {
        self.trunk = None;
        let held = self.clings && hold_on(m, cx);
        if self.climbing && !held {
            m.no_gravity = false;
        }
        self.climbing = false;
    }

    fn pick(&self, m: &mut Body) -> [f64; 3] {
        if !self.area.is_empty() {
            let b = self.area[m.random.next_int_bound(self.area.len() as i32).unwrap_or(0) as usize];
            return [f64::from(b[0]) + 0.5, f64::from(b[1]) + 1.0, f64::from(b[2]) + 0.5];
        }
        let mut r = || (f64::from(m.random.next_float()) * 2.0 - 1.0) * self.how.wander;
        let (dx, dz) = (r(), r());
        [m.position.x + dx, m.position.y, m.position.z + dz]
    }
}

/// Whether a creature walks through block `name` (air, grass, flowers and the like) rather than climbing it.
pub fn passable(cx: &World<'_>, p: [i32; 3]) -> bool {
    let name = cx.block(p);
    matches!(
        name.as_str(),
        "minecraft:air"
            | "minecraft:cave_air"
            | "minecraft:short_grass"
            | "minecraft:tall_grass"
            | "minecraft:fern"
            | "minecraft:large_fern"
            | "minecraft:dead_bush"
            | "minecraft:vine"
            | "minecraft:snow"
    ) || cx.ground.flower_at(p)
}

/// Whether block `name` is taller than a step a walker climbs (a fence, a wall, a fence gate): a creature that steps
/// up a block turns aside at it.
pub fn too_tall(name: &str) -> bool {
    name.ends_with("fence") || name.ends_with("wall") || name.ends_with("fence_gate")
}

impl Behaviour for Creep {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        if !(self.how.may)(m, cx) {
            self.halt(m, cx);
            return false;
        }
        self.ticks -= 1;
        if self.pausing {
            if self.ticks > 0 {
                return false;
            }
            self.pausing = false;
            self.ticks = m.random.between(self.how.crawl);
            self.area = (self.how.area)(cx.host(), m.id);
            self.target = Some(self.pick(m));
        }
        if self.ticks <= 0 {
            self.pausing = true;
            self.ticks = m.random.between(self.how.pause);
            self.target = None;
            self.halt(m, cx);
            return false;
        }
        true
    }
    fn stop(&mut self, m: &mut Body, cx: &World<'_>) {
        self.halt(m, cx);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        let Some(t) = self.target else {
            return;
        };
        let feet = m.position.y + 0.05;
        let cell = [
            m.position.x.floor() as i32,
            feet.floor() as i32,
            m.position.z.floor() as i32,
        ];
        // A tree climber under a target well above it, with nothing to climb beside it: to the nearest trunk first.
        if self.clings && !self.climbing && t[1] > feet + WELL_ABOVE && side_beside(cx, cell, m.yaw).is_none() {
            self.trunk = self.trunk.or_else(|| trunk_near(cx, cell));
        } else {
            self.trunk = None;
        }
        let way = self.trunk.unwrap_or(t);
        let (dx, dz) = (way[0] - m.position.x, way[2] - m.position.z);
        let flat = dx.hypot(dz);
        if self.trunk.is_none() && flat < 0.15 && (m.position.y - t[1]).abs() < 0.6 {
            // There: a pause.
            self.ticks = 0;
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            return;
        }
        let dir = if flat > 0.001 {
            [dx / flat, dz / flat]
        } else {
            [0.0, 0.0]
        };
        let ahead = [
            (m.position.x + dir[0] * 0.4).floor() as i32,
            cell[1],
            (m.position.z + dir[1] * 0.4).floor() as i32,
        ];
        let wall = !passable(cx, ahead);
        // Its target above it: it climbs the wall in its way, or, under its target, any side next to it (a trunk).
        let above = t[1] > feet + if self.climbing { 0.0 } else { 0.5 };
        let side = flat < 0.8
            && [[1, 0], [-1, 0], [0, 1], [0, -1]]
                .iter()
                .any(|d| !passable(cx, [cell[0] + d[0], cell[1], cell[2] + d[1]]));
        if self.how.climb > 0.0 && above && (wall || side || self.climbing) {
            // Up, through foliage as an insect goes (leaves don't stop it); a solid ceiling ends the crawl.
            let head = [cell[0], (m.position.y + 0.3).floor() as i32 + 1, cell[2]];
            let ceiling = !passable(cx, head) && !cx.block(head).ends_with("leaves");
            if ceiling {
                self.ticks = 0;
                return;
            }
            self.climbing = true;
            self.trunk = None;
            m.position = Vec3 {
                y: m.position.y + self.how.climb,
                ..m.position
            };
            // A tree climber goes up with its belly to the bark (`hold_on` faces it there).
            let held = self.clings && hold_on(m, cx);
            m.no_gravity = true;
            m.velocity = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
            if held {
                self.rustle(m);
                return;
            }
        } else {
            // Over the top of a climb, or off a flower it sat on: walking, it is held by the ground.
            self.climbing = false;
            let_go(m.id);
            m.no_gravity = false;
            m.velocity.x = dir[0] * self.how.speed;
            m.velocity.z = dir[1] * self.how.speed;
            if wall {
                // A wall in the way of a target below it: given up.
                self.ticks = 0;
            }
        }
        self.rustle(m);
        let yaw = (-dir[0]).atan2(dir[1]).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
}

impl Creep {
    /// Now and then, its sound as it moves.
    fn rustle(&self, m: &mut Body) {
        if let Some(sound) = self.how.rustle
            && m.random.next_int_bound(RUSTLE_ONE_IN) == Some(0)
        {
            crate::sounds::queue(m.id, sound, 0.3, 0.9 + m.random.next_float() * 0.3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_creep_starts_paused() {
        static HOW: Creeping = Creeping {
            speed: 0.02,
            climb: 0.03,
            crawl: (40, 80),
            pause: (40, 80),
            wander: 2.0,
            area: |_, _| Vec::new(),
            may: |_, _| true,
            rustle: None,
        };
        let c = Creep::new(&HOW);
        assert!(c.pausing && c.target.is_none());
    }

    /// A trunk east of the cell at the origin, a block up, open air around it otherwise; `bare`, none.
    fn solid(bare: bool) -> impl Fn([i32; 3]) -> bool {
        move |p| !bare && p == [1, 1, 0]
    }

    #[test]
    fn a_climber_holds_on_to_the_bark_belly_to_it_and_lets_go_when_it_is_gone() {
        let mut m = crate::body::tests::body(4_100_000);
        m.position = Vec3 { x: 0.5, y: 1.0, z: 0.5 };
        m.on_ground = false;
        assert!(hold(&mut m, solid(false)));
        assert!(m.no_gravity && clinging(m.id));
        assert!((m.position.x - 0.7).abs() < 1e-9, "its middle 0.3 from the bark");
        assert_eq!(m.yaw, -90.0, "facing east, the bark");
        assert_eq!(side_of(solid(false), [0, 1, 0], m.yaw), Some([1, 0]));
        assert!(!hold(&mut m, solid(true)));
        assert!(!m.no_gravity && !clinging(m.id), "let go, to fall");
    }

    #[test]
    fn a_creature_on_the_ground_never_clings() {
        let mut m = crate::body::tests::body(4_100_010);
        m.position = Vec3 { x: 0.5, y: 1.0, z: 0.5 };
        m.on_ground = true;
        assert!(!hold(&mut m, solid(false)));
        assert!(!m.no_gravity && !clinging(m.id));
    }

    #[test]
    fn the_nearest_trunk_is_found() {
        let block = |p: [i32; 3]| match p {
            [1, 1, 0] => "minecraft:oak_log".to_string(),
            [3, 2, 3] => "minecraft:birch_log".to_string(),
            _ => "minecraft:air".to_string(),
        };
        assert_eq!(trunk_in(block, [0, 1, 0]), Some([1.5, 1.0, 0.5]));
        assert_eq!(trunk_in(|_| "minecraft:air".to_string(), [0, 1, 0]), None);
    }
}
