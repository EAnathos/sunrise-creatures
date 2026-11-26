//! Riders (SPECS §6.1): who rides one of the series' creatures and which keys they hold. Users: the Vulture (its
//! ridden flight), the Elephant (walking ridden).
//!
//! The server hears each player's movement keys (`player_input`, vanilla's `ServerboundPlayerInputPacket` flags:
//! forward, backward, left, right, jump, sneak, sprint) and the library keeps those of the players riding one of our
//! creatures (`heard`), forgotten when they log out, or by the mod when they get off (`left`). Each tick, before its
//! behaviours run, a mod reads its creature's rider (`read`: its first passenger, since vanilla names a controlling one
//! only on its own saddled mounts; the player's look and keys), which its behaviours take (`of`).
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::state::EntityMap;
use crate::sync::Locked;
use mod_api::ServerHost;

/// The rider of a creature this tick: where they look and what they press.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rider {
    /// The player riding it.
    pub player: u128,
    pub yaw: f32,
    /// 1 forward (W), -1 backward (S).
    pub forward: f32,
    /// 1 left (A), -1 right (D).
    pub strafe: f32,
    /// Space.
    pub jumping: bool,
    /// Sneak or sprint: down.
    pub descend: bool,
}

/// Vanilla's `Input` flags.
const FORWARD: i32 = 1;
const BACKWARD: i32 = 2;
const LEFT: i32 = 4;
const RIGHT: i32 = 8;
const JUMP: i32 = 16;
const SNEAK: i32 = 32;
const SPRINT: i32 = 64;

/// The keys of each player riding one of our creatures, as last heard.
static KEYS: Mutex<BTreeMap<u128, i32>> = Mutex::new(BTreeMap::new());
/// Each ridden creature's rider this tick.
static RIDERS: EntityMap<Rider> = EntityMap::new();

/// From the library's `player_input` hook: `riding` whether the player rides one of our creatures (only their keys are
/// kept).
pub fn heard(player: u128, flags: i32, riding: bool) {
    let mut keys = KEYS.locked();
    if riding {
        keys.insert(player, flags);
    } else {
        keys.remove(&player);
    }
}

/// From the mod's `entity_mount` (getting off) and the library's `player_logged_out`: player `player`'s keys are
/// forgotten.
pub fn left(player: u128) {
    KEYS.locked().remove(&player);
}

/// From the creature's tick, before its behaviours: its rider now, if a player rides it.
pub fn read(host: &dyn ServerHost, id: u32) {
    let rider = host
        .entity(id)
        .and_then(|e| e.passengers(host))
        .and_then(|passengers| passengers.into_iter().next())
        .and_then(|first| {
            let player = host
                .players()
                .server_players()
                .into_iter()
                .find(|p| p.entity().id() == first.id())?;
            let flags = KEYS.locked().get(&player.uuid()).copied().unwrap_or(0);
            let yaw = player.entity().rotation(host)?[0];
            let axis = |plus: i32, minus: i32| {
                f32::from(i8::from(flags & plus != 0)) - f32::from(i8::from(flags & minus != 0))
            };
            Some(Rider {
                player: player.uuid(),
                yaw,
                forward: axis(FORWARD, BACKWARD),
                strafe: axis(LEFT, RIGHT),
                jumping: flags & JUMP != 0,
                descend: flags & (SNEAK | SPRINT) != 0,
            })
        });
    let mut riders = RIDERS.locked();
    match rider {
        Some(r) => riders.insert(id, r),
        None => riders.remove(&id),
    };
}

/// Its rider this tick, if any.
pub fn of(id: u32) -> Option<Rider> {
    RIDERS.locked().get(&id).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_make_the_rider_s_axes() {
        heard(7, FORWARD | LEFT | SNEAK, true);
        let flags = KEYS.locked()[&7];
        assert_eq!(flags & FORWARD, FORWARD);
        assert_eq!(flags & (SNEAK | SPRINT), SNEAK);
        heard(8, FORWARD, false);
        assert!(!KEYS.locked().contains_key(&8));
        left(7);
        assert!(!KEYS.locked().contains_key(&7));
    }
}
