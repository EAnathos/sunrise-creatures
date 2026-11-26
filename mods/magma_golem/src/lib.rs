//! Magma Golem (SPECS.md): a giant of living magma asleep in the Nether, a rare mini-boss of the Sunrise Creatures
//! series.
//!
//! A magma golem is a copy of the wandering trader (its behaviour is all goals) with its own 3 × 6 block box, drawn with
//! its own model (`model.rs`, from `sources/models/magma_golem.bbmodel`), immune to fire and lava. In development
//! (SPECS §0): it walks, looks about and is drawn with its heavy, ape-like gait (`pose`); its sleep, its fight and its
//! boss bar, its snow weakness, its spawning and its items come next.
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]

mod model;

use std::sync::Arc;

use creature::bestiary::Species;
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityJoinLevelEvent, EntityRenderState, EntityRenderer,
    EntityTickPostEvent, Flow, GoalSpec, Hooks, HostSide, InteractionResult, RegisterEntityRenderersEvent, Registrar,
    RegistrationContext, ServerHost, SoundSource,
};

pub const GOLEM: &str = "magma_golem:magma_golem";
const EGG: &str = "magma_golem:magma_golem_spawn_egg";
const TEXTURE: &str = "magma_golem:entity/magma_golem/magma_golem";

/// Its box (blocks: width, height, eye height), SPECS §1: 3 × 6, its eyes low in front, under its shoulders.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 3.0,
    height: 6.0,
    eye_height: 4.2,
};
/// Its health and pace (SPECS §2), as its content file sets them, and what its template (a trader) lacks: armour, no
/// knockback, its fist's blow.
const HEALTH: f64 = 250.0;
const SPEED: f64 = 0.2;
const ARMOR: f64 = 10.0;
const KNOCKBACK_RESISTANCE: f64 = 1.0;
const ATTACK_DAMAGE: f64 = 14.0;
/// How far it sees a player (blocks; its follow range).
const SIGHT: f64 = 24.0;

/// Where it is met (SPECS §2): the Nether's basalt deltas and wastes, by their lava seas.
const BIOMES: &[&str] = &["minecraft:basalt_deltas", "minecraft:nether_wastes"];

static BESTIARY: Species = Species {
    id: GOLEM,
    biomes: BIOMES,
    wild: |_| true,
    facts: &[],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::VeryRare,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.0,
    ..Species::DEFAULT
};

/// Its drawing in a jar or the book: standing, its whole height in the view.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    creature::model::draw(draw, &model::MAGMA_GOLEM, TEXTURE, &|_| [0.0; 3]);
}
const BOOK_SCALE: f32 = 0.15;

/// How it holds itself this frame (SPECS §2.1), from its render state: heavy steps, one leg then the other, the body
/// rolling on each footfall and the arms swinging opposite the legs, its fists brushing the ground; at rest a slow
/// breath lifting its back and shoulders; its head turned toward what it looks at.
fn pose(name: &str, now: &Frame) -> [f32; 3] {
    let Frame {
        walk,
        speed,
        head_yaw,
        pitch,
        age,
    } = *now;
    let speed = speed.min(1.0);
    // A slow stride: the legs swing 22° each way at a full walk, the arms a little less.
    let phase = walk * 0.45;
    let stride = phase.sin() * 22.0 * speed;
    let knee = (phase.cos().max(0.0)) * 18.0 * speed;
    let roll = phase.sin() * 4.0 * speed;
    let dip = (phase * 2.0).cos().abs() * 2.0 * speed;
    // A breath every 4 seconds or so, faded out as it walks.
    let breath = (age * 0.08).sin() * 2.5 * (1.0 - speed);
    match name {
        "head" => [pitch * 0.6, -head_yaw * 0.8, 0.0],
        "body" => [dip + breath * 0.4, 0.0, roll],
        "back" => [-breath, 0.0, 0.0],
        "shoulder_left" | "shoulder_right" => [-breath * 0.6, 0.0, 0.0],
        "leg_left" => [stride, 0.0, 0.0],
        "leg_right" => [-stride, 0.0, 0.0],
        "shin_left" => [knee, 0.0, 0.0],
        "shin_right" => [(-phase).cos().max(0.0) * 18.0 * speed, 0.0, 0.0],
        "arm_left" => [-stride * 0.7, 0.0, 0.0],
        "arm_right" => [stride * 0.7, 0.0, 0.0],
        _ => [0.0; 3],
    }
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

/// Its own model, posed (`pose`).
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
        creature::model::draw_shown(
            draw,
            &model::MAGMA_GOLEM,
            TEXTURE,
            &pose,
            &|_| true,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct MagmaGolem;

#[mod_api::sunrise_mod]
impl Hooks for MagmaGolem {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(GOLEM, SIZE);
        creature::mob::scale_client_box(GOLEM);
        // Its attribute row (a type only its content file declares has none to add to), with the values its content
        // file sets, then what its template lacks.
        let _ = r.declare_entity_attributes(
            "magma_golem",
            "minecraft:wandering_trader",
            vec![
                ("minecraft:max_health".into(), HEALTH),
                ("minecraft:movement_speed".into(), SPEED),
            ],
        );
        let _ = r.modify_entity_attributes(
            GOLEM,
            vec![
                ("minecraft:armor".into(), Some(ARMOR)),
                ("minecraft:knockback_resistance".into(), Some(KNOCKBACK_RESISTANCE)),
                ("minecraft:attack_damage".into(), Some(ATTACK_DAMAGE)),
                ("minecraft:follow_range".into(), Some(SIGHT)),
            ],
        );
        creature::bestiary::register(&BESTIARY);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(GOLEM.into(), Arc::new(Draw));
    }

    /// A magma golem joining: its client box as tall as it is, immune to fire and lava (a grant of the live body, not
    /// saved: given at each join), and vanilla's goals for its wandering and looking about.
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if event.kind != GOLEM {
            return Flow::Continue;
        }
        let Some(e) = host.entity(event.entity) else {
            return Flow::Continue;
        };
        if let Some(living) = host.living(e) {
            let scale = creature::mob::client_box_scale(GOLEM);
            living.set_attribute_base(host, "minecraft:scale", f64::from(scale));
        }
        e.set_fire_immunity(host, true);
        let Some(m) = host.mob(e) else {
            return Flow::Continue;
        };
        m.add_goal(
            host,
            5,
            &GoalSpec::Stroll {
                speed: 0.7,
                interval: 160,
                water_avoiding: Some(0.001),
                swim: false,
                flying: false,
            },
        );
        m.add_goal(
            host,
            6,
            &GoalSpec::LookAtPlayer {
                range: 16.0,
                probability: 0.04,
                only_horizontal: false,
                mobs: false,
                only: None,
            },
        );
        m.add_goal(host, 7, &GoalSpec::RandomLookAround);
        Flow::Continue
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == GOLEM).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == GOLEM {
                creature::sounds::play_queued(host, event.entity, SoundSource::Hostile);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_stands_still_at_rest_and_strides_when_it_walks() {
        assert_eq!(pose("leg_left", &Frame::default()), [0.0; 3]);
        let walking = Frame {
            walk: 3.0,
            speed: 1.0,
            ..Frame::default()
        };
        let left = pose("leg_left", &walking)[0];
        let right = pose("leg_right", &walking)[0];
        assert!(left != 0.0 && (left + right).abs() < 1e-4, "the legs swing opposite");
    }
}
