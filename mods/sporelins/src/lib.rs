//! Sporelins (SPECS.md): small, shy mushroom sprites of dark forests and mushroom fields, of the Sunrise Creatures
//! series.
//!
//! A sporelin is a copy of the wandering trader drawn with its own model, a red cap (`model_sporelin.rs`) or a brown
//! cap (`model_sporelin_brown.rs`): its looks (the library's `colours`, rolled by biome). It comes out in the dark (the
//! library's spawning of small creatures, on given blocks), hides as a mushroom from a player who comes near, puffs
//! spores when startled, plants mushrooms (`life`). Given a mushroom of its colour it may befriend the player (the
//! library's friends): it follows them or stays and tends a mushroom patch, fills a bowl with stew once a day and its
//! cap's spots glow at night. Killed, it drops a mushroom of its colour.
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]
mod life;
mod model_sporelin;
mod model_sporelin_brown;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use creature::colours::{Colour, Pattern, Scheme, Variety};
use creature::state::EntityMap;
use creature::territory::Ground;
use life::Action;
use mod_api::{
    AttachmentCodec, AttachmentType, BlockEntityDrawContext, ClientHost, Compound, EmissiveEvent, EntityInteractEvent,
    EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Holder, Hooks, HostSide, InteractionResult, LevelRef,
    LightLayer, LivingDeathEvent, PlayerRef, RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost,
    SoundSource, Tag, Text,
};

pub const KIND: &str = "sporelins:sporelin";

/// Its box (blocks, SPECS §1): about two thirds of a block tall, drawn at its model's size.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.5,
    height: 0.65,
    eye_height: 0.45,
};

/// Its looks (SPECS §1): a red cap and a brown cap, each its model and texture; at night a befriended one is drawn in
/// its lit texture, whose spots glow (`emissive`).
const TEXTURES: [&str; 2] = ["sporelins:entity/sporelin/red", "sporelins:entity/sporelin/brown"];
const LIT: [&str; 2] = [
    "sporelins:entity/sporelin/red_lit",
    "sporelins:entity/sporelin/brown_lit",
];
const GLOWS: [&str; 2] = [
    "sporelins:entity/sporelin/red_lit_glow",
    "sporelins:entity/sporelin/brown_lit_glow",
];
const MUSHROOMS: [&str; 2] = ["minecraft:red_mushroom", "minecraft:brown_mushroom"];

static PLAIN: [Colour; 1] = [Colour {
    id: "plain",
    rgb: 0xFFFFFF,
}];
static PATTERNS: [Pattern; 2] = [
    Pattern {
        id: "red",
        texture: Some(TEXTURES[0]),
        swatch: Some(0xB8281F),
    },
    Pattern {
        id: "brown",
        texture: Some(TEXTURES[1]),
        swatch: Some(0x9A6B47),
    },
];

/// Where it lives (SPECS §2).
const BIOMES: &[&str] = &[
    "minecraft:dark_forest",
    "minecraft:mushroom_fields",
    "minecraft:old_growth_pine_taiga",
    "minecraft:old_growth_spruce_taiga",
    "minecraft:swamp",
];
/// Red caps are the more common in dark forests and mushroom fields; elsewhere both alike.
static VARIETIES: [Variety; 2] = [
    Variety {
        biomes: &["minecraft:dark_forest", "minecraft:mushroom_fields"],
        base: &[(0, 1)],
        patterns: &[(0, 3), (1, 1)],
        pattern_colours: &[(0, 1)],
    },
    Variety {
        biomes: &[],
        base: &[(0, 1)],
        patterns: &[(0, 1), (1, 1)],
        pattern_colours: &[(0, 1)],
    },
];
static SCHEME: Scheme = Scheme {
    colours: &PLAIN,
    patterns: &PATTERNS,
    base_texture: TEXTURES[0],
    varieties: &VARIETIES,
};

/// When it comes out (the library's spawning takes a small flier's hours; it doesn't fly): any time but the rain; the
/// dark is its block's (`spawns_on`).
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

/// 1 to 3 together, up to 3 within 48 blocks of a player (SPECS §2).
static SPAWNING: creature::flutter::Spawning = creature::flutter::Spawning {
    kind: KIND,
    flight: &HOURS,
    biomes: BIOMES,
    group: (1, 3),
    cap: 3,
    chance: 4,
    ..creature::flutter::Spawning::DEFAULT
};

