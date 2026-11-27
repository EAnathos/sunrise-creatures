//! What a toucan does (SPECS §2, §3), by hand (the library's `body`), from the server tick. Its flights are the
//! library's (`creature::flight`); what a behaviour can't do itself (eat an item, befriend) it queues as an `Action`,
//! which the mod's tick applies (`crate::act`).
//!
//! In priority order: `Upkeep` (no fall damage, its calls), `Stay` (sat, still, until told to follow), `Flee` from a
//! monster, following its friend (the library's `Follow`, as `FOLLOWING` says), `Eat` (a melon slice on the ground), `Flee` from a player,
//! `Flit` (a short flight to the leaves of a nearby tree, or down when it finds itself in the air), `Hop` (about the
//! branches).
use std::sync::Mutex;

use creature::body::{Behaviour, Body, World};
use creature::flight::{Flight, Following, heading_of};
use creature::state::EntityMap;
use creature::sync::Locked;

use crate::{CALL, WINGS};

/// What the mod's tick does for a behaviour.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Action {
    /// Eats the item (entity id) on the ground, thrown by `thrower` (an entity id), who may befriend it.
    Eat {
        toucan: u32,
        item: u32,
        thrower: Option<u32>,
    },
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

fn queue(action: Action) {
    ACTIONS.locked().push(action);
}

/// What the behaviours queued since the last call.
pub(crate) fn actions() -> Vec<Action> {
    std::mem::take(&mut *ACTIONS.locked())
}

/// What the mod's tick finds around each toucan (every 10 ticks): the nearest melon slice on the ground, the nearest
/// monster.
#[derive(Clone, Debug, Default)]
pub(crate) struct Around {
    pub food: Option<creature::food::Found>,
    pub monster: Option<[f64; 3]>,
}

static AROUND: EntityMap<Around> = EntityMap::new();

/// Each toucan resting on its perch after a flight: the ticks it has left (SPECS §2). It hops about meanwhile, and
/// neither flits off nor flees a player before it is rested.
static RESTING: EntityMap<i32> = EntityMap::new();
/// How long it rests after a flight (ticks): 5 to 10 seconds.
const REST: (i32, i32) = (100, 200);

fn resting(id: u32) -> bool {
    RESTING.locked().get(&id).is_some_and(|&left| left > 0)
}

pub(crate) fn set_around(id: u32, around: Around) {
    AROUND.locked().insert(id, around);
}

fn around(id: u32) -> Around {
    AROUND.locked().get(&id).cloned().unwrap_or_default()
}

/// The food it takes from the ground, which tames it (SPECS §3).
pub(crate) const FOOD: &str = "minecraft:melon_slice";

/// The behaviours, in priority order.
pub(crate) fn behaviours() -> Vec<Box<dyn Behaviour>> {
    vec![
        Box::new(Upkeep),
        Box::new(Stay),
        Box::new(Flee::from_monsters()),
        Box::new(creature::flight::Follow::new(&FOLLOWING, &WINGS)),
        Box::new(Eat::default()),
        Box::new(Flee::from_players()),
        Box::new(Flit::default()),
        Box::new(Hop::default()),
    ]
}

fn wild(m: &Body) -> bool {
    creature::friend::of(m.id).is_none()
}

fn chance(m: &mut Body, one_in: i32) -> bool {
    m.random.next_int_bound(one_in) == Some(0)
}

/// Every tick: no fall damage (it flies), its croaking call now and then by day.
#[derive(Debug)]
struct Upkeep;

impl Behaviour for Upkeep {
    fn claims(&self) -> bool {
        false
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        // Its rest counts down on its perch only.
        if m.on_ground
            && let Some(left) = RESTING.locked().get_mut(&m.id)
        {
            *left -= 1;
        }
        if cx.daylight && chance(m, 600) {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            creature::sounds::queue(m.id, CALL, 1.0, pitch);
        }
    }
}

/// How high over the column a perch is looked for, and how far down (blocks).
const SCAN_UP: i32 = 8;
const SCAN_DOWN: i32 = 20;

/// Whether nothing is in block `name` (a perch has air over it).
fn open(name: &str) -> bool {
    matches!(name, "minecraft:air" | "minecraft:cave_air" | "")
}

/// Where it may perch in the column at `x`, `z` near height `y`: on the first block down from `SCAN_UP` over it, with
/// air over it, and whether it is leaves; none over water or lava, or where the column is full from its top.
fn column_top(cx: &World<'_>, x: i32, y: i32, z: i32) -> Option<([f64; 3], bool)> {
    let top = y + SCAN_UP;
    if !open(&cx.block([x, top, z])) {
        return None;
    }
    let (at, name) = (y - SCAN_DOWN..top)
        .rev()
        .map(|y| (y, cx.block([x, y, z])))
        .find(|(_, name)| !open(name))?;
    if name.contains("water") || name.contains("lava") {
        return None;
    }
    let spot = [f64::from(x) + 0.5, f64::from(at + 1), f64::from(z) + 0.5];
    Some((spot, name.ends_with("leaves")))
}

