//! Toucan (SPECS.md): a tropical bird of the jungle canopy, of the Sunrise Creatures series.
//!
//! A toucan is a copy of the wandering trader drawn with its own model (`model.rs`, from
//! `sources/models/toucan.bbmodel`). It comes out on the leaves of the jungles (the library's spawning of small
//! creatures, `creature::flutter`), hops about the branches, flits between the trees with the library's flight
//! (`creature::flight`, its `WINGS`) and flees players and monsters. A melon slice thrown near a wild one may tame it
//! (the library's `friend`): it follows its friend, or stays. Its behaviours are in `canopy.rs`; what they can't do
//! themselves, the tick does (`act`).
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]
mod canopy;
mod model;

use std::sync::Arc;
use std::time::Instant;

use canopy::{Action, Around};
use creature::bestiary::{Fact, Species};
use creature::flight::Wings;
use creature::state::EntityMap;
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Hooks,
    HostSide, InteractionResult, MobCategory, RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost,
};

pub const KIND: &str = "toucan:toucan";
const EGG: &str = "toucan:toucan_spawn_egg";
const RAW: &str = "toucan:raw_toucan";
const COOKED: &str = "toucan:cooked_toucan";
const TEXTURE: &str = "toucan:entity/toucan/toucan";

/// Its box (blocks, SPECS §2); its model is drawn at its size (16 texels a block), its feet this far above its
/// origin (blocks).
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.8,
    height: 1.2,
    eye_height: 1.05,
};
const FEET: f32 = 1.9 / 16.0;

/// Its model faces -z where the series' face +z (`creature::model::living_pose`): it is drawn turned half round, which
/// also turns its pitch and roll the other way (`pose`).
fn face_forward(draw: &mut dyn BlockEntityDrawContext) {
    draw.rotate_about([0.0, 1.0, 0.0], 180.0, [0.0, 0.0, 0.0]);
    draw.translate([0.0, -FEET, 0.0]);
}

/// Its sounds (`sources/sounds.toml`, cut from CC0 freesound recordings of toucans and the vulture's wings): its
/// croaking call, its wing beats, its take-off; hurt and death are its entity type's.
pub(crate) const CALL: &str = "toucan:entity.toucan.call";
const FLAP: &str = "toucan:entity.toucan.flap";
const TAKEOFF: &str = "toucan:entity.toucan.takeoff";

/// Its flight (the library's `creature::flight`): short and slow, between the trees.
pub(crate) static WINGS: Wings = Wings {
    cruise: 0.3,
    dive: 0.6,
    flap: FLAP,
    takeoff: TAKEOFF,
    pace: |_| 1.0,
    ..Wings::DEFAULT
};

/// Where it lives (SPECS §2).
const BIOMES: &[&str] = &["minecraft:jungle", "minecraft:sparse_jungle", "minecraft:bamboo_jungle"];

/// When it comes out (the library's spawning takes a small flier's hours; it flies its own way): by day.
static HOURS: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Day,
    ..creature::flutter::Kind::DEFAULT
};

/// On top of the leaves in its biomes (SPECS §2), rare: alone or in pairs, up to 2 within 48 blocks of a player.
static SPAWNING: creature::flutter::Spawning = creature::flutter::Spawning {
    kind: KIND,
    flight: &HOURS,
    biomes: BIOMES,
    group: (1, 2),
    cap: 2,
    chance: 6,
    ..creature::flutter::Spawning::DEFAULT
};

/// Where it comes out (`creature::flutter::spawns_on`): on leaves with room for it over them (two open blocks: it is
/// 1.2 blocks tall), not wedged under a higher layer of the canopy.
fn on_leaves(ground: creature::territory::Ground<'_>, p: [i32; 3]) -> bool {
    let open = |y: i32| {
        matches!(
            ground.block([p[0], y, p[2]]).as_str(),
            "minecraft:air" | "minecraft:cave_air"
        )
    };
    ground.block(p).ends_with("leaves") && open(p[1] + 1) && open(p[1] + 2)
}

const FEED_FACT: &str = "feed";

/// Its page (SPECS §5): the biomes met in, the melon slice that tames it.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |id| creature::friend::of(id).is_none(),
    facts: &[Fact {
        id: FEED_FACT,
        item: canopy::FOOD,
        key: "bestiary.toucan.feed",
    }],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::Rare,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.0,
    ..Species::DEFAULT
};
const BOOK_SCALE: f32 = 0.4;

static RAW_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: RAW,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:chicken",
        key: "bestiary.toucan.item.raw_toucan.use",
    }],
    ..creature::items::ItemPage::DEFAULT
};
static COOKED_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: COOKED,
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:cooked_chicken",
        key: "bestiary.toucan.item.cooked_toucan.use",
    }],
    ..creature::items::ItemPage::DEFAULT
};

/// One melon slice in this many tames it (SPECS §3).
const FRIEND_CHANCE: u64 = 4;
/// How far it looks for a melon slice and a monster (blocks), and how often (ticks).
const FOOD_RANGE: f64 = 10.0;
const MONSTER_RANGE: f64 = 8.0;
const LOOK_EVERY: i32 = 10;

