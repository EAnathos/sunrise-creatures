//! A dung beetle's own behaviour (SPECS §2.1), before its crawl (the library's creeping): it keeps near the grazing
//! herd around it, now and then shapes a ball where an animal stood, rolls it away backwards in a straight line
//! (perfectly straight under a clear night sky, steered by the stars; wandering under clouds or a roof), and buries it
//! in the ground. A player who takes its ball (`take`) leaves it looking around.
//!
//! What it can't do itself (turn the ground greener, teach a fact) it queues as an `Action` (`crate::act`).
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World};
use creature::territory;

/// What the mod's tick does for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Action {
    /// Beetle `id` started shaping a ball (the players near learn it).
    Shaped { id: u32 },
    /// Beetle `id` buried its ball in `block`, after a roll perfectly straight under the stars (`stars`).
    Buried { id: u32, block: [i32; 3], stars: bool },
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

pub(crate) fn actions() -> Vec<Action> {
    std::mem::take(&mut *ACTIONS.lock().unwrap())
}

/// Its ball, for its drawing: being shaped (how far along, 0 to 1), rolled, or buried (how far along).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Ball {
    Shaping(f32),
    Rolling,
    Burying(f32),
}

/// Each beetle's ball, the beetles busy with theirs (their crawl waits), the ones whose ball a player took.
static BALL: Mutex<BTreeMap<u32, Ball>> = Mutex::new(BTreeMap::new());
static BUSY: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
static TAKEN: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

pub(crate) fn ball(id: u32) -> Option<Ball> {
    BALL.lock().unwrap().get(&id).copied()
}

fn set_ball(id: u32, ball: Option<Ball>) {
    let mut all = BALL.lock().unwrap();
    match ball {
        Some(b) => all.insert(id, b),
        None => all.remove(&id),
    };
}

/// For its crawl (`creature::creep::Creeping::may`): busy with its ball or its herd.
pub(crate) fn busy(id: u32) -> bool {
    BUSY.lock().unwrap().contains(&id)
}

/// A player took beetle `id`'s ball (SPECS §2.1): whether it had one to take.
pub(crate) fn take(id: u32) -> bool {
    if ball(id).is_none_or(|b| matches!(b, Ball::Burying(_))) {
        return false;
    }
    set_ball(id, None);
    TAKEN.lock().unwrap().insert(id);
    true
}

pub(crate) fn forget(id: u32) {
    BALL.lock().unwrap().remove(&id);
    BUSY.lock().unwrap().remove(&id);
    TAKEN.lock().unwrap().remove(&id);
}

/// The herd (SPECS §2): how far it sees grazing animals and how often it looks (ticks), how near one it keeps
/// (blocks).
const HERD_RANGE: f64 = 16.0;
const LOOK_EVERY: i32 = 40;
const NEAR_HERD: f64 = 5.0;
/// A ball now and then near a herd (ticks between two, about two minutes), how long shaping and burying take, how far it
/// rolls (blocks) and how fast (blocks a tick), how long it may roll at most (ticks), how long it looks around when its
/// ball is taken.
const BALL_EVERY: (i32, i32) = (1800, 3000);
const SHAPE: i32 = 80;
const BURY: i32 = 60;
const ROLL_LENGTH: (f64, f64) = (8.0, 16.0);
const ROLL_SPEED: f64 = 0.04;
const ROLL_MAX: i32 = 900;
const LOOK_AROUND: i32 = 60;
/// Under clouds or a roof, how much its heading wanders each tick at most (radians); how often it looks at the sky
/// (ticks) and how high a roof counts (blocks).
const WANDER: f64 = 0.08;
const SKY_EVERY: i32 = 20;
const SKY_HEIGHT: i32 = 24;

