//! Elephant (SPECS.md): a savanna elephant of the Sunrise Creatures series.
//!
//! An elephant is a copy of the wandering trader (its behaviour is all goals) with its own 4.4 × 5.2 block box, drawn
//! with its own model (`model.rs`, from `sources/models/elephant.bbmodel`; the baby's and the juvenile's beside it,
//! for its young stages, `Stage`). It roams the savannas in small herds, walks with a heavy four-legged gait, its ears,
//! trunk and tail swaying, charges whoever hits it or its riders (`charge.rs`) and flees mice (`flee.rs`); tamed with
//! sugar cane, it follows or stays, breeds and grows up with hay (`tame.rs`); saddled and packed, it carries two riders
//! and two chests a flank (`gear.rs`).
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]

mod charge;
mod flee;
mod gear;
mod model;
mod model_baby;
mod model_juvenile;
mod tame;
mod trumpet;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use creature::state::EntityMap;
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityJoinLevelEvent, EntityMountEvent, EntityRenderState,
    EntityRenderer, EntityTickPostEvent, Flow, GoalSpec, HitHost, Hooks, Host, HostSide, InteractionResult,
    LivingDeathEvent, LivingDropsEvent, LivingIncomingDamageEvent, MobSpawnPlacementCheckEvent,
    RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost, SoundSource, SpawnCheck,
};

pub const ELEPHANT: &str = "elephant:elephant";
const EGG: &str = "elephant:elephant_spawn_egg";
const TEXTURE: &str = "elephant:entity/elephant";
/// Its texture with its blanket in neutral colours, for the dye to tint (SPECS §3).
const SADDLE_TEXTURE: &str = "elephant:entity/elephant_saddle";

/// Its box (blocks: width, height, eye height), SPECS §2.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 4.4,
    height: 5.2,
    eye_height: 4.4,
};
/// Where the crosshair finds it (`creature::pick`): its trunk and tusks reach out before its box, its tail behind.
const OUTLINE: creature::pick::Outline = creature::pick::Outline {
    side: 2.3,
    depth: 3.8,
    height: 5.4,
    ..creature::pick::Outline::DEFAULT
};
/// Its health and pace, as its content file sets them, until its own stats are rolled (SPECS §2).
const HEALTH: f64 = 50.0;
const SPEED: f64 = 0.16;
/// Its own stats (SPECS §2, the library's `creature::stats`): its maximum health, and its walking speed's share.
const STATS: creature::stats::Ranges = creature::stats::Ranges {
    health: (40.0, 60.0),
    pace: (0.8, 1.2),
    walk: Some(SPEED),
    ..creature::stats::Ranges::DEFAULT
};
/// How high it steps without jumping (blocks).
const STEP: f64 = 1.1;
/// Its knockback resistance: a heavy beast barely pushed.
const KNOCKBACK_RESISTANCE: f64 = 0.8;

/// Where it is met (SPECS §2): the savannas.
const BIOMES: &[&str] = &["minecraft:savanna", "minecraft:savanna_plateau"];
/// The ground it comes out on (SPECS §2): no more than `STEEP` blocks up or down within `FLAT` blocks of its spot.
const FLAT: i32 = 3;
const STEEP: i32 = 1;

static BESTIARY: Species = Species {
    id: ELEPHANT,
    biomes: BIOMES,
    wild: |id| creature::friend::of(id).is_none(),
    facts: &[
        Fact {
            id: tame::TAMED_FACT,
            item: "minecraft:sugar_cane",
            key: "bestiary.elephant.tamed",
        },
        Fact {
            id: gear::SADDLED_FACT,
            item: gear::SADDLE,
            key: "bestiary.elephant.saddled",
        },
        Fact {
            id: gear::COLOURED_FACT,
            item: "minecraft:red_dye",
            key: "bestiary.elephant.coloured",
        },
        Fact {
            id: gear::RODE_FACT,
            item: gear::SADDLE,
            key: "bestiary.elephant.rode",
        },
        Fact {
            id: gear::TWO_RIDERS_FACT,
            item: "minecraft:player_head",
            key: "bestiary.elephant.two_riders",
        },
        Fact {
            id: tame::BRED_FACT,
            item: "minecraft:hay_block",
            key: "bestiary.elephant.bred",
        },
        Fact {
            id: tame::RAISED_FACT,
            item: "minecraft:hay_block",
            key: "bestiary.elephant.raised",
        },
        Fact {
            id: gear::PACKED_FACT,
            item: "minecraft:chest",
            key: "bestiary.elephant.packed",
        },
        Fact {
            id: gear::BOTH_SIDES_FACT,
            item: "minecraft:chest",
            key: "bestiary.elephant.both_sides",
        },
    ],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::Uncommon,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.0,
    ..Species::DEFAULT
};

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static TUSK_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "elephant:ivory_tusk",
    dropped_by: &[ELEPHANT],
    given_by: &[],
    uses: &[],
    ..creature::items::ItemPage::DEFAULT
};
static TRUMPET_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "elephant:ceremonial_trumpet",
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "elephant:ceremonial_trumpet",
        key: "bestiary.elephant.item.ceremonial_trumpet.use",
    }],
    ..creature::items::ItemPage::DEFAULT
};
static MEAT_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "elephant:raw_elephant",
    dropped_by: &[ELEPHANT],
    given_by: &[],
    uses: &[],
    ..creature::items::ItemPage::DEFAULT
};

