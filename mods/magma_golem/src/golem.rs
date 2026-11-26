//! A magma golem's life and fight (SPECS §2), run from its server tick (`tick`), where the game can be asked to hurt,
//! set on fire and move: asleep as a mound of glowing rock, waking when a player comes near, hits it or mines by it,
//! fighting under a boss bar (a fist slam in reach, a ground pound against players close around it, a leap onto a
//! player farther off, a magma glob at range), healing in lava, enraged under half health, crusted over by snow or water, and lying down again after a
//! while without a target. What its drawing needs is kept by its id (`Shown`, single player: the renderer reads it).
use creature::state::{EntityMap, EntitySet};
use mod_api::{
    BlockState, BlockUpdateFlags, BossBarColor, BossBarOverlay, Compound, EntityRef, HostBlockStates, LevelRef,
    LivingRef, MobRef, ServerHost, SoundSource, Tag, Text,
};

use crate::{GOLEM, glob};

/// Waking and lying down (SPECS §2): a player within `WAKE_NEAR` blocks wakes it, rising for `RISE` ticks; it lies
/// down again after `IDLE` ticks without a target, over `RISE` ticks, its health restored.
const WAKE_NEAR: f64 = 6.0;
pub(crate) const RISE: i32 = 40;
const IDLE: i32 = 2400;
/// How near a mined block wakes it (blocks).
const MINED_NEAR: f64 = 4.0;
/// The fist slam: in `SLAM_REACH` blocks, its arm raised for `SLAM_WINDUP` ticks, then down; one every `SLAM_EVERY`
/// ticks. Its damage is its attack damage; the struck burn for `BURN` ticks.
const SLAM_REACH: f64 = 4.5;
pub(crate) const SLAM_WINDUP: i32 = 12;
const SLAM_EVERY: i32 = 40;
const BURN: i32 = 100;
/// The ground pound: players within `POUND_NEAR` blocks, both arms raised for `POUND_WINDUP` ticks (the warning), then
/// a shockwave of `POUND_DAMAGE` throwing them up and back, fire left around; one every `POUND_EVERY` ticks.
const POUND_NEAR: f64 = 6.0;
pub(crate) const POUND_WINDUP: i32 = 20;
const POUND_EVERY: i32 = 140;
const POUND_DAMAGE: f32 = 8.0;
const POUND_UP: f64 = 0.8;
const POUND_BACK: f64 = 1.2;
const POUND_FIRES: usize = 6;
/// Where its pound lights fires: a ring 3 blocks around it.
const RING: [(i32, i32); 8] = [(3, 0), (-3, 0), (0, 3), (0, -3), (2, 2), (-2, -2), (2, -2), (-2, 2)];
/// The magma glob: a target `GLOB_FROM` to `GLOB_TO` blocks away, torn off and held overhead for `GLOB_WINDUP` ticks,
/// then thrown from `GLOB_HEIGHT` blocks up at its chest (`GLOB_AIM` over its feet); one every `GLOB_EVERY` ticks.
const GLOB_FROM: f64 = 12.0;
const GLOB_TO: f64 = 24.0;
pub(crate) const GLOB_WINDUP: i32 = 32;
const GLOB_EVERY: i32 = 80;
const GLOB_HEIGHT: f64 = 8.5;
const GLOB_AIM: f64 = 1.0;
/// Its chase: straight at its target at `STRIDE` blocks a tick, turned to face it, over steps up to `STEP` blocks high
/// (its `minecraft:step_height`), stopping `CLOSE` blocks from it. A target more than `DIRECT_RISE` blocks above or
/// below, or a straight run that has come no nearer by `PROGRESS` blocks in `STALLED` ticks, and it follows a path
/// instead for `DETOUR` ticks (a new one every `REPATH`, at `CHASE` times its speed). Held still `BLOCKED` ticks on
/// the way, it jumps.
const STRIDE: f64 = 0.22;
pub(crate) const STEP: f64 = 2.0;
const CLOSE: f64 = 3.0;
const DIRECT_RISE: f64 = 3.0;
const PROGRESS: f64 = 1.0;
const STALLED: i32 = 30;
const DETOUR: i32 = 80;
const REPATH: i32 = 10;
const CHASE: f64 = 1.2;
const BLOCKED: i32 = 8;
const STILL: f64 = 0.03;
/// The leap: a target `LEAP_FROM` to `LEAP_TO` blocks away across (never nearer), a crouch of `LEAP_WINDUP` ticks, then a spring of
/// `LEAP_UP` blocks a tick up (more for a target up to `LEAP_RISE` blocks higher), at most `LEAP_FASTEST` across,
/// landing `LEAP_SHORT` blocks short of it; at most `LEAP_AIRTIME` ticks in the air. Its landing hurts the players
/// within `LEAP_REACH` blocks by `LEAP_DAMAGE`, throwing them `LEAP_BACK` across and `LEAP_THROW` up. One every
/// `LEAP_EVERY` ticks.
const LEAP_FROM: f64 = 9.0;
const LEAP_TO: f64 = 18.0;
pub(crate) const LEAP_WINDUP: i32 = 14;
const LEAP_UP: f64 = 0.9;
const LEAP_RISE: f64 = 6.0;
const LEAP_FASTEST: f64 = 1.6;
const LEAP_SHORT: f64 = 1.5;
const LEAP_AIRTIME: i32 = 60;
const LEAP_REACH: f64 = 4.0;
const LEAP_DAMAGE: f32 = 10.0;
const LEAP_BACK: f64 = 1.0;
const LEAP_THROW: f64 = 0.5;
const LEAP_EVERY: i32 = 300;
/// Healing in lava: health a tick (2 a second).
const LAVA_HEAL: f32 = 0.1;
/// Enraged under this share of its health: faster (its speed times `RAGE_SPEED`), its attacks' wind-ups shorter, and
/// each ground pound sheds 1 or 2 small magma cubes.
const RAGE: f32 = 0.5;
const RAGE_SPEED: f64 = 1.3;
/// Cooled by snow: `SNOW_DAMAGE` a snowball; `CRUST_HITS` within `CRUST_WITHIN` ticks crust it for `CRUST` ticks:
/// slowed, its damage taken doubled. A splash water bottle within `SPLASH_NEAR` blocks crusts it at once.
pub(crate) const SNOW_DAMAGE: f32 = 3.0;
const CRUST_HITS: usize = 5;
const CRUST_WITHIN: i32 = 80;
const CRUST: i32 = 120;
pub(crate) const SPLASH_NEAR: f64 = 4.0;
/// Who sees its boss bar (blocks).
const BAR_REACH: f64 = 48.0;

