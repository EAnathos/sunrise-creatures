//! Its gear and its riders (SPECS §3): a tamed elephant wears no gear until its owner fits a vanilla saddle, drawn as a
//! woven blanket (the model's `saddle` group, in `elephant_saddle.png`'s neutral colours) and dyed in place with any of
//! the 16 dyes. Saddled, it carries two riders: its owner in front, who steers (`Ride`), and any player behind, who
//! rides along. Its saddle drops when it dies.
//!
//! The gear is saved on the elephant (`elephant:gear`), read the first time its tick sees it, and kept here by its id
//! for the drawing (single player: the renderer reads it).
use creature::body::{Behaviour, Body, World};
use creature::state::EntityMap;
use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, EntityRef, Hand, Holder, PlayerRef, Registrar,
    ServerHost, Tag, Text,
};

use crate::ELEPHANT;

pub(crate) const SADDLE: &str = "minecraft:saddle";
/// The 16 dyes, in vanilla's order, and the colours their blankets are drawn in (vanilla's `DyeColor.textureDiffuse`).
const DYES: [(&str, u32); 16] = [
    ("white", 0xF9FFFE),
    ("orange", 0xF9801D),
    ("magenta", 0xC74EBD),
    ("light_blue", 0x3AB3DA),
    ("yellow", 0xFED83D),
    ("lime", 0x80C71F),
    ("pink", 0xF38BAA),
    ("gray", 0x474F52),
    ("light_gray", 0x9D9D97),
    ("cyan", 0x169C9C),
    ("purple", 0x8932B8),
    ("blue", 0x3C44AA),
    ("brown", 0x835432),
    ("green", 0x5E7C16),
    ("red", 0xB02E26),
    ("black", 0x1D1D21),
];
/// Riding (SPECS §3): its speed under its owner (blocks a tick, times its pace), backward and sideways a share of it,
/// and how fast it turns toward where its rider looks (degrees a tick).
const RIDE_SPEED: f64 = 0.22;
const BACKWARD: f64 = 0.4;
const SIDEWAYS: f64 = 0.4;
const TURN: f32 = 9.0;

/// Its guide facts (SPECS §6).
pub(crate) const SADDLED_FACT: &str = "saddled";
pub(crate) const COLOURED_FACT: &str = "coloured";
pub(crate) const RODE_FACT: &str = "rode";
pub(crate) const TWO_RIDERS_FACT: &str = "two_riders";

/// What it wears: its saddle, and the dye of its blanket (an index in `DYES`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Gear {
    pub saddle: bool,
    pub dye: Option<u8>,
}

static GEAR: AttachmentType<Gear> = AttachmentType {
    namespace: "elephant",
    name: "gear",
    default: Gear::default,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// Each elephant's gear, once read.
static KNOWN: EntityMap<Gear> = EntityMap::new();

/// From `register`: its gear's attachment is declared.
pub(crate) fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("elephant:gear", false);
}

fn encode(g: &Gear) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    c.insert("Saddle".into(), Tag::Byte(i8::from(g.saddle)));
    c.insert("Dye".into(), Tag::Byte(g.dye.map_or(-1, |d| d as i8)));
    Ok(c)
}

fn decode(c: &Compound) -> Result<Gear, AttachmentError> {
    let byte = |key: &str| match c.get(&key.into()) {
        Some(Tag::Byte(b)) => *b,
        _ => 0,
    };
    let dye = byte("Dye");
    Ok(Gear {
        saddle: byte("Saddle") == 1,
        dye: (0..16).contains(&dye).then_some(dye as u8),
    })
}

/// Its gear (none until read).
pub(crate) fn of(id: u32) -> Gear {
    KNOWN.locked().get(&id).copied().unwrap_or_default()
}

/// The colour its blanket is drawn in (0xRRGGBB, white undyed).
pub(crate) fn blanket(gear: Gear) -> u32 {
    gear.dye.map_or(creature::model::WHITE, |d| DYES[usize::from(d)].1)
}

/// From its tick: its gear read the first time.
pub(crate) fn tick(host: &mut dyn ServerHost, id: u32) {
    if KNOWN.locked().contains_key(&id) {
        return;
    }
    // Not readable yet (its first tick after loading): next tick.
    if let Ok(saved) = GEAR.get_existing(host, Holder::Entity(id)) {
        KNOWN.locked().insert(id, saved.unwrap_or_default());
    }
}

/// Its gear changed: kept and saved.
fn set(host: &mut dyn ServerHost, id: u32, gear: Gear) {
    KNOWN.locked().insert(id, gear);
    let _ = GEAR.set(host, Holder::Entity(id), gear);
}

