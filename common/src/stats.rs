//! A creature's own stats (SPECS §3.18): each one rolls its maximum health and its pace (a share of its kind's speed)
//! within its kind's ranges the first time it is seen, keeps them all its life (saved on it, `creature:stats`), and a
//! baby inherits them: each stat the average of both parents' and a fresh roll's, as a horse's. Hidden from players.
//! Users: the vulture (its pace is its flight speed's), the elephant (its walking speed's).
//!
//! A mod calls `tick` from its creature's `entity_tick_post`: the first time, it reads the creature's stats, or rolls
//! them (or takes those an older save of the mod's kept, `legacy`), and sets its maximum health (and its walking speed,
//! for a kind that gives one); `pace` and `of` read them back.
use mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, ServerHost, Tag};

use crate::colours::Random;
use crate::state::{EntityMap, EntitySet};

/// A kind's stats' ranges, and its base walking speed when its pace sets it (`minecraft:movement_speed`, times its
/// pace; none when its pace is used otherwise, a flier's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ranges {
    pub health: (f32, f32),
    pub pace: (f32, f32),
    pub walk: Option<f64>,
}

impl Ranges {
    /// Its neutral values, for a mod's literal to end with (`..Ranges::DEFAULT`).
    pub const DEFAULT: Ranges = Ranges {
        health: (20.0, 20.0),
        pace: (1.0, 1.0),
        walk: None,
    };
}

/// A creature's stats: its maximum health, and its pace (a share of its kind's speed).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub health: f32,
    pub pace: f32,
}

static STATS: AttachmentType<Stats> = AttachmentType {
    namespace: "creature",
    name: "stats",
    default: Stats::default,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// Each creature's stats, once read or rolled; the creatures whose stats were just `set`, to be applied with their
/// health full.
static KNOWN: EntityMap<Stats> = EntityMap::new();
static FRESH: EntitySet = EntitySet::new();

/// From the library's `register`: the stats' attachment is declared.
pub(crate) fn declare(r: &mut dyn mod_api::Registrar) {
    let _ = r.register_attachment_type("creature:stats", false);
}

fn encode(s: &Stats) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    c.insert("Health".into(), Tag::Float(s.health.to_bits()));
    c.insert("Pace".into(), Tag::Float(s.pace.to_bits()));
    Ok(c)
}

fn decode(c: &Compound) -> Result<Stats, AttachmentError> {
    let float = |key: &str| match c.get(&key.into()) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 0.0,
    };
    Ok(Stats {
        health: float("Health"),
        pace: float("Pace"),
    })
}

/// A stat between `range`'s ends from `roll` (0 to 1).
pub fn stat(range: (f32, f32), roll: f32) -> f32 {
    range.0 + (range.1 - range.0) * roll
}

/// Stats rolled within `ranges` from `rolls` (0 to 1 each).
pub fn roll(ranges: &Ranges, rolls: [f32; 2]) -> Stats {
    Stats {
        health: stat(ranges.health, rolls[0]),
        pace: stat(ranges.pace, rolls[1]),
    }
}

/// A baby's stats from its parents' (SPECS §3.18): each the average of both parents' and a fresh roll's.
pub fn inherit(ranges: &Ranges, a: Stats, b: Stats, rolls: [f32; 2]) -> Stats {
    let fresh = roll(ranges, rolls);
    Stats {
        health: (a.health + b.health + fresh.health) / 3.0,
        pace: (a.pace + b.pace + fresh.pace) / 3.0,
    }
}

/// A creature's stats, once its first `tick` has read or rolled them.
pub fn of(id: u32) -> Option<Stats> {
    KNOWN.locked().get(&id).copied()
}

/// Its pace: 1 until known.
pub fn pace(id: u32) -> f64 {
    of(id).map_or(1.0, |s| f64::from(s.pace))
}

/// From the mod's `entity_tick_post`, for each of its creatures: the first time, its stats read from its save, or else
/// taken from an older save of the mod's (`legacy`), or else rolled, and applied (its health full when rolled).
pub fn tick(
    host: &mut dyn ServerHost,
    id: u32,
    ranges: &Ranges,
    legacy: impl FnOnce(&mut dyn ServerHost) -> Option<Stats>,
) {
    if KNOWN.locked().contains_key(&id) {
        return;
    }
    // Not readable yet (its first tick after loading): next tick, rather than rolling over saved stats.
    let Ok(saved) = STATS.get_existing(host, Holder::Entity(id)) else {
        return;
    };
    let (stats, new) = match saved.filter(|s| s.pace > 0.0) {
        Some(s) => (s, false),
        None => match legacy(host).filter(|s| s.pace > 0.0) {
            Some(s) => (s, true),
            None => {
                let mut random = Random::new(id ^ 0x5EED);
                let mut float = || (random.next() % 10_000) as f32 / 10_000.0;
                (roll(ranges, [float(), float()]), true)
            }
        },
    };
    if new {
        let _ = STATS.set(host, Holder::Entity(id), stats);
    }
    let fill = new || FRESH.locked().remove(&id);
    apply(host, id, ranges, stats, fill);
    KNOWN.locked().insert(id, stats);
}

/// A creature's stats set (a baby's, `inherit`): saved, and applied on its next `tick`, its health full.
pub fn set(host: &mut dyn ServerHost, id: u32, stats: Stats) {
    let _ = STATS.set(host, Holder::Entity(id), stats);
    KNOWN.locked().remove(&id);
    FRESH.locked().insert(id);
}

/// Its maximum health set (its health full when `fill`, or kept under it), and its walking speed for a kind whose pace
/// sets it.
fn apply(host: &mut dyn ServerHost, id: u32, ranges: &Ranges, stats: Stats, fill: bool) {
    let Some(living) = host.entity(id).and_then(|e| host.living(e)) else {
        return;
    };
    living.set_attribute_base(host, "minecraft:max_health", f64::from(stats.health));
    let health = living.health(&*host).unwrap_or(stats.health);
    living.set_health(host, if fill { stats.health } else { health.min(stats.health) });
    if let Some(walk) = ranges.walk {
        living.set_attribute_base(host, "minecraft:movement_speed", walk * f64::from(stats.pace));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANGES: Ranges = Ranges {
        health: (40.0, 60.0),
        pace: (0.8, 1.2),
        walk: Some(0.16),
    };

    #[test]
    fn a_baby_takes_a_third_of_each_parent_and_of_a_fresh_roll() {
        let a = Stats {
            health: 60.0,
            pace: 1.2,
        };
        let b = Stats {
            health: 40.0,
            pace: 0.8,
        };
        let baby = inherit(&RANGES, a, b, [1.0, 0.0]);
        assert!((baby.health - (60.0 + 40.0 + 60.0) / 3.0).abs() < 1e-4);
        assert!((baby.pace - (1.2 + 0.8 + 0.8) / 3.0).abs() < 1e-4);
    }

    #[test]
    fn stats_are_saved_and_read_back() {
        let s = Stats {
            health: 47.5,
            pace: 1.07,
        };
        assert_eq!(decode(&encode(&s).unwrap()).unwrap(), s);
    }
}
