//! Taming (SPECS §2, §5): a wild elephant goes to the sugar cane a player throws near it and eats it, each piece one
//! chance in `TAME_ONE_IN` to tame it for that player (the library's `food` and `friend`, as the Seagulls'). Tamed, it
//! follows its owner on foot (the library's `friend::Follow`) or stays where it is told (`Stay`), never despawns, and a
//! hay bale from its owner heals it, makes a young one grow faster, or puts a grown one in love to breed (`BREEDING`).
//!
//! The hook finds the food (`look_for_food`, every `SCAN` ticks: it sees the items on the ground), the `Eat` behaviour
//! walks to it and eats it (`EATEN`), the hook takes one from the stack and rolls the taming (`eat`). Single player:
//! the behaviours and the hooks share the mob's id.
use creature::body::{Behaviour, Body, World};
use creature::food::Found;
use creature::state::EntityMap;
use mod_api::{Hand, PlayerRef, ServerHost, Text};

use crate::ELEPHANT;

/// What tames it, thrown (SPECS §2), and its chance: one in `TAME_ONE_IN` a piece.
const TAMING_FOOD: &str = "minecraft:sugar_cane";
const TAME_ONE_IN: u64 = 4;
/// Hay (SPECS §4, §5): it heals a tamed adult (`HEAL`), makes a young one grow faster, and puts a grown, rested one in
/// love.
const HAY: &str = "minecraft:hay_block";
const HEAL: f32 = 10.0;

/// Breeding and growth (SPECS §4, the library's `creature::growth`): a baby for 10 minutes, a juvenile 10 more, a hay
/// bale a minute sooner; parents wait 5 minutes; love lasts 30 seconds; partners meet within 6 blocks, found within 16.
pub(crate) static BREEDING: creature::growth::Breeding = creature::growth::Breeding {
    kind: ELEPHANT,
    young: 24_000,
    pause: 6_000,
    love: 600,
    meal: 1_200,
    search: 16.0,
    near: 6.0,
    pace: 1.0,
    stats: Some(&crate::STATS),
    ..creature::growth::Breeding::DEFAULT
};

/// A baby is a baby until this share of its growth, then a juvenile (SPECS §4: 10 minutes each).
pub(crate) const JUVENILE: f32 = 0.5;
/// How far it sees food (blocks), how often the hook looks (ticks), how near it eats it (blocks from its middle) and
/// how fast it walks to it (a multiplier of its speed).
const FOOD_RANGE: f64 = 12.0;
const SCAN: i32 = 10;
const EAT_REACH: f64 = 3.6;
const FOOD_PACE: f64 = 1.0;

/// Its guide fact (SPECS §6).
pub(crate) const TAMED_FACT: &str = "tamed";
pub(crate) const BRED_FACT: &str = "bred";
pub(crate) const RAISED_FACT: &str = "raised";

/// Following its owner on foot (the library's): from 9 blocks away, stopping within 5, always walking.
pub(crate) static FOLLOWING: creature::friend::Following = creature::friend::Following {
    follow: 9.0,
    near: 5.0,
    // It walks to its owner, however far: never brought to them at once.
    teleport: f64::INFINITY,
    speed: 1.1,
    ..creature::friend::Following::DEFAULT
};

/// The food its hook found for it, and the ticks until it looks again.
static FOOD: EntityMap<Found> = EntityMap::new();
static SCANS: EntityMap<i32> = EntityMap::new();
/// The food it reached, for the hook to take and roll the taming of.
static EATEN: EntityMap<Found> = EntityMap::new();

