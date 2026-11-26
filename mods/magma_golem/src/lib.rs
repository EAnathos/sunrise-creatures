//! Magma Golem (SPECS.md): a giant of living magma asleep in the Nether, a rare mini-boss of the Sunrise Creatures
//! series.
//!
//! A magma golem is a copy of the wandering trader (its behaviour is all goals) with its own 3 × 6 block box, drawn with
//! its own model (`model.rs`, from `sources/models/magma_golem.bbmodel`; `draw.rs`), immune to fire and lava. It sleeps
//! as a mound, wakes, fights under a boss bar and lies down again (`golem.rs`), lobs globs of magma (`glob.rs`),
//! appears in the Nether (`spawn.rs`), and drops its heart, which makes the molten pickaxe (`pickaxe.rs`).
// The library's settings are built with `..Type::DEFAULT` even when every field is given: a field the library adds
// later then takes its default (docs/CONVENTIONS.md, "Library structs").
#![allow(clippy::needless_update)]

mod draw;
mod glob;
mod golem;
mod model;
mod pickaxe;
mod spawn;

use std::sync::Arc;

use creature::bestiary::{Fact, Species};
use mod_api::{
    BlockDropsEvent, BlockEntityDrawContext, BreakBlockEvent, Compound, EntityInteractEvent, EntityJoinLevelEvent,
    EntityTickPostEvent, Flow, GoalSpec, HearsProjectileImpactsEvent, HitHost, Hooks, Host, HostSide,
    InteractionResult, LevelHost, LevelTickPostEvent, LivingDeathEvent, LivingIncomingDamageEvent, MobFilter,
    ProjectileImpactEvent, RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost, SoundSource, Tag,
};

pub const GOLEM: &str = "magma_golem:magma_golem";
const EGG: &str = "magma_golem:magma_golem_spawn_egg";
const HEART: &str = "magma_golem:magma_heart";
const TEXTURE: &str = "magma_golem:entity/magma_golem/magma_golem";

/// Its box (blocks: width, height, eye height), SPECS §1: 3 × 6, its eyes low in front, under its shoulders.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 3.0,
    height: 6.0,
    eye_height: 4.2,
};
/// Its health and pace (SPECS §2), as its content file sets them, and what its template (a trader) lacks: armour, no
/// knockback, its fist's blow.
const HEALTH: f64 = 250.0;
const SPEED: f64 = 0.3;
const ARMOR: f64 = 10.0;
const KNOCKBACK_RESISTANCE: f64 = 1.0;
const ATTACK_DAMAGE: f64 = 14.0;
/// How far it sees a player (blocks; its follow range).
const SIGHT: f64 = 24.0;
/// The experience it leaves (SPECS §2).
const EXPERIENCE: i32 = 50;
/// How often its boss bars are swept (ticks).
const SWEEP_EVERY: i64 = 100;

/// Where it is met (SPECS §2): the Nether's basalt deltas and wastes, by their lava seas.
const BIOMES: &[&str] = &["minecraft:basalt_deltas", "minecraft:nether_wastes"];

/// Its light (SPECS §2): the orange of its seams, dimmer asleep.
const LIGHT: creature::glow::Light = creature::glow::Light {
    rgb: 0xFF7A1E,
    radius: 12.0,
    intensity: 0.9,
    lift: 3.0,
    ..creature::glow::Light::DEFAULT
};
const LIGHT_ASLEEP: creature::glow::Light = creature::glow::Light {
    radius: 6.0,
    intensity: 0.5,
    lift: 1.5,
    ..LIGHT
};

/// Its facts in the guide (SPECS §4).
const HEART_FACT: &str = "heart";

static BESTIARY: Species = Species {
    id: GOLEM,
    biomes: BIOMES,
    wild: |_| true,
    facts: &[
        Fact {
            id: golem::AWAKE_FACT,
            item: "minecraft:magma_block",
            key: "bestiary.magma_golem.awake",
        },
        Fact {
            id: golem::CRUST_FACT,
            item: "minecraft:snowball",
            key: "bestiary.magma_golem.crust",
        },
        Fact {
            id: golem::LAVA_FACT,
            item: "minecraft:lava_bucket",
            key: "bestiary.magma_golem.lava_bath",
        },
        Fact {
            id: HEART_FACT,
            item: HEART,
            key: "bestiary.magma_golem.heart",
        },
    ],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::VeryRare,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.0,
    ..Species::DEFAULT
};

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static HEART_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: HEART,
    dropped_by: &[GOLEM],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:smithing_table",
        key: "bestiary.magma_golem.item.magma_heart.use",
    }],
    ..creature::items::ItemPage::DEFAULT
};
static PICKAXE_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: pickaxe::PICKAXE,
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:furnace",
        key: "bestiary.magma_golem.item.molten_pickaxe.use",
    }],
    ..creature::items::ItemPage::DEFAULT
};

