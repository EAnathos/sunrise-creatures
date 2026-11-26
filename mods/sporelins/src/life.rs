//! A sporelin's behaviours (SPECS §2, §3), in priority order, from the server tick (the library's `body`):
//!
//! - `Startle`: hit, or a player sprinting at it, it bursts a puff of spores and runs off.
//! - `Hide`: a player coming near without sneaking (not its friend) makes it sit down and pull its cap over its stem,
//!   passing for a mushroom, until they are far or stand still a few seconds.
//! - `Follow`: befriended and not told to stay, it waddles after its friend, and comes to them at once when far.
//! - `Plant` (every tick, claiming nothing): now and then it plants a mushroom of its colour by it, or, told to stay,
//!   around where it stays (its patch).
//! - `Stroll`: it waddles about, around where it stays when told to.
//!
//! What a behaviour can't do itself (place a mushroom, puff spores, teach a fact) it queues as an `Action`
//! (`crate::act`).
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World, distance, flat_distance};
use creature::state::{EntityMap, EntitySet};
use creature::sync::Locked;

/// What the mod's tick does for a behaviour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    /// Sporelin `id` hid from a player (the players near learn it).
    Hid { id: u32 },
    /// Sporelin `id` bursts a puff of spores.
    Puff { id: u32 },
    /// Sporelin `id` plants a mushroom within `radius` blocks of `around` (its patch when `tending`).
    Plant {
        id: u32,
        around: [f64; 3],
        radius: i32,
        tending: bool,
    },
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

pub(crate) fn actions() -> Vec<Action> {
    std::mem::take(&mut *ACTIONS.locked())
}

fn queue(action: Action) {
    ACTIONS.locked().push(action);
}

/// The sporelins hiding (for their drawing), where each one told to stay stays (from the mod's tick), and the ones
/// shaking their cap (a bowl asked twice the same day), for so many more ticks.
static HIDING: EntitySet = EntitySet::new();
static STAYS_AT: EntityMap<[f64; 3]> = EntityMap::new();
static SHAKING: EntityMap<i32> = EntityMap::new();

pub(crate) fn hiding(id: u32) -> bool {
    HIDING.locked().contains(&id)
}

fn set_hiding(id: u32, on: bool) {
    let mut all = HIDING.locked();
    if on {
        all.insert(id);
    } else {
        all.remove(&id);
    }
}

pub(crate) fn stays_at(id: u32) -> Option<[f64; 3]> {
    STAYS_AT.locked().get(&id).copied()
}

pub(crate) fn set_stays_at(id: u32, at: Option<[f64; 3]>) {
    let mut all = STAYS_AT.locked();
    match at {
        Some(at) => all.insert(id, at),
        None => all.remove(&id),
    };
}

/// It shakes its cap for `SHAKE` ticks (a bowl it won't fill).
pub(crate) fn shake(id: u32) {
    SHAKING.locked().insert(id, SHAKE);
}

/// How far into its shake it is (0 to 1), if it shakes.
pub(crate) fn shaking(id: u32) -> Option<f32> {
    SHAKING.locked().get(&id).map(|left| 1.0 - *left as f32 / SHAKE as f32)
}

/// From the mod's tick: its shake goes on, and ends.
pub(crate) fn tick_shake(id: u32) {
    let mut all = SHAKING.locked();
    if let Some(left) = all.get_mut(&id) {
        *left -= 1;
        if *left <= 0 {
            all.remove(&id);
        }
    }
}

pub(crate) fn forget(id: u32) {
    HIDING.locked().remove(&id);
    STAYS_AT.locked().remove(&id);
    SHAKING.locked().remove(&id);
}

const SHAKE: i32 = 30;

/// Its friend, if the nearest player is its friend and not told to stay: where they are.
fn friend_near(m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
    let f = creature::friend::of(m.id)?;
    cx.player
        .as_ref()
        .filter(|p| p.alive && f.is(p.uuid))
        .map(|p| p.position.array())
}

/// The nearest player if they aren't its friend (whom it doesn't hide from).
fn stranger<'a>(m: &Body, cx: &'a World<'_>) -> Option<&'a creature::body::Player> {
    let friend = creature::friend::of(m.id);
    cx.player
        .as_ref()
        .filter(|p| p.alive && !p.spectator && !friend.is_some_and(|f| f.is(p.uuid)))
}

