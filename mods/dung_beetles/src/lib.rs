//! Dung Beetles (SPECS.md): small beetles that follow the grazing herds, roll balls of dung in straight lines, steered
//! by the stars, and bury them, turning worn ground back into grass, of the Sunrise Creatures series.
//!
//! A beetle is a copy of the wandering trader drawn with its own model (`model_dung_beetle.rs`, from
//! `sources/models/`), its ball a part of it. It comes out on grassy ground (the library's spawning of small creatures,
//! on given blocks), keeps near the herd around it, shapes, rolls and buries its balls (`roll`), and crawls about the
//! rest of the time (the library's creeping). Chickens peck it up (the library's prey). Caught in the glass jar, it
//! settles in a pasture (the library's pasture rule). A buried ball, or a dung ball a player took from a beetle and
//! uses on the ground, turns that block greener (`fertilize`).
mod model_dung_beetle;
mod roll;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use creature::jar::Catchable;
use creature::territory::{self, Ground};
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Hooks,
    Host, HostSide, InteractionResult, ItemPart, LevelRef, LivingDeathEvent, RegisterEntityRenderersEvent, Registrar,
    RegistrationContext, ServerHost, SoundSource, UseOnEvent,
};
use roll::{Action, Ball};

pub const KIND: &str = "dung_beetles:dung_beetle";
pub const DUNG_BALL: &str = "dung_beetles:dung_ball";

/// Its box (blocks, SPECS §1) and the scale its model is drawn at: about a third of a block long, its ball as big.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.4,
    height: 0.25,
    eye_height: 0.15,
};
const SCALE: f32 = 0.5;
const TEXTURE: &str = "dung_beetles:entity/dung_beetle";

/// Where it lives (SPECS §2): grasslands and savannas.
const BIOMES: &[&str] = &[
    "minecraft:savanna",
    "minecraft:savanna_plateau",
    "minecraft:plains",
    "minecraft:sunflower_plains",
    "minecraft:meadow",
    "minecraft:desert",
];

/// When it comes out (the library's spawning takes a small flier's hours; it doesn't fly): by day and at night, not in
/// the rain.
static HOURS: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Always,
    speed: 0.0,
    height: (0.0, 0.0),
    flight: (1, 2),
    rest: (1, 2),
    wander: 0.0,
    drawn_to_light: false,
    lands_on_flowers: false,
    takeoff: None,
};

/// On grass, coarse dirt or dirt, or sand next to grass (the edge of a desert), 1 to 3 together, up to 4 within 48
/// blocks of a player (SPECS §2).
static SPAWNING: creature::flutter::Spawning = creature::flutter::Spawning {
    kind: KIND,
    flight: &HOURS,
    biomes: BIOMES,
    group: (1, 3),
    cap: 4,
    chance: 3,
};

fn spawns_on(ground: Ground<'_>, p: [i32; 3]) -> bool {
    match ground.block(p).as_str() {
        "minecraft:grass_block" | "minecraft:coarse_dirt" | "minecraft:dirt" => true,
        "minecraft:sand" => [[1, 0], [-1, 0], [0, 1], [0, -1]]
            .iter()
            .any(|d| ground.block([p[0] + d[0], p[1], p[2] + d[1]]) == "minecraft:grass_block"),
        _ => false,
    }
}

/// Caught in the library's glass jar, it settles in a pasture (SPECS §4).
static IN_JAR: Catchable = Catchable {
    kind: KIND,
    model: "dung_beetles:dung_beetle_jar",
    settles: Some(territory::Rule::Pasture),
    glows: false,
};

const BALL_FACT: &str = "ball";
const BURIED_FACT: &str = "buried";
const STARS_FACT: &str = "stars";
const CHICKEN_FACT: &str = "chicken";

/// Chickens peck it up (SPECS §2): one within 6 blocks walks to it.
static PREY: creature::prey::Prey = creature::prey::Prey {
    kind: KIND,
    hunters: &[creature::prey::Hunter {
        kind: "minecraft:chicken",
        range: 6.0,
        speed: 1.0,
    }],
    fact: Some(CHICKEN_FACT),
};