/// Its drawing in a jar or the book: standing, its whole height in the view.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    creature::model::draw(draw, &model::MAGMA_GOLEM, TEXTURE, &|_| [0.0; 3]);
}
const BOOK_SCALE: f32 = 0.15;

/// Whether the world is on Peaceful: none appears, and those there go, as vanilla's monsters.
fn peaceful(host: &dyn Host, level: mod_api::LevelRef) -> bool {
    level.difficulty(host) == Some("peaceful")
}

#[derive(Default)]
struct MagmaGolem;

#[mod_api::sunrise_mod]
impl Hooks for MagmaGolem {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(GOLEM, SIZE);
        creature::mob::scale_client_box(GOLEM);
        creature::projectile::register(glob::GLOB, glob::SIZE);
        // Their attribute rows (a type only its content file declares has none to add to), with the values their
        // content files set, then what their template lacks.
        let _ = r.declare_entity_attributes(
            "magma_golem",
            "minecraft:wandering_trader",
            vec![
                ("minecraft:max_health".into(), HEALTH),
                ("minecraft:movement_speed".into(), SPEED),
            ],
        );
        let _ = r.declare_entity_attributes(
            "magma_glob",
            "minecraft:wandering_trader",
            vec![
                ("minecraft:max_health".into(), 1.0),
                ("minecraft:movement_speed".into(), 0.0),
            ],
        );
        let _ = r.modify_entity_attributes(
            GOLEM,
            vec![
                ("minecraft:armor".into(), Some(ARMOR)),
                ("minecraft:knockback_resistance".into(), Some(KNOCKBACK_RESISTANCE)),
                ("minecraft:attack_damage".into(), Some(ATTACK_DAMAGE)),
                ("minecraft:follow_range".into(), Some(SIGHT)),
            ],
        );
        let _ = r.modify_entity_attributes(glob::GLOB, vec![("minecraft:attack_damage".into(), Some(glob::DAMAGE))]);
        spawn::declare(r);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&HEART_PAGE);
        creature::bestiary::register_item(&PICKAXE_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(GOLEM.into(), Arc::new(draw::Draw));
        event.renderers.register(glob::GLOB.into(), Arc::new(glob::Draw));
    }

    /// A magma golem joining: its client box as tall as it is, immune to fire and lava (a grant of the live body, not
    /// saved: given at each join), vanilla's goals for its wandering and looking about, and its targets: whoever hurt
    /// it, the players it sees. It sleeps first (`golem::tick`).
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if event.kind != GOLEM {
            return Flow::Continue;
        }
        let Some(e) = host.entity(event.entity) else {
            return Flow::Continue;
        };
        if let Some(living) = host.living(e) {
            let scale = creature::mob::client_box_scale(GOLEM);
            living.set_attribute_base(host, "minecraft:scale", f64::from(scale));
        }
        e.set_fire_immunity(host, true);
        let Some(m) = host.mob(e) else {
            return Flow::Continue;
        };
        m.set_persistence_required(host);
        m.add_goal(
            host,
            5,
            &GoalSpec::Stroll {
                speed: 0.7,
                interval: 160,
                water_avoiding: Some(0.001),
                swim: false,
                flying: false,
            },
        );
        m.add_goal(
            host,
            6,
            &GoalSpec::LookAtPlayer {
                range: 16.0,
                probability: 0.04,
                only_horizontal: false,
                mobs: false,
                only: None,
            },
        );
        m.add_goal(host, 7, &GoalSpec::RandomLookAround);
        m.add_target_goal(host, 1, &GoalSpec::HurtByTarget { alert: false });
        m.add_target_goal(
            host,
            3,
            &GoalSpec::NearestTarget {
                prey: MobFilter::Player,
                must_see: true,
                must_reach: false,
                interval: 10,
            },
        );
        Flow::Continue
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == GOLEM || event.kind == glob::GLOB).then_some(InteractionResult::Consume)
    }

    /// Hurt, it wakes; crusted over, it takes double damage. Falls don't hurt it.
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if event.kind != GOLEM {
            return Flow::Continue;
        }
        // Its leaps don't hurt it.
        if event.source.damage_type == "minecraft:fall" {
            return Flow::Cancel;
        }
        let Ok(id) = u32::try_from(event.entity) else {
            return Flow::Continue;
        };
        golem::wake(id);
        if golem::crusted(id) {
            let doubled = event.container.new_damage() * 2.0;
            event.container.set_new_damage(doubled);
        }
        Flow::Continue
    }

    /// A block mined by a sleeping golem wakes it.
    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        if let Some(level) = host.players().acting_player().and_then(|p| p.level(&*host)) {
            golem::mined(&*host, level, event.pos);
        }
        Flow::Continue
    }

    /// Mined with the molten pickaxe, blocks drop smelted.
    fn block_drops(&self, _: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
        pickaxe::smelt(event);
        Flow::Continue
    }

    /// Snowballs and splash water bottles cool it (`projectile_impact`).
    fn hears_projectile_impacts(&self, _: &dyn Host, event: &mut HearsProjectileImpactsEvent) {
        event.hears = true;
    }

    fn projectile_impact(&self, host: &mut dyn ServerHost, event: &mut ProjectileImpactEvent<'_>) -> Flow {
        match event.kind {
            "minecraft:snowball" if event.entity_kind == GOLEM => {
                if let Some(id) = event.entity {
                    golem::snowball(host, id, event.projectile, event.owner);
                }
            }
            "minecraft:splash_potion" if golem::is_water(host, event.projectile) => {
                if let Some(level) = host.level(event.dimension) {
                    golem::splash(&*host, level, event.at);
                }
            }
            _ => {}
        }
        Flow::Continue
    }

    /// Its death: its boss bar goes, its light fades, the experience it leaves (its template leaves none), and its
    /// killer learns its heart.
    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind != GOLEM {
            return Flow::Continue;
        }
        golem::died(host, event.entity);
        creature::glow::dim(event.entity);
        let e = host.entity(event.entity);
        if let (Some(at), Some(dimension)) = (
            e.and_then(|e| e.position(&*host)),
            e.and_then(|e| e.level(&*host)).and_then(|l| l.dimension().id(&*host)),
        ) {
            spawn::forget(host, at);
            let mut orb = Compound::new();
            orb.insert("Value".into(), Tag::Short(EXPERIENCE as i16));
            host.summon(&dimension, "minecraft:experience_orb", [at[0], at[1] + 1.0, at[2]], orb);
        }
        if let Ok(killer) = u32::try_from(event.source.causing)
            && let Some(player) = creature::mob::player_of(&*host, killer)
        {
            creature::bestiary::learn(host, player, GOLEM, HEART_FACT);
        }
        Flow::Continue
    }

    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let level = event.level;
        pickaxe::prepare(&*host);
        if level.dimension() == mod_api::DimensionKey::NETHER {
            spawn::tick(host, level);
        }
        if level.dimension() == mod_api::DimensionKey::OVERWORLD
            && level.game_time(&*host).is_some_and(|t| t % SWEEP_EVERY == 0)
        {
            golem::sweep_bars(host);
        }
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == glob::GLOB {
                glob::tick(host, event.entity);
                return;
            }
            if event.kind != GOLEM {
                return;
            }
            let Some(e) = host.entity(event.entity) else {
                return;
            };
            if e.level(&*host).is_some_and(|l| peaceful(&*host, l)) {
                golem::died(host, event.entity);
                e.discard(host);
                return;
            }
            golem::tick(host, event.dimension, event.entity);
            let shown = golem::shown(event.entity);
            if shown.crusted {
                creature::glow::dim(event.entity);
            } else {
                let light = if shown.mode == golem::Mode::Dormant {
                    LIGHT_ASLEEP
                } else {
                    LIGHT
                };
                creature::glow::shine(&*host, event.dimension, event.entity, light);
            }
            creature::sounds::play_queued(host, event.entity, SoundSource::Hostile);
        });
    }
}
