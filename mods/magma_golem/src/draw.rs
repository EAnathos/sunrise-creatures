//! Its drawing (SPECS §2.1): its model posed from its render state and what its tick shows of it (`golem::shown`,
//! single player), then its seams drawn full-bright over it (`GLOW`, the texture's glowing pixels alone): pulsing
//! slowly asleep, its eyes dark; blazing enraged; gone dark crusted. The glob it is about to throw is drawn in its fist.
use creature::state::EntityMap;
use mod_api::{BlockEntityDrawContext, EntityRenderState, EntityRenderer};

use crate::golem::{self, Attack, Mode, Shown};
use crate::{TEXTURE, glob, model};

/// Its texture's glowing pixels (the seams, the lava, the eyes, the maw), drawn full-bright over it.
const GLOW: &str = "magma_golem:entity/magma_golem/magma_golem_glow";
/// Full light, as `BlockEntityDrawContext::set_light` takes it: sky and block light 15.
const FULL_BRIGHT: u32 = 0xF0F0;
/// The glow's colour (a multiplier): dimmer awake than enraged.
const GLOW_AWAKE: u32 = 0xFFD8D8D8;
const GLOW_RAGE: u32 = 0xFFFFFFFF;
/// How far the mound sinks into a crouch (blocks): its folded legs no longer hold it up.
const MOUND_DROP: f32 = 1.6;
/// How much larger its seams are drawn than its rock (texels each side), so the two don't fight for the same pixels.
const GLOW_OVER: f32 = 0.05;
/// The glob in its hand: where (blocks, in its fist's space: beyond its knuckles, on top once raised), how big at the
/// most (a block's width), how far into the wind-up it is torn off whole and its arm is up.
const HELD_AT: [f32; 3] = [-25.0 / 16.0, -0.3, 3.0 / 16.0];
const HELD_SIZE: f32 = 1.1;
const TORN: f32 = 0.4;
const RAISED: f32 = 0.6;

/// Its flinch when hurt: how long (ticks), and when each golem's began (its age then; single player, by its id).
const FLINCH: f32 = 8.0;
static FLINCHES: EntityMap<f32> = EntityMap::new();
/// Its death (vanilla's 20 ticks, then it is gone): down on its knees by `KNEELED`, slumped on its fists from `SLUMP`.
const DEATH: f32 = 20.0;
const KNEELED: f32 = 0.4;
const SLUMP: f32 = 0.3;
/// How far it sinks kneeling (blocks: its shins' length), and slumped.
const KNEEL_DROP: f32 = 1.05;
const SLUMP_DROP: f32 = 0.3;

/// What its pose reads of a frame: its walk's progress and speed, its head's turn and pitch (degrees), its age (ticks),
/// how far into its flinch and its death (0 to 1).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Frame {
    pub walk: f32,
    pub speed: f32,
    pub head_yaw: f32,
    pub pitch: f32,
    pub age: f32,
    pub flinch: f32,
    pub dying: f32,
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
            flinch: flinch(entity),
            dying: mob.map_or(0.0, |m| (m.death_time / DEATH).clamp(0.0, 1.0)),
        }
    }
}

/// Its flinch this frame, 0 to 1: from the moment its hurt flash shows, up and back over `FLINCH` ticks.
fn flinch(entity: &EntityRenderState) -> f32 {
    let red = entity.living.as_ref().is_some_and(|m| m.has_red_overlay);
    if entity.id == creature::model::IN_A_SCREEN {
        return 0.0;
    }
    let mut all = FLINCHES.locked();
    if !red {
        all.remove(&entity.id);
        return 0.0;
    }
    let since = entity.age - *all.entry(entity.id).or_insert(entity.age);
    ((since / FLINCH).clamp(0.0, 1.0) * std::f32::consts::PI).sin()
}

/// How far down on its knees and slumped it is, `dying` into its death.
fn fallen(dying: f32) -> (f32, f32) {
    (
        (dying / KNEELED).min(1.0),
        ((dying - SLUMP) / (1.0 - SLUMP)).clamp(0.0, 1.0),
    )
}

