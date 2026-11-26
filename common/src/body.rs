//! Behaviours that move a creature by hand (SPECS §3.5, §6): flight, following, carrying.
//!
//! On Sunrise a mob's goals can only steer through the engine (`GoalContext::move_to`, `look_at`, `jump`), which is
//! right for walking (`crawl`) but not for a butterfly's flutter or a vulture's flight, which set the mob's velocity
//! and facing themselves. Those run here instead, from the mod's `entity_tick_post` with a server host: each tick the
//! library reads the mob into a `Body` (where it is, how it moves, which way it faces), runs its behaviours on it, and
//! writes back what they changed (`EntityRef::set_velocity`, `set_rotation`, `set_no_gravity`, `reset_fall_distance`).
//!
//! A mod gives each of its creatures its behaviours on its first tick (`tick`'s `make`), in priority order: the first
//! that can run each tick runs (a lower priority number first), a passive one (`claims` false) runs every tick beside
//! it. They are not saved, as goals aren't: what must outlive a reload goes in attachments.

use mod_api::{Host, LivingRef, ServerHost};

use crate::colours::Random;
use crate::state::EntityMap;
use crate::territory::Ground;

/// A position or a motion, in blocks (a tick).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn of(a: [f64; 3]) -> Vec3 {
        Vec3 {
            x: a[0],
            y: a[1],
            z: a[2],
        }
    }
    pub fn array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

/// The distance between two points, in blocks.
pub fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// The distance between two points across, leaving their heights out, in blocks.
pub fn flat_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

/// A heading across (x, z) turned by `angle` radians.
pub fn turned(v: [f64; 2], angle: f64) -> [f64; 2] {
    let (s, c) = angle.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

/// `v` made a unit heading, if it has a length.
pub fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let n = v[0].hypot(v[1]);
    (n > 1e-6).then(|| [v[0] / n, v[1] / n])
}

/// A creature's own random numbers, kept from tick to tick.
#[derive(Clone, Debug)]
pub struct Rand(Random);

impl Rand {
    pub fn new(seed: u32) -> Rand {
        Rand(Random::new(seed))
    }
    /// 0 to 1.
    pub fn next_float(&mut self) -> f32 {
        (self.0.next() % 1_000_000) as f32 / 1_000_000.0
    }
    /// 0 to `bound` - 1; `None` for a bound under 1.
    pub fn next_int_bound(&mut self, bound: i32) -> Option<i32> {
        (bound > 0).then(|| (self.0.next() % bound as u64) as i32)
    }
    /// From `range.0` to under `range.1` (ticks, say); `range.0` for an empty range.
    pub fn between(&mut self, range: (i32, i32)) -> i32 {
        range.0 + self.next_int_bound(range.1 - range.0).unwrap_or(0)
    }
}

/// A creature this tick, as its behaviours read and change it.
#[derive(Clone, Debug)]
pub struct Body {
    pub id: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    /// Which way it faces (degrees, 0 south): its body and head follow `yaw` unless set apart.
    pub yaw: f32,
    pub body_yaw: f32,
    pub head_yaw: f32,
    /// Its pitch (degrees, positive nose down): a flier's, drawn by its renderer (`x_rot`).
    pub pitch: f32,
    pub on_ground: bool,
    pub in_water: bool,
    pub no_gravity: bool,
    /// Set to 0 to spare it the fall it is in (a 1-health flier).
    pub fall_distance: f32,
    /// Ticks it has lived (`Entity.tickCount`).
    pub age: i32,
    /// Asks the engine to walk it there, at this multiplier of its speed (pathfinding, as a goal's `move_to`).
    pub walk_to: Option<([f64; 3], f64)>,
    /// Asks the engine to stop the walk it is on.
    pub stop_walking: bool,
    /// Asks its head to turn towards this point.
    pub look_at: Option<[f64; 3]>,
    /// Its health and its maximum (`minecraft:max_health`), both written back when changed.
    pub health: f32,
    pub max_health: f32,
    pub random: Rand,
}