/// Its facts in the guide (SPECS §4).
pub(crate) const AWAKE_FACT: &str = "awake";
pub(crate) const CRUST_FACT: &str = "crust";
pub(crate) const LAVA_FACT: &str = "lava_bath";

/// Its sounds: vanilla's until its own are cut (SPECS §1).
const RUMBLE: &str = "minecraft:entity.ravager.roar";
const THUD: &str = "minecraft:entity.iron_golem.attack";
const POUND: &str = "minecraft:entity.generic.explode";
const HISS: &str = "minecraft:block.lava.extinguish";
const GRIND: &str = "minecraft:block.basalt.break";

/// What it is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Mode {
    /// Asleep, a mound of glowing rock.
    #[default]
    Dormant,
    /// Rising, so many ticks in.
    Waking(i32),
    Awake,
    /// Lying down, so many ticks in.
    LyingDown(i32),
}

/// Its attack under way, so many ticks in.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Attack {
    #[default]
    None,
    Slam(i32),
    Pound(i32),
    Glob(i32),
    /// Crouched, then in the air.
    Leap(i32),
}

/// A golem's state.
#[derive(Clone, Debug, Default)]
struct Golem {
    mode: Mode,
    attack: Attack,
    /// Ticks without a target, awake.
    idle: i32,
    /// Ticks until it may slam, pound, lob, leap again.
    slam: i32,
    pound: i32,
    glob: i32,
    leap: i32,
    /// Ticks in the air since it sprang (0 on the ground).
    aloft: i32,
    /// Its chase: ticks left following a path (0: straight), until its next path; its nearest to its target on this
    /// run and the ticks since; where it stood last tick and the ticks it has hardly moved.
    detour: i32,
    repath: i32,
    nearest: f64,
    stalled: i32,
    last: Option<[f64; 3]>,
    held: i32,
    enraged: bool,
    /// Its snow hits (the game's tick of each), and the ticks it stays crusted.
    snow: Vec<i64>,
    crusted: i32,
    /// Whether its no-AI flag was set for its sleep.
    still: bool,
}