/// Whether each toucan is in the air, from its server tick, for the drawing (single player).
static FLYING: EntityMap<bool> = EntityMap::new();
/// When each toucan was last told to stay or follow, and which (single player: the renderer reads it; a clock, as the
/// client counts a creature's age from when it came into view).
static TOLD: EntityMap<(Instant, bool)> = EntityMap::new();
/// Its sit while it stays (seconds): settling back onto its rump, getting up again when told to follow.
const SIT_DOWN: f32 = 0.35;
const SIT_UP: f32 = 0.45;
/// Sat: how far back its body tips (degrees, its breast up), how far its legs fold forward from the body, how far
/// it comes down (blocks: its rump on its perch, its feet before it).
const SIT_TIP: f32 = 30.0;
const SIT_LEGS: f32 = 40.0;
const SIT_DROP: f32 = 4.2 / 16.0;

/// How sat toucan `id` is: 1 while it stays, 0 standing, between as it sits down or gets up. One staying since it
/// was loaded sits.
fn sitting(id: u32) -> f32 {
    match TOLD.locked().get(&id).copied() {
        Some((at, staying)) => sit_at(at.elapsed().as_secs_f32(), staying),
        None => creature::friend::of(id).map_or(0.0, |f| if f.staying { 1.0 } else { 0.0 }),
    }
}

/// How sat it is `t` seconds after it was told to stay (`staying`) or to follow: eased down, or eased back up.
fn sit_at(t: f32, staying: bool) -> f32 {
    let ease = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    if staying {
        ease(t / SIT_DOWN)
    } else {
        1.0 - ease(t / SIT_UP)
    }
}

/// From `entity_tick_post`, for each toucan: its friend's save, its behaviours (the library's tick of small
/// creatures: despawning far from players unless befriended), what they queued, what it finds around it.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    creature::friend::tick(host, id);
    creature::flutter::tick_post(host, dimension, KIND, id, canopy::behaviours);
    for action in canopy::actions() {
        act(host, action);
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    let flying = !e.on_ground(&*host).unwrap_or(true) && !e.is_in_water(&*host).unwrap_or(false);
    FLYING.locked().insert(id, flying);
    if (e.tick_count(&*host).unwrap_or(0) + id as i32) % LOOK_EVERY == 0 {
        look_around(host, id);
    }
}

/// What toucan `id` finds around it (`canopy::Around`): the nearest melon slice on the ground and monster.
fn look_around(host: &mut dyn ServerHost, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let food = creature::food::nearest(&*host, level, at, FOOD_RANGE, |item| item == canopy::FOOD);
    let r = MONSTER_RANGE;
    let monster = level
        .entities_in(
            &*host,
            [at[0] - r, at[1] - r, at[2] - r],
            [at[0] + r, at[1] + r, at[2] + r],
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|m| m.classification(&*host) == Some(MobCategory::Monster))
        .filter_map(|m| m.position(&*host))
        .filter(|p| creature::body::distance(*p, at) <= r)
        .min_by(|a, b| creature::body::distance(*a, at).total_cmp(&creature::body::distance(*b, at)));
    canopy::set_around(id, Around { food, monster });
}

/// Where toucan `id` is (a little over its feet) and its level, for a sign around it.
fn place(host: &dyn ServerHost, id: u32) -> Option<(mod_api::LevelRef, [f64; 3])> {
    let e = host.entity(id)?;
    let at = e.position(host)?;
    Some((e.level(host)?, [at[0], at[1] + 0.9, at[2]]))
}

/// What a behaviour queued (`canopy::Action`).
fn act(host: &mut dyn ServerHost, action: Action) {
    match action {
        Action::Eat { toucan, item, thrower } => {
            if !creature::food::take_one(host, item) {
                return;
            }
            creature::sounds::queue(toucan, "minecraft:entity.parrot.eat", 1.0, 0.8);
            // A slice thrown by a player, to a wild one: one in `FRIEND_CHANCE` tames it for them (SPECS §3).
            let Some(player) = thrower.and_then(|t| creature::mob::player_of(&*host, t)) else {
                return;
            };
            if creature::friend::of(toucan).is_some() {
                return;
            }
            let befriended = creature::colours::Random::new(toucan ^ item)
                .next()
                .is_multiple_of(FRIEND_CHANCE);
            if befriended {
                creature::friend::befriend(host, toucan, player);
                creature::bestiary::learn(host, player, KIND, FEED_FACT);
            }
            if let Some((level, at)) = place(&*host, toucan) {
                creature::friend::sign(host, level, at, befriended);
            }
        }
    }
}

/// What a frame of a toucan shows, for `pose`.
#[derive(Clone, Copy, Debug, Default)]
struct Frame {
    age: f32,
    pitch: f32,
    head_yaw: f32,
    flying: bool,
    flapping: bool,
    /// Its sit (`sitting`): 0 standing, 1 sat.
    sit: f32,
}

