//! Where it appears (SPECS §2): rarely, in the Nether's basalt deltas and wastes near lava, always asleep, on a floor
//! with room for it, never within `APART` blocks of another. Every `SPAWN_EVERY` ticks, one chance in `SPAWN_CHANCE`
//! for each player in the Nether: `SPAWN_TRIES` columns `SPAWN_MIN` to `SPAWN_MAX` blocks away are tried (`floor`).
//! Where each one appeared is saved with the level (`LAIRS`, an attachment of the Nether's): the golems there, loaded
//! or not, keep the next ones away, until it dies.
use creature::territory::Ground;
use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, LevelRef, Registrar, ServerHost, Tag,
};

use crate::{BIOMES, GOLEM};

const SPAWN_EVERY: u32 = 1200;
const SPAWN_CHANCE: u64 = 8;
const SPAWN_TRIES: usize = 16;
const SPAWN_MIN: f64 = 32.0;
const SPAWN_MAX: f64 = 64.0;
/// How far apart two golems appear (blocks).
const APART: f64 = 256.0;
/// Its room (blocks): 3 × 6 × 3 of air over a solid floor.
const ROOM: (i32, i32) = (1, 6);
/// How far up and down from the player a floor is looked for, and how near lava it must be (blocks).
const SEARCH_UP: i32 = 16;
const SEARCH_DOWN: i32 = 24;
const LAVA_NEAR: i32 = 12;
/// How far from its lair a golem's death frees it (blocks): it may have wandered.
const LAIR_LEFT: f64 = 128.0;

static TICKS: std::sync::Mutex<u32> = std::sync::Mutex::new(0);

/// Where the golems appeared, in the Nether's saved data.
static LAIRS: AttachmentType<Vec<[i32; 3]>> = AttachmentType {
    namespace: "magma_golem",
    name: "lairs",
    default: Vec::new,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From `register`: the lairs' attachment is declared.
pub fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("magma_golem:lairs", false);
}

// The codec's signature: the attachment's value by reference.
#[allow(clippy::ptr_arg)]
fn encode(lairs: &Vec<[i32; 3]>) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    c.insert("Lairs".into(), Tag::IntArray(lairs.iter().flatten().copied().collect()));
    Ok(c)
}

fn decode(c: &Compound) -> Result<Vec<[i32; 3]>, AttachmentError> {
    Ok(match c.get(&"Lairs".into()) {
        Some(Tag::IntArray(a)) => a.as_chunks::<3>().0.to_vec(),
        _ => Vec::new(),
    })
}

/// From the Nether's `level_tick_post`.
pub fn tick(host: &mut dyn ServerHost, level: LevelRef) {
    use creature::sync::Locked;
    {
        let mut t = TICKS.locked();
        *t += 1;
        if *t < SPAWN_EVERY {
            return;
        }
        *t = 0;
    }
    if level.difficulty(&*host) == Some("peaceful") {
        return;
    }
    let Some(dimension) = level.dimension().id(&*host) else {
        return;
    };
    let mut random = creature::colours::Random::new(level.game_time(&*host).unwrap_or(0) as u32);
    let players: Vec<_> = host
        .players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(&*host) == Some(level) && p.is_spectator(&*host) != Some(true))
        .filter_map(|p| p.entity().position(&*host))
        .collect();
    for p in players {
        if !random.next().is_multiple_of(SPAWN_CHANCE) {
            continue;
        }
        for _ in 0..SPAWN_TRIES {
            let angle = ((random.next() % 3600) as f64 / 10.0).to_radians();
            let distance = SPAWN_MIN + (random.next() % 1000) as f64 / 1000.0 * (SPAWN_MAX - SPAWN_MIN);
            let x = (p[0] + angle.cos() * distance).floor() as i32;
            let z = (p[2] + angle.sin() * distance).floor() as i32;
            let Some(at) = floor(&*host, level, x, p[1].floor() as i32, z) else {
                continue;
            };
            if !level.biome_at(host, at).is_some_and(|b| BIOMES.contains(&b.as_str())) {
                continue;
            }
            let feet = [f64::from(at[0]) + 0.5, f64::from(at[1]), f64::from(at[2]) + 0.5];
            let lairs = LAIRS.get(host, Holder::Level).unwrap_or_default();
            let far = |q: [f64; 3]| (q[0] - feet[0]).hypot(q[2] - feet[2]) >= APART;
            let alone = lairs.iter().all(|l| far([f64::from(l[0]), 0.0, f64::from(l[2])]))
                && crate::golem::golems_near(&*host, level, feet, APART).is_empty();
            if !alone {
                break;
            }
            if host.summon(&dimension, GOLEM, feet, Compound::new()).is_some() {
                let mut lairs = lairs;
                lairs.push(at);
                let _ = LAIRS.set(host, Holder::Level, lairs);
            }
            break;
        }
    }
}

/// From its death at `at`: the nearest lair within `LAIR_LEFT` blocks is forgotten, so another may come there.
pub fn forget(host: &mut dyn ServerHost, at: [f64; 3]) {
    let Ok(mut lairs) = LAIRS.get(host, Holder::Level) else {
        return;
    };
    let far = |l: &[i32; 3]| (f64::from(l[0]) + 0.5 - at[0]).hypot(f64::from(l[2]) + 0.5 - at[2]);
    let nearest = (0..lairs.len())
        .filter(|&i| far(&lairs[i]) <= LAIR_LEFT)
        .min_by(|&a, &b| far(&lairs[a]).total_cmp(&far(&lairs[b])));
    if let Some(i) = nearest {
        lairs.remove(i);
        let _ = LAIRS.set(host, Holder::Level, lairs);
    }
}

/// The floor at column (`x`, `z`) near height `y`, scanning down: a solid block (not lava) under 3 × `ROOM.1` × 3
/// blocks of air, with lava within `LAVA_NEAR` blocks. Where it stands, or none.
fn floor(host: &dyn ServerHost, level: LevelRef, x: i32, y: i32, z: i32) -> Option<[i32; 3]> {
    let ground = Ground::new(host, level);
    let air = |p: [i32; 3]| ground.block(p).ends_with("air");
    let solid = |p: [i32; 3]| {
        let b = ground.block(p);
        !b.ends_with("air") && !b.contains("lava") && !b.contains("fire")
    };
    let r = ROOM.0;
    let mut top = y + SEARCH_UP;
    while top > y - SEARCH_DOWN {
        let feet = [x, top, z];
        let room = (-r..=r).all(|dx| {
            (-r..=r).all(|dz| solid([x + dx, top - 1, z + dz]) && (0..ROOM.1).all(|dy| air([x + dx, top + dy, z + dz])))
        });
        if room {
            return lava_near(&ground, feet).then_some(feet);
        }
        top -= 1;
    }
    None
}

/// Whether lava lies within `LAVA_NEAR` blocks of `at`, around its height (every other block looked at).
fn lava_near(ground: &Ground<'_>, at: [i32; 3]) -> bool {
    (-LAVA_NEAR..=LAVA_NEAR).step_by(2).any(|dx| {
        (-LAVA_NEAR..=LAVA_NEAR)
            .step_by(2)
            .any(|dz| (-4..=1).any(|dy| ground.block([at[0] + dx, at[1] + dy, at[2] + dz]).contains("lava")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lairs_are_saved_and_read_back() {
        let lairs = vec![[1, 2, 3], [-40, 31, 900]];
        assert_eq!(decode(&encode(&lairs).unwrap()).unwrap(), lairs);
    }
}