static GOLEMS: EntityMap<Golem> = EntityMap::new();
/// Golems asked to wake (hurt, or a block mined by them), from the hooks.
static WAKE: EntitySet = EntitySet::new();
/// Golems crusted by a splash of water, from the hooks.
static SPLASHED: EntitySet = EntitySet::new();

/// What the drawing shows of a golem (single player: the renderer reads it by its id).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Shown {
    pub mode: Mode,
    pub attack: Attack,
    pub enraged: bool,
    pub crusted: bool,
    /// In the air, leaping.
    pub aloft: bool,
}

pub(crate) fn shown(id: u32) -> Shown {
    GOLEMS.locked().get(&id).map_or(Shown::default(), |g| Shown {
        mode: g.mode,
        attack: g.attack,
        enraged: g.enraged,
        crusted: g.crusted > 0,
        aloft: g.aloft > 0,
    })
}

/// From the hooks: golem `id` was hurt, or a block was mined by it: it wakes.
pub(crate) fn wake(id: u32) {
    WAKE.locked().insert(id);
}

/// From `living_incoming_damage`: its damage taken is doubled while it is crusted.
pub(crate) fn crusted(id: u32) -> bool {
    GOLEMS.locked().get(&id).is_some_and(|g| g.crusted > 0)
}

/// From `break_block`: a block mined at `pos` wakes the sleeping golems within `MINED_NEAR` of it.
pub(crate) fn mined(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    let at = [
        f64::from(pos[0]) + 0.5,
        f64::from(pos[1]) + 0.5,
        f64::from(pos[2]) + 0.5,
    ];
    for (id, _) in golems_near(host, level, at, MINED_NEAR + 3.0) {
        wake(id);
    }
}

/// The golems within `range` blocks of `at`: their ids and where they are.
pub(crate) fn golems_near(host: &dyn ServerHost, level: LevelRef, at: [f64; 3], range: f64) -> Vec<(u32, [f64; 3])> {
    let (from, to) = (
        [at[0] - range, at[1] - range, at[2] - range],
        [at[0] + range, at[1] + range, at[2] + range],
    );
    level
        .entities_in(host, from, to)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.kind(host).is_some_and(|k| k.id(host) == GOLEM))
        .filter_map(|e| Some((e.id(), e.position(host)?)))
        .filter(|(_, p)| creature::body::distance(*p, at) <= range)
        .collect()
}

/// From the snowball's impact (`projectile_impact`): it hurts, and enough in a row crust it.
pub(crate) fn snowball(host: &mut dyn ServerHost, id: u32, projectile: u32, owner: u32) {
    let (Some(e), Some(level)) = (host.entity(id), host.entity(id).and_then(|e| e.level(&*host))) else {
        return;
    };
    let now = level.game_time(&*host).unwrap_or(0);
    let (by, from) = (host.entity(projectile), host.entity(owner));
    e.hurt(host, "minecraft:thrown", SNOW_DAMAGE, by, from);
    let crust = {
        let mut all = GOLEMS.locked();
        let g = all.entry(id).or_default();
        g.snow.retain(|t| now - *t < i64::from(CRUST_WITHIN));
        g.snow.push(now);
        g.snow.len() >= CRUST_HITS
    };
    if crust {
        SPLASHED.locked().insert(id);
    }
}

/// From a splash potion's impact: a water bottle crusts the golems within `SPLASH_NEAR` blocks.
pub(crate) fn splash(host: &dyn ServerHost, level: LevelRef, at: [f64; 3]) {
    for (id, _) in golems_near(host, level, at, SPLASH_NEAR) {
        SPLASHED.locked().insert(id);
    }
}

/// Whether splash potion `id` holds water (its item's `potion_contents`).
pub(crate) fn is_water(host: &mut dyn ServerHost, id: u32) -> bool {
    let Some(nbt) = host.entity(id).and_then(|e| e.nbt(&mut *host)) else {
        return false;
    };
    let get = |c: &Compound, k: &str| c.get(&k.into()).cloned();
    let item = match get(&nbt, "Item") {
        Some(Tag::Compound(c)) => c,
        _ => return false,
    };
    let components = match get(&item, "components") {
        Some(Tag::Compound(c)) => c,
        _ => return false,
    };
    match get(&components, "minecraft:potion_contents") {
        Some(Tag::String(s)) => s.to_string_lossy() == "minecraft:water",
        Some(Tag::Compound(c)) => {
            matches!(get(&c, "potion"), Some(Tag::String(s)) if s.to_string_lossy() == "minecraft:water")
        }
        _ => false,
    }
}

