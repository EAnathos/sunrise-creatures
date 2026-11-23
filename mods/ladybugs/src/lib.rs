//! Ladybugs (SPECS.md): ladybugs on the flowers of meadows and forests, of the Sunrise Creatures series.
//!
//! A ladybug is a copy of the wandering trader drawn with its own model (`model_ladybug.rs`, from
//! `sources/models/ladybug.bbmodel`). Its kind is its look (the library's `colours`, rolled by biome: the pattern is the
//! kind), drawn as a grey shell tinted by its kind's colour with its spots over it, the rest of the body as it is. It
//! flies short hops between flowers and rests on them (the library's small fliers, `flutter`), gathers at night
//! (`luck::Cluster`), and lands on a player who stands still, giving them luck for as many minutes as it has spots
//! (`luck::Perch`, seated by `act`). Caught in the glass jar, it settles on a flower patch; chickens peck it up.
mod luck;
mod model_ladybug;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use creature::colours::{Colour, Look, Pattern, Scheme, Variety};
use creature::jar::Catchable;
use creature::model::{WHITE, draw_layer, multiply};
use creature::sync::Locked;
use luck::Action;
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Hooks,
    HostSide, InteractionResult, LivingDeathEvent, RegisterEntityRenderersEvent, Registrar, RegistrationContext,
    ServerHost,
};

pub const KIND: &str = "ladybugs:ladybug";
const EGG: &str = "ladybugs:ladybug_spawn_egg";

/// Its box (blocks, SPECS §1) and the scale its model (9 texels long with its head) is drawn at: about a fifth of a
/// block.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.25,
    height: 0.15,
    eye_height: 0.1,
};
const SCALE: f32 = 0.4;

/// The shell and body's texture (the shell grey, tinted per kind) and each kind's spots (white, tinted).
const BASE: &str = "ladybugs:entity/ladybug/base";

/// A kind (SPECS §2): its spots' texture, its shell's and spots' colours, how many spots (its luck's minutes) and its
/// luck's level (0 for Luck, 1 for Luck II).
struct Kind {
    spots_texture: &'static str,
    shell: u32,
    spot: u32,
    spots: i32,
    level: i32,
}

const SEVEN: usize = 0;
const TWO: usize = 1;
const TWENTY_TWO: usize = 2;
const ORANGE: usize = 3;
const HARLEQUIN: usize = 4;

static KINDS: [Kind; 5] = [
    Kind {
        spots_texture: "ladybugs:entity/ladybug/seven_spot",
        shell: 0xC8202A,
        spot: 0x1A1818,
        spots: 7,
        level: 0,
    },
    Kind {
        spots_texture: "ladybugs:entity/ladybug/two_spot",
        shell: 0xC8202A,
        spot: 0x1A1818,
        spots: 2,
        level: 0,
    },
    Kind {
        spots_texture: "ladybugs:entity/ladybug/twenty_two_spot",
        shell: 0xE8C428,
        spot: 0x1A1818,
        spots: 22,
        level: 0,
    },
    Kind {
        spots_texture: "ladybugs:entity/ladybug/sixteen_spot",
        shell: 0xE07A22,
        spot: 0xF4F0E6,
        spots: 16,
        level: 0,
    },
    Kind {
        spots_texture: "ladybugs:entity/ladybug/four_spot",
        shell: 0x2A2624,
        spot: 0xD02828,
        spots: 4,
        level: 1,
    },
];

/// Its kinds as the look's patterns (the morph saved and shown in the guide), each with its shell's colour as its
/// swatch.
static COLOURS: [Colour; 1] = [Colour {
    id: "plain",
    rgb: 0xFFFFFF,
}];
static PATTERNS: [Pattern; 5] = [
    Pattern {
        id: "seven_spot",
        texture: None,
        swatch: Some(0xC8202A),
    },
    Pattern {
        id: "two_spot",
        texture: None,
        swatch: Some(0xB01C24),
    },
    Pattern {
        id: "twenty_two_spot",
        texture: None,
        swatch: Some(0xE8C428),
    },
    Pattern {
        id: "orange",
        texture: None,
        swatch: Some(0xE07A22),
    },
    Pattern {
        id: "harlequin",
        texture: None,
        swatch: Some(0x2A2624),
    },
];