/// Its right-click (SPECS §3), on the server, before its owner's other uses (`tame`): its owner fits a saddle, dyes the
/// blanket, or gets on in front; another player gets on behind its owner. Whether it took the click.
pub(crate) fn interact(host: &mut dyn ServerHost, id: u32, player: PlayerRef, hand: Hand, item: Option<&str>) -> bool {
    let Some(e) = host.entity(id) else {
        return false;
    };
    let owner = creature::friend::of(id);
    let mine = owner.is_some_and(|f| f.is(player.uuid()));
    let gear = of(id);
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    let use_one = |host: &mut dyn ServerHost| {
        if !creative {
            creature::hand::use_one(host, player, hand);
        }
    };
    match item {
        Some(SADDLE) if mine && !gear.saddle => {
            use_one(host);
            set(host, id, Gear { saddle: true, ..gear });
            creature::sounds::queue(id, "minecraft:entity.horse.saddle", 1.0, 0.8);
            creature::bestiary::learn(host, player.uuid(), ELEPHANT, SADDLED_FACT);
            true
        }
        Some(dye) if mine && gear.saddle => {
            let Some(index) = DYES.iter().position(|(name, _)| dye == format!("minecraft:{name}_dye")) else {
                return false;
            };
            if gear.dye == Some(index as u8) {
                return false;
            }
            use_one(host);
            set(
                host,
                id,
                Gear {
                    dye: Some(index as u8),
                    ..gear
                },
            );
            creature::sounds::queue(id, "minecraft:item.dye.use", 1.0, 1.0);
            creature::bestiary::learn(host, player.uuid(), ELEPHANT, COLOURED_FACT);
            true
        }
        None if gear.saddle && owner.is_some() => mount(host, e, player, mine),
        _ => false,
    }
}

/// `player` gets on: its owner in front (any rider already on moving behind), another player behind its owner only.
fn mount(host: &mut dyn ServerHost, elephant: EntityRef, player: PlayerRef, owner: bool) -> bool {
    let riders = elephant.passengers(&*host).unwrap_or_default();
    if riders.len() >= 2 || riders.iter().any(|r| r.id() == player.entity().id()) {
        return false;
    }
    if !owner {
        if riders.is_empty() {
            let _ = player.action_bar(host, Text::translate("elephant.owner_in_front", vec![]));
            return true;
        }
        if player.entity().start_riding(host, elephant, true).is_ok() {
            two_riders(host, elephant);
        }
        return true;
    }
    // Its owner takes the front seat: whoever sat there gets back on behind.
    for r in &riders {
        r.stop_riding(host);
    }
    creature::friend::set_staying(elephant.id(), false);
    if player.entity().start_riding(host, elephant, true).is_ok() {
        creature::bestiary::learn(host, player.uuid(), ELEPHANT, RODE_FACT);
    }
    for r in riders {
        let _ = r.start_riding(host, elephant, true);
    }
    if elephant.passengers(&*host).is_some_and(|p| p.len() == 2) {
        two_riders(host, elephant);
    }
    true
}

/// Both its riders learn the guide's "Two riders".
fn two_riders(host: &mut dyn ServerHost, elephant: EntityRef) {
    let riders = elephant.passengers(&*host).unwrap_or_default();
    for r in riders {
        if let Some(player) = creature::mob::player_of(&*host, r.id()) {
            creature::bestiary::learn(host, player, ELEPHANT, TWO_RIDERS_FACT);
        }
    }
}

/// From `living_death`: its saddle drops.
pub(crate) fn died(host: &mut dyn ServerHost, id: u32) {
    if !of(id).saddle {
        return;
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    if let Some(stack) = creature::hand::stack(&*host, SADDLE, 1) {
        creature::hand::drop_stack(host, level, [at[0], at[1] + 1.0, at[2]], stack);
    }
}

/// Ridden (SPECS §3): its owner in front steers, toward where they look, forward and back, sideways a little; with
/// another in front (its owner got off), it stands.
#[derive(Debug, Default)]
pub(crate) struct Ride;

impl Behaviour for Ride {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        creature::ride::of(m.id).is_some()
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        let Some(rider) = creature::ride::of(m.id) else {
            return;
        };
        let steering = creature::friend::of(m.id).is_some_and(|f| f.is(rider.player));
        if !steering {
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            return;
        }
        let turn = creature::flight::wrap(rider.yaw - m.yaw).clamp(-TURN, TURN);
        m.yaw = creature::flight::wrap(m.yaw + turn);
        m.body_yaw = m.yaw;
        m.head_yaw = m.yaw;
        let speed = RIDE_SPEED * creature::stats::pace(m.id);
        let forward = f64::from(rider.forward) * if rider.forward < 0.0 { BACKWARD } else { 1.0 };
        let sideways = f64::from(rider.strafe) * SIDEWAYS;
        let (sin, cos) = f64::from(m.yaw).to_radians().sin_cos();
        // Facing (-sin, cos); its left (cos, sin).
        m.velocity.x = (-sin * forward + cos * sideways) * speed;
        m.velocity.z = (cos * forward + sin * sideways) * speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_gear_is_saved_and_read_back() {
        for gear in [
            Gear::default(),
            Gear {
                saddle: true,
                dye: Some(14),
            },
        ] {
            assert_eq!(decode(&encode(&gear).unwrap()).unwrap(), gear);
        }
    }
}
