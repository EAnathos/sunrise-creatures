//! How a flying fish swims and glides (SPECS §3), by hand (the library's `body`). In water it swims as a shoal in the
//! top blocks, turning with its neighbours. Frightened (a player near, a hit, a dolphin or a boat the mod sees, a
//! neighbour taking off), and now and then for nothing, it races up, bursts out of the water and glides a block or two
//! over the waves, beats its tail on a wave to glide again, and falls back in. Out of water and not gliding it is
//! beached: the library's `aquatic` makes it fall and flop.
use std::f64::consts::TAU;
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World, turned, unit};
use creature::state::{EntityMap, EntitySet};
use creature::sync::Locked;

/// Its speed swimming and gliding (blocks a tick), and how fast it climbs out.
const SWIM_SPEED: f64 = 0.1;
const GLIDE_SPEED: f64 = 0.8;
const RISE_SPEED: (f64, f64) = (0.5, 0.45);
/// How deep under the surface it swims (blocks of water over it).
const DEPTH: (f64, f64) = (0.8, 2.5);
/// A glide's length (ticks, so 10 to 20 blocks), how high over the water, the tail-beats it may add, a dip's length.
const GLIDE_TICKS: (i32, i32) = (13, 25);
const GLIDE_HEIGHT: (f64, f64) = (1.0, 2.0);
const BEATS: i32 = 2;
const DIP_TICKS: i32 = 5;
/// The longest it tries to reach the surface (ticks), and its rest after a glide before it glides again.
const RISE_TICKS: i32 = 30;
const REST: i32 = 100;
/// How near a player frightens it, how far a fright spreads through the shoal (blocks), and one glide for nothing in
/// so many ticks.
const FRIGHT: f64 = 6.0;
const SPREAD: f64 = 8.0;
const FOR_NOTHING: i32 = 3000;
/// How far ahead it makes sure of open water before it takes off (blocks), and the turns it tries for it (radians).
const OPEN_WATER: i32 = 12;
const TRY_TURNS: [f64; 9] = [0.0, 0.4, -0.4, 0.8, -0.8, 1.2, -1.2, 1.6, -1.6];
/// Its wandering: how much its heading veers each tick at most (radians), changed every so many ticks.
const VEER: f64 = 0.03;
/// Its shoal: the neighbours it turns with, the gap it keeps from them (blocks).
const SHOAL: f64 = 5.0;
const GAP: f64 = 0.7;
/// A glide of more than this many blocks is a fact; a takeoff of this many is a shoal's.
pub(crate) const LONG_GLIDE: f64 = 30.0;
const SHOAL_TAKEOFF: usize = 4;

/// Its sounds (`crate`): the splash out of and into the water, the buzz of its fins gliding, its tail on a wave.
use crate::{BEAT, GLIDE, SPLASH};

/// A fish's place and heading.
type Place = ([f64; 3], [f64; 2]);
/// Each fish's place and heading, for its shoal (single player: the hooks share the ids).
static FISH: EntityMap<Place> = EntityMap::new();
/// Fish told to take off: in so many ticks, away along this heading.
static PANIC: EntityMap<(i32, [f64; 2])> = EntityMap::new();
/// The fish out of the water gliding, for their drawing (fins spread).
static GLIDING: EntitySet = EntitySet::new();
/// What the mod's tick turns into facts: glides longer than `LONG_GLIDE` (by the fish) and shoals taking off (where).
static GLIDED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static TOOK_OFF: Mutex<Vec<[f64; 3]>> = Mutex::new(Vec::new());
/// Gliders that ran into a player or a mob (the mod's tick sees it): the heading they bounce off along.
static BUMPED: EntityMap<[f64; 2]> = EntityMap::new();

/// Flying fish `id`, gliding, ran into something at `from`: it bounces off and drops (SPECS §3).
pub(crate) fn bump(id: u32, at: [f64; 3], from: [f64; 3]) {
    let away = unit([at[0] - from[0], at[2] - from[2]]).unwrap_or([0.0, 1.0]);
    BUMPED.locked().insert(id, away);
}

