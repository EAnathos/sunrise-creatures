//! Its drawing (SPECS §2.1): its model posed from its render state and what its tick shows of it (`golem::shown`,
//! single player), then its seams drawn full-bright over it (`GLOW`, the texture's glowing pixels alone): pulsing
//! slowly asleep, its eyes dark; blazing enraged; gone dark crusted.
use mod_api::{BlockEntityDrawContext, EntityRenderState, EntityRenderer};

use crate::golem::{self, Attack, Mode, Shown};
use crate::{TEXTURE, model};

/// Its texture's glowing pixels (the seams, the lava, the eyes, the maw), drawn full-bright over it.
const GLOW: &str = "magma_golem:entity/magma_golem/magma_golem_glow";
/// Full light, as `BlockEntityDrawContext::set_light` takes it: sky and block light 15.
const FULL_BRIGHT: u32 = 0xF0F0;
/// The glow's colour (a multiplier): dimmer awake than enraged.
const GLOW_AWAKE: u32 = 0xFFD8D8D8;
const GLOW_RAGE: u32 = 0xFFFFFFFF;
/// How far the mound sinks into a crouch (blocks): its folded legs no longer hold it up.
const MOUND_DROP: f32 = 1.6;
/// How much larger its seams are drawn than its rock.
const GLOW_OVER: f32 = 1.003;

/// What its pose reads of a frame: its walk's progress and speed, its head's turn and pitch (degrees), its age (ticks).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Frame {
    pub walk: f32,
    pub speed: f32,
    pub head_yaw: f32,
    pub pitch: f32,
    pub age: f32,
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

/// How curled up it is: 1 a mound, 0 standing; between while it rises or lies down.
pub(crate) fn curled(mode: Mode) -> f32 {
    match mode {
        Mode::Dormant => 1.0,
        Mode::Waking(t) => 1.0 - t as f32 / golem::RISE as f32,
        Mode::LyingDown(t) => t as f32 / golem::RISE as f32,
        Mode::Awake => 0.0,
    }
}

