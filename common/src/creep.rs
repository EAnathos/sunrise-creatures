//! Creeping (SPECS §3.16): a small creature crawling by hand over the tops of blocks and up their sides, as a beetle or
//! a stick insect does: the ladybugs between flowers, the phasmids over the canopy and up the trunks. Unlike the small
//! crawlers' goal (`crawl`, walked by the engine's path finding), it moves itself, so it climbs.
//!
//! A mod gives a `Creeping` and adds `Creep::new(&HOW)` to its behaviours. A crawl picks a block top of its area (or a
//! point around where it is, for one with none), creeps straight toward it, and climbs while its target is above: up
//! the block in its way, or, under its target, up any side next to it (a trunk), through leaves as an insect goes
//! through foliage; then over the top. It pauses between crawls; while it pauses it doesn't claim, so the creature's
//! other behaviours (the small fliers' rest and flights) go on.
use mod_api::Host;

use crate::body::{Behaviour, Body, Vec3, World};

/// How a creature creeps.
pub struct Creeping {
    /// Its speed on a top and up a side (blocks a tick).
    pub speed: f64,
    pub climb: f64,
    /// How long a crawl and a pause last (ticks).
    pub crawl: (i32, i32),
    pub pause: (i32, i32),
    /// How far around where it is it creeps when its area is empty (blocks).
    pub wander: f64,
    /// The blocks it may creep onto (their tops), from its mod (`creature::territory`'s area of a settled one, the
    /// leaves of a canopy); empty for anywhere around it. Asked at each crawl's start.
    pub area: fn(&dyn Host, u32) -> Vec<[i32; 3]>,
    /// Whether it may creep now (by day, not playing dead...).
    pub may: fn(&Body, &World<'_>) -> bool,
}

/// Creeping, as a `Creeping` says.
pub struct Creep {
    how: &'static Creeping,
    target: Option<[f64; 3]>,
    ticks: i32,
    pausing: bool,
    area: Vec<[i32; 3]>,
    climbing: bool,
}

impl std::fmt::Debug for Creep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Creep")
            .field("target", &self.target)
            .field("climbing", &self.climbing)
            .finish()
    }
}

impl Creep {
    pub fn new(how: &'static Creeping) -> Self {
        Self {
            how,
            target: None,
            ticks: 0,
            pausing: true,
            area: Vec::new(),
            climbing: false,
        }
    }

    fn between(m: &mut Body, range: (i32, i32)) -> i32 {
        range.0 + m.random.next_int_bound((range.1 - range.0).max(1)).unwrap_or(0)
    }

    fn pick(&self, m: &mut Body) -> [f64; 3] {
        if !self.area.is_empty() {
            let b = self.area[m.random.next_int_bound(self.area.len() as i32).unwrap_or(0) as usize];
            return [f64::from(b[0]) + 0.5, f64::from(b[1]) + 1.0, f64::from(b[2]) + 0.5];
        }
        let mut r = || (f64::from(m.random.next_float()) * 2.0 - 1.0) * self.how.wander;
        let (dx, dz) = (r(), r());
        [m.position.x + dx, m.position.y, m.position.z + dz]
    }
}

/// Whether a creature walks through block `name` (air, grass, flowers and the like) rather than climbing it.
pub fn passable(cx: &World<'_>, p: [i32; 3]) -> bool {
    let name = cx.block(p);
    matches!(
        name.as_str(),
        "minecraft:air"
            | "minecraft:cave_air"
            | "minecraft:short_grass"
            | "minecraft:tall_grass"
            | "minecraft:fern"
            | "minecraft:large_fern"
            | "minecraft:dead_bush"
            | "minecraft:vine"
            | "minecraft:snow"
    ) || cx.ground.flower_at(p)
}

impl Behaviour for Creep {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        if !(self.how.may)(m, cx) {
            self.climbing = false;
            return false;
        }
        self.ticks -= 1;
        if self.pausing {
            if self.ticks > 0 {
                return false;
            }
            self.pausing = false;
            self.ticks = Self::between(m, self.how.crawl);
            self.area = (self.how.area)(cx.host(), m.id);
            self.target = Some(self.pick(m));
        }
        if self.ticks <= 0 {
            self.pausing = true;
            self.ticks = Self::between(m, self.how.pause);
            self.target = None;
            self.climbing = false;
            return false;
        }
        true
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        if self.climbing {
            m.no_gravity = false;
            self.climbing = false;
        }
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        let Some(t) = self.target else {
            return;
        };
        let (dx, dz) = (t[0] - m.position.x, t[2] - m.position.z);
        let flat = dx.hypot(dz);
        if flat < 0.15 && (m.position.y - t[1]).abs() < 0.6 {
            // There: a pause.
            self.ticks = 0;
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            return;
        }
        let dir = if flat > 0.001 {
            [dx / flat, dz / flat]
        } else {
            [0.0, 0.0]
        };
        let feet = m.position.y + 0.05;
        let cell = [
            m.position.x.floor() as i32,
            feet.floor() as i32,
            m.position.z.floor() as i32,
        ];
        let ahead = [
            (m.position.x + dir[0] * 0.4).floor() as i32,
            cell[1],
            (m.position.z + dir[1] * 0.4).floor() as i32,
        ];
        let wall = !passable(cx, ahead);
        // Its target above it: it climbs the wall in its way, or, under its target, any side next to it (a trunk).
        let above = t[1] > feet + if self.climbing { 0.0 } else { 0.5 };
        let side = flat < 0.8
            && [[1, 0], [-1, 0], [0, 1], [0, -1]]
                .iter()
                .any(|d| !passable(cx, [cell[0] + d[0], cell[1], cell[2] + d[1]]));
        if above && (wall || side || self.climbing) {
            // Up, through foliage as an insect goes (leaves don't stop it); a solid ceiling ends the crawl.
            let head = [cell[0], (m.position.y + 0.3).floor() as i32 + 1, cell[2]];
            let ceiling = !passable(cx, head) && !cx.block(head).ends_with("leaves");
            if ceiling {
                self.ticks = 0;
                return;
            }
            self.climbing = true;
            m.no_gravity = true;
            m.velocity = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
            m.position = Vec3 {
                y: m.position.y + self.how.climb,
                ..m.position
            };
        } else {
            // Over the top of a climb, or off a flower it sat on: walking, it is held by the ground.
            self.climbing = false;
            m.no_gravity = false;
            m.velocity.x = dir[0] * self.how.speed;
            m.velocity.z = dir[1] * self.how.speed;
            if wall {
                // A wall in the way of a target below it: given up.
                self.ticks = 0;
            }
        }
        let yaw = (-dir[0]).atan2(dir[1]).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_creep_starts_paused() {
        static HOW: super::Creeping = super::Creeping {
            speed: 0.02,
            climb: 0.03,
            crawl: (40, 80),
            pause: (40, 80),
            wander: 2.0,
            area: |_, _| Vec::new(),
            may: |_, _| true,
        };
        let c = super::Creep::new(&HOW);
        assert!(c.pausing && c.target.is_none());
    }
}
