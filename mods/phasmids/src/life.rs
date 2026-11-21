//! A phasmid's own behaviours (SPECS §3), before its crawl (the library's creeping): what keeps it still,
//! what makes it seen, and its playing dead.
//!
//! - `Watch` (every tick, claiming nothing): whether a player is looking at it (spotting it, SPECS §3), and whether it
//!   keeps still: by day, with a player near who hasn't spotted it, or while it plays dead (`busy`, its crawl waits).
//! - `PlayDead`: hit, or a player sprinting right by, it drops to the ground and lies still on its back, then goes back
//!   up: to the nearest trunk, up it through the leaves, onto the canopy.
//!
//! What a behaviour can't do itself (count a spotting, teach a fact) it queues as an `Action` (`crate::act`).
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World};

/// What the mod's tick does for a behaviour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    /// `player` (UUID) spotted phasmid `id`.
    Spotted { id: u32, player: u128 },
    /// Phasmid `id` dropped to play dead (the players near learn it).
    Dropped { id: u32 },
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

pub(crate) fn actions() -> Vec<Action> {
    std::mem::take(&mut *ACTIONS.lock().unwrap())
}

/// Whether each phasmid keeps still (its crawl waits), and whether it lies playing dead (for its drawing).
static BUSY: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
static DEAD: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
/// The players who spotted each phasmid (each counts once).
static SPOTTED: Mutex<BTreeMap<u32, BTreeSet<u128>>> = Mutex::new(BTreeMap::new());

/// For its crawl (`creature::creep::Creeping::may`): it keeps still.
pub(crate) fn busy(id: u32) -> bool {
    BUSY.lock().unwrap().contains(&id)
}

pub(crate) fn dead(id: u32) -> bool {
    DEAD.lock().unwrap().contains(&id)
}

pub(crate) fn forget(id: u32) {
    BUSY.lock().unwrap().remove(&id);
    DEAD.lock().unwrap().remove(&id);
    SPOTTED.lock().unwrap().remove(&id);
}

fn set(set: &Mutex<BTreeSet<u32>>, id: u32, on: bool) {
    let mut all = set.lock().unwrap();
    if on {
        all.insert(id);
    } else {
        all.remove(&id);
    }
}

/// Spotting (SPECS §3): looking at it from within `SPOT_RANGE` blocks, within `SPOT_ANGLE` of the crosshair, for
/// `SPOT_TICKS`. A player within `STILL_NEAR` blocks who hasn't spotted it makes it keep still.
const SPOT_RANGE: f64 = 6.0;
const SPOT_ANGLE: f64 = 4.0;
const SPOT_TICKS: i32 = 20;
const STILL_NEAR: f64 = 6.0;

/// Its middle, a little over its feet, where a look aims.
fn middle(m: &Body) -> [f64; 3] {
    [m.position.x, m.position.y + 0.1, m.position.z]
}

/// Whether a player looking along `look` from `eye` looks at `at` (within `SPOT_ANGLE`, from within `SPOT_RANGE`).
fn looks_at(eye: [f64; 3], look: [f64; 3], at: [f64; 3]) -> bool {
    let to = [at[0] - eye[0], at[1] - eye[1], at[2] - eye[2]];
    let d = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
    let l = (look[0] * look[0] + look[1] * look[1] + look[2] * look[2]).sqrt();
    if d > SPOT_RANGE || d < 0.01 || l < 0.01 {
        return false;
    }
    let cos = (to[0] * look[0] + to[1] * look[1] + to[2] * look[2]) / (d * l);
    cos >= SPOT_ANGLE.to_radians().cos()
}

#[derive(Debug, Default)]
pub(crate) struct Watch {
    looked: i32,
}

impl Behaviour for Watch {
    fn claims(&self) -> bool {
        false
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        let player = cx.player.as_ref().filter(|p| p.alive && !p.spectator);
        let mut near = false;
        if let Some(p) = player {
            near = creature::body::distance(p.position.array(), m.position.array()) < STILL_NEAR;
            let already = SPOTTED.lock().unwrap().get(&m.id).is_some_and(|s| s.contains(&p.uuid));
            if looks_at(p.eye, p.look, middle(m)) {
                self.looked += 1;
                if self.looked == SPOT_TICKS && !already {
                    SPOTTED.lock().unwrap().entry(m.id).or_default().insert(p.uuid);
                    ACTIONS.lock().unwrap().push(Action::Spotted {
                        id: m.id,
                        player: p.uuid,
                    });
                }
            } else {
                self.looked = 0;
            }
            if already {
                near = false;
            }
        }
        let still = cx.daylight || near || dead(m.id);
        set(&BUSY, m.id, still);
    }
}

/// Playing dead (SPECS §3): how long it lies still (ticks), how near a sprinting player makes it drop, how far it looks
/// for a trunk to climb back up, and how fast it climbs (blocks a tick).
const LIE: (i32, i32) = (200, 400);
const STARTLED: f64 = 3.0;
const TRUNK_SEARCH: i32 = 6;
const CLIMB: f64 = 0.06;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
    #[default]
    Up,
    /// Falling and lying still, for so many more ticks.
    Lying(i32),
    /// Walking to the trunk it climbs (where it stands beside it).
    ToTrunk([f64; 3], i32),
    /// Up the trunk until it is over the leaves.
    Climbing([f64; 3]),
}

#[derive(Debug, Default)]
pub(crate) struct PlayDead {
    phase: Phase,
    health: f32,
    /// Where it drops to: the ground under the leaves it was on.
    drop: Option<f64>,
}

