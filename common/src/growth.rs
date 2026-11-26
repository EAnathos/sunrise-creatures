//! Babies, growth and breeding (SPECS §6.3): a creature's age, saved on it (`creature:age`): negative while young,
//! counting up to 0 (grown), and after breeding a pause counting down to 0. Fed in love by its owner (the mod's
//! right-click, `fall_in_love`), a grown tamed creature looks for another of its kind in love (`Mate`); close, they have
//! a baby, tamed to the first one's owner and inheriting their stats (`creature::stats`), and wait. Feeding a young one
//! makes it grow faster (`feed_young`). Users: the Vulture, the Elephant.
//!
//! A mod calls `tick` from its creature's tick (its age read the first time, or taken from an older save of the mod's,
//! `legacy`), `births` from its kind's tick, and reads `age`, `growth`, `is_young`.
use std::sync::Mutex;

use mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, ServerHost, Tag};

use crate::body::{Behaviour, Body, World};
use crate::state::{EntityMap, EntitySet};
use crate::sync::Locked;

/// A kind's breeding and growth: its id, how long its young take to grow up, its parents' pause after a baby, how
/// long love lasts, how much a meal speeds a young one (ticks); how far it looks for a partner, how near they breed
/// (blocks), how fast it walks to them (a share of its speed); its stats' ranges, for a baby's.
#[derive(Clone, Copy, Debug)]
pub struct Breeding {
    pub kind: &'static str,
    pub young: i32,
    pub pause: i32,
    pub love: i32,
    pub meal: i32,
    pub search: f64,
    pub near: f64,
    pub pace: f64,
    pub stats: Option<&'static crate::stats::Ranges>,
}

impl Breeding {
    /// Its neutral values, for a mod's literal to end with (`..Breeding::DEFAULT`).
    pub const DEFAULT: Breeding = Breeding {
        kind: "",
        young: 24_000,
        pause: 6_000,
        love: 600,
        meal: 1_200,
        search: 8.0,
        near: 2.0,
        pace: 1.0,
        stats: None,
    };
}