/// From its tick: sugar cane a player threw near a wild elephant, for its `Eat` behaviour.
pub(crate) fn look_for_food(host: &dyn ServerHost, id: u32) {
    let wild = creature::friend::of(id).is_none();
    let due = {
        let mut scans = SCANS.locked();
        let t = scans.entry(id).or_insert(0);
        *t -= 1;
        let due = *t <= 0;
        if due {
            *t = SCAN;
        }
        due
    };
    if !wild {
        FOOD.locked().remove(&id);
        return;
    }
    if !due {
        return;
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let found = creature::food::nearest(host, level, at, FOOD_RANGE, |item| item == TAMING_FOOD)
        .filter(|f| f.thrower.is_some());
    match found {
        Some(f) => FOOD.locked().insert(id, f),
        None => FOOD.locked().remove(&id),
    };
}

/// Going to the sugar cane thrown near it, and eating it.
#[derive(Debug, Default)]
pub(crate) struct Eat;

impl Behaviour for Eat {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        FOOD.locked().contains_key(&m.id)
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        let Some(food) = FOOD.locked().get(&m.id).cloned() else {
            return;
        };
        let (dx, dz) = (food.at[0] - m.position.x, food.at[2] - m.position.z);
        if dx.hypot(dz) <= EAT_REACH {
            m.stop_walking = true;
            m.look_at = Some(food.at);
            FOOD.locked().remove(&m.id);
            EATEN.locked().insert(m.id, food);
        } else {
            m.walk_to = Some((food.at, FOOD_PACE));
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
    }
}

/// Staying where its owner told it to (no wandering).
#[derive(Debug, Default)]
pub(crate) struct Stay;

impl Behaviour for Stay {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        creature::friend::of(m.id).is_some_and(|f| f.staying)
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
}

/// From its tick: the sugar cane it reached, eaten (one of the stack), and the taming rolled for whoever threw it.
pub(crate) fn eat(host: &mut dyn ServerHost, id: u32) {
    let Some(food) = EATEN.locked().remove(&id) else {
        return;
    };
    if !creature::food::take_one(host, food.item) {
        return;
    }
    creature::sounds::queue(id, "minecraft:entity.generic.eat", 1.0, 0.7);
    let Some(player) = food.thrower.and_then(|t| creature::mob::player_of(&*host, t)) else {
        return;
    };
    if creature::friend::of(id).is_some() {
        return;
    }
    let tamed = creature::colours::Random::new(id ^ food.item)
        .next()
        .is_multiple_of(TAME_ONE_IN);
    if tamed {
        tame(host, id, player);
    }
    if let Some(e) = host.entity(id)
        && let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host))
    {
        creature::friend::sign(host, level, [at[0], at[1] + 2.0, at[2]], tamed);
    }
}

/// Tamed for `player`: its friend, kept for good (never despawning), and the guide's fact.
fn tame(host: &mut dyn ServerHost, id: u32, player: u128) {
    creature::friend::befriend(host, id, player);
    if let Some(m) = host.entity(id).and_then(|e| host.mob(e)) {
        m.set_persistence_required(host);
    }
    creature::bestiary::learn(host, player, ELEPHANT, TAMED_FACT);
}

/// Its right-click by its owner (SPECS §2, §5), on the server: a hay bale heals it; an empty hand tells it to stay or
/// follow. Whether it took the click.
pub(crate) fn interact(host: &mut dyn ServerHost, id: u32, player: PlayerRef, hand: Hand, item: Option<&str>) -> bool {
    let friend = creature::friend::of(id);
    if !friend.is_some_and(|f| f.is(player.uuid())) {
        return false;
    }
    let Some(e) = host.entity(id) else {
        return false;
    };
    match item {
        Some(HAY) => {
            let Some(living) = host.living(e) else {
                return false;
            };
            let (health, most) = (
                living.health(&*host).unwrap_or(0.0),
                living.max_health(&*host).unwrap_or(0.0),
            );
            // Young, it grows faster; hurt, it heals; else, grown and rested, it falls in love.
            let (fed, particle) = if creature::growth::is_young(id) {
                (creature::growth::feed_young(id, &BREEDING), "minecraft:happy_villager")
            } else if health < most {
                let _ = living.set_health(host, (health + HEAL).min(most));
                (true, "minecraft:happy_villager")
            } else {
                (creature::growth::fall_in_love(id, &BREEDING), "minecraft:heart")
            };
            if !fed {
                return false;
            }
            if !player.has_infinite_materials(&*host).unwrap_or(false) {
                creature::hand::use_one(host, player, hand);
            }
            creature::sounds::queue(id, "minecraft:entity.generic.eat", 1.0, 0.7);
            if let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) {
                creature::particles::puff(host, level, particle, [at[0], at[1] + 2.0, at[2]], 0.8, 8);
            }
            true
        }
        None => {
            let staying = !friend.is_some_and(|f| f.staying);
            creature::friend::set_staying(id, staying);
            let key = if staying { "elephant.stays" } else { "elephant.follows" };
            let _ = player.action_bar(host, Text::translate(key, vec![]));
            true
        }
        _ => false,
    }
}