/// A perch between `near` and `far` blocks across from it, with a clear way there (`clear`): the top of the leaves of
/// a tree, else the ground, tried at random ways (within a quarter turn of `away`, a heading in degrees, when it
/// flees).
fn perch(m: &mut Body, cx: &World<'_>, near: f64, far: f64, away: Option<f32>) -> Option<[f64; 3]> {
    let mut ground = None;
    for _ in 0..24 {
        let angle = match away {
            Some(yaw) => f64::from(yaw) + (f64::from(m.random.next_float()) - 0.5) * 90.0,
            None => f64::from(m.random.next_float()) * 360.0,
        };
        let r = near + f64::from(m.random.next_float()) * (far - near);
        let (s, c) = angle.to_radians().sin_cos();
        let (x, z) = (m.position.x - s * r, m.position.z + c * r);
        let Some((spot, leaves)) = column_top(cx, x.floor() as i32, m.position.y.floor() as i32, z.floor() as i32)
        else {
            continue;
        };
        if !clear(cx, m.position.array(), spot) {
            continue;
        }
        if leaves {
            return Some(spot);
        }
        ground.get_or_insert(spot);
    }
    ground
}

/// How high over its spot a trip goes before it comes down onto it (blocks), and how high its body's middle is.
const OVER: f64 = 1.5;
const MIDDLE: f64 = 0.6;

/// Where a trip to `to` passes first: over it.
fn over(to: [f64; 3]) -> [f64; 3] {
    [to[0], to[1] + OVER, to[2]]
}

/// Whether nothing is in the way of a trip from `from` to `to` (its feet's places): along the straight line to the
/// point over it, then straight down, at its body's middle, a sample every half block.
fn clear(cx: &World<'_>, from: [f64; 3], to: [f64; 3]) -> bool {
    let up = |p: [f64; 3]| [p[0], p[1] + MIDDLE, p[2]];
    let open_line = |a: [f64; 3], b: [f64; 3]| {
        let steps = (creature::body::distance(a, b) * 2.0).ceil().max(1.0) as i32;
        (1..=steps).all(|i| {
            let t = f64::from(i) / f64::from(steps);
            let p = [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ];
            open(&cx.block([p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32]))
        })
    };
    open_line(up(from), up(over(to))) && open_line(up(over(to)), up(to))
}

/// How near counts as there (blocks), how near across to its spot it starts down onto it, the longest a trip lasts,
/// and how long it may go without getting nearer before it gives up (ticks; blocks nearer that count).
const THERE: f64 = 0.6;
const DOWN_FROM: f64 = 0.8;
const TRIP_FOR: i32 = 300;
const STUCK_FOR: i32 = 30;
const NEARER: f64 = 0.3;

/// Whether a creature going somewhere still gets nearer: given its distance each tick, false once it hasn't for
/// `STUCK_FOR` ticks (something in its way).
#[derive(Debug, Default)]
struct Progress {
    best: f64,
    since: i32,
}

impl Progress {
    fn reset(&mut self) {
        self.best = f64::MAX;
        self.since = 0;
    }

    fn going(&mut self, distance: f64) -> bool {
        if distance < self.best - NEARER {
            self.best = distance;
            self.since = 0;
        } else {
            self.since += 1;
        }
        self.since < STUCK_FOR
    }
}

/// A short flight to a spot (`perch`), on beating wings: straight to the point over it, then down onto it. Stuck (a
/// branch in its way), it gives up, and lands wherever it is or looks for another perch.
#[derive(Debug, Default)]
struct Trip {
    flight: Flight,
    to: Option<[f64; 3]>,
    down: bool,
    ticks: i32,
    progress: Progress,
}

impl Trip {
    fn start(&mut self, m: &Body, to: [f64; 3]) {
        self.begin(m, to, false);
    }

    /// Straight onto `to`, not over it first (food on the ground).
    fn start_direct(&mut self, m: &Body, to: [f64; 3]) {
        self.begin(m, to, true);
    }

    fn begin(&mut self, m: &Body, to: [f64; 3], direct: bool) {
        self.flight = Flight::default();
        self.to = Some(to);
        self.down = direct;
        self.ticks = 0;
        self.progress.reset();
        creature::sounds::queue(m.id, WINGS.takeoff, 0.8, 1.0);
    }

    fn on(&self) -> bool {
        self.to.is_some()
    }

