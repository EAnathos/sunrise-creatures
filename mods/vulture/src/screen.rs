//! The vulture's screen and its chests (SPECS §6.1, §6.3), shown and kept by the library (`creature::pack`).
//!
//! Its owner opens it with sneak + right-click (`interaction.rs`). One page: the top row is the vulture's (its saddle,
//! sit / follow, a button per chest), then 15 slots per chest. A chest taken off, the saddle taken off (only without
//! chests) or the vulture's death gives everything back (to the owner, or on the ground). A save from before kept the
//! chests' contents in `vulture:chests`: they are moved to the library's `creature:pack` once (`migrate`).
use creature::pack::{Page, Screens};
use creature::state::EntitySet;
use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, ItemStack, MenuReplace, PlayerRef, Registrar,
    ServerHost, Tag, Text,
};

use crate::state::{self, Change};

/// Slots per chest, and both chests'.
pub const PER_CHEST: usize = 15;
const SIZE: usize = 2 * PER_CHEST;
const ROW: usize = 9;
/// The top row's buttons.
const HARNESS: usize = 0;
const SIT: usize = 2;
const CHEST_LEFT: usize = 6;
const CHEST_RIGHT: usize = 7;

/// The chests' contents as a save from before kept them, read once to be moved (`migrate`).
static OLD_CHESTS: AttachmentType<Vec<Option<Compound>>> = AttachmentType {
    namespace: "vulture",
    name: "chests",
    default: || vec![None; SIZE],
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From the mod's `register`: its attachments (the old chests', to read a save from before).
pub fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("vulture:chests", false);
}

// `&Vec`: the attachment's codec takes `fn(&T)` with `T` its type, a `Vec`.
#[allow(clippy::ptr_arg)]
fn encode(items: &Vec<Option<Compound>>) -> Result<Compound, AttachmentError> {
    let list = items
        .iter()
        .enumerate()
        .filter_map(|(k, s)| {
            let mut c = s.clone()?;
            c.insert("Slot".into(), Tag::Byte(k as i8));
            Some(Tag::Compound(c))
        })
        .collect();
    Ok([("Items".into(), Tag::List(list))].into())
}

fn decode(data: &Compound) -> Result<Vec<Option<Compound>>, AttachmentError> {
    let mut items = vec![None; SIZE];
    if let Some(Tag::List(list)) = data.get(&"Items".into()) {
        for tag in list {
            let Tag::Compound(c) = tag else { continue };
            let Some(Tag::Byte(slot)) = c.get(&"Slot".into()) else {
                continue;
            };
            if let Some(cell) = items.get_mut(*slot as usize) {
                let mut item = c.clone();
                item.remove(&"Slot".into());
                *cell = Some(item);
            }
        }
    }
    Ok(items)
}

/// The vultures whose old chests were looked at.
static MIGRATED: EntitySet = EntitySet::new();

/// From its tick, once: a save from before's chests moved to the library's storage.
pub fn migrate(host: &mut dyn ServerHost, id: u32) {
    if MIGRATED.locked().contains(&id) {
        return;
    }
    let Ok(old) = OLD_CHESTS.get_existing(host, Holder::Entity(id)) else {
        return;
    };
    MIGRATED.locked().insert(id);
    let Some(old) = old.filter(|o| o.iter().any(Option::is_some)) else {
        return;
    };
    if creature::pack::is_empty(host, id) {
        let items: Vec<Option<ItemStack>> = old
            .iter()
            .map(|c| c.as_ref().and_then(|c| ItemStack::from_nbt(&*host, c)))
            .collect();
        creature::pack::set_contents(host, id, &items);
    }
    let _ = OLD_CHESTS.set(host, Holder::Entity(id), vec![None; SIZE]);
}

/// Its screen's page (the library's `pack`).
pub struct VultureScreens;

pub static SCREENS: VultureScreens = VultureScreens;

impl Screens for VultureScreens {
    fn size(&self) -> usize {
        SIZE
    }

    fn page(&self, host: &dyn ServerHost, id: u32, _: u8) -> Option<Page> {
        let saved = state::saved(id);
        let storage = PER_CHEST * usize::from(saved.chests);
        let item = |id: &str| creature::hand::stack(host, id, 1);
        let mut buttons = vec![
            (
                HARNESS,
                if saved.harness {
                    item(crate::interaction::SADDLE)
                } else {
                    None
                },
            ),
            (
                SIT,
                item(if saved.sitting {
                    "minecraft:lead"
                } else {
                    "minecraft:feather"
                }),
            ),
        ];
        if saved.chests >= 1 {
            buttons.push((CHEST_LEFT, item("minecraft:chest")));
        }
        if saved.chests >= 2 {
            buttons.push((CHEST_RIGHT, item("minecraft:chest")));
        }
        Some(Page {
            rows: (1 + storage.div_ceil(ROW)) as u8,
            title: Text::translate("container.vulture.vulture", vec![]),
            buttons,
            first: ROW,
            storage: (0..storage).collect(),
        })
    }

    /// A button of the top row: sit / follow, a chest or the saddle taken off (given back with what the chest held).
    /// The screen opens again in place, showing the change.
    fn button(&self, host: &mut dyn ServerHost, player: PlayerRef, id: u32, _: u8, slot: usize, _: bool) -> Option<u8> {
        let saved = state::saved(id);
        match slot {
            SIT => state::request(id, Change::ToggleSit),
            CHEST_LEFT | CHEST_RIGHT if saved.chests > 0 => {
                // The last chest put on comes off first; what it held goes back with it.
                let index = usize::from(saved.chests) - 1;
                let taken = creature::pack::take(host, id, SIZE, index * PER_CHEST..(index + 1) * PER_CHEST);
                for s in taken
                    .into_iter()
                    .chain(creature::hand::stack(&*host, "minecraft:chest", 1))
                {
                    creature::hand::give(host, player, s);
                }
                state::request(id, Change::RemoveChest);
            }
            HARNESS if saved.harness && saved.chests == 0 => {
                if let Some(s) = creature::hand::stack(&*host, crate::interaction::SADDLE, 1) {
                    creature::hand::give(host, player, s);
                }
                state::request(id, Change::RemoveHarness);
            }
            _ => {}
        }
        Some(0)
    }
}

/// Sneak + right-click on its own vulture (`interaction.rs` checked it is the owner): its screen.
pub fn ask(host: &mut dyn ServerHost, player: PlayerRef, id: u32) {
    creature::pack::open(host, player.uuid(), id, &SCREENS, 0, MenuReplace::CloseFirst);
}

/// Its death: its screen closes, its saddle, chests and their contents fall where it died.
pub fn died(host: &mut dyn ServerHost, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let saved = state::saved(id);
    let mut stacks: Vec<ItemStack> = Vec::new();
    if saved.harness {
        stacks.extend(creature::hand::stack(&*host, crate::interaction::SADDLE, 1));
    }
    if saved.chests > 0 {
        stacks.extend(creature::hand::stack(
            &*host,
            "minecraft:chest",
            u32::from(saved.chests),
        ));
    }
    for s in stacks {
        creature::hand::drop_stack(host, level, at, s);
    }
    creature::pack::died(host, id, SIZE, level, at);
}