/// How it holds itself this frame (SPECS §2): perched, its wings folded, its head (with its bill) turned to what it
/// looks at, its tail bobbing; in the air, its body takes the flight's pitch and banks with the turn, its wings beat
/// (or hold out in a glide), its legs tucked back. Its model is drawn turned half round (`face_forward`): a pitch or a
/// roll is given the other way.
fn pose(name: &str, frame: &Frame) -> [f32; 3] {
    let bob = (frame.age * 0.15).sin() * 4.0;
    if !frame.flying {
        // Sitting (`sit`), it tips back onto its rump, its breast up and its feet before it, its head and tail
        // turned back about level, its wing tips lifted off its perch.
        let sit = frame.sit;
        return match name {
            "toucan" => [SIT_TIP * sit, 0.0, 0.0],
            "head" => [
                -frame.pitch.clamp(-30.0, 30.0) - SIT_TIP * 0.7 * sit,
                -frame.head_yaw.clamp(-75.0, 75.0),
                0.0,
            ],
            "tail" => [bob * (1.0 - sit) - SIT_TIP * sit, 0.0, 0.0],
            "legs" => [SIT_LEGS * sit, 0.0, 0.0],
            "left_wing" | "right_wing" => [-12.0 * sit, 0.0, 0.0],
            _ => [0.0; 3],
        };
    }
    let beat = if frame.flapping {
        (frame.age * 1.2).sin() * 55.0
    } else {
        20.0
    };
    let bank = (frame.head_yaw * 0.6).clamp(-35.0, 35.0);
    match name {
        "toucan" => [-frame.pitch, 0.0, -bank],
        "left_wing" => [0.0, 0.0, -beat],
        "right_wing" => [0.0, 0.0, beat],
        "legs" => [-60.0, 0.0, 0.0],
        "tail" => [10.0 + bob * 0.5, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Its drawing in the book: perched.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    draw.push();
    face_forward(draw);
    creature::model::draw(draw, &model::TOUCAN, TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}

/// Its own model, posed by `pose` every frame.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let frame = Frame {
            age: entity.age,
            pitch: mob.map_or(0.0, |m| m.x_rot),
            head_yaw: mob.map_or(0.0, |m| m.y_rot),
            flying: FLYING.locked().get(&entity.id).copied().unwrap_or(false),
            flapping: creature::flight::flapping(entity.id),
            sit: sitting(entity.id),
        };
        draw.push();
        creature::model::living_pose(draw, entity);
        face_forward(draw);
        if frame.sit > 0.0 && !frame.flying {
            draw.translate([0.0, -SIT_DROP * frame.sit, 0.0]);
        }
        let pose = |part: &str| pose(part, &frame);
        let all = |_: &str| true;
        creature::model::draw_shown(draw, &model::TOUCAN, TEXTURE, &pose, &all, creature::model::tint(mob));
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Toucan;

#[mod_api::sunrise_mod]
impl Hooks for Toucan {
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {
        creature::flutter::register(KIND, SIZE);
        creature::flutter::spawns_on(&SPAWNING, on_leaves);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&RAW_PAGE);
        creature::bestiary::register_item(&COOKED_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click by its friend: it stays, or follows again. A name tag names it as vanilla's would. Never a trading
    /// screen, which its template would open, on both sides.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        let host = match host {
            HostSide::Server(host) => host,
            HostSide::Client(client) => {
                let name_tag = event
                    .stack
                    .is_some_and(|s| creature::hand::is(&*client, s, "minecraft:name_tag"));
                return (!name_tag).then_some(InteractionResult::Consume);
            }
        };
        if event
            .stack
            .is_some_and(|s| creature::hand::is(&*host, s, "minecraft:name_tag"))
        {
            return None;
        }
        if let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity))
            && let Some(f) = creature::friend::of(id).filter(|f| f.is(player.uuid()))
        {
            creature::friend::set_staying(id, !f.staying);
            TOLD.locked().insert(id, (Instant::now(), !f.staying));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_sits_while_it_stays_and_gets_up_to_follow() {
        assert_eq!(sit_at(0.0, true), 0.0);
        assert_eq!(sit_at(SIT_DOWN, true), 1.0);
        assert_eq!(sit_at(60.0, true), 1.0, "sat as long as it stays");
        assert_eq!(sit_at(0.0, false), 1.0);
        assert_eq!(sit_at(SIT_UP, false), 0.0);
        let sat = Frame {
            sit: 1.0,
            ..Frame::default()
        };
        assert_eq!(pose("toucan", &sat), [SIT_TIP, 0.0, 0.0]);
        assert_eq!(pose("toucan", &Frame::default()), [0.0; 3]);
    }

    #[test]
    fn its_wings_stay_folded_on_a_perch_and_beat_in_the_air() {
        let perched = Frame::default();
        assert_eq!(pose("left_wing", &perched), [0.0; 3]);
        let flying = Frame {
            age: 1.0,
            flying: true,
            flapping: true,
            ..Frame::default()
        };
        let left = pose("left_wing", &flying)[2];
        assert!(left != 0.0);
        assert_eq!(pose("right_wing", &flying)[2], -left, "the wings beat together");
    }
}
