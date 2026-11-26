//! Friends (SPECS §6.2): a creature befriended (or tamed) by a player, who follows them or stays (sits) where it is
//! told, following on foot (`Follow`). Users: the fairies, the vulture, the seagulls, the sporelins, the elephant.
//!
//! The friend is saved on the entity (`creature:friend`: the player's UUID and whether it stays). The mod's goals read
//! it through a static map (`of`), filled by `tick` from the mod's `entity_tick_post` (single player, as the
//! territory); a change (`befriend`, `set_staying`) is saved by the next `tick`. A glass jar keeps the friend
//! of the creature it holds (`jar`).

use mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Registrar, ServerHost, Tag};

use crate::body::{Behaviour, Body, Vec3, World, distance};
use crate::state::EntityMap;

/// Whom a creature is the friend of, and whether it stays where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Friend {
    /// The player's UUID as four ints (`UUIDUtil.uuidToIntArray`, as `PlayerView.uuid` holds it).
    pub uuid: [i32; 4],
    pub staying: bool,
}

impl Friend {
    /// Whether `player` (as a server hook gives it) is this friend.
    pub fn is(&self, player: u128) -> bool {
        self.uuid == ints(player)
    }
}

/// A player's UUID as four ints, as `Friend.uuid` holds it.
pub fn ints(uuid: u128) -> [i32; 4] {
    [
        (uuid >> 96) as u32 as i32,
        (uuid >> 64) as u32 as i32,
        (uuid >> 32) as u32 as i32,
        uuid as u32 as i32,
    ]
}

/// A UUID back from its four ints.
pub fn uuid(ints: [i32; 4]) -> u128 {
    ints.iter()
        .fold(0u128, |acc, part| acc << 32 | u128::from(*part as u32))
}

static FRIEND: AttachmentType<Option<Friend>> = AttachmentType {
    namespace: "creature",
    name: "friend",
    default: || None,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From the library's `register`: the friend attachment is declared.
pub fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("creature:friend", false);
}

fn encode(friend: &Option<Friend>) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    if let Some(f) = friend {
        c.insert("Friend".into(), Tag::IntArray(f.uuid.to_vec()));
        c.insert("Staying".into(), Tag::Byte(i8::from(f.staying)));
    }
    Ok(c)
}