/// Whether flying fish `id` is gliding (its fins spread).
pub(crate) fn gliding(id: u32) -> bool {
    GLIDING.locked().contains(&id)
}

/// Whether flying fish `id` just ended a long glide.
pub(crate) fn glided(id: u32) -> bool {
    let mut all = GLIDED.locked();
    let was = all.contains(&id);
    all.retain(|g| *g != id);
    was
}

/// Where shoals took off since the last call.
pub(crate) fn took_off() -> Vec<[f64; 3]> {
    std::mem::take(&mut *TOOK_OFF.locked())
}

/// A fish gone: its glides waiting for their fact are dropped (what is kept by its id goes with it, `creature::state`).
pub(crate) fn forget(id: u32) {
    GLIDED.locked().retain(|g| *g != id);
}

/// A threat at `from` frightens the fish at `at` and its shoal: each takes off within half a second, away from it.
pub(crate) fn scare(at: [f64; 3], from: [f64; 3]) {
    let fish = FISH.locked();
    let mut panic = PANIC.locked();
    let mut count = 0;
    for (id, (p, _)) in fish.iter() {
        if creature::body::distance(*p, at) > SPREAD || panic.contains_key(id) {
            continue;
        }
        let away = unit([p[0] - from[0], p[2] - from[2]]).unwrap_or([0.0, 1.0]);
        // Fanning out: the nearest first, each turned a little.
        let delay = (creature::body::distance(*p, at) * 1.2) as i32 + (*id % 4) as i32;
        let turn = (f64::from(*id % 7) - 3.0) * 0.12;
        panic.insert(*id, (delay, turned(away, turn)));
        count += 1;
    }
    if count >= SHOAL_TAKEOFF {
        TOOK_OFF.locked().push(at);
    }
}

fn cell(p: Vec3) -> [i32; 3] {
    [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32]
}

fn water(cx: &World<'_>, p: [i32; 3]) -> bool {
    cx.block(p) == "minecraft:water"
}

/// The water over it (blocks), up to 8.
fn depth(m: &Body, cx: &World<'_>) -> f64 {
    let middle = [m.position.x, m.position.y + 0.2, m.position.z];
    creature::aquatic::water_over(cx, middle, 8)
}

/// The top of the water under `at` (its y), if there is water within a few blocks down before anything else.
fn surface_under(cx: &World<'_>, at: Vec3) -> Option<f64> {
    let [x, y, z] = cell(at);
    (0..6).map(|d| y - d).find_map(|y| match cx.block([x, y, z]).as_str() {
        "minecraft:air" | "minecraft:cave_air" => None,
        "minecraft:water" => Some(Some(f64::from(y) + 0.9)),
        _ => Some(None),
    })?
}

/// The heading nearest `dir` with open water along it for a glide (none near a shore all around).
fn open_heading(cx: &World<'_>, at: Vec3, dir: [f64; 2]) -> Option<[f64; 2]> {
    TRY_TURNS.iter().map(|&a| turned(dir, a)).find(|d| {
        (1..=OPEN_WATER / 2).all(|i| {
            let step = f64::from(i * 2);
            let over = Vec3 {
                x: at.x + d[0] * step,
                y: at.y + 0.5,
                z: at.z + d[1] * step,
            };
            surface_under(cx, over).is_some()
        })
    })
}

/// Its heading as a yaw (degrees), as the game turns a mob.
fn yaw(dir: [f64; 2]) -> f32 {
    (-dir[0]).atan2(dir[1]).to_degrees() as f32
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Swim,
    /// Racing up to the surface, for so many more ticks.
    Rise(i32),
    /// Over the water: ticks left of this stretch, tail-beats left.
    Glide(i32, i32),
    /// Its tail on a wave between two stretches.
    Dip(i32, i32),
}

#[derive(Debug)]
pub(crate) struct Fish {
    phase: Phase,
    dir: [f64; 2],
    /// Ticks to its next wander, and of rest left after a glide.
    wander: i32,
    rest: i32,
    /// How its heading veers each tick, and whether its heading was picked yet (at random, its first tick).
    veer: f64,
    started: bool,
    /// Its health last tick (a hit frightens it), where its glide began, how high it glides, which way it turns at a
    /// shore.
    health: f32,
    start: [f64; 3],
    /// Whether it is falling back into the water from a glide (a splash as it goes in).
    diving: bool,
    height: f64,
    turn: f64,
}

