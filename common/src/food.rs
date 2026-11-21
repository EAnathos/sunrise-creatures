//! Food on the ground (SPECS §3.15): the vulture's raw meat, the seagull's fish and stolen food. A creature's tick
//! looks for the nearest item it wants near it (`nearest`, with who threw it: a thrown food may befriend, the
//! library's `friend`), its behaviour goes to it, and its tick eats one of the stack (`take_one`).
use mod_api::{LevelRef, ServerHost};

/// An item of food lying on the ground: its item entity, where it is, its item's id, and who threw it (an entity id),
/// if anyone did.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub item: u32,
    pub at: [f64; 3],
    pub id: String,
    pub thrower: Option<u32>,
}

/// The nearest item within `range` blocks of `at` (a box) that `wanted` (given its id) takes.
pub fn nearest(
    host: &dyn ServerHost,
    level: LevelRef,
    at: [f64; 3],
    range: f64,
    wanted: impl Fn(&str) -> bool,
) -> Option<Found> {
    let r = range;
    let distance = |p: [f64; 3]| crate::body::distance(p, at);
    level
        .entities_in(
            host,
            [at[0] - r, at[1] - r, at[2] - r],
            [at[0] + r, at[1] + r, at[2] + r],
        )
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let item = host.item_entity(e)?;
            let id = crate::hand::id(host, &item.stack(host)?);
            wanted(&id).then_some(Found {
                item: e.id(),
                at: e.position(host)?,
                id,
                thrower: item.owner(host).map(|o| o.id()),
            })
        })
        .min_by(|a, b| distance(a.at).total_cmp(&distance(b.at)))
}

/// One of the stack of item entity `item` eaten (the entity gone with the last). Whether there was one.
pub fn take_one(host: &mut dyn ServerHost, item: u32) -> bool {
    let Some(e) = host.entity(item) else {
        return false;
    };
    let Some(it) = host.item_entity(e) else {
        return false;
    };
    let Some(stack) = it.stack(&*host) else {
        return false;
    };
    match stack.shrink(1) {
        Some(rest) => {
            let _ = it.set_stack(host, rest);
        }
        None => {
            e.discard(host);
        }
    }
    true
}