fn decode(c: &Compound) -> Result<Option<Friend>, AttachmentError> {
    Ok(match c.get(&"Friend".into()) {
        Some(Tag::IntArray(a)) if a.len() == 4 => Some(Friend {
            uuid: [a[0], a[1], a[2], a[3]],
            staying: matches!(c.get(&"Staying".into()), Some(Tag::Byte(1))),
        }),
        _ => None,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct Known {
    friend: Option<Friend>,
    /// Changed since it was saved.
    dirty: bool,
}

static FRIENDS: EntityMap<Known> = EntityMap::new();

/// What a player sees of their try to befriend a creature at `at`: hearts when it worked, smoke when not (as vanilla's
/// taming).
pub fn sign(host: &mut dyn ServerHost, level: mod_api::LevelRef, at: [f64; 3], befriended: bool) {
    let id = if befriended {
        "minecraft:heart"
    } else {
        "minecraft:smoke"
    };
    crate::particles::puff(host, level, id, at, 0.3, 7);
}

/// `entity` becomes the friend of `player` (following them), saved at once.
pub fn befriend(host: &mut dyn ServerHost, entity: u32, player: u128) {
    let friend = Some(Friend {
        uuid: ints(player),
        staying: false,
    });
    let saved = FRIEND.set(host, Holder::Entity(entity), friend).is_ok();
    FRIENDS.locked().insert(entity, Known { friend, dirty: !saved });
}

/// Whether `entity`'s save has been read (`tick`): until then, `of` says nothing about it.
pub fn loaded(entity: u32) -> bool {
    FRIENDS.locked().contains_key(&entity)
}

/// Whose friend `entity` is, if it has one (from a goal or a hook).
pub fn of(entity: u32) -> Option<Friend> {
    FRIENDS.locked().get(&entity).and_then(|k| k.friend)
}

/// Told to stay where it is, or to follow again; saved by the next `tick`.
pub fn set_staying(entity: u32, staying: bool) {
    if let Some(k) = FRIENDS.locked().get_mut(&entity)
        && let Some(f) = k.friend.as_mut()
    {
        f.staying = staying;
        k.dirty = true;
    }
}

/// From the mod's `entity_tick_post`: its saved friend the first time, a change saved.
pub fn tick(host: &mut dyn ServerHost, entity: u32) {
    let known = FRIENDS.locked().get(&entity).copied();
    match known {
        None => {
            // Not readable yet (its first tick after loading): next tick.
            let Ok(saved) = FRIEND.get_existing(host, Holder::Entity(entity)) else {
                return;
            };
            FRIENDS.locked().insert(
                entity,
                Known {
                    friend: saved.flatten(),
                    dirty: false,
                },
            );
        }
        Some(k) if k.dirty => {
            if FRIEND.set(host, Holder::Entity(entity), k.friend).is_ok()
                && let Some(k) = FRIENDS.locked().get_mut(&entity)
            {
                k.dirty = false;
            }
        }
        Some(_) => {}
    }
}

/// How a befriended creature follows its friend on foot (SPECS §6.2): from `follow` blocks away it walks to them at
/// `speed` (a multiplier of its own), stops within `near`, and past `teleport` comes to them at once, as a tamed wolf
/// does. Users: the Sporelins, the Elephant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Following {
    pub follow: f64,
    pub near: f64,
    pub teleport: f64,
    pub speed: f64,
}

impl Following {
    /// Its neutral values, for a mod's literal to end with (`..Following::DEFAULT`).
    pub const DEFAULT: Following = Following {
        follow: 4.0,
        near: 2.0,
        teleport: 24.0,
        speed: 1.0,
    };
}

/// Following its friend on foot (`Following`), unless told to stay.
#[derive(Debug)]
pub struct Follow {
    how: &'static Following,
    on: bool,
    ticks: i32,
}

impl Follow {
    pub fn new(how: &'static Following) -> Follow {
        Follow {
            how,
            on: false,
            ticks: 0,
        }
    }
}

/// Where its friend stands, when they are the player near it and alive.
pub fn friend_near(m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
    let f = of(m.id)?;
    cx.player
        .as_ref()
        .filter(|p| p.alive && f.is(p.uuid))
        .map(|p| p.position.array())
}

impl Behaviour for Follow {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let following = of(m.id).is_some_and(|f| !f.staying);
        let Some(at) = friend_near(m, cx).filter(|_| following) else {
            self.on = false;
            return false;
        };
        let d = distance(at, m.position.array());
        self.on = if self.on {
            d > self.how.near
        } else {
            d > self.how.follow
        };
        self.on
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(at) = friend_near(m, cx) else {
            return;
        };
        if distance(at, m.position.array()) > self.how.teleport {
            // Onto ground by them (not in the air by a flying friend, which it would fall from): else it waits.
            if let Some(p) = ground_by(cx, at) {
                m.position = Vec3::of(p);
                m.velocity = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
                m.fall_distance = 0.0;
            }
            return;
        }
        self.ticks -= 1;
        if self.ticks <= 0 {
            self.ticks = 10;
            m.walk_to = Some((at, self.how.speed));
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
    }
}

/// A spot to stand on by `at` (a block next to it, its own top solid, two open blocks over it), up to 3 blocks down.
fn ground_by(cx: &World<'_>, at: [f64; 3]) -> Option<[f64; 3]> {
    let open = |p: [i32; 3]| crate::creep::passable(cx, p);
    let (x, y, z) = (at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32);
    [[1, 0], [-1, 0], [0, 1], [0, -1], [1, 1], [-1, -1]]
        .iter()
        .find_map(|d| {
            (0..=3).map(|down| y - down).find_map(|y| {
                let p = [x + d[0], y, z + d[1]];
                let floor = [p[0], y - 1, p[2]];
                let solid = !open(floor) && cx.block(floor) != "minecraft:water";
                (solid && open(p) && open([p[0], y + 1, p[2]]))
                    .then(|| [f64::from(p[0]) + 0.5, f64::from(y), f64::from(p[2]) + 0.5])
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uuid_goes_through_its_four_ints_and_back() {
        let player: u128 = 0x40e3_27d0_b410_3010_829a_3199_8893_a5b7;
        let four = ints(player);
        assert_eq!(
            four,
            [
                0x40e3_27d0_u32 as i32,
                0xb410_3010_u32 as i32,
                0x829a_3199_u32 as i32,
                0x8893_a5b7_u32 as i32
            ]
        );
        assert_eq!(uuid(four), player);
        let friend = Friend {
            uuid: four,
            staying: false,
        };
        assert!(friend.is(player));
        assert!(!friend.is(player + 1));
    }
}
