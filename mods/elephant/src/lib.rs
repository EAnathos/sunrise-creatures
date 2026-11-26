//! Elephant (SPECS.md): a savanna elephant of the Sunrise Creatures series.
//!
//! An elephant is a copy of the wandering trader (its behaviour is all goals) with its own 2.2 × 2.6 block box, drawn
//! with its own model (`model.rs`, from `sources/models/elephant.bbmodel`; the baby's and the juvenile's beside it,
//! for its young stages). In development (SPECS §0): it roams the savannas in small herds, walks with a heavy
//! four-legged gait, its ears, trunk and tail swaying, and charges whoever hits it (`charge.rs`).
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]

mod charge;
mod model;
mod tame;
// The young stages' models: drawn once elephants grow (SPECS §4).
#[allow(dead_code)]
mod model_baby;
#[allow(dead_code)]
mod model_juvenile;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityJoinLevelEvent, EntityRenderState, EntityRenderer,
    EntityTickPostEvent, Flow, GoalSpec, Hooks, Host, HostSide, InteractionResult, MobSpawnPlacementCheckEvent,
    RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost, SoundSource, SpawnCheck,
};

pub const ELEPHANT: &str = "elephant:elephant";
const EGG: &str = "elephant:elephant_spawn_egg";
const TEXTURE: &str = "elephant:entity/elephant";

/// Its box (blocks: width, height, eye height), SPECS §2.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 2.2,
    height: 2.6,
    eye_height: 2.2,
};
/// Where the crosshair finds it (`creature::pick`): its trunk and tusks reach out before its box, its tail behind.
const OUTLINE: creature::pick::Outline = creature::pick::Outline {
    side: 1.15,
    depth: 1.9,
    height: 2.7,
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
/// Its knockback resistance: a heavy beast barely pushed.
const KNOCKBACK_RESISTANCE: f64 = 0.8;

/// Where it is met (SPECS §2): the savannas.
const BIOMES: &[&str] = &["minecraft:savanna", "minecraft:savanna_plateau"];
/// The ground it comes out on (SPECS §2): no more than `STEEP` blocks up or down within `FLAT` blocks of its spot.
const FLAT: i32 = 2;
const STEEP: i32 = 1;

static BESTIARY: Species = Species {
    id: ELEPHANT,
    biomes: BIOMES,
    wild: |id| creature::friend::of(id).is_none(),
    facts: &[Fact {
        id: tame::TAMED_FACT,
        item: "minecraft:sugar_cane",
        key: "bestiary.elephant.tamed",
    }],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::Uncommon,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.0,
    ..Species::DEFAULT
};

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

/// Its gear, drawn only once fitted (SPECS §3): none yet.
fn bare(part: &str) -> bool {
    !matches!(part, "saddle" | "chest_left" | "chest_right")
}

/// What its pose reads of a frame: its walk's progress and speed, its head's turn and pitch (degrees), its age (ticks).
#[derive(Clone, Copy, Debug, Default)]
struct Frame {
    walk: f32,
    speed: f32,
    head_yaw: f32,
    pitch: f32,
    age: f32,
}

impl Frame {
    fn of(entity: &EntityRenderState) -> Frame {
        let mob = entity.living.as_ref();
        Frame {
            walk: mob.map_or(0.0, |m| m.walk_animation_pos),
            speed: mob.map_or(0.0, |m| m.walk_animation_speed),
            head_yaw: mob.map_or(0.0, |m| m.y_rot),
            pitch: mob.map_or(0.0, |m| m.x_rot),
            age: entity.age,
        }
    }
}

/// How it holds itself this frame (SPECS §2): a slow four-legged walk, each front leg with the hind leg across from
/// it, the body rocking; its ears fanning slowly, now and then a wide flap; its trunk swaying and swinging as it walks;
/// its tail flicking; its head turned toward what it looks at.
fn pose(name: &str, now: &Frame) -> [f32; 3] {
    let speed = now.speed.min(1.0);
    let phase = now.walk * 0.5;
    let swing = phase.sin() * 24.0 * speed;
    // Its ears: a slow fanning, and a wide flap every 10 seconds or so.
    let flap = (now.age * 0.031).sin().max(0.0).powi(8) * 30.0;
    let fan = (now.age * 0.09).sin() * 5.0 + flap;
    let sway = (now.age * 0.05).sin() * 6.0;
    match name {
        "leg_front_left" | "leg_back_right" => [swing, 0.0, 0.0],
        "leg_front_right" | "leg_back_left" => [-swing, 0.0, 0.0],
        "body" => [0.0, 0.0, phase.sin() * 2.0 * speed],
        "head" => [
            now.pitch * 0.5 + (phase * 2.0).cos() * 2.0 * speed,
            -now.head_yaw * 0.6,
            0.0,
        ],
        "ear_left" => [0.0, fan, 0.0],
        "ear_right" => [0.0, -fan, 0.0],
        "trunk" => [phase.cos() * 8.0 * speed + (now.age * 0.04).sin() * 3.0, 0.0, sway],
        "tail" => [10.0 + speed * 10.0, 0.0, (now.age * 0.12).sin() * 12.0],
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
        // Its scale is for its client box only (`creature::mob::scale_client_box`): the model keeps its size.
        let scale = mob.map_or(1.0, |m| m.scale);
        if scale > 0.0 && scale != 1.0 {
            draw.scale([1.0 / scale; 3]);
        }
        let now = Frame::of(entity);
        let pose = |part: &str| pose(part, &now);
        let tint = creature::model::tint(mob);
        creature::model::draw_shown(draw, &model::ELEPHANT, TEXTURE, &pose, &bare, tint);
        draw.pop();
        Ok(())
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
        creature::bestiary::register(&BESTIARY);
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
        if let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity)) {
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

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != ELEPHANT {
                return;
            }
            let id = event.entity;
            creature::friend::tick(host, id);
            creature::stats::tick(host, id, &STATS, |_| None);
            tame::look_for_food(&*host, id);
            creature::body::tick(host, id, || {
                vec![
                    Box::new(charge::Charge::default()),
                    Box::new(tame::Stay),
                    Box::new(tame::Eat),
                    Box::new(creature::friend::Follow::new(&tame::FOLLOWING)),
                ]
            });
            charge::strike(host, id);
            tame::eat(host, id);
            creature::sounds::play_queued(host, id, SoundSource::Neutral);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_stands_still_at_rest_and_its_legs_go_in_diagonal_pairs() {
        assert_eq!(pose("leg_front_left", &Frame::default()), [0.0; 3]);
        let walking = Frame {
            walk: 3.0,
            speed: 1.0,
            ..Frame::default()
        };
        let front_left = pose("leg_front_left", &walking)[0];
        assert!(front_left != 0.0);
        assert_eq!(
            pose("leg_back_right", &walking)[0],
            front_left,
            "diagonal legs together"
        );
        assert_eq!(
            pose("leg_front_right", &walking)[0],
            -front_left,
            "the other pair opposite"
        );
    }
}