/// What a behaviour sees around its creature this tick.
pub struct World<'a> {
    pub ground: Ground<'a>,
    pub raining: bool,
    pub daylight: bool,
    /// The nearest player in its level.
    pub player: Option<Player>,
}

/// A player as a behaviour sees it.
#[derive(Clone, Debug)]
pub struct Player {
    pub entity_id: u32,
    pub uuid: u128,
    pub position: Vec3,
    /// Its eyes, and the way it looks (a unit vector).
    pub eye: [f64; 3],
    pub look: [f64; 3],
    pub sprinting: bool,
    pub sneaking: bool,
    pub alive: bool,
    pub spectator: bool,
    pub creative: bool,
    /// The items in its hands (`minecraft:glow_berries`), none for an empty hand.
    pub main_hand: Option<String>,
    pub off_hand: Option<String>,
}

impl World<'_> {
    pub fn host(&self) -> &dyn Host {
        self.ground.host
    }
    /// The block at `p` (`minecraft:farmland`), empty where it can't be read.
    pub fn block(&self, p: [i32; 3]) -> String {
        self.ground.block(p)
    }
    /// The block light at `p` (a torch's own block 14, its neighbours less), 0 for none.
    pub fn light_emission(&self, p: [i32; 3]) -> u8 {
        let host = self.ground.host;
        self.ground
            .level
            .brightness(host, mod_api::LightLayer::Block, p)
            .map_or(0, |l| l.clamp(0, 15) as u8)
    }
}

