//! Its fear of mice (SPECS §2): a mouse of the Mice mod (`mice:mouse`, when it is installed: nothing requires it) within
//! `NEAR` blocks, and it trumpets and flees from it for `FOR` ticks, its riders staying on. The hook looks for mice
//! every `SCAN` ticks (`look`), the `Flee` behaviour runs.
use creature::body::{Behaviour, Body, World};
use creature::state::EntityMap;
use mod_api::ServerHost;

const MOUSE: &str = "mice:mouse";
/// How near a mouse frightens it (blocks), how long it flees and how often it looks (ticks), how fast it runs (blocks a
/// tick).
const NEAR: f64 = 8.0;
const FOR: i32 = 100;
const SCAN: i64 = 10;
const SPEED: f64 = 0.3;

/// Each frightened elephant: where the mouse was, and the ticks it has left to flee.
static FRIGHT: EntityMap<([f64; 3], i32)> = EntityMap::new();

/// From its tick: a mouse near it, and it takes fright (trumpeting) unless already fleeing.
pub(crate) fn look(host: &dyn ServerHost, id: u32) {
    if FRIGHT.locked().contains_key(&id) {
        return;
    }
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    if level.game_time(host).is_none_or(|t| (t + i64::from(id)) % SCAN != 0) {
        return;
    }
    let r = NEAR;
    let mouse = level
        .entities_in(
            host,
            [at[0] - r, at[1] - r, at[2] - r],
            [at[0] + r, at[1] + r, at[2] + r],
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|m| m.kind(host).is_some_and(|k| k.id(host) == MOUSE))
        .filter_map(|m| m.position(host))
        .find(|p| creature::body::distance(*p, at) <= NEAR);
    if let Some(p) = mouse {
        FRIGHT.locked().insert(id, (p, FOR));
        creature::sounds::queue(id, crate::charge::TRUMPET, 1.5, 1.2);
    }
}

/// Running from the mouse, for as long as its fright lasts.
#[derive(Debug, Default)]
pub(crate) struct Flee;

impl Behaviour for Flee {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        FRIGHT.locked().contains_key(&m.id)
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        let from = {
            let mut all = FRIGHT.locked();
            let Some((from, left)) = all.get_mut(&m.id) else {
                return;
            };
            *left -= 1;
            let from = *from;
            if *left <= 0 {
                all.remove(&m.id);
            }
            from
        };
        let (dx, dz) = (m.position.x - from[0], m.position.z - from[2]);
        let away = dx.hypot(dz).max(0.01);
        m.stop_walking = true;
        m.velocity.x = dx / away * SPEED;
        m.velocity.z = dz / away * SPEED;
        let yaw = creature::flight::heading_of(dx, dz);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
}