/// Its stage of life (SPECS §4): its model, and its box's size against a grown elephant's (as its body is high).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Baby,
    Juvenile,
    Adult,
}

impl Stage {
    fn of(id: u32) -> Stage {
        let growth = creature::growth::growth(id, &tame::BREEDING);
        if growth < tame::JUVENILE {
            Stage::Baby
        } else if growth < 1.0 {
            Stage::Juvenile
        } else {
            Stage::Adult
        }
    }

    fn model(self) -> &'static creature::model::Part {
        match self {
            Stage::Baby => &model_baby::BABY,
            Stage::Juvenile => &model_juvenile::JUVENILE,
            Stage::Adult => &model::ELEPHANT,
        }
    }

    /// How much larger than its model it is drawn: a grown elephant twice its model's size, the young less.
    fn drawn(self) -> f32 {
        match self {
            Stage::Baby => 1.3,
            Stage::Juvenile => 1.6,
            Stage::Adult => 2.0,
        }
    }

    /// Its box against a grown elephant's: its model's height against the adult model's (its body's), times how much
    /// larger each is drawn.
    fn size(self) -> f64 {
        let model = match self {
            Stage::Baby => 0.56,
            Stage::Juvenile => 0.78,
            Stage::Adult => 1.0,
        };
        model * f64::from(self.drawn() / Stage::Adult.drawn())
    }
}

/// Each elephant's stage as its box was last sized.
static STAGES: EntityMap<u8> = EntityMap::new();

/// From its tick: its box sized to its stage (its `minecraft:scale`: its client box's, times its stage's size), when
/// its stage changed (or it just joined).
fn size_to_stage(host: &mut dyn ServerHost, id: u32) {
    let stage = Stage::of(id);
    if STAGES.locked().insert(id, stage as u8) == Some(stage as u8) {
        return;
    }
    if let Some(living) = host.entity(id).and_then(|e| host.living(e)) {
        let scale = f64::from(creature::mob::client_box_scale(ELEPHANT)) * stage.size();
        living.set_attribute_base(host, "minecraft:scale", scale);
    }
}

/// Its drawing in a jar or the book: standing, without its gear.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    creature::model::draw_shown(
        draw,
        &model::ELEPHANT,
        TEXTURE,
        &|_| [0.0; 3],
        &bare,
        creature::model::WHITE,
    );
}
const BOOK_SCALE: f32 = 0.3;

/// Its body without its gear: its blanket is drawn on its own, tinted, and its chests only once fitted (`Draw`).
fn bare(part: &str) -> bool {
    !matches!(part, "saddle" | "chest_left" | "chest_right")
}

/// What its pose reads of a frame: its walk's progress and speed, its age (ticks).
#[derive(Clone, Copy, Debug, Default)]
struct Frame {
    walk: f32,
    speed: f32,
    age: f32,
}

impl Frame {
    fn of(entity: &EntityRenderState) -> Frame {
        let mob = entity.living.as_ref();
        Frame {
            walk: mob.map_or(0.0, |m| m.walk_animation_pos),
            speed: mob.map_or(0.0, |m| m.walk_animation_speed),
            age: entity.age,
        }
    }
}

/// Its walk (SPECS §2): an elephant's, one foot at a time, hind left, front left, hind right, front right, each a
/// quarter of the stride after the other. A foot stays planted while the body passes over it (`STANCE` of its stride,
/// the leg sweeping back), then swings forward quickly, eased; the body dips at each footfall and shifts its weight to
/// the side of the feet on the ground.
const STRIDE: f32 = 16.0;
const STANCE: f32 = 0.7;
/// How far a leg swings each way at a walk (degrees), reached from this much of its walk's speed.
const LEG_SWING: f32 = 26.0;
const FULL_STRIDE_AT: f32 = 0.35;