/// From its server tick.
pub(crate) fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level), Some(living)) = (e.position(&*host), e.level(&*host), host.living(e)) else {
        return;
    };
    let Some(m) = host.mob(e) else {
        return;
    };
    // Dying: nothing more (its bar went at its death); it keeps its pose for its fall.
    if living.alive(&*host) == Some(false) {
        return;
    }
    let mut g = GOLEMS.locked().get(&id).cloned().unwrap_or_default();
    let woken = WAKE.locked().remove(&id);
    let splashed = SPLASHED.locked().remove(&id);
    let (health, max) = (
        living.health(&*host).unwrap_or(1.0),
        living.max_health(&*host).unwrap_or(1.0),
    );
    match g.mode {
        Mode::Dormant => {
            if !g.still {
                m.set_no_ai(host, true);
                g.still = true;
            }
            e.set_velocity(host, [0.0, -0.1, 0.0]);
            let near = player_within(&*host, dimension, at, WAKE_NEAR);
            if woken || near {
                g.mode = Mode::Waking(0);
                sound(host, level, at, RUMBLE, 2.0, 0.5);
            }
        }
        Mode::Waking(t) if t + 1 >= RISE => {
            g.mode = Mode::Awake;
            g.idle = 0;
            m.set_no_ai(host, false);
            g.still = false;
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                creature::bestiary::learn(host, player, GOLEM, AWAKE_FACT);
            }
        }
        Mode::Waking(t) => g.mode = Mode::Waking(t + 1),
        Mode::LyingDown(t) if t + 1 >= RISE => {
            g = Golem::default();
            living.set_health(host, max);
            set_speed(host, living, 1.0);
        }
        Mode::LyingDown(t) => {
            if !g.still {
                m.set_no_ai(host, true);
                g.still = true;
            }
            g.mode = Mode::LyingDown(t + 1);
        }
        Mode::Awake => fight(host, dimension, id, &mut g, at, level),
    }
    // Lava heals it; snow and water crust it.
    if e.is_in_lava(&*host) == Some(true) && health < max {
        living.heal(host, LAVA_HEAL);
        if (level.game_time(&*host).unwrap_or(0)) % 40 == 0 {
            sound(host, level, at, HISS, 0.8, 0.8);
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                creature::bestiary::learn(host, player, GOLEM, LAVA_FACT);
            }
        }
    }
    if splashed && g.mode != Mode::Dormant {
        g.crusted = CRUST;
        g.snow.clear();
        sound(host, level, at, HISS, 1.5, 0.6);
        creature::mob::give_effect_level(host, id, "minecraft:slowness", CRUST, 2);
        for player in creature::bestiary::players_near(&*host, dimension, at) {
            creature::bestiary::learn(host, player, GOLEM, CRUST_FACT);
        }
    }
    g.crusted = (g.crusted - 1).max(0);
    // Enraged under half health.
    let enraged = g.mode == Mode::Awake && health < max * RAGE;
    if enraged && !g.enraged {
        set_speed(host, living, RAGE_SPEED);
        sound(host, level, at, RUMBLE, 2.5, 0.7);
    }
    g.enraged = enraged || (g.enraged && g.mode == Mode::Awake);
    bar(host, id, &g, at, health / max.max(1.0));
    GOLEMS.locked().insert(id, g);
}

/// Its movement speed made `times` its base.
fn set_speed(host: &mut dyn ServerHost, living: LivingRef, times: f64) {
    living.set_attribute_base(host, "minecraft:movement_speed", crate::SPEED * times);
}