/// The blocks it buries a ball in (SPECS §2.1).
pub(crate) fn buriable(name: &str) -> bool {
    matches!(
        name,
        "minecraft:grass_block"
            | "minecraft:dirt"
            | "minecraft:coarse_dirt"
            | "minecraft:podzol"
            | "minecraft:rooted_dirt"
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
    #[default]
    Idle,
    /// Walking to `at` (to its herd, or where an animal stood to shape a ball there: `shape`), for so many more ticks.
    To { at: [f64; 3], left: i32, shape: bool },
    /// Shaping its ball, for so many more ticks.
    Shape(i32),
    /// Rolling it along `dir`: how far it has come and must go, whether its line is perfectly straight, for how long.
    Roll {
        dir: [f64; 2],
        done: f64,
        length: f64,
        straight: bool,
        ticks: i32,
    },
    /// Burying it, for so many more ticks; its roll was straight under the stars.
    Bury(i32, bool),
    /// Its ball taken: looking around, for so many more ticks.
    Startled(i32),
}

#[derive(Debug, Default)]
pub(crate) struct Roll {
    phase: Phase,
    /// Ticks to its next ball, to its next look for its herd and at the sky.
    wait: Option<i32>,
    look: i32,
    sky: bool,
    sky_in: i32,
    /// The nearest grazing animal it saw, and where it was last tick.
    herd: Option<[f64; 3]>,
    last: Option<Vec3>,
}

fn between(m: &mut Body, range: (i32, i32)) -> i32 {
    range.0 + m.random.next_int_bound((range.1 - range.0).max(1)).unwrap_or(0)
}

fn flat(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

fn turned(v: [f64; 2], angle: f64) -> [f64; 2] {
    let (s, c) = angle.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

fn cell(p: Vec3) -> [i32; 3] {
    [p.x.floor() as i32, (p.y + 0.05).floor() as i32, p.z.floor() as i32]
}

/// Whether nothing is over block `p` up to `SKY_HEIGHT` blocks: it sees the sky.
fn open_sky(cx: &World<'_>, p: [i32; 3]) -> bool {
    (1..=SKY_HEIGHT).all(|dy| matches!(cx.block([p[0], p[1] + dy, p[2]]).as_str(), "minecraft:air" | ""))
}

impl Roll {
    /// The nearest grazing animal within `HERD_RANGE` (within its pasture for a settled one).
    fn find_herd(m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
        let at = m.position.array();
        let anchor = territory::anchor(m.id);
        territory::grazers_near(cx.ground, at, HERD_RANGE)
            .into_iter()
            .map(|(_, p)| p)
            .filter(|p| {
                anchor.is_none_or(|a| {
                    let a = [f64::from(a[0]) + 0.5, f64::from(a[1]), f64::from(a[2]) + 0.5];
                    flat(*p, a) <= f64::from(territory::PASTURE_RANGE)
                })
            })
            .min_by(|a, b| flat(*a, at).total_cmp(&flat(*b, at)))
    }

    /// A heading to roll along: any way, or for a settled one far out in its pasture, back across it.
    fn heading(m: &mut Body) -> [f64; 2] {
        let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
        let any = [angle.cos(), angle.sin()];
        let Some(a) = territory::anchor(m.id) else {
            return any;
        };
        let (dx, dz) = (
            f64::from(a[0]) + 0.5 - m.position.x,
            f64::from(a[2]) + 0.5 - m.position.z,
        );
        let d = dx.hypot(dz);
        if d < f64::from(territory::PASTURE_RANGE) / 2.0 {
            return any;
        }
        let spread = (f64::from(m.random.next_float()) - 0.5) * 1.2;
        turned([dx / d, dz / d], spread)
    }

    fn face(m: &mut Body, dir: [f64; 2]) {
        let yaw = creature::flight::heading_of(dir[0], dir[1]);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }

    fn still(m: &mut Body) {
        m.stop_walking = true;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }

    fn idle(&mut self, m: &mut Body) {
        self.phase = Phase::Idle;
        self.wait = Some(between(m, BALL_EVERY));
    }

    fn roll(&mut self, m: &mut Body, cx: &World<'_>) {
        let Phase::Roll {
            mut dir,
            mut done,
            mut length,
            mut straight,
            ticks,
        } = self.phase
        else {
            return;
        };
        m.fall_distance = 0.0;
        let here = cell(m.position);
        self.sky_in -= 1;
        if self.sky_in <= 0 {
            self.sky_in = SKY_EVERY;
            self.sky = open_sky(cx, here);
        }
        // Under a clear night sky its line holds; under clouds, rain or a roof it wanders.
        let stars = !cx.daylight && !cx.raining && self.sky;
        if !stars {
            dir = turned(dir, (f64::from(m.random.next_float()) - 0.5) * 2.0 * WANDER);
            straight = false;
        }
        // A block in the way: it climbs a one-block step, or turns aside.
        let ahead = [
            (m.position.x + dir[0] * 0.5).floor() as i32,
            here[1],
            (m.position.z + dir[1] * 0.5).floor() as i32,
        ];
        if !creature::creep::passable(cx, ahead) && m.on_ground {
            let step = [ahead[0], ahead[1] + 1, ahead[2]];
            let head = [here[0], here[1] + 1, here[2]];
            // A fence, a wall or a gate is taller than a step: it turns aside.
            let name = cx.block(ahead);
            let tall = name.ends_with("fence") || name.ends_with("wall") || name.ends_with("fence_gate");
            if !tall && creature::creep::passable(cx, step) && creature::creep::passable(cx, head) {
                m.position = Vec3 {
                    x: m.position.x + dir[0] * 0.3,
                    y: f64::from(here[1]) + 1.0,
                    z: m.position.z + dir[1] * 0.3,
                };
            } else {
                let side = if m.random.next_int_bound(2) == Some(0) {
                    1.0
                } else {
                    -1.0
                };
                dir = turned(dir, side * std::f64::consts::FRAC_PI_2);
                straight = false;
            }
        }
        if let Some(last) = self.last {
            done += (m.position.x - last.x).hypot(m.position.z - last.z);
        }
        m.velocity.x = dir[0] * ROLL_SPEED;
        m.velocity.z = dir[1] * ROLL_SPEED;
        // Backwards: it faces away from where it goes, pushing with its hind legs.
        Self::face(m, [-dir[0], -dir[1]]);
        // A settled one stops at the edge of its pasture.
        let out = territory::anchor(m.id).is_some_and(|a| {
            let a = [f64::from(a[0]) + 0.5, f64::from(a[1]), f64::from(a[2]) + 0.5];
            flat(m.position.array(), a) > f64::from(territory::PASTURE_RANGE)
        });
        if done >= length || out || ticks >= ROLL_MAX {
            let below = [here[0], here[1] - 1, here[2]];
            if buriable(&cx.block(below)) {
                Self::still(m);
                self.phase = Phase::Bury(BURY, straight && stars);
                set_ball(m.id, Some(Ball::Burying(0.0)));
                return;
            }
            if ticks >= ROLL_MAX {
                // Nowhere to bury it (sand, stone): it gives it up.
                set_ball(m.id, None);
                return self.idle(m);
            }
            // Further on, to soft ground.
            length += 2.0;
        }
        self.phase = Phase::Roll {
            dir,
            done,
            length,
            straight,
            ticks: ticks + 1,
        };
    }
}

impl Behaviour for Roll {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        if self.wait.is_none() {
            self.wait = Some(between(m, BALL_EVERY));
        }
        if TAKEN.lock().unwrap().remove(&m.id) {
            self.phase = Phase::Startled(LOOK_AROUND);
            self.wait = Some(between(m, BALL_EVERY));
        }
        self.look -= 1;
        if self.look <= 0 {
            self.look = LOOK_EVERY;
            self.herd = Self::find_herd(m, cx);
        }
        if self.phase == Phase::Idle {
            let wait = self.wait.unwrap_or(0) - 1;
            self.wait = Some(wait);
            if let Some(h) = self.herd {
                let far = flat(h, m.position.array()) > NEAR_HERD;
                if wait <= 0 {
                    // Where the animal stands: there it shapes a ball.
                    self.phase = Phase::To {
                        at: h,
                        left: 300,
                        shape: true,
                    };
                } else if far && m.random.next_int_bound(LOOK_EVERY) == Some(0) {
                    let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
                    let near = [h[0] + angle.cos() * 2.0, h[1], h[2] + angle.sin() * 2.0];
                    self.phase = Phase::To {
                        at: near,
                        left: 300,
                        shape: false,
                    };
                }
            }
        }
        let busy = self.phase != Phase::Idle;
        let mut all = BUSY.lock().unwrap();
        if busy {
            all.insert(m.id);
        } else {
            all.remove(&m.id);
        }
        busy
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        match self.phase {
            Phase::Idle => {}
            Phase::To { at, left, shape } => {
                // There, or near enough (an animal may stand on the spot still): it shapes its ball where it is.
                let there = flat(at, m.position.array()) < if shape { 1.5 } else { 0.8 };
                if there || shape && left <= 0 {
                    Self::still(m);
                    if shape {
                        self.phase = Phase::Shape(SHAPE);
                        set_ball(m.id, Some(Ball::Shaping(0.0)));
                        ACTIONS.lock().unwrap().push(Action::Shaped { id: m.id });
                    } else {
                        self.phase = Phase::Idle;
                    }
                } else if left <= 0 {
                    Self::still(m);
                    self.phase = Phase::Idle;
                } else {
                    if left % 20 == 0 {
                        m.walk_to = Some((at, 1.0));
                    }
                    self.phase = Phase::To {
                        at,
                        left: left - 1,
                        shape,
                    };
                }
            }
            Phase::Shape(left) => {
                Self::still(m);
                set_ball(m.id, Some(Ball::Shaping(1.0 - left as f32 / SHAPE as f32)));
                if left > 0 {
                    self.phase = Phase::Shape(left - 1);
                } else {
                    let length = ROLL_LENGTH.0 + f64::from(m.random.next_float()) * (ROLL_LENGTH.1 - ROLL_LENGTH.0);
                    self.phase = Phase::Roll {
                        dir: Self::heading(m),
                        done: 0.0,
                        length,
                        straight: true,
                        ticks: 0,
                    };
                    self.sky_in = 0;
                    set_ball(m.id, Some(Ball::Rolling));
                }
            }
            Phase::Roll { .. } => self.roll(m, cx),
            Phase::Bury(left, stars) => {
                Self::still(m);
                set_ball(m.id, Some(Ball::Burying(1.0 - left as f32 / BURY as f32)));
                if left > 0 {
                    self.phase = Phase::Bury(left - 1, stars);
                } else {
                    let here = cell(m.position);
                    ACTIONS.lock().unwrap().push(Action::Buried {
                        id: m.id,
                        block: [here[0], here[1] - 1, here[2]],
                        stars,
                    });
                    set_ball(m.id, None);
                    self.idle(m);
                }
            }
            Phase::Startled(left) => {
                Self::still(m);
                // It looks this way and that.
                let yaw = m.body_yaw + ((left as f32) * 0.3).sin() * 6.0;
                m.yaw = yaw;
                m.body_yaw = yaw;
                m.head_yaw = yaw;
                self.phase = if left > 0 {
                    Phase::Startled(left - 1)
                } else {
                    Phase::Idle
                };
            }
        }
        self.last = Some(m.position);
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        if self.phase != Phase::Idle {
            set_ball(m.id, None);
            self.idle(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ball_is_taken_once_and_not_while_buried() {
        set_ball(7, Some(Ball::Rolling));
        assert!(take(7));
        assert!(!take(7));
        set_ball(8, Some(Ball::Burying(0.5)));
        assert!(!take(8));
        forget(7);
        forget(8);
    }

    #[test]
    fn soft_ground_takes_a_ball_sand_doesnt() {
        assert!(buriable("minecraft:grass_block"));
        assert!(buriable("minecraft:coarse_dirt"));
        assert!(!buriable("minecraft:sand"));
    }
}
