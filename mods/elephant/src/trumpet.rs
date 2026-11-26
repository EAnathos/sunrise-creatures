//! The ceremonial trumpet (SPECS §5): an ivory tusk, a copper ingot and string. A right-click plays the elephant's
//! trumpet where its player stands; again after `COOLDOWN` ticks. It commands no elephant and never wears out.
use mod_api::{HostSide, InteractionResult, ItemPart, SoundSource, UseItemEvent};

pub(crate) const TRUMPET: &str = "elephant:ceremonial_trumpet";
/// Its rest between calls (ticks).
const COOLDOWN: i32 = 60;

pub(crate) struct Trumpet;

#[mod_api::sunrise_part]
impl ItemPart for Trumpet {
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return Some(InteractionResult::Consume);
        };
        let player = host.player(event.player)?;
        if player.has_item_cooldown(&*host, TRUMPET) == Some(true) {
            return Some(InteractionResult::Fail);
        }
        if let (Some(at), Some(level)) = (player.entity().position(&*host), player.level(&*host)) {
            level.play_sound(
                host,
                crate::charge::TRUMPET,
                [at[0], at[1] + 1.5, at[2]],
                SoundSource::Players,
                2.0,
                1.0,
                None,
            );
        }
        player.set_item_cooldown(host, TRUMPET, COOLDOWN);
        Some(InteractionResult::Consume)
    }
}