    /// Its trip over (or given up): it rests before the next.
    fn end(&mut self, m: &mut Body) {
        if self.to.take().is_some() {
            RESTING.locked().insert(m.id, m.random.between(REST));
        }
        self.flight.land(m);
    }

    /// One tick of it: whether it is still on its way.
    fn tick(&mut self, m: &mut Body) -> bool {
        let Some(to) = self.to else {
            return false;
        };
        self.ticks += 1;
        let at = m.position.array();
        let there = creature::body::distance(at, to) < THERE
            || (m.on_ground && creature::body::flat_distance(at, to) < 1.0 && self.ticks > 5);
        if !self.down && creature::body::flat_distance(at, to) < DOWN_FROM {
            self.down = true;
            self.progress.reset();
        }
        let aim = if self.down { to } else { over(to) };
        let going = self.progress.going(creature::body::distance(at, aim));
        if there || !going || self.ticks > TRIP_FOR {
            self.end(m);
            return false;
        }
        let yaw = if creature::body::flat_distance(at, to) < DOWN_FROM {
            m.yaw
        } else {
            heading_of(to[0] - m.position.x, to[2] - m.position.z)
        };
        self.flight.hover(m, aim, yaw, &WINGS);
        true
    }
}

/// How near a player frightens it (closer when they sneak), in blocks (a monster: `crate::MONSTER_RANGE`); how far it
/// flies off.
const PLAYER_NEAR: f64 = 6.0;
const SNEAKING_NEAR: f64 = 3.0;
const FLEE_TO: (f64, f64) = (8.0, 16.0);

/// Fleeing (SPECS §2): a monster near it, or, a wild one, a player, and it flies off to a perch away from them.
#[derive(Debug)]
struct Flee {
    monsters: bool,
    trip: Trip,
}

impl Flee {
    fn from_monsters() -> Flee {
        Flee {
            monsters: true,
            trip: Trip::default(),
        }
    }

    fn from_players() -> Flee {
        Flee {
            monsters: false,
            trip: Trip::default(),
        }
    }

    /// What frightens it now: where it is.
    fn threat(&self, m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
        if self.monsters {
            return around(m.id).monster;
        }
        let p = cx
            .player
            .as_ref()
            .filter(|p| p.alive && !p.spectator && wild(m) && !resting(m.id))?;
        let near = if p.sneaking { SNEAKING_NEAR } else { PLAYER_NEAR };
        let at = p.position.array();
        (creature::body::distance(at, m.position.array()) < near).then_some(at)
    }
}

impl Behaviour for Flee {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        if self.trip.on() {
            return true;
        }
        let Some(from) = self.threat(m, cx) else {
            return false;
        };
        let away = heading_of(m.position.x - from[0], m.position.z - from[2]);
        let to = perch(m, cx, FLEE_TO.0, FLEE_TO.1, Some(away)).unwrap_or_else(|| {
            // Nowhere to perch: up and away, then down wherever it is.
            let (s, c) = f64::from(away).to_radians().sin_cos();
            [
                m.position.x - s * FLEE_TO.0,
                m.position.y + 4.0,
                m.position.z + c * FLEE_TO.0,
            ]
        });
        self.trip.start(m, to);
        true
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.trip.end(m);
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        self.trip.tick(m);
    }
}

/// Following its friend (SPECS §3), the library's `creature::flight::Follow`: past 6 blocks it flies to them, 2 blocks
/// over their head, lands within 3, comes to them past 24; not when told to stay.
pub(crate) static FOLLOWING: Following = Following {
    follow: 6.0,
    land: 3.0,
    teleport: 24.0,
    above: 2.0,
    may: |m, _| creature::friend::of(m.id).is_some_and(|f| !f.staying),
    lands_on_water: false,
    ..Following::DEFAULT
};

/// Told to stay: it sits where it is (its drawing, `crate::sitting`) and keeps still, whatever comes near, until told
/// to follow.
#[derive(Debug)]
struct Stay;

impl Behaviour for Stay {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        creature::friend::of(m.id).is_some_and(|f| f.staying)
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        m.no_gravity = false;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
}

/// How near it takes a melon slice (blocks), and how long it tries for one (ticks).
const BITE: f64 = 1.2;
const EAT_FOR: i32 = 300;

/// A melon slice on the ground (SPECS §3): it flies down to it (a `Trip`) and eats it; one thrown by a player may
/// tame it (`crate::act`). One it can't reach (stuck on the way), it leaves.
#[derive(Debug, Default)]
struct Eat {
    trip: Trip,
    /// The item it flew to last, and whether it gave it up.
    item: Option<(u32, bool)>,
    ticks: i32,
}