/// The ground mushrooms grow on, and where it comes out.
const SOILS: &[&str] = &[
    "minecraft:grass_block",
    "minecraft:dirt",
    "minecraft:coarse_dirt",
    "minecraft:rooted_dirt",
    "minecraft:podzol",
    "minecraft:mycelium",
    "minecraft:moss_block",
];
/// Where it is dark (SPECS §2): this much light or less, the sky's and the blocks'.
const DARK: i32 = 7;
/// Mushrooms keep under this much light, but on mycelium or podzol (as vanilla's).
const MUSHROOM_LIGHT: i32 = 13;

/// The light at `p`: the sky's (its raw light, by day; at night none) and the blocks', the higher.
fn light(ground: Ground<'_>, p: [i32; 3], night: bool) -> i32 {
    let read = |layer| ground.level.brightness(ground.host, layer, p).unwrap_or(15);
    let sky = if night { 0 } else { read(LightLayer::Sky) };
    sky.max(read(LightLayer::Block))
}

fn night(ground: Ground<'_>) -> bool {
    ground.level.day_time(ground.host).unwrap_or(0).rem_euclid(24_000) >= 12_500
}

/// It comes out on its soil, in the dark.
fn spawns_on(ground: Ground<'_>, p: [i32; 3]) -> bool {
    SOILS.contains(&ground.block(p).as_str()) && light(ground, [p[0], p[1] + 1, p[2]], night(ground)) <= DARK
}

const HIDES_FACT: &str = "hides";
const MUSHROOM_FACT: &str = "mushroom";
const PATCH_FACT: &str = "patch";
const STEW_FACT: &str = "stew";

/// Its page (SPECS §5): the biomes met in, its looks, its facts.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |id| creature::friend::of(id).is_none(),
    facts: &[
        Fact {
            id: HIDES_FACT,
            item: "minecraft:spyglass",
            key: "bestiary.sporelins.hides",
        },
        Fact {
            id: MUSHROOM_FACT,
            item: "minecraft:red_mushroom",
            key: "bestiary.sporelins.mushroom",
        },
        Fact {
            id: PATCH_FACT,
            item: "minecraft:mycelium",
            key: "bestiary.sporelins.patch",
        },
        Fact {
            id: STEW_FACT,
            item: "minecraft:mushroom_stew",
            key: "bestiary.sporelins.stew",
        },
    ],
    icon: "sporelins:sporelin_spawn_egg",
    scheme: Some(&SCHEME),
    rarity: creature::bestiary::Rarity::Uncommon,
    draw: |draw, look| {
        let k = look.map_or(0, |l| l.pattern).min(1);
        creature::model::draw(draw, model(k), TEXTURES[k], &|_| [0.0; 3]);
    },
    draw_scale: 1.0,
    draw_lift: 0.0,
    ..Species::DEFAULT
};

fn model(look: usize) -> &'static creature::model::Part {
    if look == 1 {
        &model_sporelin_brown::SPORELIN_BROWN
    } else {
        &model_sporelin::SPORELIN
    }
}

fn look_of(id: u32) -> usize {
    creature::colours::look(id).map_or(0, |l| l.pattern).min(1)
}

/// Its night light when befriended (SPECS §3): soft, the colour of its cap's spots, lighting nothing.
const NIGHT_LIGHT: [u32; 2] = [0xFFE0C8, 0xF0D8A8];

/// The befriended ones glowing now, for their drawing.
static GLOWING: EntityMap<()> = EntityMap::new();

fn glowing(id: u32) -> bool {
    GLOWING.locked().contains_key(&id)
}

/// From `entity_tick_post`, for each sporelin: its look, its behaviours (the library's tick of small creatures:
/// friends, despawning), what they queued, where it stays, its glow at night.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    creature::colours::tick(host, id, &SCHEME);
    creature::friend::tick(host, id);
    creature::flutter::tick_post(host, dimension, KIND, id, || {
        vec![
            Box::new(life::Startle::default()),
            Box::new(life::Hide::default()),
            Box::new(life::Follow::default()),
            Box::new(life::Plant::default()),
            Box::new(life::Stroll::default()),
        ]
    });
    for action in life::actions() {
        act(host, dimension, action);
    }
    life::tick_shake(id);
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let friend = creature::friend::of(id);
    // Told to stay: it stays (and tends its patch) where it was told.
    match friend.filter(|f| f.staying) {
        Some(_) if life::stays_at(id).is_none() => life::set_stays_at(id, Some(at)),
        None => life::set_stays_at(id, None),
        _ => {}
    }
    let glows = friend.is_some() && night(Ground::new(&*host, level));
    if glows {
        GLOWING.locked().insert(id, ());
        let light = creature::glow::Light {
            rgb: NIGHT_LIGHT[look_of(id)],
            radius: 3.0,
            intensity: 0.35,
            lift: 0.5,
            shown: |_| true,
            ..creature::glow::Light::DEFAULT
        };
        creature::glow::shine(host, dimension, id, light);
    } else if GLOWING.locked().remove(&id).is_some() {
        creature::glow::dim(id);
    }
}