impl Default for Fish {
    fn default() -> Fish {
        Fish {
            phase: Phase::Swim,
            dir: [0.0, 1.0],
            wander: 0,
            rest: 0,
            veer: 0.0,
            started: false,
            health: -1.0,
            start: [0.0; 3],
            diving: false,
            height: 1.5,
            turn: 0.35,
        }
    }
}

impl Fish {
    /// Its shoal's pull on its heading: their heading, toward them when it strays, away when too near.
    fn shoal(&self, m: &Body) -> [f64; 2] {
        let at = m.position.array();
        let fish = FISH.locked();
        let (mut heading, mut toward, mut n) = ([0.0, 0.0], [0.0, 0.0], 0.0);
        for (id, (p, d)) in fish.iter() {
            let dist = creature::body::distance(*p, at);
            if *id == m.id || dist > SHOAL {
                continue;
            }
            heading = [heading[0] + d[0], heading[1] + d[1]];
            let push = if dist < GAP { -1.0 } else { 0.15 };
            toward = [toward[0] + (p[0] - at[0]) * push, toward[1] + (p[2] - at[2]) * push];
            n += 1.0;
        }
        if n == 0.0 {
            return self.dir;
        }
        let blend = [
            self.dir[0] + heading[0] / n * 0.08 + toward[0] / n * 0.05,
            self.dir[1] + heading[1] / n * 0.08 + toward[1] / n * 0.05,
        ];
        unit(blend).unwrap_or(self.dir)
    }

    /// What frightens it this tick, as the heading to flee along.
    fn fright(&mut self, m: &mut Body, cx: &World<'_>) -> Option<[f64; 2]> {
        let hit = self.health >= 0.0 && m.health < self.health;
        self.health = m.health;
        let told = {
            let mut panic = PANIC.locked();
            match panic.get_mut(&m.id) {
                Some((0, away)) => {
                    let away = *away;
                    panic.remove(&m.id);
                    Some(away)
                }
                Some((wait, _)) => {
                    *wait -= 1;
                    None
                }
                None => None,
            }
        };
        if self.rest > 0 {
            self.rest -= 1;
            return None;
        }
        if told.is_some() {
            return told;
        }
        let at = m.position.array();
        let player = cx
            .player
            .as_ref()
            .filter(|p| p.alive && !p.spectator)
            .map(|p| p.position.array())
            .filter(|p| creature::body::distance(*p, at) < FRIGHT);
        let from = if hit { Some(player.unwrap_or(at)) } else { player };
        if let Some(from) = from {
            scare(at, from);
            let away = unit([at[0] - from[0], at[2] - from[2]]).unwrap_or(self.dir);
            PANIC.locked().remove(&m.id);
            return Some(away);
        }
        let for_nothing = !cx.raining && m.random.next_int_bound(FOR_NOTHING) == Some(0);
        for_nothing.then_some(self.dir)
    }