/// Awake: it goes for its target, and slams, pounds, leaps or lobs as it can; with no target for `IDLE` ticks it lies
/// down.
fn fight(host: &mut dyn ServerHost, dimension: &str, id: u32, g: &mut Golem, at: [f64; 3], level: LevelRef) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let Some(m) = host.mob(e) else {
        return;
    };
    g.slam = (g.slam - 1).max(0);
    g.pound = (g.pound - 1).max(0);
    g.glob = (g.glob - 1).max(0);
    g.leap = (g.leap - 1).max(0);
    // Its wind-ups: shorter enraged, longer crusted.
    let pace = |ticks: i32| {
        let t = if g.enraged { ticks * 7 / 10 } else { ticks };
        if g.crusted > 0 { t * 2 } else { t }
    };
    // Its target: the game's, or else the nearest player within its sight.
    if m.target(&*host).is_none()
        && let Some(player) = nearest_player(&*host, dimension, at, crate::SIGHT)
    {
        m.set_target(host, Some(player));
    }
    let target = m.target(&*host).and_then(|l| {
        let e = l.entity();
        let p = e.position(&*host)?;
        (l.alive(&*host) != Some(false)).then_some((e.id(), p))
    });
    // An attack under way goes on.
    match g.attack {
        Attack::Slam(t) if t >= pace(SLAM_WINDUP) => {
            if let Some((tid, p)) = target
                && creature::body::distance(p, at) <= SLAM_REACH + 1.0
            {
                slam(host, id, tid);
            }
            sound(host, level, at, THUD, 1.5, 0.6);
            g.attack = Attack::None;
            g.slam = SLAM_EVERY;
        }
        Attack::Pound(t) if t >= pace(POUND_WINDUP) => {
            pound(host, dimension, id, at, level, g.enraged);
            g.attack = Attack::None;
            g.pound = POUND_EVERY;
        }
        Attack::Glob(t) if t >= pace(GLOB_WINDUP) => {
            if let Some((tid, p)) = target {
                let from = [at[0], at[1] + GLOB_HEIGHT, at[2]];
                glob::throw(host, dimension, from, [p[0], p[1] + GLOB_AIM, p[2]], tid);
            }
            g.attack = Attack::None;
            g.glob = GLOB_EVERY;
        }
        // Crouched long enough: it springs at where its target stands.
        Attack::Leap(t) if g.aloft == 0 && t >= pace(LEAP_WINDUP) => {
            if let (Some((_, p)), Some(v)) = (target, e.velocity(&*host)) {
                let jump = leap_velocity(at, p);
                e.set_velocity(host, [jump[0], jump[1].max(v[1]), jump[2]]);
                sound(host, level, at, RUMBLE, 2.0, 1.1);
            }
            g.aloft = 1;
            g.attack = Attack::Leap(t + 1);
        }
        // In the air until it lands (or gives up): its landing shakes the ground.
        Attack::Leap(t) if g.aloft > 0 => {
            let landed = g.aloft > 3 && e.on_ground(&*host) == Some(true);
            if landed || g.aloft > LEAP_AIRTIME {
                if landed {
                    land(host, id, at, level);
                }
                g.attack = Attack::None;
                g.aloft = 0;
                g.leap = LEAP_EVERY;
            } else {
                g.aloft += 1;
                g.attack = Attack::Leap(t + 1);
            }
        }
        Attack::Slam(t) => g.attack = Attack::Slam(t + 1),
        Attack::Pound(t) => g.attack = Attack::Pound(t + 1),
        Attack::Glob(t) => g.attack = Attack::Glob(t + 1),
        Attack::Leap(t) => g.attack = Attack::Leap(t + 1),
        Attack::None => {}
    }
    let Some((_, p)) = target else {
        g.idle += 1;
        g.last = None;
        if g.idle >= IDLE && g.attack == Attack::None {
            g.mode = Mode::LyingDown(0);
            m.stop(host);
            sound(host, level, at, GRIND, 1.5, 0.5);
        }
        return;
    };
    g.idle = 0;
    m.look_at(host, [p[0], p[1] + 1.0, p[2]]);
    let d = creature::body::distance(p, at);
    let across = (p[0] - at[0]).hypot(p[2] - at[2]);
    let grounded = e.on_ground(&*host) == Some(true);
    if g.attack == Attack::None {
        if g.pound == 0 && player_within(&*host, dimension, at, POUND_NEAR) {
            g.attack = Attack::Pound(0);
            m.stop(host);
            sound(host, level, at, RUMBLE, 1.5, 0.9);
        } else if g.slam == 0 && d <= SLAM_REACH {
            g.attack = Attack::Slam(0);
            m.stop(host);
        } else if g.leap == 0 && grounded && (LEAP_FROM..=LEAP_TO).contains(&across) {
            g.attack = Attack::Leap(0);
            m.stop(host);
        } else if g.glob == 0 && (GLOB_FROM..=GLOB_TO).contains(&d) {
            g.attack = Attack::Glob(0);
            m.stop(host);
            glob::prepare(&*host);
            sound(host, level, at, GRIND, 1.2, 0.7);
        }
    }
    if g.attack != Attack::None || d <= CLOSE {
        g.last = None;
        return;
    }
    chase(host, g, e, m, at, p, across);
}

