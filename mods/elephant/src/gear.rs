//! Its gear and its riders (SPECS §3): a tamed elephant wears no gear until its owner fits a vanilla saddle, drawn as a
//! woven blanket (the model's `saddle` group, in `elephant_saddle.png`'s neutral colours) and dyed in place with any of
//! the 16 dyes. Saddled, it carries two riders: its owner in front, who steers (`Ride`), and any player behind, who
//! rides along. Its owner fits two chests together on each flank, a 54-slot pack a side, kept and shown by the
//! library (`creature::pack`): sneak + right-click opens its screen (`ElephantScreens`). Its saddle, its chests and all
//! they hold drop when it dies.
//!
//! The gear is saved on the elephant (`elephant:gear`), read the first time its tick sees it, and kept here by its id
//! for the drawing (single player: the renderer reads it).
use creature::body::{Behaviour, Body, World};
use creature::pack::{Page, Screens};
use creature::state::EntityMap;
use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, EntityRef, Hand, Holder, MenuReplace, PlayerRef,
    Registrar, ServerHost, Tag, Text,
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
pub(crate) const PACKED_FACT: &str = "packed";
pub(crate) const BOTH_SIDES_FACT: &str = "both_sides";

const CHEST: &str = "minecraft:chest";
/// Its packs (SPECS §3): two chests together on each flank, 54 slots a side (the left's first in its storage).
const PER_SIDE: usize = 54;
const SIZE: usize = 2 * PER_SIDE;
/// Its screen's gear page: its saddle, staying or following, a button per side (a click opens that side's page, a
/// shift-click takes its chests off, with what they hold); then a page per side, its 54 slots.
const GEAR_PAGE: u8 = 0;
const LEFT_PAGE: u8 = 1;
const RIGHT_PAGE: u8 = 2;
const SADDLE_BUTTON: usize = 0;
const STAY_BUTTON: usize = 2;
const LEFT_BUTTON: usize = 6;
const RIGHT_BUTTON: usize = 7;