/// What a creature does by hand, as a goal would: `can_use` decides whether it runs this tick, `tick` moves it.
pub trait Behaviour: Send {
    /// Whether it takes control (only one of those runs a tick); false for one that only watches (runs every tick).
    fn claims(&self) -> bool {
        true
    }
    fn can_use(&mut self, _: &mut Body, _: &World<'_>) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Body, world: &World<'_>);
    /// It stopped running (another took over).
    fn stop(&mut self, _: &mut Body, _: &World<'_>) {}
}

struct Brain {
    behaviours: Vec<Box<dyn Behaviour>>,
    running: Option<usize>,
    random: Rand,
}

/// Each creature's brain, by id. A brain is taken out (`None` in its place) while its behaviours run, so that they run
/// with no lock held: they may call the game, the library (`prey::quarry`), or see their creature forgotten
/// (`crate::state::forget`, when it leaves the level).
static BRAINS: EntityMap<Option<Brain>> = EntityMap::new();

/// From the mod's `entity_tick_post`, for each of its creatures moved by hand: runs its behaviours (`make` gives them,
/// in priority order, the first time) and writes back what they changed.
pub fn tick(host: &mut dyn ServerHost, entity: u32, make: impl FnOnce() -> Vec<Box<dyn Behaviour>>) {
    let Some(mut body) = read(&*host, entity) else {
        return;
    };
    // Dead (its death's last ticks, until it leaves the level): its brain, dropped at its death, isn't made again.
    if !alive(&*host, entity) {
        forget(entity);
        return;
    }
    let before = body.clone();
    {
        let player = nearest_player(host, &body);
        let Some(mut world) = world(&*host, entity) else {
            return;
        };
        world.player = player;
        let Some(mut brain) = Taken::take(entity, make) else {
            return;
        };
        think(&mut brain, &mut body, &world);
    }
    write(host, &before, &body);
}

/// Runs a creature's behaviours on `body` for this tick: the passive ones, then the first that claims it.
fn think(brain: &mut Brain, body: &mut Body, world: &World<'_>) {
    std::mem::swap(&mut body.random, &mut brain.random);
    let mut chosen = None;
    for i in 0..brain.behaviours.len() {
        let b = &mut brain.behaviours[i];
        if !b.claims() {
            if b.can_use(body, world) {
                b.tick(body, world);
            }
        } else if chosen.is_none() && b.can_use(body, world) {
            chosen = Some(i);
        }
    }
    if brain.running != chosen
        && let Some(old) = brain.running
    {
        brain.behaviours[old].stop(body, world);
    }
    brain.running = chosen;
    if let Some(i) = chosen {
        brain.behaviours[i].tick(body, world);
    }
    std::mem::swap(&mut body.random, &mut brain.random);
}

/// A creature's brain, out of `BRAINS` while it thinks, put back when dropped: unless the creature was forgotten
/// meanwhile, or a behaviour panicked (its brain may be half changed: the creature gets new behaviours next tick).
struct Taken {
    entity: u32,
    brain: Option<Brain>,
}

impl Taken {
    /// Creature `entity`'s brain, made by `make` the first time; none while it is already out (a behaviour of its own
    /// ticking it again).
    fn take(entity: u32, make: impl FnOnce() -> Vec<Box<dyn Behaviour>>) -> Option<Taken> {
        let known = BRAINS.locked().get_mut(&entity).map(Option::take);
        let brain = match known {
            Some(Some(brain)) => brain,
            Some(None) => return None,
            // The mod's behaviours are made with no lock held.
            None => {
                let brain = Brain {
                    behaviours: make(),
                    running: None,
                    random: Rand::new(entity),
                };
                BRAINS.locked().insert(entity, None);
                brain
            }
        };
        Some(Taken {
            entity,
            brain: Some(brain),
        })
    }
}

impl std::ops::Deref for Taken {
    type Target = Brain;
    fn deref(&self) -> &Brain {
        self.brain.as_ref().expect("a brain until dropped")
    }
}

impl std::ops::DerefMut for Taken {
    fn deref_mut(&mut self) -> &mut Brain {
        self.brain.as_mut().expect("a brain until dropped")
    }
}

impl Drop for Taken {
    fn drop(&mut self) {
        let mut brains = BRAINS.locked();
        if std::thread::panicking() {
            brains.remove(&self.entity);
        } else if let Some(slot) = brains.get_mut(&self.entity) {
            *slot = self.brain.take();
        }
    }
}

/// A dead creature's behaviours are dropped (`tick`); what it left is dropped with it when it leaves the level
/// (`crate::state`), and a brain forgotten while it thinks isn't put back (`Taken`).
fn forget(entity: u32) {
    BRAINS.locked().remove(&entity);
}

/// Whether creature `entity` is alive (one that isn't a living entity always is).
fn alive(host: &dyn Host, entity: u32) -> bool {
    host.entity(entity)
        .and_then(|e| host.living(e))
        .and_then(|l| l.alive(host))
        .unwrap_or(true)
}

/// The creature as it is now.
pub fn read(host: &dyn Host, entity: u32) -> Option<Body> {
    let e = host.entity(entity)?;
    let [yaw, pitch] = e.rotation(host).unwrap_or([0.0, 0.0]);
    let head_yaw = e.head_yaw(host).unwrap_or(yaw);
    Some(Body {
        id: entity,
        position: Vec3::of(e.position(host)?),
        velocity: Vec3::of(e.velocity(host).unwrap_or([0.0; 3])),
        yaw,
        body_yaw: yaw,
        head_yaw,
        pitch,
        on_ground: e.on_ground(host).unwrap_or(false),
        in_water: e.is_in_water(host).unwrap_or(false),
        no_gravity: e.no_gravity(host).unwrap_or(false),
        fall_distance: 1.0,
        age: e.tick_count(host).unwrap_or(0),
        walk_to: None,
        stop_walking: false,
        look_at: None,
        health: host.living(e).and_then(|l| l.health(host)).unwrap_or(1.0),
        max_health: host.living(e).and_then(|l| l.max_health(host)).unwrap_or(1.0),
        random: Rand::new(entity),
    })
}

/// The world around creature `entity` this tick (its `player` filled by `nearest_player`).
fn world<'a>(host: &'a dyn Host, entity: u32) -> Option<World<'a>> {
    let ground = Ground::of_entity(host, entity)?;
    let level = ground.level;
    let day = level.day_time(host).unwrap_or(0).rem_euclid(24_000) < 12_500;
    let raining = level.weather(host).is_some_and(|(rain, _)| rain > 0.2);
    Some(World {
        ground,
        raining,
        daylight: day,
        player: None,
    })
}