/// The mound (SPECS §2.1): legs folded under, back rounded high, arms wrapped around, head tucked under the brow.
fn mound(name: &str) -> [f32; 3] {
    match name {
        "body" => [38.0, 0.0, 0.0],
        "head" => [40.0, 0.0, 0.0],
        "leg_left" | "leg_right" => [-75.0, 0.0, 0.0],
        "shin_left" | "shin_right" => [110.0, 0.0, 0.0],
        "arm_left" => [-25.0, 0.0, 28.0],
        "arm_right" => [-25.0, 0.0, -28.0],
        "forearm_left" => [-40.0, 0.0, 0.0],
        "forearm_right" => [-40.0, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// How far into an attack's wind-up it is (0 to 1), and the wind-up's length in ticks.
fn windup(attack: Attack) -> Option<(f32, f32)> {
    let (t, of) = match attack {
        Attack::Slam(t) => (t, golem::SLAM_WINDUP),
        Attack::Pound(t) => (t, golem::POUND_WINDUP),
        Attack::Glob(t) => (t, golem::GLOB_WINDUP),
        Attack::Leap(t) => (t, golem::LEAP_WINDUP),
        Attack::None => return None,
    };
    Some(((t as f32 / of as f32).min(1.0), of as f32))
}

/// How it holds itself this frame (SPECS §2.1).
pub(crate) fn pose(name: &str, now: &Frame, shown: &Shown) -> [f32; 3] {
    let speed = now.speed.min(1.0) * if shown.crusted { 0.5 } else { 1.0 };
    // A slow stride: the legs swing 22° each way at a full walk, the arms a little less, the knees bending.
    let phase = now.walk * 0.45;
    let stride = phase.sin() * 22.0 * speed;
    let roll = phase.sin() * 4.0 * speed;
    let dip = (phase * 2.0).cos().abs() * 2.0 * speed;
    // A breath every 4 seconds or so, faded out as it walks; asleep, slower and deeper.
    let rate = if shown.mode == Mode::Dormant { 0.04 } else { 0.08 };
    let breath = (now.age * rate).sin() * 2.5 * (1.0 - speed);
    let standing: [f32; 3] = match name {
        "head" => [now.pitch * 0.6, -now.head_yaw * 0.8, 0.0],
        "body" => [dip + breath * 0.4, 0.0, roll],
        "back" => [-breath, 0.0, 0.0],
        "shoulder_left" | "shoulder_right" => [-breath * 0.6, 0.0, 0.0],
        "leg_left" => [stride, 0.0, 0.0],
        "leg_right" => [-stride, 0.0, 0.0],
        "shin_left" => [phase.cos().max(0.0) * 18.0 * speed, 0.0, 0.0],
        "shin_right" => [(-phase).cos().max(0.0) * 18.0 * speed, 0.0, 0.0],
        "arm_left" => [-stride * 0.7, 0.0, 0.0],
        "arm_right" => [stride * 0.7, 0.0, 0.0],
        _ => [0.0; 3],
    };
    let attack = attack(name, shown);
    let roar = roar(name, shown);
    let c = curled(shown.mode);
    let m = mound(name);
    let mut out = [0.0; 3];
    for k in 0..3 {
        out[k] = (standing[k] + attack[k] + roar[k]) * (1.0 - c) + m[k] * c;
    }
    out
}

/// What an attack adds to the pose: the fist slam (one arm high behind, then down in front), the ground pound (both
/// arms over its head, then down together, the body dropping with them), the glob (one hand lifting a glob over its
/// head), the leap (a crouch, the arms swung back; in the air the arms thrown up and forward, the legs tucked).
fn attack(name: &str, shown: &Shown) -> [f32; 3] {
    let attack = shown.attack;
    let Some((t, _)) = windup(attack) else {
        return [0.0; 3];
    };
    if let Attack::Leap(_) = attack {
        return leap(name, if shown.aloft { None } else { Some(t) });
    }
    match (attack, name) {
        (Attack::Slam(_), "arm_right") => [-160.0 * t, 0.0, 0.0],
        (Attack::Slam(_), "body") => [-6.0 * t, 0.0, 0.0],
        (Attack::Pound(_), "arm_left" | "arm_right") => [-170.0 * t, 0.0, 0.0],
        (Attack::Pound(_), "jaw") => [25.0 * t, 0.0, 0.0],
        (Attack::Pound(_), "head") => [-15.0 * t, 0.0, 0.0],
        (Attack::Glob(_), "arm_left") => [-175.0 * t, 0.0, -15.0 * t],
        (Attack::Glob(_), "forearm_left") => [-30.0 * t, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Its leap: crouched `Some(t)` of the way, or in the air (`None`).
fn leap(name: &str, crouch: Option<f32>) -> [f32; 3] {
    match (crouch, name) {
        (Some(t), "body") => [22.0 * t, 0.0, 0.0],
        (Some(t), "leg_left" | "leg_right") => [-45.0 * t, 0.0, 0.0],
        (Some(t), "shin_left" | "shin_right") => [70.0 * t, 0.0, 0.0],
        (Some(t), "arm_left" | "arm_right") => [45.0 * t, 0.0, 0.0],
        (Some(t), "head") => [-20.0 * t, 0.0, 0.0],
        (None, "body") => [-10.0, 0.0, 0.0],
        (None, "arm_left" | "arm_right") => [-150.0, 0.0, 0.0],
        (None, "leg_left" | "leg_right") => [-35.0, 0.0, 0.0],
        (None, "shin_left" | "shin_right") => [55.0, 0.0, 0.0],
        (None, "jaw") => [30.0, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// How far it sinks in its crouch before a leap (blocks).
fn crouch_drop(shown: &Shown) -> f32 {
    match (shown.attack, shown.aloft) {
        (Attack::Leap(t), false) => 0.6 * (t as f32 / golem::LEAP_WINDUP as f32).min(1.0),
        _ => 0.0,
    }
}

/// Its roar as it wakes (its head thrown back, the jaw open), and its jaw ajar enraged.
fn roar(name: &str, shown: &Shown) -> [f32; 3] {
    let waking = match shown.mode {
        // The roar comes as it straightens, in the last half of its rise.
        Mode::Waking(t) => ((t as f32 / golem::RISE as f32) * 2.0 - 1.0).clamp(0.0, 1.0),
        _ => 0.0,
    };
    let jaw = waking.max(if shown.enraged { 0.5 } else { 0.0 });
    match name {
        "jaw" => [30.0 * jaw, 0.0, 0.0],
        "head" => [-25.0 * waking, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Its own model, posed (`pose`), and its seams full-bright over it.
pub struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let shown = golem::shown(entity.id);
        let now = Frame::of(entity);
        draw.push();
        creature::model::living_pose(draw, entity);
        // Its scale is for its client box only (`creature::mob::scale_client_box`): the model keeps its size.
        let scale = mob.map_or(1.0, |m| m.scale);
        if scale > 0.0 && scale != 1.0 {
            draw.scale([1.0 / scale; 3]);
        }
        draw.translate([0.0, -MOUND_DROP * curled(shown.mode) - crouch_drop(&shown), 0.0]);
        let pose = |part: &str| pose(part, &now, &shown);
        let tint = creature::model::tint(mob);
        creature::model::draw_shown(draw, &model::MAGMA_GOLEM, TEXTURE, &pose, &|_| true, tint);
        // Its seams: dark crusted; asleep a slow pulse and its eyes shut.
        let asleep = shown.mode == Mode::Dormant;
        if !shown.crusted {
            let colour = if asleep {
                let pulse = 0.55 + 0.25 * (now.age * 0.05).sin();
                let v = (pulse * 255.0) as u32;
                0xFF00_0000 | v << 16 | v << 8 | v
            } else if shown.enraged {
                GLOW_RAGE
            } else {
                GLOW_AWAKE
            };
            draw.set_light(FULL_BRIGHT);
            // A hair larger than the rock it lies on, so the two don't fight for the same pixels.
            draw.scale([GLOW_OVER; 3]);
            let lit = |part: &str| !(asleep && part.starts_with("eye_"));
            creature::model::draw_shown(draw, &model::MAGMA_GOLEM, GLOW, &pose, &lit, colour);
        }
        draw.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_stands_still_at_rest_and_strides_when_it_walks() {
        let awake = Shown {
            mode: Mode::Awake,
            ..Shown::default()
        };
        assert_eq!(pose("leg_left", &Frame::default(), &awake), [0.0; 3]);
        let walking = Frame {
            walk: 3.0,
            speed: 1.0,
            ..Frame::default()
        };
        let left = pose("leg_left", &walking, &awake)[0];
        let right = pose("leg_right", &walking, &awake)[0];
        assert!(left != 0.0 && (left + right).abs() < 1e-4, "the legs swing opposite");
    }

    #[test]
    fn asleep_it_is_a_mound_and_it_unfolds_as_it_rises() {
        let asleep = Shown::default();
        assert_eq!(pose("leg_left", &Frame::default(), &asleep), mound("leg_left"));
        assert_eq!(curled(Mode::Waking(golem::RISE / 2)), 0.5);
        assert_eq!(curled(Mode::Awake), 0.0);
    }
}