/// Its chase (`STRIDE`, `DETOUR`): straight at the target at `p`, `across` blocks off, or along a path when the
/// straight way fails.
fn chase(host: &mut dyn ServerHost, g: &mut Golem, e: EntityRef, m: MobRef, at: [f64; 3], p: [f64; 3], across: f64) {
    let pace = if g.crusted > 0 {
        0.5
    } else if g.enraged {
        RAGE_SPEED
    } else {
        1.0
    };
    // A new run (after an attack, or none before): its progress counted from here.
    if g.last.is_none() {
        g.nearest = across;
        g.stalled = 0;
    }
    // No nearer for a while on the straight way: a path round.
    if across < g.nearest - PROGRESS {
        g.nearest = across;
        g.stalled = 0;
    } else {
        g.stalled += 1;
    }
    if g.detour == 0 && ((p[1] - at[1]).abs() > DIRECT_RISE || g.stalled >= STALLED) {
        g.detour = DETOUR;
        g.repath = 0;
    }
    if g.detour > 0 {
        g.detour -= 1;
        g.repath = (g.repath - 1).max(0);
        if g.repath == 0 {
            m.move_to(host, p, CHASE * pace);
            g.repath = REPATH;
        }
        if g.detour == 0 {
            // Back to the straight way, its progress counted afresh.
            g.nearest = across;
            g.stalled = 0;
            m.stop(host);
        }
    } else if let Some(v) = e.velocity(&*host) {
        let (dx, dz) = ((p[0] - at[0]) / across.max(0.01), (p[2] - at[2]) / across.max(0.01));
        let stride = STRIDE * pace;
        e.set_velocity(host, [dx * stride, v[1], dz * stride]);
        e.set_rotation(host, creature::flight::heading_of(dx, dz), 0.0);
    }
    // Held still on the way (a wall its steps don't take): it jumps.
    let moved = g.last.map_or(1.0, |l| (at[0] - l[0]).hypot(at[2] - l[2]));
    g.last = Some(at);
    g.held = if moved < STILL { g.held + 1 } else { 0 };
    if g.held >= BLOCKED && e.on_ground(&*host) == Some(true) {
        m.jump(host);
        g.held = 0;
    }
}

/// The velocity that throws it from `from` onto `to`: up by `LEAP_UP` (more for a higher target), across as far as its
/// flight carries it (vanilla's: gravity 0.08 and drag 0.98 down, drag 0.91 across), landing just short.
fn leap_velocity(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    let rise = (to[1] - from[1]).clamp(0.0, LEAP_RISE);
    let up = LEAP_UP + rise * 0.06;
    // Its time in the air, until it is back down to its target's height.
    let (mut y, mut vy, mut ticks) = (0.0, up, 0);
    while ticks < 100 && (vy > 0.0 || y > rise) {
        y += vy;
        vy = (vy - 0.08) * 0.98;
        ticks += 1;
    }
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let across = dx.hypot(dz).max(0.01);
    let carried = (1.0 - 0.91f64.powi(ticks)) / 0.09;
    let speed = ((across - LEAP_SHORT).max(0.0) / carried).min(LEAP_FASTEST);
    [dx / across * speed, up, dz / across * speed]
}

/// Its landing: the players within `LEAP_REACH` hurt and thrown up and back, the ground shaking.
fn land(host: &mut dyn ServerHost, id: u32, at: [f64; 3], level: LevelRef) {
    let Some(me) = host.entity(id) else {
        return;
    };
    sound(host, level, at, POUND, 1.6, 0.5);
    sound(host, level, at, THUD, 2.0, 0.5);
    for (particle, count, spread) in [("minecraft:explosion", 4, 1.5), ("minecraft:large_smoke", 24, 1.5)] {
        creature::particles::puff(host, level, particle, [at[0], at[1] + 0.3, at[2]], spread, count);
    }
    let players: Vec<u32> = host
        .players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(&*host) == Some(level) && p.is_spectator(&*host) != Some(true))
        .filter(|p| p.is_creative(&*host) != Some(true))
        .filter_map(|p| {
            let e = p.entity();
            let pos = e.position(&*host)?;
            (creature::body::distance(pos, at) <= LEAP_REACH).then_some(e.id())
        })
        .collect();
    for target in players {
        if let Some(t) = host.entity(target) {
            t.hurt(host, "minecraft:mob_attack", LEAP_DAMAGE, Some(me), Some(me));
        }
        push(host, id, target, LEAP_BACK, LEAP_THROW);
    }
}