/// The ground under it (the first block down that is neither air nor leaves), within 24 blocks: where it drops to
/// through the foliage.
fn ground_below(m: &Body, cx: &World<'_>) -> Option<f64> {
    let (x, y, z) = (
        m.position.x.floor() as i32,
        m.position.y.floor() as i32,
        m.position.z.floor() as i32,
    );
    (1..24)
        .map(|d| y - d)
        .find(|&y| {
            let b = cx.block([x, y, z]);
            b != "minecraft:air" && !b.ends_with("leaves")
        })
        // Only onto open ground: in a bush of leaves down to the ground, it stays where it is.
        .filter(|&y| cx.block([x, y + 1, z]) == "minecraft:air")
        .map(|y| f64::from(y + 1))
}

/// The nearest log within `TRUNK_SEARCH` blocks across of where it lies, at its height or a little above, and the
/// spot beside it (an open block next to the log) it climbs from.
fn trunk(m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
    let (x0, y0, z0) = (
        m.position.x.floor() as i32,
        m.position.y.floor() as i32,
        m.position.z.floor() as i32,
    );
    let mut best: Option<(i32, [f64; 3])> = None;
    for dx in -TRUNK_SEARCH..=TRUNK_SEARCH {
        for dz in -TRUNK_SEARCH..=TRUNK_SEARCH {
            for dy in 0..=2 {
                let p = [x0 + dx, y0 + dy, z0 + dz];
                if !cx.block(p).ends_with("_log") {
                    continue;
                }
                for (sx, sz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let side = [p[0] + sx, y0, p[2] + sz];
                    if cx.block(side) != "minecraft:air" {
                        continue;
                    }
                    let d = dx * dx + dz * dz;
                    if best.is_none_or(|b| d < b.0) {
                        // In the open block beside the log (its box clear of the bark, or it would suffocate).
                        let at = [f64::from(side[0]) + 0.5, f64::from(y0), f64::from(side[2]) + 0.5];
                        best = Some((d, at));
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// Whether it is over the canopy: on top of leaves, open air where it stands.
fn on_canopy(m: &Body, cx: &World<'_>) -> bool {
    let p = [
        m.position.x.floor() as i32,
        m.position.y.floor() as i32,
        m.position.z.floor() as i32,
    ];
    let below = cx.block([p[0], p[1] - 1, p[2]]);
    below.ends_with("leaves") && cx.block(p) == "minecraft:air"
}

impl Behaviour for PlayDead {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let hit = self.health > 0.0 && m.health < self.health;
        self.health = m.health;
        let startled = cx.player.as_ref().is_some_and(|p| {
            p.sprinting && p.alive && creature::body::distance(p.position.array(), m.position.array()) < STARTLED
        });
        if self.phase == Phase::Up && (hit || startled) {
            self.phase = Phase::Lying(LIE.0 + m.random.next_int_bound(LIE.1 - LIE.0).unwrap_or(0));
            self.drop = ground_below(m, cx);
            set(&DEAD, m.id, true);
            ACTIONS.lock().unwrap().push(Action::Dropped { id: m.id });
        }
        self.phase != Phase::Up
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.stop_walking = true;
        match self.phase {
            Phase::Up => {}
            Phase::Lying(left) => {
                // It drops through the foliage to the ground, and lies there.
                if let Some(y) = self.drop.take()
                    && y < m.position.y
                {
                    m.position = Vec3 { y, ..m.position };
                }
                m.no_gravity = false;
                m.velocity.x = 0.0;
                m.velocity.z = 0.0;
                if left > 0 {
                    self.phase = Phase::Lying(left - 1);
                    return;
                }
                set(&DEAD, m.id, false);
                self.phase = match trunk(m, cx) {
                    Some(at) => Phase::ToTrunk(at, 200),
                    // No tree near: it crawls where it is.
                    None => Phase::Up,
                };
            }
            Phase::ToTrunk(at, left) => {
                let d = (at[0] - m.position.x).hypot(at[2] - m.position.z);
                if d < 0.4 {
                    self.phase = Phase::Climbing(at);
                } else if left <= 0 {
                    self.phase = Phase::Up;
                } else {
                    if left % 20 == 0 {
                        m.walk_to = Some((at, 1.0));
                    }
                    self.phase = Phase::ToTrunk(at, left - 1);
                }
            }
            Phase::Climbing(at) => {
                // Up the bark, through the leaves, until it stands on them (no collision while it climbs).
                m.no_gravity = true;
                m.velocity = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
                m.position = Vec3 {
                    x: at[0],
                    y: m.position.y + CLIMB,
                    z: at[2],
                };
                let top = m.position.y - at[1] > 24.0;
                if on_canopy(m, cx) || top {
                    m.no_gravity = false;
                    self.phase = Phase::Up;
                }
            }
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        if self.phase != Phase::Up {
            m.no_gravity = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_straight_at_it_spots_it_a_look_aside_doesnt() {
        assert!(looks_at([0.0, 1.6, 0.0], [0.0, 0.0, 1.0], [0.0, 1.6, 4.0]));
        assert!(!looks_at([0.0, 1.6, 0.0], [0.0, 0.0, 1.0], [1.0, 1.6, 4.0]));
        assert!(!looks_at([0.0, 1.6, 0.0], [0.0, 0.0, 1.0], [0.0, 1.6, 9.0]));
    }
}