/// A leg's swing `u` into its stride (0 to 1): from forward to back while planted, then quickly forward again.
fn leg(u: f32) -> f32 {
    let u = u.rem_euclid(1.0);
    if u < STANCE {
        1.0 - 2.0 * u / STANCE
    } else {
        let t = (u - STANCE) / (1.0 - STANCE);
        -(std::f32::consts::PI * t).cos()
    }
}

/// How it holds itself this frame (SPECS §2): its walk (`leg`), the body dipping and rolling with it; its ears fanning
/// slowly, now and then a wide flap; its trunk swaying and swinging as it walks; its tail flicking; its head nodding
/// with its steps (its tusks and trunk, beside its head in the model, with it: the whole elephant turns rather than its
/// head).
fn pose(name: &str, now: &Frame) -> [f32; 3] {
    let amount = (now.speed / FULL_STRIDE_AT).min(1.0);
    let u = now.walk / STRIDE;
    let swing = |offset: f32| -leg(u + offset) * LEG_SWING * amount;
    let tau = std::f32::consts::TAU;
    // Four footfalls a stride: a dip at each; its weight on its left feet for the first half, its right the second.
    let dip = (u * 4.0 * tau).cos() * 1.5 * amount;
    let roll = (u * tau).sin() * 3.0 * amount;
    // Its ears: a slow fanning, and a wide flap every 10 seconds or so.
    let flap = (now.age * 0.031).sin().max(0.0).powi(8) * 30.0;
    let fan = (now.age * 0.09).sin() * 5.0 + flap;
    let sway = (now.age * 0.05).sin() * 6.0;
    let nod = (u * 2.0 * tau).cos() * 2.5 * amount;
    match name {
        "leg_back_left" => [swing(0.0), 0.0, 0.0],
        "leg_front_left" => [swing(0.25), 0.0, 0.0],
        "leg_back_right" => [swing(0.5), 0.0, 0.0],
        "leg_front_right" => [swing(0.75), 0.0, 0.0],
        "body" => [dip, 0.0, roll],
        "head" | "tusk_left" | "tusk_right" => [nod, 0.0, 0.0],
        "ear_left" => [0.0, fan, 0.0],
        "ear_right" => [0.0, -fan, 0.0],
        "trunk" => [
            nod + (u * tau).cos() * 8.0 * amount + (now.age * 0.04).sin() * 3.0,
            0.0,
            sway,
        ],
        "tail" => [10.0 + amount * 10.0, 0.0, (now.age * 0.12).sin() * 12.0],
        _ => [0.0; 3],
    }
}

/// Its own model, posed (`pose`), without the gear it doesn't wear.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        draw.push();
        creature::model::living_pose(draw, entity);
        // Its scale is for its box only (its client box, `creature::mob::scale_client_box`, and its stage's size): the
        // model keeps its size, each stage's its own.
        let scale = mob.map_or(1.0, |m| m.scale);
        if scale > 0.0 && scale != 1.0 {
            draw.scale([1.0 / scale; 3]);
        }
        let now = Frame::of(entity);
        let pose = |part: &str| pose(part, &now);
        let tint = creature::model::tint(mob);
        let stage = if entity.id == creature::model::IN_A_SCREEN {
            Stage::Adult
        } else {
            Stage::of(entity.id)
        };
        draw.scale([stage.drawn(); 3]);
        creature::model::draw_shown(draw, stage.model(), TEXTURE, &pose, &bare, tint);
        // Its chests, on each flank fitted, and its blanket: a grown elephant's only.
        let gear = if stage == Stage::Adult {
            gear::of(entity.id)
        } else {
            gear::Gear::default()
        };
        if gear.left || gear.right {
            let chests = |part: &str| match part {
                "elephant" => true,
                "chest_left" => gear.left,
                "chest_right" => gear.right,
                _ => false,
            };
            creature::model::draw_shown(draw, &model::ELEPHANT, TEXTURE, &pose, &chests, tint);
        }
        // Its blanket, saddled: its neutral texture tinted by its dye.
        if gear.saddle {
            let colour = creature::model::multiply(tint, gear::blanket(gear));
            let blanket = |part: &str| matches!(part, "elephant" | "saddle");
            creature::model::draw_shown(draw, &model::ELEPHANT, SADDLE_TEXTURE, &pose, &blanket, colour);
        }
        draw.pop();
        Ok(())
    }
}