/// What it wears: its saddle, the dye of its blanket (an index in `DYES`), and its chests on each flank.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Gear {
    pub saddle: bool,
    pub dye: Option<u8>,
    pub left: bool,
    pub right: bool,
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
    c.insert("Left".into(), Tag::Byte(i8::from(g.left)));
    c.insert("Right".into(), Tag::Byte(i8::from(g.right)));
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
        left: byte("Left") == 1,
        right: byte("Right") == 1,
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
    if mine && player.entity().is_shift_key_down(&*host).unwrap_or(false) {
        creature::pack::open(host, player.uuid(), id, &SCREENS, GEAR_PAGE, MenuReplace::CloseFirst);
        return true;
    }
    match item {
        Some(CHEST) if mine && !(gear.left && gear.right) => {
            let held = creature::hand::held(&*host, player, hand).map_or(0, |s| s.count());
            if held < 2 && !creative {
                let _ = player.action_bar(host, Text::translate("elephant.two_chests", vec![]));
                return true;
            }
            use_one(host);
            use_one(host);
            let fitted = if gear.left {
                Gear { right: true, ..gear }
            } else {
                Gear { left: true, ..gear }
            };
            set(host, id, fitted);
            creature::sounds::queue(id, "minecraft:entity.donkey.chest", 1.0, 0.8);
            creature::bestiary::learn(host, player.uuid(), ELEPHANT, PACKED_FACT);
            if fitted.left && fitted.right {
                creature::bestiary::learn(host, player.uuid(), ELEPHANT, BOTH_SIDES_FACT);
            }
            true
        }
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

/// From `living_death`: its saddle, its chests and all they hold drop, and its screen closes.
pub(crate) fn died(host: &mut dyn ServerHost, id: u32) {
    let gear = of(id);
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let at = [at[0], at[1] + 1.0, at[2]];
    let chests = 2 * (u32::from(gear.left) + u32::from(gear.right));
    let stacks = [
        gear.saddle.then(|| creature::hand::stack(&*host, SADDLE, 1)).flatten(),
        (chests > 0)
            .then(|| creature::hand::stack(&*host, CHEST, chests))
            .flatten(),
    ];
    for stack in stacks.into_iter().flatten() {
        creature::hand::drop_stack(host, level, at, stack);
    }
    creature::pack::died(host, id, SIZE, level, at);
}

/// Its screen (the library's `pack`).
pub(crate) struct ElephantScreens;

pub(crate) static SCREENS: ElephantScreens = ElephantScreens;

impl Screens for ElephantScreens {
    fn size(&self) -> usize {
        SIZE
    }

    fn page(&self, host: &dyn ServerHost, id: u32, page: u8) -> Option<Page> {
        let gear = of(id);
        let item = |name: &str| creature::hand::stack(host, name, 1);
        let side = |storage: std::ops::Range<usize>, title: &str| Page {
            rows: 6,
            title: Text::translate(title, vec![]),
            buttons: Vec::new(),
            first: 0,
            storage: storage.collect(),
        };
        match page {
            GEAR_PAGE => {
                let staying = creature::friend::of(id).is_some_and(|f| f.staying);
                let mut buttons = vec![
                    (SADDLE_BUTTON, if gear.saddle { item(SADDLE) } else { None }),
                    (
                        STAY_BUTTON,
                        item(if staying { "minecraft:lead" } else { "minecraft:feather" }),
                    ),
                ];
                if gear.left {
                    buttons.push((LEFT_BUTTON, item(CHEST)));
                }
                if gear.right {
                    buttons.push((RIGHT_BUTTON, item(CHEST)));
                }
                Some(Page {
                    rows: 1,
                    title: Text::translate("container.elephant.elephant", vec![]),
                    buttons,
                    first: 0,
                    storage: Vec::new(),
                })
            }
            LEFT_PAGE if gear.left => Some(side(0..PER_SIDE, "container.elephant.left")),
            RIGHT_PAGE if gear.right => Some(side(PER_SIDE..SIZE, "container.elephant.right")),
            _ => None,
        }
    }

    /// The gear page's buttons: the saddle taken off (its riders getting off), staying or following, a side's page
    /// opened, or with a shift-click its chests taken off (given back with what they held).
    fn button(
        &self,
        host: &mut dyn ServerHost,
        player: PlayerRef,
        id: u32,
        _: u8,
        slot: usize,
        shift: bool,
    ) -> Option<u8> {
        let gear = of(id);
        match slot {
            SADDLE_BUTTON if gear.saddle => {
                if let Some(e) = host.entity(id) {
                    e.eject_passengers(host);
                }
                set(
                    host,
                    id,
                    Gear {
                        saddle: false,
                        dye: None,
                        ..gear
                    },
                );
                if let Some(s) = creature::hand::stack(&*host, SADDLE, 1) {
                    creature::hand::give(host, player, s);
                }
            }
            STAY_BUTTON => {
                let staying = creature::friend::of(id).is_some_and(|f| f.staying);
                creature::friend::set_staying(id, !staying);
            }
            LEFT_BUTTON | RIGHT_BUTTON => {
                let left = slot == LEFT_BUTTON;
                if !(if left { gear.left } else { gear.right }) {
                    return Some(GEAR_PAGE);
                }
                if !shift {
                    return Some(if left { LEFT_PAGE } else { RIGHT_PAGE });
                }
                let range = if left { 0..PER_SIDE } else { PER_SIDE..SIZE };
                let taken = creature::pack::take(host, id, SIZE, range);
                for s in taken.into_iter().chain(creature::hand::stack(&*host, CHEST, 2)) {
                    creature::hand::give(host, player, s);
                }
                let off = if left {
                    Gear { left: false, ..gear }
                } else {
                    Gear { right: false, ..gear }
                };
                set(host, id, off);
            }
            _ => {}
        }
        Some(GEAR_PAGE)
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
                left: true,
                right: false,
            },
        ] {
            assert_eq!(decode(&encode(&gear).unwrap()).unwrap(), gear);
        }
    }
}
