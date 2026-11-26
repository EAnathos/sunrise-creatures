//! Phasmids (SPECS.md): stick insects and leaf insects hiding in the trees, of the Sunrise Creatures series.
//!
//! Each is a copy of the wandering trader drawn with its own model (`model_stick_insect.rs`, `model_leaf_insect.rs`,
//! from `sources/models/`), in its kind's texture: its kinds are its looks (the library's `colours`, rolled by biome),
//! each matching the trees it lives in. They come out on top of leaves (the library's spawning of small fliers, on
//! leaves), crawl over the canopy at night and climb its trunk and steps (the library's creeping), keep still by day or
//! when a player is near,
//! and are spotted by a player who looks at them a moment (`life::Watch`, counted on the player). Hit, they drop and
//! play dead, then climb back up (`life::PlayDead`). Caught in the glass jar, they settle on a tree (the library's tree
//! rule).
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]
mod life;
mod model_leaf_insect;
mod model_stick_insect;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use creature::colours::{Colour, Look, Pattern, Scheme, Variety};
use creature::jar::Catchable;
use creature::territory::{self, Ground};
use life::Action;
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Hooks,
    Host, HostSide, InteractionResult, LivingDeathEvent, RegisterEntityRenderersEvent, Registrar, RegistrationContext,
    ServerHost, Tag,
};

pub const STICK: &str = "phasmids:stick_insect";
pub const LEAF: &str = "phasmids:leaf_insect";

fn ours(kind: &str) -> bool {
    kind == STICK || kind == LEAF
}

/// Their boxes (blocks, SPECS §1) and the scale their models are drawn at: the stick insect about two thirds of a
/// block long with its antennae, the leaf insect half a block.
const STICK_SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.6,
    height: 0.2,
    eye_height: 0.1,
};
const LEAF_SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.5,
    height: 0.2,
    eye_height: 0.1,
};
const STICK_SCALE: f32 = 0.6;
const LEAF_SCALE: f32 = 0.7;

/// The stick insect's kinds and their textures (SPECS §2), each matching its trees.
const STICK_TEXTURES: [&str; 4] = [
    "phasmids:entity/stick_insect/oak",
    "phasmids:entity/stick_insect/birch",
    "phasmids:entity/stick_insect/spruce",
    "phasmids:entity/stick_insect/jungle",
];
const LEAF_TEXTURES: [&str; 2] = ["phasmids:entity/leaf_insect/green", "phasmids:entity/leaf_insect/dry"];

static PLAIN: [Colour; 1] = [Colour {
    id: "plain",
    rgb: 0xFFFFFF,
}];
static STICK_PATTERNS: [Pattern; 4] = [
    Pattern {
        id: "oak",
        texture: Some(STICK_TEXTURES[0]),
        swatch: Some(0x7A5A34),
    },
    Pattern {
        id: "birch",
        texture: Some(STICK_TEXTURES[1]),
        swatch: Some(0xE4E0D2),
    },
    Pattern {
        id: "spruce",
        texture: Some(STICK_TEXTURES[2]),
        swatch: Some(0x4A3420),
    },
    Pattern {
        id: "jungle",
        texture: Some(STICK_TEXTURES[3]),
        swatch: Some(0x5C8A34),
    },
];
static LEAF_PATTERNS: [Pattern; 2] = [
    Pattern {
        id: "green",
        texture: Some(LEAF_TEXTURES[0]),
        swatch: Some(0x5AA03C),
    },
    Pattern {
        id: "dry",
        texture: Some(LEAF_TEXTURES[1]),
        swatch: Some(0x9A7A40),
    },
];

const FORESTS: &[&str] = &[
    "minecraft:forest",
    "minecraft:flower_forest",
    "minecraft:dark_forest",
    "minecraft:plains",
    "minecraft:meadow",
];
const BIRCH: &[&str] = &["minecraft:birch_forest", "minecraft:old_growth_birch_forest"];
const TAIGA: &[&str] = &[
    "minecraft:taiga",
    "minecraft:old_growth_pine_taiga",
    "minecraft:old_growth_spruce_taiga",
];
const JUNGLE: &[&str] = &["minecraft:jungle", "minecraft:bamboo_jungle"];
const DRY: &[&str] = &["minecraft:sparse_jungle", "minecraft:savanna"];