/// What a behaviour queued (`life::Action`).
fn act(host: &mut dyn ServerHost, dimension: &str, action: Action) {
    match action {
        Action::Hid { id } => {
            let Some(at) = host.entity(id).and_then(|e| e.position(&*host)) else {
                return;
            };
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                creature::bestiary::learn(host, player, KIND, HIDES_FACT);
            }
        }
        Action::Puff { id } => {
            let Some(e) = host.entity(id) else {
                return;
            };
            let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
                return;
            };
            puff(host, level, at);
        }
        Action::Plant {
            id,
            around,
            radius,
            tending,
        } => {
            let Some(e) = host.entity(id) else {
                return;
            };
            let Some(level) = e.level(&*host) else {
                return;
            };
            let seed = e.tick_count(&*host).unwrap_or(0) as u32 ^ id;
            if let Some(p) = plant(host, level, look_of(id), around, radius, tending, seed) {
                let at = [f64::from(p[0]) + 0.5, f64::from(p[1]), f64::from(p[2]) + 0.5];
                for player in creature::bestiary::players_near(&*host, dimension, at) {
                    creature::bestiary::learn(host, player, KIND, PATCH_FACT);
                }
            }
        }
    }
}

/// A puff of spores and a soft "pff" (SPECS §2): harmless.
fn puff(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3]) {
    let middle = [at[0], at[1] + 0.4, at[2]];
    creature::particles::puff(host, level, "minecraft:spore_blossom_air", middle, 0.4, 14);
    creature::particles::puff(host, level, "minecraft:poof", middle, 0.2, 4);
    level.play_sound(
        host,
        "minecraft:entity.puffer_fish.blow_out",
        at,
        SoundSource::Neutral,
        0.4,
        1.6,
        None,
    );
}

/// Mushrooms near: never more than `WILD_MUSHROOMS` within `WILD_REACH` blocks of a wild one (SPECS §2), up to
/// `PATCH_MUSHROOMS` in a tended patch.
const WILD_MUSHROOMS: usize = 4;
const WILD_REACH: i32 = 5;
const PATCH_MUSHROOMS: usize = 16;

/// Plants a mushroom of look `look`'s colour within `radius` blocks of `around` (SPECS §2, §3): on its soil, in the
/// dark (or on mycelium or podzol), never too many near; tending, half the time it spreads one already there. Where,
/// if it did.
fn plant(
    host: &mut dyn ServerHost,
    level: LevelRef,
    look: usize,
    around: [f64; 3],
    radius: i32,
    tending: bool,
    seed: u32,
) -> Option<[i32; 3]> {
    let ground = Ground::new(&*host, level);
    let c = around.map(|v| v.floor() as i32);
    let mushroom = |p: [i32; 3]| ground.block(p).ends_with("_mushroom");
    let (reach, most) = if tending {
        (life::PATCH, PATCH_MUSHROOMS)
    } else {
        (WILD_REACH, WILD_MUSHROOMS)
    };
    let mut near = Vec::new();
    for dx in -reach..=reach {
        for dz in -reach..=reach {
            for dy in -2..=2 {
                let p = [c[0] + dx, c[1] + dy, c[2] + dz];
                if mushroom(p) {
                    near.push(p);
                }
            }
        }
    }
    if near.len() >= most {
        return None;
    }
    let night = night(ground);
    let fits = |p: [i32; 3]| {
        let below = ground.block([p[0], p[1] - 1, p[2]]);
        let open = matches!(ground.block(p).as_str(), "minecraft:air" | "minecraft:cave_air");
        let rich = below == "minecraft:mycelium" || below == "minecraft:podzol";
        open && SOILS.contains(&below.as_str()) && (rich || light(ground, p, night) < MUSHROOM_LIGHT)
    };
    let mut random = creature::colours::Random::new(seed);
    // Tending: a mushroom already there spreads to a block next to it.
    let spread: Vec<[i32; 3]> = if tending && random.next().is_multiple_of(2) {
        near.iter()
            .flat_map(|m| [[1, 0], [-1, 0], [0, 1], [0, -1]].map(|d| [m[0] + d[0], m[1], m[2] + d[1]]))
            .filter(|p| fits(*p))
            .collect()
    } else {
        Vec::new()
    };
    let spots = if spread.is_empty() {
        let mut all = Vec::new();
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                for dy in -2..=2 {
                    let p = [c[0] + dx, c[1] + dy, c[2] + dz];
                    if fits(p) {
                        all.push(p);
                    }
                }
            }
        }
        all
    } else {
        spread
    };
    if spots.is_empty() {
        return None;
    }
    let p = spots[(random.next() % spots.len() as u64) as usize];
    let state = host.block_state_id(MUSHROOMS[look])?;
    let flags = mod_api::BlockUpdateFlags::NEIGHBORS | mod_api::BlockUpdateFlags::CLIENTS;
    if !level.set_block(host, p, state, flags).unwrap_or(false) {
        return None;
    }
    let top = [f64::from(p[0]) + 0.5, f64::from(p[1]) + 0.3, f64::from(p[2]) + 0.5];
    creature::particles::puff(host, level, "minecraft:spore_blossom_air", top, 0.2, 5);
    Some(p)
}

