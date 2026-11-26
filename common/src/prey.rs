//! Hunters eat small creatures (SPECS §3.7): a mod registers its kind as the prey of hunters, vanilla mobs or another
//! mod's (by entity id: a hunter whose mod isn't installed simply never shows up, so neither mod requires the other).
//! From the prey's tick (`tick`), the nearest hunter within its range is sent to it (`MobRef::move_to`, as a goal
//! would), and eats it once it reaches it: the creature is gone, with no drop, and the players near learn the mod's
//! fact. A creature in a jar or a bucket isn't in the world, so it is safe. A hunter of the series' mods (a seagull)
//! isn't walked there: it goes for its prey itself (its flight), asking which one is its own (`quarry`). A mod can add
//! its creature as a hunter of another mod's prey (`add_hunter`: the seagull, of the flying fish).
//!
//! Single player: the hooks share the mob's id (as the rest of the series).
use std::sync::Mutex;

use mod_api::{ParticleOptions, ServerHost, SoundSource};

use crate::state::EntityMap;
use crate::sync::Locked;

/// A hunter of a prey: its entity type, how far it sees the prey (blocks) and its speed going for it (its navigation's
/// multiplier).
pub struct Hunter {
    pub kind: &'static str,
    pub range: f64,
    pub speed: f64,
}

impl Hunter {
    /// Its neutral values, for a mod's literal to end with (`..Hunter::DEFAULT`): a field the library adds later then
    /// takes its default, and the mods built before go on building.
    pub const DEFAULT: Hunter = Hunter {
        kind: "",
        range: 0.0,
        speed: 1.0,
    };
}

/// A prey kind, its hunters, and the fact learnt by the players who see one eaten.
pub struct Prey {
    pub kind: &'static str,
    pub hunters: &'static [Hunter],
    pub fact: Option<&'static str>,
}

impl Prey {
    /// Its neutral values, for a mod's literal to end with (`..Prey::DEFAULT`): a field the library adds later then
    /// takes its default, and the mods built before go on building.
    pub const DEFAULT: Prey = Prey {
        kind: "",
        hunters: &[],
        fact: None,
    };
}

/// What a mod does to the hunter of its prey after a catch (`when_caught`), given the hunter's id.
pub type Caught = fn(&mut dyn ServerHost, u32);

static PREY: Mutex<Vec<&'static Prey>> = Mutex::new(Vec::new());
static CAUGHT: Mutex<Vec<(&'static str, Caught)>> = Mutex::new(Vec::new());
/// Hunters added by other mods, by their prey's kind.
static ADDED: Mutex<Vec<(&'static str, &'static Hunter)>> = Mutex::new(Vec::new());
/// The hunter going for each prey.
static CHASED: EntityMap<u32> = EntityMap::new();
/// How often a prey looks out for a new hunter (ticks), and how near a hunter eats it (blocks).
const LOOK_EVERY: i32 = 20;
const REACH: f64 = 1.3;

/// From a mod's `register`.
pub fn register(prey: &'static Prey) {
    let mut all = PREY.locked();
    if !all.iter().any(|p| p.kind == prey.kind) {
        all.push(prey);
    }
}

/// From a mod's `register`: after a catch of `kind`, `caught` runs on its hunter (the flying fish's dolphin is fed).
pub fn when_caught(kind: &'static str, caught: Caught) {
    let mut all = CAUGHT.locked();
    if !all.iter().any(|(k, _)| *k == kind) {
        all.push((kind, caught));
    }
}

/// From a mod's `register`: its creature `hunter` hunts `prey` too, another mod's (by its entity id). Nothing happens
/// when that mod isn't installed.
pub fn add_hunter(prey: &'static str, hunter: &'static Hunter) {
    let mut all = ADDED.locked();
    if !all.iter().any(|(p, h)| *p == prey && h.kind == hunter.kind) {
        all.push((prey, hunter));
    }
}

/// The hunters of `prey`: its own, then those added by other mods.
fn hunters(prey: &Prey) -> Vec<&'static Hunter> {
    let added = ADDED.locked();
    prey.hunters
        .iter()
        .chain(added.iter().filter(|(p, _)| *p == prey.kind).map(|(_, h)| *h))
        .collect()
}

/// The prey hunter `hunter` goes for, if any: for a hunter of the series' mods, which goes for it itself.
pub fn quarry(hunter: u32) -> Option<u32> {
    CHASED.locked().iter().find(|(_, h)| **h == hunter).map(|(p, _)| *p)
}

/// The hunter still going for prey `entity`, if `near` finds it still near (the engine's): `CHASED` is read under its
/// lock, released before `near` asks the engine anything.
fn kept<T>(entity: u32, near: impl FnOnce(u32) -> Option<T>) -> Option<T> {
    let chased = CHASED.locked().get(&entity).copied();
    chased.and_then(near)
}