/// The way away from `from` for `far` blocks: straight away, or turned aside up to a quarter turn and more when a wall
/// (a fence, a tree) is in the way, the first way open.
fn away(m: &Body, cx: &World<'_>, from: [f64; 3], far: f64) -> [f64; 2] {
    let dir = creature::body::unit([m.position.x - from[0], m.position.z - from[2]]).unwrap_or([1.0, 0.0]);
    let open = |d: [f64; 2]| {
        (1..=far as i32).all(|step| {
            let p = [
                (m.position.x + d[0] * f64::from(step)).floor() as i32,
                m.position.y.floor() as i32,
                (m.position.z + d[1] * f64::from(step)).floor() as i32,
            ];
            // Open, or a one-block step (a fence, a wall or a gate is taller).
            creature::creep::passable(cx, p)
                || !creature::creep::too_tall(&cx.block(p)) && creature::creep::passable(cx, [p[0], p[1] + 1, p[2]])
        })
    };
    [0.0, 0.7, -0.7, 1.4, -1.4, 2.1, -2.1]
        .iter()
        .map(|&a| creature::body::turned(dir, a))
        .find(|d| open(*d))
        .unwrap_or(dir)
}

/// Startled (SPECS §2): how near a sprinting player startles it, how long it runs and how far, how fast.
const STARTLED: f64 = 4.0;
const RUN_TICKS: i32 = 60;
const RUN_FAR: f64 = 8.0;
const RUN_SPEED: f64 = 0.2;

/// Running off, it moves itself (no path finding: a hit may knock it against a fence, where no path starts).
#[derive(Debug, Default)]
pub(crate) struct Startle {
    health: f32,
    run: i32,
    from: [f64; 3],
    way: [f64; 2],
}

impl Behaviour for Startle {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        // Its health brought down to its own maximum on its first ticks (from its template's) is no hit.
        let hit = self.health > 0.0 && m.health < self.health && self.health <= m.max_health;
        self.health = m.health;
        if self.run > 0 {
            return true;
        }
        let runner = stranger(m, cx)
            .filter(|p| p.sprinting && distance(p.position.array(), m.position.array()) < STARTLED)
            .map(|p| p.position.array());
        let from = if hit {
            Some(cx.player.as_ref().map_or(m.position.array(), |p| p.position.array()))
        } else {
            runner
        };
        let Some(from) = from else {
            return false;
        };
        queue(Action::Puff { id: m.id });
        set_hiding(m.id, false);
        self.run = RUN_TICKS;
        self.from = from;
        self.way = away(m, cx, from, RUN_FAR);
        true
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        self.run -= 1;
        m.stop_walking = true;
        if self.run % 10 == 0 {
            self.way = away(m, cx, self.from, RUN_FAR);
        }
        let ahead = [
            (m.position.x + self.way[0] * 0.6).floor() as i32,
            m.position.y.floor() as i32,
            (m.position.z + self.way[1] * 0.6).floor() as i32,
        ];
        // A one-block step: it hops up.
        if m.on_ground && !creature::creep::passable(cx, ahead) && !creature::creep::too_tall(&cx.block(ahead)) {
            m.velocity.y = 0.42;
        }
        let speed = if self.run > 0 { RUN_SPEED } else { 0.0 };
        m.velocity.x = self.way[0] * speed;
        m.velocity.z = self.way[1] * speed;
        let yaw = creature::flight::heading_of(self.way[0], self.way[1]);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
}

/// Hiding (SPECS §2): a player within `SHY` blocks not sneaking makes it hide, until they are `CALM` blocks away or
/// have stood still `STILL` ticks.
const SHY: f64 = 6.0;
const CALM: f64 = 10.0;
const STILL: i32 = 60;

#[derive(Debug, Default)]
pub(crate) struct Hide {
    hidden: bool,
    /// Out again because the player stood still: it stays out until they move.
    calm: bool,
    still: i32,
    last: Option<[f64; 3]>,
}

impl Behaviour for Hide {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let Some(p) = stranger(m, cx) else {
            self.hidden = false;
            set_hiding(m.id, false);
            return false;
        };
        let at = p.position.array();
        let moved = self.last.is_none_or(|l| distance(l, at) > 0.05);
        self.last = Some(at);
        self.still = if moved { 0 } else { self.still + 1 };
        if moved {
            self.calm = false;
        }
        let d = distance(at, m.position.array());
        if self.hidden {
            if d > CALM || self.still >= STILL {
                self.hidden = false;
                self.calm = self.still >= STILL;
            }
        } else if d < SHY && !p.sneaking && !self.calm {
            self.hidden = true;
            queue(Action::Hid { id: m.id });
        }
        set_hiding(m.id, self.hidden);
        self.hidden
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        set_hiding(m.id, false);
    }
}