/// In love: a heart now and then, as vanilla's animals show.
fn hearts(host: &mut dyn ServerHost, id: u32) {
    if !creature::growth::in_love(id) {
        return;
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    if e.tick_count(&*host).is_some_and(|t| t % 10 == 0)
        && let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host))
    {
        creature::particles::puff(host, level, "minecraft:heart", [at[0], at[1] + 2.4, at[2]], 0.6, 1);
    }
}

/// Whether the ground around `pos` is open and about flat (SPECS §2): no water at its feet, and no block of the
/// ground more than `STEEP` up or down within `FLAT` blocks.
fn flat(host: &dyn Host, level: mod_api::LevelRef, pos: [i32; 3]) -> bool {
    let ground = creature::territory::Ground::new(host, level);
    if ground.block(pos).contains("water") || ground.block([pos[0], pos[1] - 1, pos[2]]).contains("water") {
        return false;
    }
    let solid = |p: [i32; 3]| {
        let b = ground.block(p);
        !b.ends_with("air") && !b.contains("water") && !b.contains("grass") && !b.contains("flower")
    };
    let floor = pos[1] - 1;
    // Each column's ground: its highest solid block with room over it, within a step and one of the spot's.
    let height = |x: i32, z: i32| {
        (floor - STEEP - 1..=floor + STEEP + 1)
            .rev()
            .find(|&y| solid([x, y, z]) && !solid([x, y + 1, z]))
    };
    (-FLAT..=FLAT)
        .all(|dx| (-FLAT..=FLAT).all(|dz| height(pos[0] + dx, pos[2] + dz).is_some_and(|h| (h - floor).abs() <= STEEP)))
}

#[derive(Default)]
struct Elephant;