/// One chance in this many that a mushroom of its colour befriends (SPECS §3); what a mushroom heals a friend.
const BEFRIEND_ONE_IN: u64 = 3;
const HEAL: f32 = 2.0;

/// The day a sporelin last filled a bowl (the game's day count), saved on it (`sporelins:stew`): once a day each, a
/// reload included.
static STEW: AttachmentType<Option<i64>> = AttachmentType {
    namespace: "sporelins",
    name: "stew",
    default: || None,
    codec: Some(AttachmentCodec {
        encode: |day| {
            let mut c = Compound::new();
            if let Some(d) = day {
                c.insert("Day".into(), Tag::Long(*d));
            }
            Ok(c)
        },
        decode: |c| {
            Ok(match c.get(&"Day".into()) {
                Some(Tag::Long(d)) => Some(*d),
                _ => None,
            })
        },
    }),
    copy_on_death: false,
    sync: false,
};

/// Its right-click (SPECS §3), on the server: a mushroom of its colour to befriend it (or heal a friend), a bowl for a
/// stew, an empty hand to tell it to stay or follow.
fn interact(host: &mut dyn ServerHost, id: u32, player: PlayerRef, hand: mod_api::Hand, item: Option<String>) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let look = look_of(id);
    let friend = creature::friend::of(id);
    let mine = friend.is_some_and(|f| f.is(player.uuid()));
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    let middle = [at[0], at[1] + 0.5, at[2]];
    match item.as_deref() {
        Some(m) if m == MUSHROOMS[look] && friend.is_none() => {
            if !creative {
                creature::hand::use_one(host, player, hand);
            }
            let seed = e.tick_count(&*host).unwrap_or(0) as u32 ^ id;
            let befriended = creature::colours::Random::new(seed)
                .next()
                .is_multiple_of(BEFRIEND_ONE_IN);
            if befriended {
                creature::friend::befriend(host, id, player.uuid());
                creature::bestiary::learn(host, player.uuid(), KIND, MUSHROOM_FACT);
                let _ = player.action_bar(host, Text::translate("sporelins.befriended", vec![]));
            } else {
                puff(host, level, at);
            }
            creature::friend::sign(host, level, middle, befriended);
        }
        Some(m) if m == MUSHROOMS[look] && mine => {
            if !creative {
                creature::hand::use_one(host, player, hand);
            }
            if let Some(living) = host.living(e) {
                let health = living.health(&*host).unwrap_or(0.0);
                let most = living.max_health(&*host).unwrap_or(health);
                let _ = living.set_health(host, (health + HEAL).min(most));
            }
            creature::particles::puff(host, level, "minecraft:happy_villager", middle, 0.3, 5);
        }
        Some("minecraft:bowl") if mine => {
            let day = level.day_time(&*host).unwrap_or(0).div_euclid(24_000);
            let last = STEW.get_existing(host, Holder::Entity(id)).ok().flatten().flatten();
            if last == Some(day) {
                life::shake(id);
                return;
            }
            let _ = STEW.set(host, Holder::Entity(id), Some(day));
            let Some(stew) = creature::hand::stack(&*host, "minecraft:mushroom_stew", 1) else {
                return;
            };
            creature::hand::use_one(host, player, hand);
            creature::hand::give(host, player, stew);
            creature::bestiary::learn(host, player.uuid(), KIND, STEW_FACT);
            puff(host, level, at);
        }
        None if mine => {
            let staying = !friend.is_some_and(|f| f.staying);
            creature::friend::set_staying(id, staying);
            let key = if staying {
                "sporelins.stays"
            } else {
                "sporelins.follows"
            };
            let _ = player.action_bar(host, Text::translate(key, vec![]));
        }
        _ => {}
    }
}