/// Its death (SPECS §2.1): it drops to its knees (the shins folded back under it), then slumps forward on its fists.
fn death(name: &str, dying: f32) -> [f32; 3] {
    let (knees, slump) = fallen(dying);
    match name {
        "shin_left" | "shin_right" => [85.0 * knees, 0.0, 0.0],
        "body" => [35.0 * slump, 0.0, 0.0],
        "head" => [30.0 * slump, 0.0, 0.0],
        "arm_left" | "arm_right" => [-30.0 * slump, 0.0, 0.0],
        "jaw" => [20.0 * slump, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Its flinch (SPECS §2.1): its body jerked back, its head thrown back and aside.
fn flinched(name: &str, flinch: f32) -> [f32; 3] {
    match name {
        "body" => [-8.0 * flinch, 0.0, 0.0],
        "head" => [-18.0 * flinch, 10.0 * flinch, 0.0],
        "arm_left" => [0.0, 0.0, -8.0 * flinch],
        "arm_right" => [0.0, 0.0, 8.0 * flinch],
        _ => [0.0; 3],
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
    // Dying, its attack and roar are let go.
    let (attack, roar) = if now.dying > 0.0 {
        ([0.0; 3], [0.0; 3])
    } else {
        (attack(name, shown), roar(name, shown))
    };
    let death = death(name, now.dying);
    let flinched = flinched(name, now.flinch);
    let c = curled(shown.mode);
    let m = mound(name);
    let mut out = [0.0; 3];
    for k in 0..3 {
        out[k] = (standing[k] + attack[k] + roar[k] + death[k] + flinched[k]) * (1.0 - c) + m[k] * c;
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
        (Attack::Glob(_), "arm_left") => [-175.0 * up(t), 0.0, -15.0 * up(t)],
        (Attack::Glob(_), "forearm_left") => [-30.0 * up(t), 0.0, 0.0],
        _ => [0.0; 3],
    }
}

/// How far its arm is up, `t` into its glob's wind-up: up at `RAISED`, then held there.
fn up(t: f32) -> f32 {
    (t / RAISED).min(1.0)
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

/// A grey of brightness `v` (0 to 1), opaque: a multiplier for a colour.
fn grey(v: f32) -> u32 {
    let v = (v.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF00_0000 | v << 16 | v << 8 | v
}

/// Its own model, posed (`pose`), and its seams full-bright over it.
pub struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        // In the guide's 3D view (no golem of the world), it stands awake rather than as its default, a sleeping mound.
        let shown = if entity.id == creature::model::IN_A_SCREEN {
            Shown {
                mode: Mode::Awake,
                ..Shown::default()
            }
        } else {
            golem::shown(entity.id)
        };
        let now = Frame::of(entity);
        draw.push();
        // Upright as it dies: it kneels and slumps (`death`) rather than tipping over.
        creature::model::living_pose_upright(draw, entity);
        // Its scale is for its client box only (`creature::mob::scale_client_box`): the model keeps its size.
        let scale = mob.map_or(1.0, |m| m.scale);
        if scale > 0.0 && scale != 1.0 {
            draw.scale([1.0 / scale; 3]);
        }
        let (knees, slump) = fallen(now.dying);
        let drop = MOUND_DROP * curled(shown.mode) + crouch_drop(&shown) + KNEEL_DROP * knees + SLUMP_DROP * slump;
        draw.translate([0.0, -drop, 0.0]);
        let pose = |part: &str| pose(part, &now, &shown);
        let tint = creature::model::tint(mob);
        // The glob it is about to throw, in its left fist: torn off, growing, then lifted overhead.
        let held = match (shown.attack, glob::magma_block()) {
            (Attack::Glob(_), Some(key)) if now.dying == 0.0 => {
                windup(shown.attack).map(|(t, _)| (key, (t / TORN).min(1.0)))
            }
            _ => None,
        };
        creature::model::draw_attached(draw, &model::MAGMA_GOLEM, TEXTURE, &pose, tint, &mut |part, draw| {
            if let Some((key, grown)) = held
                && part == "fist_left"
            {
                draw.push();
                draw.translate(HELD_AT);
                draw.rotate_about([0.0, 1.0, 0.0], now.age * 6.0, [0.0; 3]);
                draw.scale([HELD_SIZE * (0.3 + 0.7 * grown); 3]);
                draw.translate([-0.5, -0.5, -0.5]);
                draw.block(key, &[]);
                draw.pop();
            }
        });
        // Its seams: dark crusted; asleep a slow pulse and its eyes shut.
        let asleep = shown.mode == Mode::Dormant;
        if !shown.crusted {
            let colour = if asleep {
                grey(0.55 + 0.25 * (now.age * 0.05).sin())
            } else if shown.enraged {
                GLOW_RAGE
            } else {
                GLOW_AWAKE
            };
            // Dying, its glow fades out.
            let colour = 0xFF00_0000 | creature::model::multiply(colour, grey(1.0 - now.dying));
            draw.set_light(FULL_BRIGHT);
            let lit = |part: &str| !(asleep && part.starts_with("eye_"));
            creature::model::draw_layer(draw, &model::MAGMA_GOLEM, GLOW, &pose, &lit, colour, GLOW_OVER);
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
    fn dying_it_kneels_then_slumps_forward() {
        let awake = Shown {
            mode: Mode::Awake,
            ..Shown::default()
        };
        let at = |dying: f32, part: &str| {
            let now = Frame {
                dying,
                ..Frame::default()
            };
            pose(part, &now, &awake)[0]
        };
        assert_eq!(at(0.0, "shin_left"), 0.0);
        assert_eq!(at(KNEELED, "shin_left"), 85.0);
        assert_eq!(at(KNEELED, "body"), 35.0 * fallen(KNEELED).1);
        assert_eq!(at(1.0, "body"), 35.0);
    }

    #[test]
    fn asleep_it_is_a_mound_and_it_unfolds_as_it_rises() {
        let asleep = Shown::default();
        assert_eq!(pose("leg_left", &Frame::default(), &asleep), mound("leg_left"));
        assert_eq!(curled(Mode::Waking(golem::RISE / 2)), 0.5);
        assert_eq!(curled(Mode::Awake), 0.0);
    }
}