/// The player nearest to `body` in its level.
fn nearest_player(host: &dyn ServerHost, body: &Body) -> Option<Player> {
    let level = host.entity(body.id)?.level(host)?;
    host.players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(host) == Some(level))
        .filter_map(|p| {
            let e = p.entity();
            let at = Vec3::of(e.position(host)?);
            let d =
                (at.x - body.position.x).powi(2) + (at.y - body.position.y).powi(2) + (at.z - body.position.z).powi(2);
            Some((d, p, at))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p, at)| {
            let e = p.entity();
            let (eye, look) =
                crate::hand::eye_and_look(host, p).unwrap_or(([at.x, at.y + 1.62, at.z], [0.0, 0.0, 1.0]));
            let hand = |h| crate::hand::held(host, p, h).map(|s| crate::hand::id(host, &s));
            Player {
                entity_id: e.id(),
                uuid: p.uuid(),
                position: at,
                eye,
                look,
                sprinting: e.is_sprinting(host).unwrap_or(false),
                sneaking: e.is_crouching(host).unwrap_or(false),
                alive: host.living(e).and_then(|l: LivingRef| l.alive(host)).unwrap_or(true),
                spectator: p.is_spectator(host).unwrap_or(false),
                creative: p.is_creative(host).unwrap_or(false),
                main_hand: hand(mod_api::Hand::Main),
                off_hand: hand(mod_api::Hand::Off),
            }
        })
}