impl Behaviour for Eat {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        let Some(food) = around(m.id).food else {
            self.ticks = 0;
            return false;
        };
        let given_up = self.item == Some((food.item, true));
        let on = !given_up && self.ticks < EAT_FOR;
        if !on {
            self.ticks = 0;
        }
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.trip.end(m);
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        self.ticks += 1;
        let Some(food) = around(m.id).food else {
            return;
        };
        if creature::body::distance(m.position.array(), food.at) < BITE {
            queue(Action::Eat {
                toucan: m.id,
                item: food.item,
                thrower: food.thrower,
            });
            AROUND.locked().entry(m.id).or_default().food = None;
            self.trip.end(m);
            self.item = None;
            self.ticks = 0;
            return;
        }
        match self.item {
            // On its way, or its trip over short of it: given up.
            Some((item, _)) if item == food.item => {
                if !self.trip.tick(m) {
                    self.item = Some((item, true));
                }
            }
            _ => {
                self.trip.start_direct(m, food.at);
                self.item = Some((food.item, false));
            }
        }
    }
}

/// Its short flights between the trees (SPECS §2): once rested, one chance in this many ticks on a perch (about 2
/// seconds), and how far (blocks across).
const FLIT_ONE_IN: i32 = 40;
const FLIT_TO: (f64, f64) = (5.0, 14.0);
/// How fast it falls before it opens its wings (blocks a tick).
const FALLING: f64 = -0.3;

/// A short flight to the leaves of a nearby tree, now and then; and, finding itself falling or in water, a flight to
/// the nearest perch.
#[derive(Debug, Default)]
struct Flit {
    trip: Trip,
}

impl Behaviour for Flit {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        if self.trip.on() {
            return true;
        }
        let lost = m.in_water || (!m.on_ground && m.velocity.y < FALLING);
        let restless = m.on_ground && !resting(m.id) && chance(m, FLIT_ONE_IN);
        if !lost && !restless {
            return false;
        }
        let (near, far) = if lost { (1.0, 8.0) } else { FLIT_TO };
        let Some(to) = perch(m, cx, near, far, None) else {
            return false;
        };
        self.trip.start(m, to);
        true
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.trip.end(m);
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        self.trip.tick(m);
    }
}

/// Its hops about a branch (SPECS §2): how long between them (ticks), how far and how high.
const HOP_EVERY: (i32, i32) = (30, 100);
const HOP_ACROSS: f64 = 0.18;
const HOP_UP: f64 = 0.36;
/// How often it turns its head between hops (one tick in so many), and how far at most (degrees).
const LOOK_ONE_IN: i32 = 25;
const LOOK_ABOUT: f32 = 70.0;

/// Hopping about where it perches: a small hop now and then, only where there is something to land on a step up or
/// down (it doesn't hop off the canopy), turning its head about in between.
#[derive(Debug, Default)]
struct Hop {
    wait: i32,
}

impl Behaviour for Hop {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        m.on_ground
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.no_gravity = false;
        m.stop_walking = true;
        self.wait -= 1;
        if self.wait > 0 {
            // Its head turned about now and then, one way or the other.
            if chance(m, LOOK_ONE_IN) {
                m.head_yaw = m.yaw + (m.random.next_float() - 0.5) * 2.0 * LOOK_ABOUT;
            }
            return;
        }
        self.wait = m.random.between(HOP_EVERY);
        let yaw = m.yaw + (m.random.next_float() - 0.5) * 180.0;
        let (s, c) = f64::from(yaw).to_radians().sin_cos();
        let (x, z) = (m.position.x - s * 1.0, m.position.z + c * 1.0);
        let feet = m.position.y.floor() as i32;
        let lands = column_top(cx, x.floor() as i32, feet, z.floor() as i32)
            .is_some_and(|(spot, _)| (spot[1] - m.position.y).abs() <= 1.0);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
        if lands {
            m.velocity.x = -s * HOP_ACROSS;
            m.velocity.z = c * HOP_ACROSS;
            m.velocity.y = HOP_UP;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_are_taken_once() {
        let eat = Action::Eat {
            toucan: 3,
            item: 4,
            thrower: None,
        };
        queue(eat.clone());
        assert_eq!(actions(), vec![eat]);
        assert!(actions().is_empty());
    }

    #[test]
    fn a_trip_that_stops_getting_nearer_is_given_up() {
        let mut progress = Progress::default();
        progress.reset();
        assert!((0..20).all(|i| progress.going(10.0 - f64::from(i))));
        assert!((0..STUCK_FOR - 1).all(|_| progress.going(5.0)));
        assert!(!progress.going(5.0), "stuck");
    }

    #[test]
    fn a_perch_has_nothing_in_it() {
        assert!(open("minecraft:air") && open(""));
        assert!(!open("minecraft:jungle_leaves"));
    }
}