#[mod_api::sunrise_mod]
impl Hooks for Elephant {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(ELEPHANT, SIZE);
        creature::mob::scale_client_box(ELEPHANT);
        creature::pick::outline(ELEPHANT, OUTLINE);
        creature::anger::register(ELEPHANT);
        gear::declare(r);
        // Its attribute row (a type only its content file declares has none to add to), with the values its content
        // file sets, then what its template lacks.
        let _ = r.declare_entity_attributes(
            "elephant",
            "minecraft:wandering_trader",
            vec![
                ("minecraft:max_health".into(), HEALTH),
                ("minecraft:movement_speed".into(), SPEED),
            ],
        );
        let _ = r.modify_entity_attributes(
            ELEPHANT,
            vec![("minecraft:knockback_resistance".into(), Some(KNOCKBACK_RESISTANCE))],
        );
        let trumpet = r.item_key(trumpet::TRUMPET);
        r.item_part(trumpet, trumpet::Trumpet);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&TUSK_PAGE);
        creature::bestiary::register_item(&TRUMPET_PAGE);
        creature::bestiary::register_item(&MEAT_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(ELEPHANT.into(), Arc::new(Draw));
    }

    /// An elephant joining: its client box as tall as it is, and vanilla's goals for its wandering and looking about.
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if event.kind != ELEPHANT {
            return Flow::Continue;
        }
        let Some(e) = host.entity(event.entity) else {
            return Flow::Continue;
        };
        if let Some(living) = host.living(e) {
            let scale = creature::mob::client_box_scale(ELEPHANT);
            living.set_attribute_base(host, "minecraft:scale", f64::from(scale));
            // A block in its stride, ridden or not.
            living.set_attribute_base(host, "minecraft:step_height", STEP);
        }
        let Some(m) = host.mob(e) else {
            return Flow::Continue;
        };
        m.add_goal(
            host,
            5,
            &GoalSpec::Stroll {
                speed: 0.8,
                interval: 120,
                water_avoiding: Some(0.001),
                swim: false,
                flying: false,
            },
        );
        m.add_goal(
            host,
            6,
            &GoalSpec::LookAtPlayer {
                range: 10.0,
                probability: 0.02,
                only_horizontal: false,
                mobs: false,
                only: None,
            },
        );
        m.add_goal(host, 7, &GoalSpec::RandomLookAround);
        Flow::Continue
    }

    /// Right-click (never its template's trading screen), on both sides: on the server, its owner's (`tame`).
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != ELEPHANT {
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
        if let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity))
            && !gear::interact(host, id, player, event.hand, item.as_deref())
        {
            tame::interact(host, id, player, event.hand, item.as_deref());
        }
        Some(InteractionResult::Consume)
    }

    /// It comes out only on open, about flat ground (SPECS §2).
    fn mob_spawn_placement_check(&self, host: &dyn Host, event: &mut MobSpawnPlacementCheckEvent<'_>) {
        if event.kind == ELEPHANT && !flat(host, event.level, [event.pos.x, event.pos.y, event.pos.z]) {
            *event.result = SpawnCheck::Fail;
        }
    }

    /// Who hit a player, for an elephant they ride to charge (`charge::riders_hit`; this hook can't look up the
    /// player's mount).
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if event.kind == "minecraft:player"
            && let (Ok(victim), Ok(by)) = (u32::try_from(event.entity), u32::try_from(event.source.causing))
        {
            charge::rider_hit(victim, by);
        }
        Flow::Continue
    }

    /// A rider getting off leaves their keys behind.
    fn entity_mount(&self, host: &mut dyn ServerHost, event: &mut EntityMountEvent<'_>) -> Flow {
        if event.vehicle_kind == ELEPHANT
            && !event.mounting
            && let Some(player) = creature::mob::player_of(&*host, event.entity)
        {
            creature::ride::left(player);
        }
        Flow::Continue
    }

    /// A young one drops nothing (SPECS §5): no meat, no tusk.
    fn living_drops(&self, _: &mut dyn ServerHost, event: &mut LivingDropsEvent<'_>) -> Flow {
        if event.kind == ELEPHANT && creature::growth::is_young(event.entity) {
            event.drops.clear();
        }
        Flow::Continue
    }

    /// Its saddle drops.
    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == ELEPHANT {
            gear::died(host, event.entity);
        }
        Flow::Continue
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != ELEPHANT {
                return;
            }
            let id = event.entity;
            creature::friend::tick(host, id);
            gear::tick(host, id);
            creature::ride::read(&*host, id);
            creature::stats::tick(host, id, &STATS, |_| None);
            if creature::growth::tick(host, id, |_| None) == creature::growth::Grew::Up
                && let Some(owner) = creature::friend::of(id)
            {
                creature::bestiary::learn(host, creature::friend::uuid(owner.uuid), ELEPHANT, tame::RAISED_FACT);
            }
            for (_, owner) in creature::growth::births(host, event.dimension, &tame::BREEDING) {
                if let Some(owner) = owner {
                    creature::bestiary::learn(host, owner, ELEPHANT, tame::BRED_FACT);
                }
            }
            size_to_stage(host, id);
            hearts(host, id);
            tame::look_for_food(&*host, id);
            flee::look(&*host, id);
            creature::body::tick(host, id, || {
                vec![
                    Box::new(charge::Charge::default()),
                    Box::new(flee::Flee),
                    Box::new(gear::Ride),
                    Box::new(tame::Stay),
                    Box::new(tame::Eat),
                    Box::new(creature::growth::Mate::new(&tame::BREEDING)),
                    Box::new(creature::friend::Follow::new(&tame::FOLLOWING)),
                ]
            });
            charge::strike(host, id);
            charge::riders_hit(&*host, id);
            tame::eat(host, id);
            creature::sounds::play_queued(host, id, SoundSource::Neutral);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_stands_still_at_rest_and_puts_one_foot_down_at_a_time() {
        assert_eq!(pose("leg_front_left", &Frame::default()), [0.0; 3]);
        let walking = |walk: f32| Frame {
            walk,
            speed: 1.0,
            ..Frame::default()
        };
        // A planted leg sweeps back steadily; then it swings forward again, quickly.
        let a = pose("leg_back_left", &walking(0.0))[0];
        let b = pose("leg_back_left", &walking(STRIDE * 0.35))[0];
        assert!(b > a, "planted, the leg sweeps back");
        assert!(
            (leg(STANCE) + 1.0).abs() < 1e-5 && (leg(1.0) - 1.0).abs() < 1e-5,
            "swung back to the front"
        );
        // Each leg a quarter of the stride after the one before: never two legs at the same point.
        let legs = ["leg_back_left", "leg_front_left", "leg_back_right", "leg_front_right"];
        let at: Vec<f32> = legs.iter().map(|l| pose(l, &walking(3.0))[0]).collect();
        for i in 0..4 {
            for j in i + 1..4 {
                assert!((at[i] - at[j]).abs() > 1e-3, "{} and {} together", legs[i], legs[j]);
            }
        }
    }
}
