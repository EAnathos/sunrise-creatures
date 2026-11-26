//! Its ridden flight's seat (SPECS §6.2): sneaking in the air descends instead of getting off (`keep_seated`); on the
//! ground it gets off as on any mount. Its rider and their keys are the library's (`creature::ride`).
use mod_api::ServerHost;

/// How far below its feet ground still counts as landed, in blocks.
const LANDED: f64 = 0.5;

/// From the `entity_mount` hook: a rider getting off vulture `vehicle` in the air (sneaking to descend) stays on. In
/// the air means no ground just under its feet: its `on_ground` flickers while it walks ridden (its flight's no
/// gravity), and a sneak there must get the rider off.
pub fn keep_seated(host: &dyn ServerHost, vehicle: u32) -> bool {
    let Some(e) = host.entity(vehicle) else {
        return false;
    };
    let (Some([x, y, z]), Some(level)) = (e.position(host), e.level(host)) else {
        return false;
    };
    let below = [x - 0.4, y - LANDED, z - 0.4, x + 0.4, y, z + 0.4];
    let grounded = level.collision_boxes(host, below).is_none_or(|b| !b.is_empty());
    !grounded && !e.is_in_water(host).unwrap_or(false)
}