/// A spot to stand on by `at` (a block next to it, its own top solid, two open blocks over it), up to 3 blocks down.
fn ground_by(cx: &World<'_>, at: [f64; 3]) -> Option<[f64; 3]> {
    let open = |p: [i32; 3]| creature::creep::passable(cx, p);
    let (x, y, z) = (at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32);
    [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [-1, -1]]
        .iter()
        .find_map(|d| {
            (0..=3).map(|down| y - down).find_map(|y| {
                let p = [x + d[0], y, z + d[1]];
                let floor = [p[0], y - 1, p[2]];
                let solid = !open(floor) && cx.block(floor) != "minecraft:water";
                (solid && open(p) && open([p[0], y + 1, p[2]]))
                    .then(|| [f64::from(p[0]) + 0.5, f64::from(y), f64::from(p[2]) + 0.5])
            })
        })
}

/// Following its friend (SPECS §3): from `FOLLOW` blocks away it waddles to them, stops within `NEAR`, and past
/// `TELEPORT` comes to them at once, as a tamed wolf does.
const FOLLOW: f64 = 4.0;
const NEAR: f64 = 2.0;
const TELEPORT: f64 = 24.0;
const FOLLOW_SPEED: f64 = 1.3;

#[derive(Debug, Default)]
pub(crate) struct Follow {
    on: bool,
    ticks: i32,
}

impl Behaviour for Follow {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let following = creature::friend::of(m.id).is_some_and(|f| !f.staying);
        let Some(at) = friend_near(m, cx).filter(|_| following) else {
            self.on = false;
            return false;
        };
        let d = distance(at, m.position.array());
        self.on = if self.on { d > NEAR } else { d > FOLLOW };
        self.on
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(at) = friend_near(m, cx) else {
            return;
        };
        if distance(at, m.position.array()) > TELEPORT {
            // Onto ground by them (not in the air by a flying friend, which it would fall from): else it waits.
            if let Some(p) = ground_by(cx, at) {
                m.position = Vec3::of(p);
                m.velocity = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
                m.fall_distance = 0.0;
            }
            return;
        }
        self.ticks -= 1;
        if self.ticks <= 0 {
            self.ticks = 10;
            m.walk_to = Some((at, FOLLOW_SPEED));
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
    }
}

/// Planting (SPECS §2, §3): a mushroom every `PLANT_EVERY` ticks or so (about 5 minutes), next to it (within `BY`
/// blocks) when wild or following, around where it stays (within `PATCH`) when told to stay.
const PLANT_EVERY: (i32, i32) = (5000, 7000);
const BY: i32 = 2;
pub(crate) const PATCH: i32 = 8;

#[derive(Debug, Default)]
pub(crate) struct Plant {
    wait: Option<i32>,
}

impl Behaviour for Plant {
    fn claims(&self) -> bool {
        false
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        let wait = self.wait.unwrap_or_else(|| m.random.between(PLANT_EVERY)) - 1;
        if wait > 0 {
            self.wait = Some(wait);
            return;
        }
        self.wait = Some(m.random.between(PLANT_EVERY));
        let action = match stays_at(m.id) {
            Some(at) => Action::Plant {
                id: m.id,
                around: at,
                radius: PATCH,
                tending: true,
            },
            None => Action::Plant {
                id: m.id,
                around: m.position.array(),
                radius: BY,
                tending: false,
            },
        };
        queue(action);
    }
}

/// Strolling (SPECS §2): now and then it waddles a few blocks, around where it stays when told to (within `STAY_ROAM`).
const STROLL_EVERY: (i32, i32) = (100, 300);
const STROLL: f64 = 5.0;
const STAY_ROAM: f64 = 4.0;

#[derive(Debug, Default)]
pub(crate) struct Stroll {
    wait: i32,
}

impl Behaviour for Stroll {
    /// The last of its behaviours: whenever nothing else holds it.
    fn can_use(&mut self, _: &mut Body, _: &World<'_>) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        self.wait -= 1;
        if self.wait > 0 {
            return;
        }
        self.wait = m.random.between(STROLL_EVERY);
        let (center, reach) = match stays_at(m.id) {
            Some(at) => (at, STAY_ROAM),
            None => (m.position.array(), STROLL),
        };
        let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
        let r = f64::from(m.random.next_float()) * reach;
        let to = [center[0] + angle.cos() * r, center[1], center[2] + angle.sin() * r];
        if flat_distance(to, m.position.array()) > 0.5 {
            m.walk_to = Some((to, 0.8));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stay_point_and_a_hiding_are_forgotten() {
        set_stays_at(5, Some([1.0, 2.0, 3.0]));
        set_hiding(5, true);
        shake(5);
        assert!(hiding(5) && stays_at(5).is_some() && shaking(5).is_some());
        forget(5);
        assert!(!hiding(5) && stays_at(5).is_none() && shaking(5).is_none());
    }
}
