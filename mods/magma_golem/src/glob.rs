//! The magma glob (SPECS §2): what a golem tears from its body and lobs at a target out of its fists' reach. A
//! projectile of the library's (`creature::projectile`: `magma_golem:magma_glob`, a copy of the wandering trader that
//! can't be hurt), thrown on an arc toward where the target stood. It hits the first time it comes within reach of its
//! target (its `minecraft:attack_damage`, `DAMAGE`), sets it on fire, and bursts there, on the ground or against a wall,
//! or after `LIFE` ticks: a small blast (`BLAST`) of flame and lava, fire on the ground around, and no block broken. Drawn as a
//! magma block tumbling (no model of its own).
use std::sync::OnceLock;

use creature::projectile::{self, Launch};
use mod_api::{
    BlockEntityDrawContext, BlockKey, EntityRenderState, EntityRenderer, ExplosionInteraction, ExplosionSettings,
    ParticleOptions, ServerHost, SoundSource,
};

pub const GLOB: &str = "magma_golem:magma_glob";
/// Its box.
pub const SIZE: creature::mob::Size = creature::mob::Size {
    width: 1.2,
    height: 1.2,
    eye_height: 0.6,
};
/// What it deals (its `minecraft:attack_damage`), and how long what it hits burns (ticks).
pub const DAMAGE: f64 = 6.0;
const BURN: i32 = 100;
/// Its gravity (blocks per tick, each tick), its speed along the ground (blocks per tick), how close it must come to
/// its target to hit it (blocks), and how long it may fly (ticks): fast and flat, a slight arc.
const GRAVITY: f64 = 0.03;
const SPEED: f64 = 1.5;
const REACH: f64 = 1.0;
const LIFE: i32 = 120;
/// Its burst: a blast of `BLAST` (a creeper's is 3) breaking no block, and fire on up to `FIRES` of the `SPLASH` spots.
const BLAST: f32 = 1.0;
const FIRES: usize = 5;
const SPLASH: [(i32, i32); 9] = [
    (0, 0),
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (2, 1),
    (-2, -1),
    (1, -2),
    (-1, 2),
];

/// The magma block's key, for the drawings (which have no host to ask): looked up at the first wind-up (single player).
static MAGMA_BLOCK: OnceLock<BlockKey> = OnceLock::new();

/// From the golem's fight, as it starts tearing a glob off: the drawings' block key, if not known yet.
pub fn prepare(host: &dyn ServerHost) {
    if MAGMA_BLOCK.get().is_none()
        && let Some(key) = host.block_key("minecraft:magma_block")
    {
        let _ = MAGMA_BLOCK.set(key);
    }
}

/// The magma block's key, for the golem's drawing of the glob in its hand.
pub fn magma_block() -> Option<BlockKey> {
    MAGMA_BLOCK.get().copied()
}

/// From the golem's fight: a glob lobbed from `from` at what stands at `to` (`target`, an entity id).
pub fn throw(host: &mut dyn ServerHost, dimension: &str, from: [f64; 3], to: [f64; 3], target: u32) {
    let launch = Launch {
        velocity: projectile::aim(from, to, SPEED, GRAVITY),
        gravity: GRAVITY,
        life: LIFE,
        target: Some((target, REACH)),
        ..Launch::DEFAULT
    };
    if let Some(level) = host.level(dimension) {
        level.play_sound(
            host,
            "minecraft:entity.blaze.shoot",
            from,
            SoundSource::Hostile,
            1.5,
            0.6,
            None,
        );
    }
    projectile::launch(host, dimension, GLOB, from, launch);
}

/// From the glob's tick: its flight, then, over, it bursts in flame where it is and is gone; what it reached burns.
pub fn tick(host: &mut dyn ServerHost, entity: u32) {
    projectile::tick_post(host, entity);
    let Some(end) = projectile::ended(entity) else {
        return;
    };
    let Some(e) = host.entity(entity) else {
        return;
    };
    if let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) {
        if matches!(end, projectile::End::Hit) {
            for id in projectile::hits(&*host, level, at, GLOB, 0, REACH + 1.0) {
                if let Some(t) = host.entity(id) {
                    t.set_remaining_fire_ticks(host, BURN);
                }
            }
        }
        // A small blast that breaks no block, and fire on the ground around.
        let _ = level.explode(
            host,
            Some(e),
            at,
            BLAST,
            true,
            ExplosionInteraction::None,
            ExplosionSettings::default(),
        );
        crate::golem::fires(host, level, at, &SPLASH, FIRES);
        for (particle, count, spread) in [("minecraft:flame", 30, 0.6), ("minecraft:lava", 12, 0.4)] {
            if let Some(k) = creature::particles::key(particle) {
                level.send_particles(
                    host,
                    &ParticleOptions::simple(k),
                    None,
                    [at[0], at[1] + 0.4, at[2]],
                    [spread, 0.3, spread],
                    0.05,
                    count,
                );
            }
        }
        level.play_sound(
            host,
            crate::golem::HISS,
            at,
            SoundSource::Hostile,
            1.2,
            0.8,
            None,
        );
    }
    e.discard(host);
}

/// Draws a glob: a magma block tumbling along its flight.
pub struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let Some(key) = magma_block() else {
            return Ok(());
        };
        let spin = entity.age * 20.0;
        draw.push();
        draw.translate([0.0, 0.6, 0.0]);
        draw.rotate_about([1.0, 0.0, 0.0], spin, [0.0; 3]);
        draw.rotate_about([0.0, 1.0, 0.0], spin * 0.6, [0.0; 3]);
        draw.scale([1.1; 3]);
        draw.translate([-0.5, -0.5, -0.5]);
        draw.block(key, &[]);
        draw.pop();
        Ok(())
    }
}