static STICK_VARIETIES: [Variety; 5] = [
    Variety {
        biomes: FORESTS,
        base: &[(0, 1)],
        patterns: &[(0, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: BIRCH,
        base: &[(0, 1)],
        patterns: &[(1, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: TAIGA,
        base: &[(0, 1)],
        patterns: &[(2, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: JUNGLE,
        base: &[(0, 1)],
        patterns: &[(3, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(0, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static LEAF_VARIETIES: [Variety; 3] = [
    Variety {
        biomes: JUNGLE,
        base: &[(0, 1)],
        patterns: &[(0, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: DRY,
        base: &[(0, 1)],
        patterns: &[(1, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(0, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static STICK_SCHEME: Scheme = Scheme {
    colours: &PLAIN,
    patterns: &STICK_PATTERNS,
    base_texture: STICK_TEXTURES[0],
    varieties: &STICK_VARIETIES,
};
static LEAF_SCHEME: Scheme = Scheme {
    colours: &PLAIN,
    patterns: &LEAF_PATTERNS,
    base_texture: LEAF_TEXTURES[0],
    varieties: &LEAF_VARIETIES,
};

const STICK_BIOMES: &[&str] = &[
    "minecraft:forest",
    "minecraft:flower_forest",
    "minecraft:dark_forest",
    "minecraft:plains",
    "minecraft:meadow",
    "minecraft:birch_forest",
    "minecraft:old_growth_birch_forest",
    "minecraft:taiga",
    "minecraft:old_growth_pine_taiga",
    "minecraft:old_growth_spruce_taiga",
    "minecraft:jungle",
    "minecraft:bamboo_jungle",
];
const LEAF_BIOMES: &[&str] = &[
    "minecraft:jungle",
    "minecraft:bamboo_jungle",
    "minecraft:sparse_jungle",
    "minecraft:savanna",
];

/// When they come out (the library's spawning takes a small flier's hours; they don't fly): any time but the rain.
static HOURS: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Always,
    speed: 0.0,
    height: (0.0, 0.0),
    flight: (1, 2),
    rest: (1, 2),
    wander: 0.0,
    drawn_to_light: false,
    lands_on_flowers: false,
    takeoff: None,
    ..creature::flutter::Kind::DEFAULT
};

/// On top of leaves in their biomes (SPECS §3): 1 or 2 together, up to 4 within 32 blocks of a player.
static SPAWNING: [creature::flutter::Spawning; 2] = [
    creature::flutter::Spawning {
        kind: STICK,
        flight: &HOURS,
        biomes: STICK_BIOMES,
        group: (1, 2),
        cap: 4,
        chance: 3,
        ..creature::flutter::Spawning::DEFAULT
    },
    creature::flutter::Spawning {
        kind: LEAF,
        flight: &HOURS,
        biomes: LEAF_BIOMES,
        group: (1, 2),
        cap: 4,
        chance: 3,
        ..creature::flutter::Spawning::DEFAULT
    },
];

/// Caught in the library's glass jar, they settle on a tree (the tree rule, SPECS §5).
static IN_JAR: [Catchable; 2] = [
    Catchable {
        kind: STICK,
        model: "phasmids:stick_insect_jar",
        settles: Some(territory::Rule::Tree),
        glows: false,
        ..Catchable::DEFAULT
    },
    Catchable {
        kind: LEAF,
        model: "phasmids:leaf_insect_jar",
        settles: Some(territory::Rule::Tree),
        glows: false,
        ..Catchable::DEFAULT
    },
];

const SPOTTED_FACT: &str = "spotted";
const KEEN_FACT: &str = "keen_eye";
const MASTER_FACT: &str = "master";
const DEAD_FACT: &str = "play_dead";
const NIGHT_FACT: &str = "night_walker";

/// The spottings that teach a fact (SPECS §4): the first, the tenth, the fiftieth.
const KEEN: i32 = 10;
const MASTER: i32 = 50;

const FACTS: &[Fact] = &[
    Fact {
        id: SPOTTED_FACT,
        item: "minecraft:spyglass",
        key: "bestiary.phasmids.spotted",
    },
    Fact {
        id: KEEN_FACT,
        item: "minecraft:ender_eye",
        key: "bestiary.phasmids.keen_eye",
    },
    Fact {
        id: MASTER_FACT,
        item: "minecraft:golden_carrot",
        key: "bestiary.phasmids.master",
    },
    Fact {
        id: DEAD_FACT,
        item: "minecraft:bone",
        key: "bestiary.phasmids.play_dead",
    },
    Fact {
        id: NIGHT_FACT,
        item: "minecraft:clock",
        key: "bestiary.phasmids.night_walker",
    },
    Fact {
        id: creature::jar::JAR_FACT,
        item: creature::jar::ITEM,
        key: "bestiary.phasmids.jar",
    },
    Fact {
        id: creature::jar::TREE_FACT,
        item: "minecraft:oak_sapling",
        key: "bestiary.phasmids.tree",
    },
];

/// A twig it holds on to in its jar and its guide page (SPECS §5): a bark-brown rod from the oak stick insect's
/// texture (its thorax's side), under its feet, in texels.
fn twig(draw: &mut dyn BlockEntityDrawContext) {
    let bark = mod_api::CubeFace::new(STICK_TEXTURES[0]).with_uv([5.5, 3.0, 8.0, 3.5]);
    let faces = [Some(bark); 6];
    draw.cube([-0.6, -0.6, -9.0], [0.6, 0.0, 9.0], faces, creature::model::WHITE);
}

fn wild(id: u32) -> bool {
    !territory::settled(id)
}

static BESTIARY: [Species; 2] = [
    Species {
        id: STICK,
        biomes: STICK_BIOMES,
        wild,
        facts: FACTS,
        icon: "phasmids:stick_insect_spawn_egg",
        scheme: Some(&STICK_SCHEME),
        rarity: creature::bestiary::Rarity::Uncommon,
        draw: |draw, look| {
            let t = STICK_TEXTURES[look.map_or(0, |l| l.pattern).min(STICK_TEXTURES.len() - 1)];
            twig(draw);
            creature::model::draw(draw, &model_stick_insect::STICK_INSECT, t, &|_| [0.0; 3]);
        },
        draw_scale: 1.2,
        draw_lift: 0.0,
        ..Species::DEFAULT
    },
    Species {
        id: LEAF,
        biomes: LEAF_BIOMES,
        wild,
        facts: FACTS,
        icon: "phasmids:leaf_insect_spawn_egg",
        scheme: Some(&LEAF_SCHEME),
        rarity: creature::bestiary::Rarity::Rare,
        draw: |draw, look| {
            let t = LEAF_TEXTURES[look.map_or(0, |l| l.pattern).min(LEAF_TEXTURES.len() - 1)];
            twig(draw);
            creature::model::draw(draw, &model_leaf_insect::LEAF_INSECT, t, &|_| [0.0; 3]);
        },
        draw_scale: 1.4,
        draw_lift: 0.0,
        ..Species::DEFAULT
    },
];

/// Where it crawls (SPECS §3): over its tree's leaves when settled (the tree rule), else over the leaves near it,
/// never down to the ground: the tops of the leaves with open air over them.
fn canopy(host: &dyn Host, id: u32) -> Vec<[i32; 3]> {
    let Some(ground) = Ground::of_entity(host, id) else {
        return Vec::new();
    };
    let tops = |blocks: Vec<[i32; 3]>| -> Vec<[i32; 3]> {
        blocks
            .into_iter()
            .filter(|p| ground.block(*p).ends_with("leaves") && ground.block([p[0], p[1] + 1, p[2]]) == "minecraft:air")
            .collect()
    };
    if territory::settled(id) {
        return tops(territory::check_tree(host, id));
    }
    let Some(at) = host.entity(id).and_then(|e| e.position(host)) else {
        return Vec::new();
    };
    let (x, y, z) = (at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32);
    // The leaves near it, up to the canopy over it (it climbs the trunk to reach them).
    let mut near = Vec::new();
    for dx in -3..=3 {
        for dz in -3..=3 {
            for dy in -3..=6 {
                near.push([x + dx, y + dy, z + dz]);
            }
        }
    }
    tops(near)
}

/// Its crawl (the library's creeping, SPECS §3): slow, over the canopy's leaves, up a trunk or a step of leaves on its
/// way; only when it doesn't keep still.
static CREEPING: creature::creep::Creeping = creature::creep::Creeping {
    speed: 0.012,
    climb: 0.02,
    crawl: (60, 160),
    pause: (100, 300),
    wander: 2.0,
    area: canopy,
    may: |m, _| !life::busy(m.id),
    rustle: Some(RUSTLE),
    ..creature::creep::Creeping::DEFAULT
};

/// Its sounds (SPECS §2): a faint rustle of leaves as it crawls, a dry click as it drops to play dead (`life`).
const RUSTLE: &str = "phasmids:entity.phasmid.rustle";
pub(crate) const CLICK: &str = "phasmids:entity.phasmid.click";

/// How often the night walker fact is looked for (ticks), and from how near (blocks).
const NIGHT_EVERY: i32 = 40;
const NIGHT_NEAR: f64 = 8.0;

/// From `entity_tick_post`, for each phasmid: its look, its behaviours (the library's tick of small creatures:
/// territory, despawning), what they queued, the night walker fact.
fn tick(host: &mut dyn ServerHost, dimension: &str, kind: &str, id: u32) {
    let scheme = if kind == STICK { &STICK_SCHEME } else { &LEAF_SCHEME };
    let species = if kind == STICK { STICK } else { LEAF };
    creature::colours::tick(host, id, scheme);
    creature::flutter::tick_post(host, dimension, kind, id, || {
        vec![
            Box::new(life::Watch::default()),
            Box::new(life::PlayDead::default()),
            Box::new(creature::creep::Creep::climber(&CREEPING)),
        ]
    });
    for action in life::actions() {
        act(host, dimension, action);
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    if e.tick_count(&*host).unwrap_or(0) % NIGHT_EVERY == 0 && !life::busy(id) {
        let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
            return;
        };
        let night = level.day_time(&*host).unwrap_or(0).rem_euclid(24_000) >= 12_500;
        if night {
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                let near = host
                    .player(player)
                    .and_then(|p| p.entity().position(&*host))
                    .is_some_and(|p| creature::body::distance(p, at) < NIGHT_NEAR);
                if near {
                    creature::bestiary::learn(host, player, species, NIGHT_FACT);
                }
            }
        }
    }
}

fn kind_of(host: &dyn ServerHost, id: u32) -> &'static str {
    let kind = host.entity(id).and_then(|e| e.kind(host)).map(|k| k.id(host));
    if kind.as_deref() == Some(LEAF) { LEAF } else { STICK }
}

/// The key, on a player's saved data, of how many phasmids they spotted.
const SPOTTED_KEY: &str = "phasmids:spotted";

/// How many phasmids `player` spotted, one more saved (SPECS §4).
fn spotted_one_more(host: &mut dyn ServerHost, player: u128) -> i32 {
    let Some(p) = host.player(player) else {
        return 0;
    };
    let Some(mut data) = p.persistent_data(&*host) else {
        return 0;
    };
    let count = match data.get(&SPOTTED_KEY.into()) {
        Some(Tag::Int(n)) => *n + 1,
        _ => 1,
    };
    data.insert(SPOTTED_KEY.into(), Tag::Int(count));
    let _ = p.set_persistent_data(host, data);
    count
}

/// What a behaviour queued (`life::Action`).
fn act(host: &mut dyn ServerHost, dimension: &str, action: Action) {
    match action {
        Action::Spotted { id, player } => {
            let kind = kind_of(&*host, id);
            // A faint shimmer shows it.
            if let Some(e) = host.entity(id)
                && let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host))
            {
                creature::particles::puff(host, level, "minecraft:glow", [at[0], at[1] + 0.2, at[2]], 0.25, 8);
            }
            let count = spotted_one_more(host, player);
            creature::bestiary::learn(host, player, kind, SPOTTED_FACT);
            if count >= KEEN {
                creature::bestiary::learn(host, player, kind, KEEN_FACT);
            }
            if count >= MASTER {
                creature::bestiary::learn(host, player, kind, MASTER_FACT);
            }
        }
        Action::Dropped { id } => {
            let kind = kind_of(&*host, id);
            let Some(at) = host.entity(id).and_then(|e| e.position(&*host)) else {
                return;
            };
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                creature::bestiary::learn(host, player, kind, DEAD_FACT);
            }
        }
    }
}

fn forget(id: u32) {
    creature::flutter::gone(id);
    life::forget(id);
}

/// How it holds itself this frame: walking, it sways side to side as a twig in the wind (SPECS §3) and its legs step;
/// playing dead, it lies on its back with its legs folded.
fn pose(name: &str, age: f32, walk: f32, speed: f32, dead: bool) -> [f32; 3] {
    if dead {
        return match name {
            "body" => [0.0, 0.0, 180.0],
            n if n.starts_with("leg_") && n.ends_with("left") => [0.0, 0.0, -40.0],
            n if n.starts_with("leg_") && n.ends_with("right") => [0.0, 0.0, 40.0],
            _ => [0.0; 3],
        };
    }
    let moving = speed.min(1.0);
    let sway = (age * 0.25).sin() * 8.0 * moving;
    let step = (walk * 0.9).sin() * 25.0 * moving;
    match name {
        "body" => [0.0, 0.0, sway],
        "leg_front_left" | "leg_hind_left" | "leg_mid_right" => [0.0, step, 0.0],
        "leg_front_right" | "leg_hind_right" | "leg_mid_left" => [0.0, -step, 0.0],
        "antenna_left" | "antenna_right" => [(age * 0.1).sin() * 4.0, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Drawn on a trunk (`creature::creep::shown_clinging`): lifted so its tail is at its feet, and moved to the bark it faces
/// (its middle is held 0.3 from it), in blocks.
const CLING_LIFT: f32 = 0.3;
const CLING_REACH: f32 = 0.27;

/// Whether it plays dead, shown to the clients for its drawing (`creature::shown`).
const DEAD: &str = "phasmids:dead";

/// Its own model in its kind's texture.
struct Draw {
    stick: bool,
}

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        // What the server shows of it (`creature::shown`): read the same on any client.
        let look: Look = creature::colours::shown_look(entity).unwrap_or_default();
        let (part, texture, scale) = if self.stick {
            (
                &model_stick_insect::STICK_INSECT,
                STICK_TEXTURES[look.pattern.min(STICK_TEXTURES.len() - 1)],
                STICK_SCALE,
            )
        } else {
            (
                &model_leaf_insect::LEAF_INSECT,
                LEAF_TEXTURES[look.pattern.min(LEAF_TEXTURES.len() - 1)],
                LEAF_SCALE,
            )
        };
        let (walk, speed) = mob.map_or((0.0, 0.0), |m| (m.walk_animation_pos, m.walk_animation_speed));
        let dead = creature::shown::flag(entity, DEAD);
        draw.push();
        creature::model::living_pose(draw, entity);
        if !dead && creature::creep::shown_clinging(entity) {
            // On a trunk: upright, head up, its belly to the bark it faces, its tail at its feet.
            draw.translate([0.0, CLING_LIFT, CLING_REACH]);
            draw.rotate_about([1.0, 0.0, 0.0], -90.0, [0.0; 3]);
        }
        draw.scale([scale; 3]);
        if dead {
            // On its back: lifted by its own height so it lies on the ground.
            draw.translate([0.0, 0.12, 0.0]);
        }
        let pose = |p: &str| pose(p, entity.age, walk, speed, dead);
        creature::model::draw_shown(draw, part, texture, &pose, &|_| true, creature::model::tint(mob));
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Phasmids;

#[mod_api::sunrise_mod]
impl Hooks for Phasmids {
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {
        creature::flutter::register(STICK, STICK_SIZE);
        creature::flutter::register(LEAF, LEAF_SIZE);
        for s in &SPAWNING {
            creature::flutter::spawns_on_leaves(s);
        }
        creature::shown::register(DEAD, &[STICK, LEAF], |id| i32::from(life::dead(id)));
        for c in &IN_JAR {
            creature::jar::register(c);
        }
        for species in &BESTIARY {
            creature::bestiary::register(species);
        }
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(STICK.into(), Arc::new(Draw { stick: true }));
        event.renderers.register(LEAF.into(), Arc::new(Draw { stick: false }));
    }

    /// Right-click: nothing (no trading screen, which their template would open), on both sides; the glass jar is the
    /// library's (its hook runs first).
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        ours(event.kind).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if ours(event.kind) {
                tick(host, event.dimension, event.kind, event.entity);
            }
        });
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if ours(event.kind) {
            forget(event.entity);
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_texture() {
        assert_eq!(STICK_PATTERNS.len(), STICK_TEXTURES.len());
        assert_eq!(LEAF_PATTERNS.len(), LEAF_TEXTURES.len());
        for (k, p) in STICK_PATTERNS.iter().enumerate() {
            assert_eq!(p.texture, Some(STICK_TEXTURES[k]));
        }
    }

    #[test]
    fn it_sways_only_when_it_walks() {
        assert_eq!(pose("body", 3.0, 0.0, 0.0, false), [0.0; 3]);
        assert_ne!(pose("body", 3.0, 1.0, 1.0, false), [0.0; 3]);
        assert_eq!(pose("body", 0.0, 0.0, 0.0, true), [0.0, 0.0, 180.0]);
    }
}
