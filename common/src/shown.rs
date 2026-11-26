//! What a creature's renderer shows, synced to the clients (docs/ARCHITECTURE.md, "Multiplayer"): its look, a wing
//! beat, playing dead, clinging to a trunk...
//!
//! A renderer has no host, and the server's state (`creature::state`) lives in the server's process: a client of
//! another computer never sees it. What a renderer needs goes through `creature:shown` instead, an attachment synced
//! to every client that sees the creature, which Sunrise hands the renderer with the entity
//! (`EntityRenderState::attachments`). It holds a few integers by key (`phasmids:dead`), written by the server only when
//! one changes.
//!
//! - a module registers each value its renderer needs (`register`): its key, the creatures it is about (their entity
//!   types; none for every one of ours), and how the server reads it (a `fn(u32) -> i32` over its own state);
//! - at each tick of one of our creatures the library reads its values and writes the attachment when they changed
//!   (`publish`, from its `entity_tick_post`): a value shows a tick after it changed;
//! - the renderer reads them (`get`, `flag`), 0 for a value not shown (a zero isn't sent).
//!
//! In single player the game's own server syncs to its own client the same way, so playing alone tries it.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, EntityRenderState, Holder, Registrar, ServerHost, Tag,
};

use crate::state::EntityMap;
use crate::sync::Locked;

/// A creature's shown values, by key; zeros left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Values(BTreeMap<String, i32>);

static SHOWN: AttachmentType<Values> = AttachmentType {
    namespace: "creature",
    name: "shown",
    default: Values::default,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: true,
};

/// From the library's `register`: the synced attachment is declared.
pub fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("creature:shown", true);
}

fn encode(v: &Values) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    for (key, value) in &v.0 {
        c.insert(key.as_str().into(), Tag::Int(*value));
    }
    Ok(c)
}

fn decode(c: &Compound) -> Result<Values, AttachmentError> {
    let mut v = Values::default();
    for (key, tag) in c.iter() {
        if let Tag::Int(n) = tag {
            v.0.insert(key.to_string_lossy(), *n);
        }
    }
    Ok(v)
}

/// A value a renderer shows: its key, the entity types it is about (none: every one of ours), how the server reads it.
#[derive(Clone, Copy)]
struct Shown {
    key: &'static str,
    kinds: &'static [&'static str],
    value: fn(u32) -> i32,
}

static REGISTERED: Mutex<Vec<Shown>> = Mutex::new(Vec::new());

/// From a module's `register`: its renderer shows `key` (namespaced: `phasmids:dead`) of the creatures of `kinds` (none:
/// every one of ours), which the server reads with `value`.
pub fn register(key: &'static str, kinds: &'static [&'static str], value: fn(u32) -> i32) {
    let mut all = REGISTERED.locked();
    if !all.iter().any(|s| s.key == key) {
        all.push(Shown { key, kinds, value });
    }
}

/// What was last written for each creature.
static WRITTEN: EntityMap<Values> = EntityMap::new();

/// Creature `entity`'s values now, as the server reads them.
fn values(entity: u32, kind: &str) -> Values {
    // Copied out of the lock: each `value` is a module's.
    let shown: Vec<Shown> = REGISTERED.locked().clone();
    let mut v = Values::default();
    for s in shown.iter().filter(|s| s.kinds.is_empty() || s.kinds.contains(&kind)) {
        let n = (s.value)(entity);
        if n != 0 {
            v.0.insert(s.key.to_owned(), n);
        }
    }
    v
}

/// From the library's `entity_tick_post`, for each of our creatures: its values written to the clients when changed.
pub fn publish(host: &mut dyn ServerHost, entity: u32, kind: &str) {
    let now = values(entity, kind);
    if WRITTEN.locked().get(&entity) == Some(&now) {
        return;
    }
    if SHOWN.set(host, Holder::Entity(entity), now.clone()).is_ok() {
        WRITTEN.locked().insert(entity, now);
    }
}

/// From a renderer: the value `key` of the creature it draws, 0 when not shown (or drawn in a screen, which syncs
/// nothing).
pub fn get(entity: &EntityRenderState, key: &str) -> i32 {
    entity
        .attachments
        .as_ref()
        .and_then(|a| a.get(&SHOWN).ok().flatten())
        .and_then(|v| v.0.get(key).copied())
        .unwrap_or(0)
}

/// From a renderer: whether the value `key` is shown (not 0).
pub fn flag(entity: &EntityRenderState, key: &str) -> bool {
    get(entity, key) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_go_through_their_codec_and_zeros_are_left_out() {
        register("test:on", &["test:a"], |id| i32::from(id == 6_000_000));
        register("test:twice", &[], |id| (id % 1000) as i32 * 2);
        let a = values(6_000_000, "test:a");
        assert_eq!(a.0.get("test:on"), Some(&1));
        assert_eq!(a.0.get("test:twice"), None, "zero: not sent");
        let b = values(6_000_001, "test:b");
        assert_eq!(b.0.get("test:on"), None, "not of its kinds");
        assert_eq!(b.0.get("test:twice"), Some(&2));
        assert_eq!(decode(&encode(&a).unwrap()).unwrap(), a);
    }
}