    fn swim(&mut self, m: &mut Body, cx: &World<'_>) {
        m.no_gravity = true;
        if std::mem::take(&mut self.diving) {
            creature::sounds::queue(m.id, SPLASH, 0.5, 1.1 + m.random.next_float() * 0.2);
        }
        if !std::mem::replace(&mut self.started, true) {
            let angle = f64::from(m.random.next_float()) * TAU;
            self.dir = [angle.cos(), angle.sin()];
        }
        // Frightened, it takes off along open water only (never toward a shore it would land on); hemmed in, it
        // swims off.
        if let Some(away) = self.fright(m, cx) {
            match open_heading(cx, m.position, away) {
                Some(dir) => {
                    self.dir = dir;
                    self.phase = Phase::Rise(RISE_TICKS);
                    self.rise(m, cx);
                    return;
                }
                None => self.dir = away,
            }
        }
        // It wanders: its heading veers this way or that, changing now and then, with a sharper turn at times.
        self.wander -= 1;
        if self.wander <= 0 {
            self.wander = 40 + m.random.next_int_bound(100).unwrap_or(0);
            self.veer = (f64::from(m.random.next_float()) * 2.0 - 1.0) * VEER;
            if m.random.next_int_bound(3) == Some(0) {
                let angle = (f64::from(m.random.next_float()) - 0.5) * 2.4;
                self.dir = turned(self.dir, angle);
            }
        }
        self.dir = turned(self.dir, self.veer);
        self.dir = self.shoal(m);
        // A wall ahead: it turns away.
        let ahead = Vec3 {
            x: m.position.x + self.dir[0] * 1.2,
            y: m.position.y + 0.2,
            z: m.position.z + self.dir[1] * 1.2,
        };
        if !water(cx, cell(ahead)) {
            let angle = TAU / 4.0 + f64::from(m.random.next_float()) * TAU / 4.0;
            self.dir = turned(self.dir, angle);
        }
        let depth = depth(m, cx);
        let up = if depth > DEPTH.1 {
            0.04
        } else if depth < DEPTH.0 {
            -0.03
        } else {
            0.0
        };
        m.velocity = Vec3 {
            x: self.dir[0] * SWIM_SPEED,
            y: up,
            z: self.dir[1] * SWIM_SPEED,
        };
        self.face(m, 0.0);
    }

    fn rise(&mut self, m: &mut Body, cx: &World<'_>) {
        let Phase::Rise(left) = self.phase else {
            return;
        };
        if !m.in_water {
            // Out: it spreads its fins and glides.
            self.start = m.position.array();
            self.height = GLIDE_HEIGHT.0 + f64::from(m.random.next_float()) * (GLIDE_HEIGHT.1 - GLIDE_HEIGHT.0);
            self.turn = if m.random.next_int_bound(2) == Some(0) {
                0.35
            } else {
                -0.35
            };
            let beats = if cx.raining {
                0
            } else {
                m.random.next_int_bound(BEATS + 1).unwrap_or(0)
            };
            self.phase = Phase::Glide(self.stretch(m, cx), beats);
            GLIDING.locked().insert(m.id);
            creature::sounds::queue(m.id, SPLASH, 0.6, 1.2 + m.random.next_float() * 0.2);
            creature::sounds::queue(m.id, GLIDE, 0.4, 0.9 + m.random.next_float() * 0.2);
            self.glide(m, cx);
            return;
        }
        if left <= 0 {
            self.phase = Phase::Swim;
            return;
        }
        self.phase = Phase::Rise(left - 1);
        m.no_gravity = true;
        m.velocity = Vec3 {
            x: self.dir[0] * RISE_SPEED.0,
            y: RISE_SPEED.1,
            z: self.dir[1] * RISE_SPEED.0,
        };
        self.face(m, -30.0);
    }

    /// A stretch of glide (ticks), shorter in the rain.
    fn stretch(&self, m: &mut Body, cx: &World<'_>) -> i32 {
        let t = m.random.between(GLIDE_TICKS);
        if cx.raining { t / 2 } else { t }
    }

