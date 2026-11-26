//! The items in a player's hands, for items whose use the server's hooks handle: the glass jar, the guide, the mods'
//! throwables (butterfly dust, confetti).
use mod_api::{Hand, Host, ItemStack, LevelRef, PlayerRef, ServerHost};

/// The item id of `stack` (`creature:glass_jar`).
pub fn id(host: &dyn Host, stack: &ItemStack) -> String {
    stack.item().id(host)
}

/// Whether `stack` is the item `item`.
pub fn is(host: &dyn Host, stack: &ItemStack, item: &str) -> bool {
    id(host, stack) == item
}

/// What `player` holds in `hand`.
pub fn held(host: &dyn Host, player: PlayerRef, hand: Hand) -> Option<ItemStack> {
    host.living(player.entity())?.hand_item(host, hand)
}

/// Puts `stack` (or nothing) in `player`'s `hand`.
pub fn set_held(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand, stack: Option<ItemStack>) -> bool {
    host.living(player.entity())
        .is_some_and(|l| l.set_hand_item(host, hand, stack).unwrap_or(false))
}

/// Uses up one of what `player` holds in `hand` (not in creative), from a use hook.
pub fn use_one(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand) {
    if player.has_infinite_materials(host).unwrap_or(false) {
        return;
    }
    if let Some(stack) = held(host, player, hand) {
        set_held(host, player, hand, stack.shrink(1));
    }
}

/// A stack of `count` of item `id` (`minecraft:cod`), if the game knows the item (another mod's that isn't installed:
/// none).
pub fn stack(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    host.item_from_snbt(&format!("{{id:\"{id}\",count:{count}}}"))
}

/// A button of a creature's screen (`pack`): one of item `id` named by language key `name`, with a line under it saying
/// what a click does (`hint`, a language key), or a plain one if the game can't read the name.
pub fn button(host: &dyn Host, id: &str, name: &str, hint: Option<&str>) -> Option<ItemStack> {
    let lore = hint.map_or(String::new(), |h| {
        format!(",\"minecraft:lore\":[{{translate:\"{h}\",italic:0b}}]")
    });
    let snbt = format!("{{id:\"{id}\",count:1,components:{{\"minecraft:item_name\":{{translate:\"{name}\"}}{lore}}}}}");
    host.item_from_snbt(&snbt).or_else(|| stack(host, id, 1))
}

/// Gives `stack` to `player`, or drops it at their feet when their inventory is full.
pub fn give(host: &mut dyn ServerHost, player: PlayerRef, stack: ItemStack) {
    if player.give_stack(host, stack.clone()).is_err() {
        let _ = player.drop_stack(host, stack, false);
    }
}

/// Drops `stack` as an item on the ground at `at`, with its components (a jar's creature, a dagger's wear).
pub fn drop_stack(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3], stack: ItemStack) {
    let _ = level.spawn_item(host, at[0], at[1], at[2], stack, 10);
}

/// A standing player's eye height.
const EYE_HEIGHT: f64 = 1.62;

/// Where `player`'s eyes are and the way they look (a unit vector, vanilla's `Entity.calculateViewVector`).
pub fn eye_and_look(host: &dyn Host, player: PlayerRef) -> Option<([f64; 3], [f64; 3])> {
    let e = player.entity();
    let at = e.position(host)?;
    let [yaw, pitch] = e.rotation(host)?;
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    Some(([at[0], at[1] + EYE_HEIGHT, at[2]], look))
}