/// Its page (SPECS §6): the biomes met in, its facts.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |id| !territory::settled(id),
    facts: &[
        Fact {
            id: BALL_FACT,
            item: DUNG_BALL,
            key: "bestiary.dung_beetles.ball",
        },
        Fact {
            id: BURIED_FACT,
            item: "minecraft:grass_block",
            key: "bestiary.dung_beetles.buried",
        },
        Fact {
            id: STARS_FACT,
            item: "minecraft:nether_star",
            key: "bestiary.dung_beetles.stars",
        },
        Fact {
            id: CHICKEN_FACT,
            item: "minecraft:chicken_spawn_egg",
            key: "bestiary.dung_beetles.chicken",
        },
        Fact {
            id: creature::jar::JAR_FACT,
            item: creature::jar::ITEM,
            key: "bestiary.dung_beetles.jar",
        },
        Fact {
            id: creature::jar::PASTURE_FACT,
            item: "minecraft:wheat",
            key: "bestiary.dung_beetles.pasture",
        },
    ],
    icon: "dung_beetles:dung_beetle_spawn_egg",
    scheme: None,
    rarity: creature::bestiary::Rarity::Common,
    draw: |draw, _| {
        creature::model::draw_shown(
            draw,
            &model_dung_beetle::DUNG_BEETLE,
            TEXTURE,
            &|_| [0.0; 3],
            &|name| name != "ball_small",
            creature::model::WHITE,
        );
    },
    draw_scale: 1.0,
    draw_lift: 0.0,
};

/// Where it crawls about between its balls (SPECS §2): around where it is, or for a settled one over its pasture's
/// ground (the tops of the blocks around its anchor).
fn pasture(host: &dyn Host, id: u32) -> Vec<[i32; 3]> {
    let (Some(a), Some(ground)) = (territory::anchor(id), Ground::of_entity(host, id)) else {
        return Vec::new();
    };
    let range = territory::PASTURE_RANGE;
    let mut tops = Vec::new();
    for dx in (-range..=range).step_by(2) {
        for dz in (-range..=range).step_by(2) {
            let (x, z) = (a[0] + dx, a[2] + dz);
            if let Some(y) = (a[1] - 3..=a[1] + 2)
                .rev()
                .find(|&y| roll::buriable(&ground.block([x, y, z])) && ground.block([x, y + 1, z]) == "minecraft:air")
            {
                tops.push([x, y, z]);
            }
        }
    }
    tops
}

/// Its crawl (the library's creeping): short and slow, between its balls.
static CREEPING: creature::creep::Creeping = creature::creep::Creeping {
    speed: 0.02,
    climb: 0.0,
    crawl: (40, 120),
    pause: (100, 300),
    wander: 3.0,
    area: pasture,
    may: |m, _| !roll::busy(m.id),
    rustle: None,
};

/// How often a settled one checks its herd is still there (ticks).
const PASTURE_EVERY: i32 = 100;

/// From `entity_tick_post`, for each beetle: its behaviours (the library's tick of small creatures: territory,
/// despawning), chickens after it, what its behaviours queued, its pasture.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    creature::flutter::tick_post(host, dimension, KIND, id, || {
        vec![
            Box::new(roll::Roll::default()),
            Box::new(creature::creep::Creep::new(&CREEPING)),
        ]
    });
    if creature::prey::tick(host, dimension, id, KIND) {
        forget(id);
        return;
    }
    for action in roll::actions() {
        act(host, dimension, action);
    }
    let every = host.entity(id).and_then(|e| e.tick_count(&*host)).unwrap_or(0) % PASTURE_EVERY == 0;
    if every && territory::settled(id) {
        territory::check_pasture(&*host, id);
    }
}

/// What a buried ball or a used dung ball does to block `p` (SPECS §3): coarse or rooted dirt becomes dirt, dirt
/// becomes grass, on grass short grass and flowers grow around (as bone meal does). Whether it changed anything.
pub(crate) fn fertilize(host: &mut dyn ServerHost, level: LevelRef, p: [i32; 3]) -> bool {
    let name = Ground::new(&*host, level).block(p);
    let into = match name.as_str() {
        "minecraft:coarse_dirt" | "minecraft:rooted_dirt" => "minecraft:dirt",
        "minecraft:dirt" => "minecraft:grass_block",
        "minecraft:grass_block" => {
            return level.bone_meal(host, p);
        }
        _ => return false,
    };
    let Some(state) = host.block_state_id(into) else {
        return false;
    };
    let flags = mod_api::BlockUpdateFlags::NEIGHBORS | mod_api::BlockUpdateFlags::CLIENTS;
    if !level.set_block(host, p, state, flags).unwrap_or(false) {
        return false;
    }
    let top = [f64::from(p[0]) + 0.5, f64::from(p[1]) + 1.1, f64::from(p[2]) + 0.5];
    creature::particles::puff(host, level, "minecraft:happy_villager", top, 0.3, 8);
    true
}