/// Where each kind lives (SPECS §2), the rare harlequin one in ten in its biomes.
const MEADOWS: &[&str] = &[
    "minecraft:meadow",
    "minecraft:flower_forest",
    "minecraft:sunflower_plains",
];
const PLAINS: &[&str] = &["minecraft:plains", "minecraft:forest", "minecraft:cherry_grove"];
const BIRCH: &[&str] = &["minecraft:birch_forest", "minecraft:old_growth_birch_forest"];
const DARK: &[&str] = &["minecraft:dark_forest"];
const SWAMPS: &[&str] = &["minecraft:swamp", "minecraft:mangrove_swamp"];
static VARIETIES: [Variety; 6] = [
    Variety {
        biomes: MEADOWS,
        base: &[(0, 1)],
        patterns: &[(SEVEN, 4), (TWO, 2), (TWENTY_TWO, 2)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: PLAINS,
        base: &[(0, 1)],
        patterns: &[(SEVEN, 3), (TWO, 3)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: BIRCH,
        base: &[(0, 1)],
        patterns: &[(SEVEN, 3), (ORANGE, 2)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: DARK,
        base: &[(0, 1)],
        patterns: &[(ORANGE, 5), (HARLEQUIN, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: SWAMPS,
        base: &[(0, 1)],
        patterns: &[(SEVEN, 9), (HARLEQUIN, 1)],
        pattern_colours: &[(0, 1)],
    },
    // Everywhere else (one let out of a jar far from its biomes): a seven-spot.
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(SEVEN, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static SCHEME: Scheme = Scheme {
    colours: &COLOURS,
    patterns: &PATTERNS,
    base_texture: BASE,
    varieties: &VARIETIES,
};
const BIOMES: &[&str] = &[
    "minecraft:meadow",
    "minecraft:flower_forest",
    "minecraft:sunflower_plains",
    "minecraft:plains",
    "minecraft:forest",
    "minecraft:cherry_grove",
    "minecraft:birch_forest",
    "minecraft:old_growth_birch_forest",
    "minecraft:dark_forest",
    "minecraft:swamp",
    "minecraft:mangrove_swamp",
];

/// The kind of ladybug `id` (a seven-spot until its look is known).
fn kind(id: u32) -> usize {
    creature::colours::look(id).map_or(SEVEN, |l| l.pattern.min(KINDS.len() - 1))
}

/// How it flies (the library's small fliers, SPECS §3): by day, short slow hops close to the ground, long rests on
/// flowers; a short buzz as it takes off (`sources/sounds.toml`, cut from a CC0 freesound recording).
static FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Day,
    speed: 0.08,
    height: (0.4, 1.5),
    flight: (30, 90),
    rest: (300, 900),
    wander: 5.0,
    drawn_to_light: false,
    lands_on_flowers: true,
    takeoff: Some("ladybugs:entity.ladybug.takeoff"),
};

/// Its crawl (the library's creeping, SPECS §3): slow, a little way around where it rests, up a flower or a step; by day
/// in fair weather, between its flights, not on a player nor gathering.
static CREEPING: creature::creep::Creeping = creature::creep::Creeping {
    speed: 0.01,
    climb: 0.015,
    crawl: (40, 120),
    pause: (200, 600),
    wander: 1.5,
    area: |_, _| Vec::new(),
    may: |m, cx| {
        cx.daylight && !cx.raining && !creature::flutter::flying(m.id) && !luck::perched(m.id) && !luck::gathering(m.id)
    },
    rustle: None,
    clings: false,
};

/// They come out near players by day in their biomes (the library's spawning): 1 to 3, up to 6 near a player.
static SPAWNING: creature::flutter::Spawning = creature::flutter::Spawning {
    kind: KIND,
    flight: &FLIGHT,
    biomes: BIOMES,
    group: (1, 3),
    cap: 6,
    chance: 2,
};

/// Caught in the library's glass jar (common/SPECS.md §4.1), it settles on a flower patch.
static IN_JAR: Catchable = Catchable {
    kind: KIND,
    model: "ladybugs:ladybug_jar",
    settles: Some(creature::territory::Rule::FlowerPatch),
    glows: false,
};

const LUCK_FACT: &str = "luck";
const CLUSTER_FACT: &str = "cluster";
const CHICKEN_FACT: &str = "chicken";
const ALL_SPOTS_FACT: &str = "all_spots";

/// Chickens peck them up (SPECS §3): one within 6 blocks walks to it.
static PREY: creature::prey::Prey = creature::prey::Prey {
    kind: KIND,
    hunters: &[creature::prey::Hunter {
        kind: "minecraft:chicken",
        range: 6.0,
        speed: 1.0,
    }],
    fact: Some(CHICKEN_FACT),
};

/// Its page (SPECS §6): the biomes met in, its kinds as morphs, its facts.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |id| !creature::territory::settled(id),
    facts: &[
        Fact {
            id: LUCK_FACT,
            item: "minecraft:rabbit_foot",
            key: "bestiary.ladybugs.luck",
        },
        Fact {
            id: ALL_SPOTS_FACT,
            item: "minecraft:golden_carrot",
            key: "bestiary.ladybugs.all_spots",
        },
        Fact {
            id: CLUSTER_FACT,
            item: "minecraft:oak_log",
            key: "bestiary.ladybugs.cluster",
        },
        Fact {
            id: CHICKEN_FACT,
            item: "minecraft:feather",
            key: "bestiary.ladybugs.chicken",
        },
        Fact {
            id: creature::jar::JAR_FACT,
            item: creature::jar::ITEM,
            key: "bestiary.ladybugs.jar",
        },
        Fact {
            id: creature::jar::FLOWER_FACT,
            item: "minecraft:oxeye_daisy",
            key: "bestiary.ladybugs.flower",
        },
    ],
    icon: EGG,
    scheme: Some(&SCHEME),
    rarity: creature::bestiary::Rarity::Common,
    draw: |draw, look| {
        let k = look.map_or(SEVEN, |l| l.pattern.min(KINDS.len() - 1));
        draw_kind(draw, k, &|_| [0.0; 3], WHITE);
    },
    draw_scale: 2.5,
    draw_lift: 0.0,
};

/// How often the night's clusters are looked for (ticks), how far a ladybug sees its cluster (blocks), how many make a
/// cluster seen (the fact).
const CLUSTER_EVERY: i32 = 20;
const CLUSTER_RANGE: f64 = 8.0;
const CLUSTER_SEEN: usize = 5;

/// Whether it is night in `level` (as the library's flight tells day from night).
fn night(host: &dyn ServerHost, level: mod_api::LevelRef) -> bool {
    level.day_time(host).unwrap_or(0).rem_euclid(24_000) >= 12_500
}

/// From `entity_tick_post`, for each ladybug: its look, its flight and behaviours (the library's small fliers,
/// despawning far from players), what they queued, a perch it lost, its cluster tonight.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    creature::colours::tick(host, id, &SCHEME);
    if creature::prey::tick(host, dimension, id, KIND) {
        forget(id);
        return;
    }
    creature::flutter::tick_post(host, dimension, KIND, id, || {
        vec![
            Box::new(luck::Perch::default()),
            Box::new(luck::Cluster),
            Box::new(creature::creep::Creep::new(&CREEPING)),
            Box::new(creature::flutter::Flutter::new(&FLIGHT)),
        ]
    });
    for action in luck::actions() {
        act(host, action);
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    // Off its player by other means (a save, the player gone): no longer perched. Its player hurt: it flies off.
    if luck::perched(id) {
        match e.vehicle(&*host) {
            None => luck::set_perched(id, false),
            Some(v) => {
                let health = host.living(v).and_then(|l| l.health(&*host));
                let before = RIDDEN.locked().get(&id).copied();
                if let (Some(h), Some(b)) = (health, before)
                    && h < b
                {
                    act(host, Action::Leave { bug: id });
                }
            }
        }
    }
    if e.tick_count(&*host).unwrap_or(0) % CLUSTER_EVERY == (id % CLUSTER_EVERY as u32) as i32 {
        cluster(host, dimension, id);
    }
}

/// The cold biomes, where ladybugs gather in clusters by day too (SPECS §3).
const COLD: &[&str] = &[
    "minecraft:snowy_plains",
    "minecraft:snowy_taiga",
    "minecraft:grove",
    "minecraft:snowy_slopes",
    "minecraft:ice_spikes",
    "minecraft:snowy_beach",
];
/// How far it looks for a shelter of leaves from the rain (blocks across), and how high leaves over it shelter it.
const SHELTER_RANGE: i32 = 6;
const SHELTER_ABOVE: i32 = 8;

/// Where ladybug `id` gathers now: in the rain, under the nearest leaves; at night or in a cold biome, by the ladybug
/// with the smallest id near it (a little off it, each its own place); none otherwise, or for that one. Players near a
/// cluster of `CLUSTER_SEEN` learn the fact.
fn cluster(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    if luck::perched(id) {
        luck::set_cluster(id, None);
        return;
    }
    let cell = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    let raining = level.weather(&*host).is_some_and(|(rain, _)| rain > 0.2);
    if raining && !night(&*host, level) {
        luck::set_cluster(id, shelter(&*host, level, cell));
        return;
    }
    let cold = level.biome_at(&*host, cell).is_some_and(|b| COLD.contains(&b.as_str()));
    if !night(&*host, level) && !cold {
        luck::set_cluster(id, None);
        return;
    }
    let r = CLUSTER_RANGE;
    let near: Vec<(u32, [f64; 3])> = level
        .entities_in(
            &*host,
            [at[0] - r, at[1] - r, at[2] - r],
            [at[0] + r, at[1] + r, at[2] + r],
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|o| o.kind(&*host).is_some_and(|k| k.id(&*host) == KIND))
        .filter_map(|o| Some((o.id(), o.position(&*host)?)))
        .collect();
    let Some(&(leader, there)) = near.iter().min_by_key(|(o, _)| *o) else {
        return;
    };
    if leader == id {
        luck::set_cluster(id, None);
    } else {
        // Packed around it, each at its own small offset.
        let angle = f64::from(id % 12) / 12.0 * std::f64::consts::TAU;
        let place = [there[0] + angle.cos() * 0.25, there[1], there[2] + angle.sin() * 0.25];
        luck::set_cluster(id, Some(place));
    }
    let packed = near
        .iter()
        .filter(|(_, p)| creature::body::distance(*p, there) < 1.5)
        .count();
    if packed >= CLUSTER_SEEN {
        for player in creature::bestiary::players_near(&*host, dimension, there) {
            creature::bestiary::learn(host, player, KIND, CLUSTER_FACT);
        }
    }
}

/// The nearest open spot on the ground with leaves within `SHELTER_ABOVE` over it, within `SHELTER_RANGE` of `cell`;
/// none when it is sheltered already or there is no such spot.
fn shelter(host: &dyn ServerHost, level: mod_api::LevelRef, cell: [i32; 3]) -> Option<[f64; 3]> {
    let ground = creature::territory::Ground::new(host, level);
    let covered = |p: [i32; 3]| (1..=SHELTER_ABOVE).any(|d| ground.block([p[0], p[1] + d, p[2]]).ends_with("leaves"));
    if covered(cell) {
        return None;
    }
    let mut best: Option<(i32, [f64; 3])> = None;
    for dx in -SHELTER_RANGE..=SHELTER_RANGE {
        for dz in -SHELTER_RANGE..=SHELTER_RANGE {
            for dy in -2..=2 {
                let p = [cell[0] + dx, cell[1] + dy, cell[2] + dz];
                let open = ground.block(p) == "minecraft:air";
                let floor = !matches!(
                    ground.block([p[0], p[1] - 1, p[2]]).as_str(),
                    "minecraft:air" | "minecraft:water"
                );
                let d = dx * dx + dz * dz + dy * dy;
                if open && floor && covered(p) && best.is_none_or(|b| d < b.0) {
                    best = Some((d, [f64::from(p[0]) + 0.5, f64::from(p[1]), f64::from(p[2]) + 0.5]));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// The key, on a player's saved data, of the kinds that gave them luck (a bit each, SPECS §4).
const LUCK_KEY: &str = "ladybugs:luck";

/// `player` got luck from kind `k`, saved; whether they now had it from every kind.
fn lucky_from(host: &mut dyn ServerHost, player: u128, k: usize) -> bool {
    let Some(p) = host.player(player) else {
        return false;
    };
    let Some(mut data) = p.persistent_data(&*host) else {
        return false;
    };
    let had = match data.get(&LUCK_KEY.into()) {
        Some(mod_api::Tag::Int(n)) => *n,
        _ => 0,
    };
    let now = had | 1 << k;
    data.insert(LUCK_KEY.into(), mod_api::Tag::Int(now));
    let _ = p.set_persistent_data(host, data);
    now == (1 << KINDS.len()) - 1
}

/// The health of the player each ladybug rides, as it landed: hurt, they make it fly off (SPECS §4).
static RIDDEN: std::sync::Mutex<std::collections::BTreeMap<u32, f32>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// What a behaviour queued (`luck::Action`).
fn act(host: &mut dyn ServerHost, action: Action) {
    match action {
        Action::Land { bug, player } => {
            let (Some(b), Some(p)) = (host.entity(bug), host.entity(player)) else {
                return;
            };
            if b.ride_player(host, p, true).is_err() {
                return;
            }
            luck::set_perched(bug, true);
            if let Some(level) = b.level(&*host) {
                luck::landed(bug, level.day_time(&*host).unwrap_or(0).div_euclid(24_000));
            }
            // Its luck: as many minutes as it has spots, the harlequin's Luck II (SPECS §4).
            let k = &KINDS[kind(bug)];
            creature::mob::give_effect_level(host, player, "minecraft:luck", k.spots * 1200, k.level);
            if let Some(uuid) = creature::mob::player_of(&*host, player) {
                creature::bestiary::learn(host, uuid, KIND, LUCK_FACT);
                if lucky_from(host, uuid, kind(bug)) {
                    creature::bestiary::learn(host, uuid, KIND, ALL_SPOTS_FACT);
                }
            }
            if let Some(l) = host.entity(player).and_then(|p| host.living(p)) {
                let health = l.health(&*host).unwrap_or(0.0);
                RIDDEN.locked().insert(bug, health);
            }
        }
        Action::Leave { bug } => {
            RIDDEN.locked().remove(&bug);
            if let Some(b) = host.entity(bug) {
                b.stop_riding(host);
            }
            luck::set_perched(bug, false);
        }
    }
}

fn forget(id: u32) {
    RIDDEN.locked().remove(&id);
    creature::flutter::gone(id);
    luck::forget(id);
}

/// How it holds itself this frame: in flight its elytra lift up and out and its hind wings spread and buzz; resting,
/// all folded.
fn pose(name: &str, age: f32, flying: bool) -> [f32; 3] {
    if !flying {
        return [0.0; 3];
    }
    let buzz = (age * 2.5).sin() * 20.0;
    match name {
        "elytron_left" => [40.0, 0.0, 25.0],
        "elytron_right" => [40.0, 0.0, -25.0],
        "wing_left" => [0.0, -70.0, buzz],
        "wing_right" => [0.0, 70.0, -buzz],
        _ => [0.0; 3],
    }
}

/// Draws ladybug kind `k` (SPECS §1): the body and wings as they are, the shell grey tinted by the kind's colour, its
/// spots over it in theirs; `tint` laid over all (the hurt flash).
fn draw_kind(draw: &mut dyn BlockEntityDrawContext, k: usize, pose: &dyn Fn(&str) -> [f32; 3], tint: u32) {
    let kind = &KINDS[k.min(KINDS.len() - 1)];
    let model = &model_ladybug::LADYBUG;
    let shell = |n: &str| !matches!(n, "body" | "wing_left" | "wing_right");
    let rest = |n: &str| !matches!(n, "elytron_left" | "elytron_right");
    draw_layer(draw, model, BASE, pose, &rest, tint, 0.0);
    draw_layer(draw, model, BASE, pose, &shell, multiply(kind.shell, tint), 0.0);
    draw_layer(
        draw,
        model,
        kind.spots_texture,
        pose,
        &shell,
        multiply(kind.spot, tint),
        0.02,
    );
}

/// Its own model in its kind's colours, its elytra open in flight (the library's small fliers tell when).
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let k = creature::colours::look(entity.id).map_or(SEVEN, |l: Look| l.pattern);
        let flying = creature::flutter::flying(entity.id) && !luck::perched(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |part: &str| pose(part, entity.age, flying);
        draw_kind(draw, k, &pose, creature::model::tint(mob));
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Ladybugs;

#[mod_api::sunrise_mod]
impl Hooks for Ladybugs {
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {
        creature::flutter::register(KIND, SIZE);
        creature::flutter::spawns(&SPAWNING);
        creature::jar::register(&IN_JAR);
        creature::prey::register(&PREY);
        creature::bestiary::register(&BESTIARY);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click: one on the player lets it go (SPECS §4); never a trading screen, which its template would open, on
    /// both sides. The glass jar is the library's (its hook runs first).
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        if let HostSide::Server(host) = host
            && let Ok(id) = u32::try_from(event.entity)
            && luck::perched(id)
        {
            act(host, Action::Leave { bug: id });
        }
        Some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == KIND {
                tick(host, event.dimension, event.entity);
            }
        });
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            forget(event.entity);
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_pattern_and_its_luck() {
        assert_eq!(KINDS.len(), PATTERNS.len());
        assert_eq!(KINDS[SEVEN].spots, 7);
        assert_eq!(KINDS[HARLEQUIN].level, 1);
        assert!(KINDS.iter().all(|k| k.spots > 0));
    }

    #[test]
    fn its_elytra_open_only_in_flight() {
        assert_eq!(pose("elytron_left", 0.0, false), [0.0; 3]);
        assert_ne!(pose("elytron_left", 0.0, true), [0.0; 3]);
    }
}