    fn glide(&mut self, m: &mut Body, cx: &World<'_>) {
        let (left, beats, dipping) = match self.phase {
            Phase::Glide(l, b) => (l, b, false),
            Phase::Dip(l, b) => (l, b, true),
            _ => return,
        };
        m.no_gravity = true;
        m.fall_distance = 0.0;
        // It ran into a player or a mob: it bounces off and drops, unhurt.
        if let Some(away) = BUMPED.locked().remove(&m.id) {
            self.dir = away;
            return self.land(m, 0.15, 0.1);
        }
        let Some(surface) = surface_under(cx, m.position) else {
            // Over land (bounced off something) or too high: it drops back the way it came, toward the water.
            self.dir = [-self.dir[0], -self.dir[1]];
            return self.land(m, 0.2, -0.3);
        };
        // The shore ahead: it turns along open water, the harder the nearer; land right ahead, it drops in here.
        let over = |d: f64| {
            let at = Vec3 {
                x: m.position.x + self.dir[0] * d,
                y: m.position.y,
                z: m.position.z + self.dir[1] * d,
            };
            surface_under(cx, at).is_some()
        };
        let (near, mid, far) = (over(2.0), over(4.0), over(8.0));
        if !near {
            return self.land(m, 0.0, -0.3);
        }
        if !mid {
            self.dir = turned(self.dir, self.turn * 2.0);
        } else if !far {
            self.dir = turned(self.dir, self.turn);
        }
        // Stopped by a block: it drops.
        let moved = m.velocity.x.hypot(m.velocity.z);
        if !dipping && left < GLIDE_TICKS.0 - 1 && moved < GLIDE_SPEED * 0.3 {
            return self.land(m, 0.3, -0.3);
        }
        let target = surface + if dipping { 0.1 } else { self.height };
        let up = ((target - m.position.y) * 0.3).clamp(-0.25, 0.25);
        let speed = if dipping { GLIDE_SPEED * 0.75 } else { GLIDE_SPEED };
        m.velocity = Vec3 {
            x: self.dir[0] * speed,
            y: up,
            z: self.dir[1] * speed,
        };
        self.face(m, 0.0);
        self.phase = match (dipping, left, beats) {
            (_, l, b) if l > 1 => {
                if dipping {
                    Phase::Dip(l - 1, b)
                } else {
                    Phase::Glide(l - 1, b)
                }
            }
            (false, _, 0) => return self.land(m, 0.3, -0.3),
            (false, _, b) => {
                creature::sounds::queue(m.id, BEAT, 0.5, 1.0 + m.random.next_float() * 0.2);
                Phase::Dip(DIP_TICKS, b - 1)
            }
            (true, _, b) => {
                creature::sounds::queue(m.id, GLIDE, 0.4, 0.9 + m.random.next_float() * 0.2);
                Phase::Glide(self.stretch(m, cx), b)
            }
        };
    }

    /// The glide ends: it falls back into the water (or onto the ground, beached), going on at `ahead` and `up` blocks a
    /// tick.
    fn land(&mut self, m: &mut Body, ahead: f64, up: f64) {
        let at = m.position.array();
        let far = (at[0] - self.start[0]).hypot(at[2] - self.start[2]);
        if far > LONG_GLIDE {
            GLIDED.locked().push(m.id);
        }
        GLIDING.locked().remove(&m.id);
        self.phase = Phase::Swim;
        self.rest = REST;
        self.diving = true;
        m.no_gravity = false;
        m.velocity = Vec3 {
            x: self.dir[0] * ahead,
            y: up,
            z: self.dir[1] * ahead,
        };
        self.face(m, 20.0);
    }

    fn face(&self, m: &mut Body, pitch: f32) {
        let yaw = yaw(self.dir);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
        m.pitch = pitch;
    }
}

impl Behaviour for Fish {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        FISH.locked().insert(m.id, (m.position.array(), self.dir));
        m.in_water || self.phase != Phase::Swim
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        match self.phase {
            Phase::Swim => self.swim(m, cx),
            Phase::Rise(_) => self.rise(m, cx),
            Phase::Glide(..) | Phase::Dip(..) => self.glide(m, cx),
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        if self.phase != Phase::Swim {
            GLIDING.locked().remove(&m.id);
            self.phase = Phase::Swim;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_turns_and_keeps_its_length() {
        let t = turned([0.0, 1.0], TAU / 4.0);
        assert!((t[0] + 1.0).abs() < 1e-9 && t[1].abs() < 1e-9);
        assert!(unit([0.0, 0.0]).is_none());
        assert!((yaw([0.0, 1.0])).abs() < 1e-6);
    }

    #[test]
    fn a_fright_spreads_through_the_shoal() {
        for id in 900..906 {
            FISH.locked().insert(id, ([f64::from(id - 900), 60.0, 0.0], [0.0, 1.0]));
        }
        FISH.locked().insert(999, ([100.0, 60.0, 0.0], [0.0, 1.0]));
        scare([0.0, 60.0, 0.0], [-3.0, 60.0, 0.0]);
        let panic = PANIC.locked();
        assert!((900..906).all(|id| panic.get(&id).is_some_and(|(_, away)| away[0] > 0.5)));
        assert!(!panic.contains_key(&999));
        assert!(!took_off().is_empty());
    }
}
