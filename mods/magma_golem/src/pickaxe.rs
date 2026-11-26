//! The molten pickaxe (SPECS §5): a netherite pickaxe that smelts what it mines. Its blocks' drops are swapped for
//! what a furnace would make of them (`SMELTS`), with the furnace's experience, in `block_drops`. That hook's host
//! reads the level only, so the items' keys are looked up once from a server tick (`prepare`).
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use creature::sync::Locked;
use mod_api::{BlockDropsEvent, Identifier, ItemKey, ItemStack, ServerHost};

pub const PICKAXE: &str = "magma_golem:molten_pickaxe";

/// What it smelts: a drop, what it becomes, the furnace's experience for one.
const SMELTS: &[(&str, &str, f32)] = &[
    ("minecraft:raw_iron", "minecraft:iron_ingot", 0.7),
    ("minecraft:raw_gold", "minecraft:gold_ingot", 1.0),
    ("minecraft:raw_copper", "minecraft:copper_ingot", 0.7),
    ("minecraft:ancient_debris", "minecraft:netherite_scrap", 2.0),
    ("minecraft:sand", "minecraft:glass", 0.1),
    ("minecraft:red_sand", "minecraft:glass", 0.1),
    ("minecraft:cobblestone", "minecraft:stone", 0.1),
    ("minecraft:cobbled_deepslate", "minecraft:deepslate", 0.1),
    ("minecraft:netherrack", "minecraft:nether_brick", 0.1),
    ("minecraft:clay_ball", "minecraft:brick", 0.3),
    ("minecraft:wet_sponge", "minecraft:sponge", 0.15),
];

struct Table {
    pickaxe: ItemKey,
    smelts: Vec<(ItemKey, ItemKey, f32)>,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);
/// Seeds for the chance of a last point of experience.
static DRAWS: AtomicU32 = AtomicU32::new(1);

/// From a server tick: the items' keys, looked up once.
pub fn prepare(host: &dyn ServerHost) {
    if TABLE.locked().is_some() {
        return;
    }
    let key = |id: &str| host.item_key(&Identifier::parse(id).ok()?);
    let Some(pickaxe) = key(PICKAXE) else {
        return;
    };
    let smelts = SMELTS
        .iter()
        .filter_map(|(from, to, xp)| Some((key(from)?, key(to)?, *xp)))
        .collect();
    *TABLE.locked() = Some(Table { pickaxe, smelts });
}

/// From `block_drops`: mined with the molten pickaxe, the drops come out smelted, with the furnace's experience.
pub fn smelt(event: &mut BlockDropsEvent<'_>) {
    let table = TABLE.locked();
    let Some(table) = table.as_ref() else {
        return;
    };
    if event.tool.is_none_or(|t| t.item() != table.pickaxe) {
        return;
    }
    let mut xp = 0.0;
    for drop in event.drops.iter_mut() {
        let Some(&(_, to, each)) = table.smelts.iter().find(|(from, ..)| *from == drop.stack.item()) else {
            continue;
        };
        let count = drop.stack.count();
        if let Some(stack) = ItemStack::new(to, count) {
            drop.stack = stack;
            xp += each * count as f32;
        }
    }
    let draw = DRAWS.fetch_add(1, Ordering::Relaxed);
    let roll = (creature::colours::Random::new(draw).next() % 1000) as f32 / 1000.0;
    event.experience += experience(xp, roll);
}

/// The furnace's way with a fraction of a point: the whole points, and the rest as a chance of one more.
fn experience(xp: f32, roll: f32) -> u32 {
    let whole = xp.floor();
    whole as u32 + u32::from(roll < xp - whole)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fraction_of_a_point_is_a_chance_of_one() {
        assert_eq!(experience(0.0, 0.0), 0);
        assert_eq!(experience(2.0, 0.9), 2);
        assert_eq!(experience(1.7, 0.5), 2);
        assert_eq!(experience(1.7, 0.8), 1);
    }
}