/// How it holds itself this frame: walking, it waddles (its body rocks, its legs step, its arms swing); hiding, it
/// sits down with its cap pulled over its stem; shaking its cap, its head turns side to side.
fn pose(name: &str, walk: f32, speed: f32, shake: Option<f32>) -> [f32; 3] {
    let moving = speed.min(1.0);
    let step = (walk * 0.8).sin() * 30.0 * moving;
    let rock = (walk * 0.8).cos() * 8.0 * moving;
    match name {
        "body" => [0.0, 0.0, rock],
        "head" => match shake {
            Some(f) => [0.0, (f * std::f32::consts::TAU * 3.0).sin() * 20.0, 0.0],
            None => [0.0; 3],
        },
        "leg_left" | "arm_right" => [step, 0.0, 0.0],
        "leg_right" | "arm_left" => [-step, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// How far it sinks when hiding (blocks): the height of its legs, so its cap rests on the ground about its stem.
const SIT: f32 = 3.0 / 16.0;

struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let k = look_of(entity.id);
        let texture = if glowing(entity.id) { LIT[k] } else { TEXTURES[k] };
        let (walk, speed) = mob.map_or((0.0, 0.0), |m| (m.walk_animation_pos, m.walk_animation_speed));
        let hiding = life::hiding(entity.id);
        let shake = life::shaking(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        if hiding {
            draw.translate([0.0, -SIT, 0.0]);
        }
        let pose = |p: &str| pose(p, walk, if hiding { 0.0 } else { speed }, shake);
        // Hiding, its arms and legs are tucked under its cap.
        let shown = |p: &str| !(hiding && (p.starts_with("arm_") || p.starts_with("leg_")));
        creature::model::draw_shown(draw, model(k), texture, &pose, &shown, creature::model::tint(mob));
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Sporelins;

#[mod_api::sunrise_mod]
impl Hooks for Sporelins {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let _ = r.register_attachment_type("sporelins:stew", false);
        creature::flutter::register(KIND, SIZE);
        creature::flutter::spawns_on(&SPAWNING, spawns_on);
        creature::bestiary::register(&BESTIARY);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// The glowing spots of a befriended one's cap at night.
    fn emissive(&self, _: &dyn ClientHost, event: &mut EmissiveEvent<'_>) {
        for k in 0..2 {
            event.layers.push((LIT[k], GLOWS[k]));
        }
    }

    /// Right-click (SPECS §3): its own (no trading screen, which its template would open), on both sides; a name tag
    /// names it as vanilla's does.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        let host = match host {
            HostSide::Client(client) => {
                let name_tag = event
                    .stack
                    .is_some_and(|s| creature::hand::is(&*client, s, "minecraft:name_tag"));
                return (!name_tag).then_some(InteractionResult::Consume);
            }
            HostSide::Server(host) => host,
        };
        let item = event
            .stack
            .filter(|s| s.count() > 0)
            .map(|s| creature::hand::id(&*host, s));
        if item.as_deref() == Some("minecraft:name_tag") {
            return None;
        }
        if let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity)) {
            interact(host, id, player, event.hand, item);
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

    /// Killed, it drops a mushroom of its colour (SPECS §4).
    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind != KIND {
            return Flow::Continue;
        }
        let id = event.entity;
        if let Some(e) = host.entity(id)
            && let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host))
            && let Some(stack) = creature::hand::stack(&*host, MUSHROOMS[look_of(id)], 1)
        {
            creature::hand::drop_stack(host, level, [at[0], at[1] + 0.3, at[2]], stack);
        }
        creature::glow::dim(id);
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_look_has_its_textures_and_mushroom() {
        for (k, p) in PATTERNS.iter().enumerate() {
            assert_eq!(p.texture, Some(TEXTURES[k]));
            assert!(LIT[k].ends_with("_lit") && GLOWS[k].starts_with(LIT[k]));
            assert!(MUSHROOMS[k].contains(p.id));
        }
    }

    #[test]
    fn it_waddles_only_when_it_walks() {
        assert_eq!(pose("body", 3.0, 0.0, None), [0.0; 3]);
        assert_ne!(pose("leg_left", 1.0, 1.0, None), [0.0; 3]);
        assert_ne!(pose("head", 0.0, 0.0, Some(0.1)), [0.0; 3]);
    }
}