/// What its behaviour queued (`roll::Action`).
fn act(host: &mut dyn ServerHost, dimension: &str, action: Action) {
    match action {
        Action::Shaped { id } => {
            let Some(at) = host.entity(id).and_then(|e| e.position(&*host)) else {
                return;
            };
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                creature::bestiary::learn(host, player, KIND, BALL_FACT);
            }
        }
        Action::Buried { id, block, stars } => {
            let Some(e) = host.entity(id) else {
                return;
            };
            let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
                return;
            };
            level.play_sound(
                host,
                "minecraft:block.rooted_dirt.place",
                at,
                SoundSource::Neutral,
                0.4,
                1.3,
                None,
            );
            let greener = fertilize(host, level, block);
            for player in creature::bestiary::players_near(&*host, dimension, at) {
                if greener {
                    creature::bestiary::learn(host, player, KIND, BURIED_FACT);
                }
                if stars {
                    creature::bestiary::learn(host, player, KIND, STARS_FACT);
                }
            }
        }
    }
}

fn forget(id: u32) {
    creature::flutter::gone(id);
    roll::forget(id);
}

/// Taking its ball (SPECS §2.1): a right-click with an empty hand on a beetle with a ball gives the nearest player a
/// dung ball.
fn take_ball(host: &mut dyn ServerHost, id: u32) {
    if !roll::take(id) {
        return;
    }
    let Some(at) = host.entity(id).and_then(|e| e.position(&*host)) else {
        return;
    };
    let nearest = host
        .players()
        .server_players()
        .into_iter()
        .filter_map(|p| Some((p, p.entity().position(&*host)?)))
        .min_by(|a, b| creature::body::distance(a.1, at).total_cmp(&creature::body::distance(b.1, at)));
    if let (Some((player, _)), Some(ball)) = (nearest, creature::hand::stack(&*host, DUNG_BALL, 1)) {
        creature::hand::give(host, player, ball);
    }
}

/// The dung ball (SPECS §5): on dirt, coarse or rooted dirt or grass it does what a buried ball does; in a composter it
/// always adds a level.
struct DungBall;

/// The block a click on `pos` reaches, through the clicking player's level.
fn clicked(host: &dyn Host, event: &UseOnEvent<'_>) -> Option<(LevelRef, String)> {
    let level = host.player(event.player)?.level(host)?;
    Some((level, Ground::new(host, level).block(event.pos)))
}

fn ours_to_use(name: &str) -> bool {
    matches!(
        name,
        "minecraft:coarse_dirt"
            | "minecraft:rooted_dirt"
            | "minecraft:dirt"
            | "minecraft:grass_block"
            | "minecraft:composter"
    )
}

#[mod_api::sunrise_part]
impl ItemPart for DungBall {
    fn use_on(&self, host: HostSide, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let host = match host {
            HostSide::Client(host) => {
                let (_, name) = clicked(&*host, event)?;
                return ours_to_use(&name).then_some(InteractionResult::Success);
            }
            HostSide::Server(host) => host,
        };
        let (level, name) = clicked(&*host, event)?;
        let used = if name == "minecraft:composter" {
            compost(host, level, event.pos)
        } else {
            fertilize(host, level, event.pos)
        };
        if !used {
            return None;
        }
        let player = host.player(event.player)?;
        if !player.has_infinite_materials(&*host).unwrap_or(false) {
            creature::hand::use_one(host, player, event.hand);
        }
        Some(InteractionResult::Success)
    }
}

/// A dung ball into the composter at `p`: one level more, always (the last one makes it ready at once). Whether it took
/// it (not a full one).
fn compost(host: &mut dyn ServerHost, level: LevelRef, p: [i32; 3]) -> bool {
    let Some(now) = level
        .block_property(&*host, p, "level")
        .and_then(|l| l.parse::<i32>().ok())
    else {
        return false;
    };
    if now >= 7 {
        return false;
    }
    let next = if now + 1 >= 7 { 8 } else { now + 1 };
    if !host.set_block_property(p, "level", &next.to_string()) {
        return false;
    }
    let at = [f64::from(p[0]) + 0.5, f64::from(p[1]) + 0.5, f64::from(p[2]) + 0.5];
    level.play_sound(
        host,
        "minecraft:block.composter.fill_success",
        at,
        SoundSource::Blocks,
        1.0,
        1.0,
        None,
    );
    true
}