static AGE: AttachmentType<i32> = AttachmentType {
    namespace: "creature",
    name: "age",
    default: || 0,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From the library's `register`: the age's attachment is declared.
pub(crate) fn declare(r: &mut dyn mod_api::Registrar) {
    let _ = r.register_attachment_type("creature:age", false);
}

fn encode(age: &i32) -> Result<Compound, AttachmentError> {
    Ok([("Age".into(), Tag::Int(*age))].into())
}

fn decode(c: &Compound) -> Result<i32, AttachmentError> {
    Ok(match c.get(&"Age".into()) {
        Some(Tag::Int(n)) => *n,
        _ => 0,
    })
}

/// Each creature's age and the ticks of love it has left, once read; whether its age is to be saved.
#[derive(Clone, Copy, Debug, Default)]
struct Known {
    age: i32,
    love: i32,
    dirty: bool,
}

static KNOWN: EntityMap<Known> = EntityMap::new();
/// Creatures that bred this time, until out of love: one baby a pair.
static BRED: EntitySet = EntitySet::new();

/// How often an age is saved while it counts (ticks), besides when it changes otherwise.
const SAVE_EVERY: i32 = 1_200;

/// Its age (0 grown; negative young, until it reaches 0; positive a pause after breeding). 0 until read.
pub fn age(id: u32) -> i32 {
    KNOWN.locked().get(&id).map_or(0, |k| k.age)
}

/// Whether it is still young.
pub fn is_young(id: u32) -> bool {
    age(id) < 0
}

/// How far it has grown: 0 at birth, 1 grown up (a pause is grown up).
pub fn growth(id: u32, how: &Breeding) -> f32 {
    growth_of(age(id), how)
}

fn growth_of(age: i32, how: &Breeding) -> f32 {
    if age < 0 {
        (1.0 + age as f32 / how.young.max(1) as f32).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Whether it is in love now.
pub fn in_love(id: u32) -> bool {
    KNOWN.locked().get(&id).is_some_and(|k| k.love > 0)
}

/// Fed by its owner: grown and not resting from breeding, it falls in love for `how.love` ticks. Whether it did.
pub fn fall_in_love(id: u32, how: &Breeding) -> bool {
    let mut all = KNOWN.locked();
    let Some(k) = all.get_mut(&id).filter(|k| k.age == 0 && k.love == 0) else {
        return false;
    };
    k.love = how.love;
    true
}

/// Fed while young: it grows `how.meal` ticks older at once. Whether it was young.
pub fn feed_young(id: u32, how: &Breeding) -> bool {
    let mut all = KNOWN.locked();
    let Some(k) = all.get_mut(&id).filter(|k| k.age < 0) else {
        return false;
    };
    k.age = (k.age + how.meal).min(-1);
    k.dirty = true;
    true
}

/// What its tick saw happen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Grew {
    /// Nothing new.
    No,
    /// It grew up this tick.
    Up,
}

/// From the mod's tick, for each of its creatures: its age read the first time (or an older save's, `legacy`), then
/// counted (a young one up, a pause down, love down), and saved now and then.
pub fn tick(host: &mut dyn ServerHost, id: u32, legacy: impl FnOnce(&mut dyn ServerHost) -> Option<i32>) -> Grew {
    // Its age read the first time, without the lock held while the game is asked.
    let known = KNOWN.locked().contains_key(&id);
    if !known {
        // Not readable yet (its first tick after loading): next tick.
        let Ok(saved) = AGE.get_existing(host, Holder::Entity(id)) else {
            return Grew::No;
        };
        let (age, dirty) = match saved {
            Some(age) => (age, false),
            None => legacy(host).map_or((0, false), |age| (age, true)),
        };
        KNOWN.locked().insert(id, Known { age, love: 0, dirty });
        return Grew::No;
    }
    // Counted under the lock (a hook's change, fed or in love, is in what it reads), saved after.
    let (grew, save, loveless) = {
        let mut all = KNOWN.locked();
        let Some(k) = all.get_mut(&id) else {
            return Grew::No;
        };
        let was = k.age;
        k.age -= k.age.signum();
        k.love = (k.love - 1).max(0);
        let grew = if was < 0 && k.age == 0 { Grew::Up } else { Grew::No };
        let save = k.dirty || (k.age != 0 && k.age % SAVE_EVERY == 0) || (was != 0 && k.age == 0);
        k.dirty = false;
        (grew, save.then_some(k.age), k.love == 0)
    };
    if loveless {
        BRED.locked().remove(&id);
    }
    if let Some(age) = save {
        let _ = AGE.set(host, Holder::Entity(id), age);
    }
    grew
}

/// A baby to bring into the world: its kind, where, its owner, its stats.
struct Birth {
    how: &'static Breeding,
    at: [f64; 3],
    owner: Option<u128>,
    stats: Option<crate::stats::Stats>,
}

static BIRTHS: Mutex<Vec<Birth>> = Mutex::new(Vec::new());

/// From the kind's tick: its babies waiting to be born (`how.kind`'s), brought into the world in `dimension`. Their
/// ids, with the owner each was born to.
pub fn births(host: &mut dyn ServerHost, dimension: &str, how: &Breeding) -> Vec<(u32, Option<u128>)> {
    let births: Vec<Birth> = {
        let mut all = BIRTHS.locked();
        let (mine, rest) = std::mem::take(&mut *all)
            .into_iter()
            .partition(|b| b.how.kind == how.kind);
        *all = rest;
        mine
    };
    let mut born = Vec::new();
    for b in births {
        let Some(id) = host.summon(dimension, b.how.kind, b.at, Compound::new()) else {
            continue;
        };
        let _ = AGE.set(host, Holder::Entity(id), -b.how.young);
        KNOWN.locked().remove(&id);
        if let Some(owner) = b.owner {
            crate::friend::befriend(host, id, owner);
        }
        if let Some(stats) = b.stats {
            crate::stats::set(host, id, stats);
        }
        born.push((id, b.owner));
    }
    born
}

/// Breeding (`Breeding`): grown, tamed and in love, it walks to the nearest other of its kind in love and tamed;
/// close, the one with the smaller id brings a baby, and both wait.
#[derive(Debug)]
pub struct Mate {
    how: &'static Breeding,
    partner: Option<u32>,
}

impl Mate {
    pub fn new(how: &'static Breeding) -> Mate {
        Mate { how, partner: None }
    }

    /// The nearest other of its kind, in love and tamed.
    fn find(&self, m: &Body, cx: &World<'_>) -> Option<u32> {
        let host = cx.host();
        let r = self.how.search;
        let (x, y, z) = (m.position.x, m.position.y, m.position.z);
        cx.ground
            .level
            .entities_in(host, [x - r, y - r, z - r], [x + r, y + r, z + r])
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.id() != m.id && e.kind(host).is_some_and(|k| k.id(host) == self.how.kind))
            .filter(|e| in_love(e.id()) && crate::friend::of(e.id()).is_some())
            .filter_map(|e| Some((e.id(), e.position(host)?)))
            .map(|(id, at)| (id, (at[0] - x).powi(2) + (at[2] - z).powi(2)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
}

impl Behaviour for Mate {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let ready = crate::friend::of(m.id).is_some() && in_love(m.id) && age(m.id) == 0;
        if !ready {
            self.partner = None;
            return false;
        }
        if self.partner.is_none() {
            self.partner = self.find(m, cx);
        }
        self.partner.is_some()
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.partner = None;
        m.stop_walking = true;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(partner) = self.partner else {
            return;
        };
        let Some(at) = cx.host().entity(partner).and_then(|e| e.position(cx.host())) else {
            self.partner = None;
            return;
        };
        m.look_at = Some(at);
        let d2 = (at[0] - m.position.x).powi(2) + (at[2] - m.position.z).powi(2);
        if d2 > self.how.near * self.how.near {
            m.walk_to = Some((at, self.how.pace));
            return;
        }
        // Both see each other close; the one with the smaller id brings the baby, both wait.
        let first = {
            let mut bred = BRED.locked();
            let first = !bred.contains(&m.id) && !bred.contains(&partner);
            bred.insert(m.id);
            bred.insert(partner);
            first
        };
        if first {
            let rolls = [m.random.next_float(), m.random.next_float()];
            let stats = self.how.stats.map(|ranges| {
                let parent = |id: u32| crate::stats::of(id).unwrap_or_else(|| crate::stats::roll(ranges, [0.5, 0.5]));
                crate::stats::inherit(ranges, parent(m.id), parent(partner), rolls)
            });
            BIRTHS.locked().push(Birth {
                how: self.how,
                at: [m.position.x, m.position.y, m.position.z],
                owner: crate::friend::of(m.id).map(|f| crate::friend::uuid(f.uuid)),
                stats,
            });
        }
        for id in [m.id, partner] {
            if let Some(k) = KNOWN.locked().get_mut(&id) {
                k.love = 0;
                k.age = self.how.pause;
                k.dirty = true;
            }
        }
        self.partner = None;
        m.stop_walking = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOW: Breeding = Breeding {
        young: 1_000,
        ..Breeding::DEFAULT
    };

    #[test]
    fn a_young_one_grows_from_nothing_to_grown_and_a_pause_is_grown() {
        assert_eq!(growth_of(-1_000, &HOW), 0.0);
        assert!((growth_of(-500, &HOW) - 0.5).abs() < 1e-6);
        assert_eq!(growth_of(0, &HOW), 1.0);
        assert_eq!(growth_of(300, &HOW), 1.0);
    }

    #[test]
    fn an_age_is_saved_and_read_back() {
        assert_eq!(decode(&encode(&-24_000).unwrap()).unwrap(), -24_000);
    }
}