/// Writes back what the behaviours changed.
fn write(host: &mut dyn ServerHost, before: &Body, after: &Body) {
    let Some(e) = host.entity(after.id) else {
        return;
    };
    if after.velocity != before.velocity {
        e.set_velocity(host, after.velocity.array());
    }
    if after.yaw != before.yaw || after.body_yaw != before.body_yaw || after.pitch != before.pitch {
        e.set_rotation(host, after.yaw, after.pitch);
    }
    if after.head_yaw != before.head_yaw {
        e.set_head_yaw(host, after.head_yaw);
    }
    if after.no_gravity != before.no_gravity {
        e.set_no_gravity(host, after.no_gravity);
    }
    if after.fall_distance == 0.0 {
        e.reset_fall_distance(host);
    }
    if after.position != before.position {
        e.teleport_to(host, after.position.array(), None);
    }
    if let Some(l) = host.living(e) {
        if after.max_health != before.max_health {
            l.set_attribute_base(host, "minecraft:max_health", f64::from(after.max_health));
        }
        if after.health != before.health {
            l.set_health(host, after.health.min(after.max_health));
        }
    }
    if let Some(m) = host.mob(e) {
        if after.stop_walking {
            m.stop(host);
        }
        if let Some((at, speed)) = after.walk_to {
            m.move_to(host, at, speed);
        }
        if let Some(at) = after.look_at {
            m.look_at(host, at);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use mod_api::testing::FakeHost;

    use super::*;
    use crate::sync::Locked;

    /// A creature standing still at the origin, for the library's tests.
    pub(crate) fn body(id: u32) -> Body {
        Body {
            id,
            position: Vec3::default(),
            velocity: Vec3::default(),
            yaw: 0.0,
            body_yaw: 0.0,
            head_yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            in_water: false,
            no_gravity: false,
            fall_distance: 1.0,
            age: 0,
            walk_to: None,
            stop_walking: false,
            look_at: None,
            health: 1.0,
            max_health: 1.0,
            random: Rand::new(id),
        }
    }

    /// `tick` without the game: creature `id`'s brain thinks once on a body of its own.
    fn think_once(id: u32, make: impl FnOnce() -> Vec<Box<dyn Behaviour>>) -> Option<Body> {
        let host = FakeHost::new();
        let world = World {
            ground: Ground::new(&host, host.level.level()),
            raining: false,
            daylight: true,
            player: None,
        };
        let mut b = body(id);
        let mut brain = Taken::take(id, make)?;
        think(&mut brain, &mut b, &world);
        Some(b)
    }

    fn out(id: u32) -> Option<bool> {
        BRAINS.locked().get(&id).map(Option::is_none)
    }

    /// Reaches the brains while it runs: under the lock, as before, it would wait for itself forever.
    struct Busy;
    impl Behaviour for Busy {
        fn tick(&mut self, m: &mut Body, _: &World<'_>) {
            assert_eq!(out(m.id), Some(true), "its brain is out while it runs");
            assert!(Taken::take(m.id, Vec::new).is_none(), "and not taken twice");
            forget(m.id + 1);
            m.velocity.y = 1.0;
        }
    }

    #[test]
    fn behaviours_run_with_no_lock_held_and_their_brain_goes_back() {
        let id = 1_000_000;
        let b = think_once(id, || vec![Box::new(Busy)]).unwrap();
        assert_eq!(b.velocity.y, 1.0);
        assert_eq!(out(id), Some(false), "put back");
        forget(id);
    }

    struct Forgets;
    impl Behaviour for Forgets {
        fn tick(&mut self, m: &mut Body, _: &World<'_>) {
            forget(m.id);
        }
    }

    #[test]
    fn a_creature_forgotten_while_it_thinks_is_not_put_back() {
        let id = 1_000_010;
        think_once(id, || vec![Box::new(Forgets)]).unwrap();
        assert_eq!(out(id), None);
    }

    /// Runs while it can (`left` ticks), counting its stops: a brain's state lives from tick to tick.
    struct Counted {
        left: u32,
        ticks: std::sync::Arc<Mutex<(u32, u32)>>,
    }
    impl Behaviour for Counted {
        fn can_use(&mut self, _: &mut Body, _: &World<'_>) -> bool {
            self.left > 0
        }
        fn tick(&mut self, _: &mut Body, _: &World<'_>) {
            self.left -= 1;
            self.ticks.locked().0 += 1;
        }
        fn stop(&mut self, _: &mut Body, _: &World<'_>) {
            self.ticks.locked().1 += 1;
        }
    }

    #[test]
    fn a_brain_keeps_its_behaviours_and_which_one_runs() {
        let id = 1_000_020;
        let ticks = std::sync::Arc::new(Mutex::new((0, 0)));
        let made = std::sync::atomic::AtomicU32::new(0);
        for _ in 0..4 {
            think_once(id, || {
                made.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                vec![Box::new(Counted {
                    left: 2,
                    ticks: ticks.clone(),
                })]
            })
            .unwrap();
        }
        assert_eq!(made.into_inner(), 1, "made once");
        assert_eq!(*ticks.locked(), (2, 1), "two ticks, then stopped once");
        forget(id);
    }

    struct Panics;
    impl Behaviour for Panics {
        fn tick(&mut self, _: &mut Body, _: &World<'_>) {
            panic!("a behaviour's fault");
        }
    }

    #[test]
    fn a_brain_whose_behaviour_panicked_is_dropped_and_its_lock_stays_usable() {
        let id = 1_000_030;
        let caught = std::panic::catch_unwind(|| think_once(id, || vec![Box::new(Panics)]));
        assert!(caught.is_err());
        assert!(!BRAINS.is_poisoned(), "the panic held no lock");
        assert_eq!(out(id), None, "made again next tick");
    }
}