/// How it holds itself this frame: walking its legs step; shaping or burying, its front legs dig; rolling, its hind
/// legs reach back onto the ball, which turns as it goes.
fn pose(name: &str, age: f32, walk: f32, speed: f32, ball: Option<Ball>) -> [f32; 3] {
    let step = (walk * 1.2).sin() * 25.0 * speed.min(1.0);
    let dig = (age * 0.8).sin() * 30.0;
    let digging = matches!(ball, Some(Ball::Shaping(_)) | Some(Ball::Burying(_)));
    match name {
        "ball" => [-walk * 60.0, 0.0, 0.0],
        "leg_front_left" if digging => [0.0, dig, 0.0],
        "leg_front_right" if digging => [0.0, -dig, 0.0],
        "leg_hind_left" | "leg_hind_right" if ball == Some(Ball::Rolling) => [0.0, 0.0, step * 0.5],
        "leg_front_left" | "leg_hind_left" | "leg_middle_right" => [0.0, step, 0.0],
        "leg_front_right" | "leg_hind_right" | "leg_middle_left" => [0.0, -step, 0.0],
        _ => [0.0; 3],
    }
}

/// Which ball it shows: the small one while it starts shaping, the big one once shaped and while rolling, none once
/// half buried or without one.
fn shown(name: &str, ball: Option<Ball>) -> bool {
    match name {
        "ball_small" => matches!(ball, Some(Ball::Shaping(f)) if f < 0.5),
        "ball" => match ball {
            Some(Ball::Shaping(f)) => f >= 0.5,
            Some(Ball::Rolling) => true,
            Some(Ball::Burying(f)) => f < 0.5,
            None => false,
        },
        _ => true,
    }
}

struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let (walk, speed) = mob.map_or((0.0, 0.0), |m| (m.walk_animation_pos, m.walk_animation_speed));
        let ball = roll::ball(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |p: &str| pose(p, entity.age, walk, speed, ball);
        let shown = |p: &str| shown(p, ball);
        creature::model::draw_shown(
            draw,
            &model_dung_beetle::DUNG_BEETLE,
            TEXTURE,
            &pose,
            &shown,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct DungBeetles;

#[mod_api::sunrise_mod]
impl Hooks for DungBeetles {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::flutter::register(KIND, SIZE);
        creature::flutter::spawns_on(&SPAWNING, spawns_on);
        creature::jar::register(&IN_JAR);
        creature::prey::register(&PREY);
        creature::bestiary::register(&BESTIARY);
        let ball = r.item_key(DUNG_BALL);
        r.item_part(ball, DungBall);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click: with an empty hand, its ball taken; nothing else (no trading screen, which its template would
    /// open), on both sides. The glass jar is the library's (its hook runs first).
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        let empty = event.stack.is_none_or(|s| s.count() == 0);
        if let HostSide::Server(host) = host
            && empty
            && let Ok(id) = u32::try_from(event.entity)
        {
            take_ball(host, id);
        }
        Some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == KIND {
                tick(host, event.dimension, event.entity);
            }
        });
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            forget(event.entity);
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_shows_one_ball_at_a_time() {
        for ball in [
            None,
            Some(Ball::Shaping(0.2)),
            Some(Ball::Shaping(0.8)),
            Some(Ball::Rolling),
            Some(Ball::Burying(0.7)),
        ] {
            let both = shown("ball", ball) && shown("ball_small", ball);
            assert!(!both);
        }
        assert!(shown("ball", Some(Ball::Rolling)));
        assert!(!shown("ball", None) && !shown("ball_small", None));
    }

    #[test]
    fn the_ball_turns_only_as_it_rolls() {
        assert_eq!(pose("ball", 0.0, 0.0, 0.0, None), [0.0; 3]);
        assert_ne!(pose("ball", 0.0, 1.0, 1.0, Some(Ball::Rolling)), [0.0; 3]);
    }
}