fn of(kind: &str) -> Option<&'static Prey> {
    PREY.locked().iter().copied().find(|p| p.kind == kind)
}

/// From the mod's `entity_tick_post`, for each creature of a prey kind: its hunter goes for it, and eats it once there.
/// Whether it was eaten (it is gone: the mod forgets it).
pub fn tick(host: &mut dyn ServerHost, dimension: &str, entity: u32, kind: &str) -> bool {
    let Some(prey) = of(kind) else {
        return false;
    };
    let Some(e) = host.entity(entity) else {
        return false;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return false;
    };
    let hunters = hunters(prey);
    let kept = kept(entity, |h| {
        let hunter = host.entity(h)?;
        let kind = hunter.kind(&*host)?.id(&*host);
        let range = hunters.iter().find(|x| x.kind == kind)?.range;
        let there = hunter.position(&*host)?;
        (crate::body::distance(there, at) <= range * 1.5).then_some((h, there))
    });
    let chase = kept.or_else(|| {
        if e.tick_count(&*host).unwrap_or(0) % LOOK_EVERY != 0 {
            return None;
        }
        let r = hunters.iter().map(|h| h.range).fold(0.0, f64::max);
        let from = [at[0] - r, at[1] - r, at[2] - r];
        let to = [at[0] + r, at[1] + r, at[2] + r];
        level
            .entities_in(&*host, from, to)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| {
                let kind = h.kind(&*host)?.id(&*host);
                let range = hunters.iter().find(|x| x.kind == kind)?.range;
                let there = h.position(&*host)?;
                let d = crate::body::distance(there, at);
                (d <= range).then_some((h.id(), there, d))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(h, there, _)| (h, there))
    });
    let Some((hunter, there)) = chase else {
        CHASED.locked().remove(&entity);
        return false;
    };
    CHASED.locked().insert(entity, hunter);
    if crate::body::distance(there, at) <= REACH {
        // The prey's fact and its mod's `when_caught` are about its own hunters, not those other mods added.
        let own = host
            .entity(hunter)
            .and_then(|h| h.kind(&*host))
            .is_some_and(|k| prey.hunters.iter().any(|x| x.kind == k.id(&*host)));
        eaten(host, dimension, prey, entity, at, own);
        let caught = CAUGHT.locked().iter().find(|(k, _)| *k == prey.kind).map(|(_, f)| *f);
        if let Some(caught) = caught.filter(|_| own) {
            caught(host, hunter);
        }
        return true;
    }
    let kind = host.entity(hunter).and_then(|h| h.kind(&*host)).map(|k| k.id(&*host));
    // One of ours goes for it itself (`quarry`).
    if kind.as_deref().is_some_and(crate::mob::is_ours) {
        return false;
    }
    let speed = kind
        .and_then(|k| hunters.iter().find(|x| x.kind == k))
        .map_or(1.0, |h| h.speed);
    if let Some(m) = host.entity(hunter).and_then(|h| host.mob(h)) {
        m.move_to(host, at, speed);
    }
    false
}

/// Prey `entity` at `at` eaten: a crunch and crumbs, the players near learn the fact (an own hunter's, `own`), it is gone
/// with no drop.
fn eaten(host: &mut dyn ServerHost, dimension: &str, prey: &Prey, entity: u32, at: [f64; 3], own: bool) {
    if let Some(level) = host.level(dimension) {
        level.play_sound(
            host,
            "minecraft:entity.generic.eat",
            at,
            SoundSource::Neutral,
            0.8,
            1.0,
            None,
        );
        if let Some(k) = crate::particles::key("minecraft:poof") {
            level.send_particles(host, &ParticleOptions::simple(k), None, at, [0.2, 0.2, 0.2], 0.02, 6);
        }
    }
    if let Some(fact) = prey.fact.filter(|_| own) {
        for player in crate::bestiary::players_near(&*host, dimension, at) {
            crate::bestiary::learn(host, player, prey.kind, fact);
        }
    }
    if let Some(e) = host.entity(entity) {
        e.discard(host);
    }
    forget(entity);
}

/// From the library when a creature is gone.
pub fn forget(entity: u32) {
    CHASED.locked().retain(|p, h| *p != entity && *h != entity);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hunter_kept_is_checked_with_no_lock_held() {
        CHASED.locked().insert(3_000_000, 3_000_001);
        // `quarry` locks `CHASED`: under the lock, as before, it would wait for itself forever.
        assert_eq!(kept(3_000_000, quarry), Some(3_000_000));
        assert_eq!(kept(3_000_002, quarry), None, "not chased");
        forget(3_000_001);
        assert_eq!(quarry(3_000_001), None, "a hunter gone is forgotten with its prey");
    }
}