/// The fist comes down on `target`: its attack damage, a push, set on fire.
fn slam(host: &mut dyn ServerHost, golem: u32, target: u32) {
    let (Some(me), Some(t)) = (host.entity(golem), host.entity(target)) else {
        return;
    };
    let damage = host
        .living(me)
        .and_then(|l| l.attribute_value(&*host, "minecraft:attack_damage"))
        .unwrap_or(crate::ATTACK_DAMAGE);
    t.hurt(host, "minecraft:mob_attack", damage as f32, Some(me), Some(me));
    t.set_remaining_fire_ticks(host, BURN);
    push(host, golem, target, 0.8, 0.3);
}

/// Both fists on the ground: every player within `POUND_NEAR` is hurt, thrown up and back and set on fire; fire is
/// left around; enraged, 1 or 2 small magma cubes are shed.
fn pound(host: &mut dyn ServerHost, dimension: &str, id: u32, at: [f64; 3], level: LevelRef, enraged: bool) {
    let Some(me) = host.entity(id) else {
        return;
    };
    sound(host, level, at, POUND, 2.0, 0.6);
    if let Some(k) = creature::particles::key("minecraft:explosion") {
        level.send_particles(
            host,
            &mod_api::ParticleOptions::simple(k),
            None,
            at,
            [2.0, 0.2, 2.0],
            0.0,
            8,
        );
    }
    let players: Vec<u32> = host
        .players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(&*host) == Some(level) && p.is_spectator(&*host) != Some(true))
        .filter(|p| p.is_creative(&*host) != Some(true))
        .filter_map(|p| {
            let e = p.entity();
            let pos = e.position(&*host)?;
            (creature::body::distance(pos, at) <= POUND_NEAR).then_some(e.id())
        })
        .collect();
    for target in players {
        if let Some(t) = host.entity(target) {
            t.hurt(host, "minecraft:mob_attack", POUND_DAMAGE, Some(me), Some(me));
            t.set_remaining_fire_ticks(host, BURN);
        }
        push(host, id, target, POUND_BACK, POUND_UP);
    }
    fires(host, level, at, &RING, POUND_FIRES);
    if enraged {
        let n = 1 + (level.game_time(&*host).unwrap_or(0) % 2) as usize;
        for k in 0..n {
            let off = if k == 0 { 2.0 } else { -2.0 };
            let mut nbt = Compound::new();
            nbt.insert("Size".into(), Tag::Int(0));
            host.summon(
                dimension,
                "minecraft:magma_cube",
                [at[0] + off, at[1] + 0.5, at[2] + off],
                nbt,
            );
        }
    }
}

/// Fire on the open blocks over the ground at the `spots` around `at` (offsets across), up to `most`.
pub(crate) fn fires(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3], spots: &[(i32, i32)], most: usize) {
    let Some(key) = host.block_key("minecraft:fire") else {
        return;
    };
    let Ok(fire) = BlockState::of(&host.block_states(), key, &[]) else {
        return;
    };
    let ground = creature::territory::Ground::new(&*host, level);
    let base = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    let mut lit = Vec::new();
    for &(dx, dz) in spots {
        for dy in [0, 1, -1] {
            let p = [base[0] + dx, base[1] + dy, base[2] + dz];
            let below = ground.block([p[0], p[1] - 1, p[2]]);
            if ground.block(p) == "minecraft:air" && !below.ends_with("air") && !below.contains("lava") {
                lit.push(p);
                break;
            }
        }
    }
    for p in lit.into_iter().take(most) {
        let _ = level.set_block(host, p, fire, BlockUpdateFlags::CLIENTS | BlockUpdateFlags::NEIGHBORS);
    }
}

/// `target` pushed away from the golem, `back` across and `up`.
fn push(host: &mut dyn ServerHost, golem: u32, target: u32, back: f64, up: f64) {
    let (Some(me), Some(t)) = (host.entity(golem), host.entity(target)) else {
        return;
    };
    if let (Some(a), Some(b), Some(v)) = (me.position(&*host), t.position(&*host), t.velocity(&*host)) {
        let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
        let len = dx.hypot(dz).max(0.01);
        t.set_velocity(host, [v[0] + dx / len * back, v[1] + up, v[2] + dz / len * back]);
    }
}

