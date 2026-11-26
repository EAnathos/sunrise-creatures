//! Taming and the tamed vulture (SPECS §5, §6).
//!
//! - `Eat`: it walks to the meat the hook found on the ground (`state::food`) and eats it. A wild vulture eating a raw
//!   chicken is tamed 1 time in 4 (hearts; smoke otherwise), by the player it sees (in single player, the one who
//!   threw it). A tamed one eats only when hurt, and heals.
//! - `Sit`: it stays put when told to (right-click, `interaction.rs`).
//! - Following its owner: the library's flight (`FOLLOWING`): flies to them when more than 10 blocks away and lands
//!   near them; teleports past 32 blocks.
//! - What hurts its owner, and what its owner attacks, becomes its target (as vanilla's wolf: the mod's damage hook,
//!   `crate::defend`).
//! - Breeding (SPECS §8): fed rotten flesh by its owner (`interaction.rs`), a grown vulture is in love for 30 s; it walks
//!   to another tamed vulture in love and, once close, they have a baby (born tamed) and wait 5 minutes: the library's
//!   `creature::growth::Mate` (`state::BREEDING`).
use creature::body::{Behaviour, Body, World};

use creature::flight::Following;

use crate::{sounds, state};

/// One raw chicken in this many tames it.
const TAME_CHANCE: i32 = 4;
/// How fast it walks to food (a share of its walking speed, `minecraft:movement_speed` 0.3 in `vulture.json`): a
/// vulture hurries to a carcass, about twice as fast as it strolls (0.6 felt too slow, the trader's 0.5 speed rushed).
const FOOD_PACE: f64 = 1.2;
/// What a meat eaten heals a tamed vulture.
const MEAT_HEAL: f32 = 4.0;

fn particles(m: &Body, kind: &'static str) {
    crate::particles(m.id, kind, [m.position.x, m.position.y + 1.0, m.position.z], 7);
}

/// Whether it has a target (fighting), as its mob says.
pub fn fighting(m: &Body, cx: &World<'_>) -> bool {
    crate::target(cx, m.id).is_some()
}

fn free(m: &Body, cx: &World<'_>) -> bool {
    creature::ride::of(m.id).is_none() && !fighting(m, cx)
}

/// Following its owner: free, not sitting, not on a lead (the lead leads it), and not a chick (a juvenile does).
fn may_follow(m: &Body, cx: &World<'_>) -> bool {
    let s = state::saved(m.id);
    free(m, cx) && !s.sitting && !state::leashed(m.id) && !state::chick(m.id)
}

#[derive(Debug, Default)]
pub struct Eat {
    ticks: i32,
}

impl Behaviour for Eat {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = free(m, cx) && !state::saved(m.id).sitting && state::food(m.id).is_some();
        if !on && self.ticks > 0 {
            self.ticks = 0;
        }
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.ticks = 0;
        m.stop_walking = true;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(food) = state::food(m.id) else {
            return;
        };
        self.ticks += 1;
        m.look_at = Some(food.at);
        let d2 = (m.position.x - food.at[0]).powi(2) + (m.position.z - food.at[2]).powi(2);
        if d2 > 1.5 * 1.5 || (m.position.y - food.at[1]).abs() > 2.0 {
            if self.ticks % 10 == 1 {
                m.walk_to = Some((food.at, FOOD_PACE));
            }
            return;
        }
        state::eaten(m.id, food);
        state::pecked(m);
        let pitch = 0.9 + m.random.next_float() * 0.2;
        sounds::queue(m.id, sounds::EAT, 1.0, pitch);
        if state::saved(m.id).owner.is_some() {
            m.health = (m.health + MEAT_HEAL).min(m.max_health);
        } else if food.taming
            && let Some(p) = cx
                .player
                .as_ref()
                .filter(|p| food.thrower.is_none_or(|t| t == p.entity_id))
        {
            // The player who threw it now knows a raw chicken tames a vulture (the bestiary).
            state::taming_seen(p.uuid);
            if m.random.next_int_bound(TAME_CHANCE).unwrap_or(1) == 0 {
                let owner = creature::friend::ints(p.uuid);
                state::with(m.id, |s| {
                    s.owner = Some(owner);
                    s.sitting = false;
                });
                state::touch(m.id);
                particles(m, "minecraft:heart");
            } else {
                particles(m, "minecraft:smoke");
            }
        }
    }
}

#[derive(Debug)]
pub struct Sit;

impl Behaviour for Sit {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        let s = state::saved(m.id);
        s.owner.is_some() && s.sitting && creature::ride::of(m.id).is_none()
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
}

/// Following its owner (the library's `creature::flight::Follow`): past 10 blocks it flies to them, 5 blocks over
/// their head, lands within 5, comes to them past 32.
pub static FOLLOWING: Following = Following {
    follow: 10.0,
    land: 5.0,
    teleport: 32.0,
    above: 5.0,
    may: may_follow,
    lands_on_water: false,
    ..Following::DEFAULT
};
