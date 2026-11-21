//! A ladybug's own behaviours (SPECS §3, §4), before the library's flight of small fliers (`creature::flutter`):
//!
//! - `Perch`, its signature: near a player who stands still, now and then it flies over and lands on them (the mod's
//!   tick seats it, `EntityRef::ride_player`), stays a while and flies off; the player has its luck (`crate::act`).
//! - `Cluster`: at night it flies to the cluster of ladybugs near it (the mod's tick finds where), and rests there.
//!
//! What a behaviour can't do itself (seat it on a player, let it go, give luck) it queues as an `Action`.
use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::body::{Behaviour, Body, World};

/// What the mod's tick does for a behaviour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    /// Lands on the player (entity id): seated on them, their luck given.
    Land { bug: u32, player: u32 },
    /// Off the player again.
    Leave { bug: u32 },
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

pub(crate) fn actions() -> Vec<Action> {
    std::mem::take(&mut *ACTIONS.lock().unwrap())
}

/// The ladybugs riding a player, for their drawing and the right-click that lets one go (single player).
static PERCHED: Mutex<BTreeMap<u32, ()>> = Mutex::new(BTreeMap::new());
/// The day each ladybug last landed on a player (the game's day count): once a day each.
static LANDED: Mutex<BTreeMap<u32, i64>> = Mutex::new(BTreeMap::new());
/// Where each ladybug's cluster gathers tonight, found by the mod's tick.
static CLUSTER: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());

pub(crate) fn perched(id: u32) -> bool {
    PERCHED.lock().unwrap().contains_key(&id)
}

pub(crate) fn set_perched(id: u32, on: bool) {
    let mut all = PERCHED.lock().unwrap();
    if on {
        all.insert(id, ());
    } else {
        all.remove(&id);
    }
}

pub(crate) fn landed(id: u32, day: i64) {
    LANDED.lock().unwrap().insert(id, day);
}

pub(crate) fn set_cluster(id: u32, at: Option<[f64; 3]>) {
    let mut all = CLUSTER.lock().unwrap();
    match at {
        Some(at) => all.insert(id, at),
        None => all.remove(&id),
    };
}

pub(crate) fn forget(id: u32) {
    PERCHED.lock().unwrap().remove(&id);
    LANDED.lock().unwrap().remove(&id);
    CLUSTER.lock().unwrap().remove(&id);
}

/// Landing on a player (SPECS §4): one chance in this many ticks while one stands still near it (one in 20 a minute),
/// how near and how long they must have stood (ticks), how long it stays (ticks).
const LAND_ONE_IN: i32 = 24_000;
const NEAR: f64 = 8.0;
const STILL_FOR: i32 = 60;
const STAY: (i32, i32) = (600, 1800);
/// Its flight to the player: its speed, and how near it lands (blocks).
const SPEED: f64 = 0.12;
const REACH: f64 = 0.7;

#[derive(Debug, Default)]
pub(crate) struct Perch {
    /// The player it flies to (entity id), once it chose to.
    to: Option<u32>,
    /// Ticks it has been on them, how long it stays, how long the nearest player has stood still where.
    ticks: i32,
    stay: i32,
    still: i32,
    last: Option<[f64; 3]>,
    ridden: bool,
}

/// The game's day count from its time of day.
pub(crate) fn day_of(cx: &World<'_>) -> i64 {
    cx.ground.level.day_time(cx.host()).unwrap_or(0).div_euclid(24_000)
}

impl Behaviour for Perch {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        // On a player: it stays until it goes (`tick`).
        if perched(m.id) {
            if !self.ridden {
                self.ridden = true;
                self.ticks = 0;
                self.stay = STAY.0 + m.random.next_int_bound(STAY.1 - STAY.0).unwrap_or(0);
            }
            return true;
        }
        self.ridden = false;
        let Some(p) = cx.player.as_ref().filter(|p| p.alive && !p.spectator) else {
            self.still = 0;
            return false;
        };
        let at = p.position.array();
        let moved = self.last.is_none_or(|l| creature::body::distance(l, at) > 0.1);
        self.last = Some(at);
        self.still = if moved { 0 } else { self.still + 1 };
        if self.to.is_some() {
            // Flying to them: given up if they walk off or it took too long.
            self.ticks += 1;
            if moved || self.ticks > 200 {
                self.to = None;
            }
            return self.to.is_some();
        }
        let near = creature::body::distance(at, m.position.array()) < NEAR;
        let today = LANDED.lock().unwrap().get(&m.id).copied() == Some(day_of(cx));
        if near && !today && cx.daylight && self.still >= STILL_FOR && m.random.next_int_bound(LAND_ONE_IN) == Some(0) {
            self.to = Some(p.entity_id);
            self.ticks = 0;
            return true;
        }
        false
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        if perched(m.id) {
            self.ticks += 1;
            let sprinting = cx.player.as_ref().is_some_and(|p| p.sprinting);
            if self.ticks >= self.stay || sprinting {
                ACTIONS.lock().unwrap().push(Action::Leave { bug: m.id });
            }
            return;
        }
        let Some(to) = self.to else {
            return;
        };
        let Some(p) = cx.player.as_ref().filter(|p| p.entity_id == to) else {
            self.to = None;
            return;
        };
        // To the top of their head.
        let head = [p.eye[0], p.eye[1] + 0.35, p.eye[2]];
        m.no_gravity = true;
        creature::flutter::steer(m, head, SPEED);
        if creature::body::distance(m.position.array(), head) < REACH {
            ACTIONS.lock().unwrap().push(Action::Land { bug: m.id, player: to });
            self.to = None;
        }
    }
    fn stop(&mut self, _: &mut Body, _: &World<'_>) {
        self.to = None;
    }
}

/// Gathering at night (SPECS §3): it flies to where its cluster gathers, a little off it, and rests there.
#[derive(Debug, Default)]
pub(crate) struct Cluster;

impl Behaviour for Cluster {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let at = CLUSTER.lock().unwrap().get(&m.id).copied();
        !cx.daylight && at.is_some_and(|at| creature::body::distance(at, m.position.array()) > 0.6)
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        let Some(at) = CLUSTER.lock().unwrap().get(&m.id).copied() else {
            return;
        };
        m.fall_distance = 0.0;
        m.no_gravity = true;
        if creature::flutter::steer(m, at, SPEED) {
            m.no_gravity = false;
            m.velocity.x = 0.0;
            m.velocity.y = 0.0;
            m.velocity.z = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_perch_and_a_cluster_are_forgotten() {
        set_perched(4, true);
        set_cluster(4, Some([1.0, 2.0, 3.0]));
        assert!(perched(4));
        forget(4);
        assert!(!perched(4));
        assert!(CLUSTER.lock().unwrap().get(&4).is_none());
    }
}