/// The nearest player (not a spectator, not in creative) within `range` blocks of `at`.
fn nearest_player(host: &dyn ServerHost, dimension: &str, at: [f64; 3], range: f64) -> Option<LivingRef> {
    let level = host.level(dimension)?;
    host.players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(host) == Some(level) && p.is_spectator(host) != Some(true))
        .filter(|p| p.is_creative(host) != Some(true))
        .filter_map(|p| {
            let d = creature::body::distance(p.entity().position(host)?, at);
            Some((d, host.living(p.entity())?))
        })
        .filter(|(d, _)| *d <= range)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, l)| l)
}

/// Whether a player (not a spectator, not in creative) is within `range` blocks of `at`.
fn player_within(host: &dyn ServerHost, dimension: &str, at: [f64; 3], range: f64) -> bool {
    let Some(level) = host.level(dimension) else {
        return false;
    };
    host.players().server_players().into_iter().any(|p| {
        p.level(host) == Some(level)
            && p.is_spectator(host) != Some(true)
            && p.is_creative(host) != Some(true)
            && p.entity()
                .position(host)
                .is_some_and(|pos| creature::body::distance(pos, at) <= range)
    })
}

fn sound(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3], sound: &str, volume: f32, pitch: f32) {
    level.play_sound(host, sound, at, SoundSource::Hostile, volume, pitch, None);
}

/// Its boss bar's id.
fn bar_id(id: u32) -> String {
    format!("magma_golem:golem_{id}")
}

/// Its boss bar while awake: its health, shown to the players within `BAR_REACH` blocks; none asleep.
fn bar(host: &mut dyn ServerHost, id: u32, g: &Golem, at: [f64; 3], health: f32) {
    let name = bar_id(id);
    let awake = matches!(g.mode, Mode::Awake | Mode::Waking(_));
    let existing = host.boss_bar(&name);
    if !awake {
        if let Some(b) = existing {
            b.remove(host);
        }
        return;
    }
    let b = match existing {
        Some(b) => b,
        None => {
            let Ok(Some(b)) = host.create_boss_bar(&name, Text::translate("entity.magma_golem.magma_golem", vec![]))
            else {
                return;
            };
            b.set_color(host, BossBarColor::Red);
            b.set_overlay(host, BossBarOverlay::Progress);
            b.set_max(host, 1000);
            b
        }
    };
    b.set_value(host, (health.clamp(0.0, 1.0) * 1000.0) as i32);
    let players = host.players().server_players();
    for p in players {
        let near = p
            .entity()
            .position(&*host)
            .is_some_and(|pos| creature::body::distance(pos, at) <= BAR_REACH);
        let has = b.players(&*host).is_some_and(|all| all.contains(&p));
        if near && !has {
            b.add_player(host, p);
        } else if !near && has {
            b.remove_player(host, p);
        }
    }
}

/// From a level tick now and then: the boss bars of golems no longer awake here (dead, gone, or the game closed while
/// one fought: the bars are saved with the world) are removed.
pub(crate) fn sweep_bars(host: &mut dyn ServerHost) {
    let awake: Vec<String> = GOLEMS
        .locked()
        .iter()
        .filter(|(_, g)| matches!(g.mode, Mode::Awake | Mode::Waking(_)))
        .map(|(id, _)| bar_id(*id))
        .collect();
    for b in host.boss_bars() {
        let Some(name) = b.id(&*host) else {
            continue;
        };
        if name.starts_with("magma_golem:golem_") && !awake.contains(&name) {
            b.remove(host);
        }
    }
}

/// From `living_death`: its boss bar goes at once.
pub(crate) fn died(host: &mut dyn ServerHost, id: u32) {
    if let Some(b) = host.boss_bar(&bar_id(id)) {
        b.remove(host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Its flight, as vanilla moves a mob through the air: where it lands at its start's height.
    fn lands(v: [f64; 3]) -> f64 {
        let (mut p, mut v) = ([0.0f64; 3], v);
        loop {
            for k in 0..3 {
                p[k] += v[k];
            }
            v = [v[0] * 0.91, (v[1] - 0.08) * 0.98, v[2] * 0.91];
            if v[1] < 0.0 && p[1] <= 0.0 {
                return p[0].hypot(p[2]);
            }
        }
    }

    #[test]
    fn its_leap_lands_just_short_of_its_target() {
        for across in [6.0, 10.0, 14.0] {
            let v = leap_velocity([0.0; 3], [across, 0.0, 0.0]);
            let landed = lands(v);
            assert!(
                (landed - (across - LEAP_SHORT)).abs() < 0.6,
                "{across}: landed at {landed}"
            );
        }
    }
}
