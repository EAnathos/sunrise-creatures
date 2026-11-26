//! Its defence (SPECS §2): it attacks nothing unprovoked, but hit (the library's `anger` notes by whom), it makes a
//! short charge at its attacker if they are within `NEAR` blocks, strikes them once on reaching them (`DAMAGE`, thrown
//! back) and lets them go: no pursuit. Hit again, it may charge again once `COOLDOWN` ticks have passed. Whoever hits
//! one of its riders is charged the same way (`riders_hit`).
use creature::body::{Behaviour, Body, World};
use creature::state::EntityMap;
use mod_api::ServerHost;

/// How near its attacker must be (blocks), how fast it runs at them (blocks a tick), how near it strikes (blocks from
/// its middle: half its width and a little), how long a charge may last and how long until the next (ticks).
const NEAR: f64 = 8.0;
const SPEED: f64 = 0.32;
const REACH: f64 = 2.0;
const LONGEST: i32 = 50;
const COOLDOWN: i32 = 60;
/// Its blow: damage, and how far it throws back and up (blocks a tick).
const DAMAGE: f32 = 4.0;
const BACK: f64 = 1.1;
const UP: f64 = 0.4;

/// Its trumpet as it charges (`assets/elephant/sounds.json`).
pub(crate) const TRUMPET: &str = "elephant:entity.elephant.trumpet";

/// The blows its charges landed, for its tick to deal (a behaviour can't hurt): the elephant's id, its target's.
static BLOWS: EntityMap<u32> = EntityMap::new();

#[derive(Debug, Default)]
pub(crate) struct Charge {
    /// Ticks into this charge, and until the next may start.
    ticks: i32,
    cooldown: i32,
}

impl Behaviour for Charge {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        self.cooldown -= 1;
        if self.cooldown > 0 {
            // Hit during its cooldown: it lets that go too.
            creature::anger::forget(m.id);
            return false;
        }
        if creature::anger::pursued(m, cx, NEAR).is_none() {
            creature::anger::forget(m.id);
            return false;
        }
        true
    }

    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some((who, t)) = creature::anger::pursued(m, cx, NEAR) else {
            return;
        };
        if self.ticks == 0 {
            creature::sounds::queue(m.id, TRUMPET, 1.5, 0.9 + m.random.next_float() * 0.2);
        }
        self.ticks += 1;
        let (dx, dz) = (t[0] - m.position.x, t[2] - m.position.z);
        let across = dx.hypot(dz).max(0.01);
        let yaw = creature::flight::heading_of(dx, dz);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
        if across <= REACH || self.ticks >= LONGEST {
            if across <= REACH {
                BLOWS.locked().insert(m.id, who);
            }
            self.done(m);
            return;
        }
        m.stop_walking = true;
        m.velocity.x = dx / across * SPEED;
        m.velocity.z = dz / across * SPEED;
    }

    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        if self.ticks > 0 {
            self.done(m);
        }
    }
}

impl Charge {
    /// The charge over: its target let go, its cooldown begun.
    fn done(&mut self, m: &mut Body) {
        creature::anger::forget(m.id);
        self.ticks = 0;
        self.cooldown = COOLDOWN;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
}

/// Players hit, and by whom (an entity id), for the elephant they ride (from the damage hook, which can't look up a
/// player's mount).
static RIDERS_HIT: EntityMap<u32> = EntityMap::new();

/// From the damage hook: player `rider` (an entity id) was hit by `by`.
pub(crate) fn rider_hit(rider: u32, by: u32) {
    RIDERS_HIT.locked().insert(rider, by);
}

/// From its tick: whoever hit one of its riders is charged (SPECS §2), unless it is one of them.
pub(crate) fn riders_hit(host: &dyn ServerHost, elephant: u32) {
    let riders: Vec<u32> = host
        .entity(elephant)
        .and_then(|e| e.passengers(host))
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.id())
        .collect();
    for rider in &riders {
        let Some(by) = RIDERS_HIT.locked().remove(rider) else {
            continue;
        };
        if by != elephant && !riders.contains(&by) {
            creature::anger::provoke(elephant, by);
        }
    }
}

/// From its tick: the blow its charge landed, dealt: damage, and its target thrown back and up.
pub(crate) fn strike(host: &mut dyn ServerHost, elephant: u32) {
    let Some(target) = BLOWS.locked().remove(&elephant) else {
        return;
    };
    let (Some(me), Some(t)) = (host.entity(elephant), host.entity(target)) else {
        return;
    };
    t.hurt(host, "minecraft:mob_attack", DAMAGE, Some(me), Some(me));
    if let (Some(a), Some(b), Some(v)) = (me.position(&*host), t.position(&*host), t.velocity(&*host)) {
        let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
        let across = dx.hypot(dz).max(0.01);
        t.set_velocity(host, [v[0] + dx / across * BACK, v[1] + UP, v[2] + dz / across * BACK]);
    }
}
