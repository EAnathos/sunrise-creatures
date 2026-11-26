# Sunrise Creatures: library specification

The mandatory library of the Sunrise Creatures series ([docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)): the systems
several creature mods share, and the series' single creative tab. It adds no creature of its own.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Everything below works and was tested in game, single player, unless said otherwise.

- **The library itself**: one native package (`creature.smod`: its content and the crate `mod-creature`) that every mod
  reaches as the `creature` crate (`[dependencies] creature = "^0.3"`).
- **Model drawing** (§3.1), **mob sounds** (§3.2), **the creative tab** (§3.3), **monsters sparing our creatures**
  (§3.4), **the small fliers' flight and spawning** (§3.5, by day, at night or at any hour) and **friends** (§6.2, the
  Fairies', the seagulls' and the vulture's owner).
- **The big fliers' flight** (§3.14, `creature::flight`): the vulture's flight and soaring, moved here with the Seagulls;
  the Vulture runs on it (tested in game: soaring, gliding down, ridden flight), and the seagull.
- **The small crawlers' crawl** (§3.6, `creature::crawl`): the worm's ground crawl, now a library goal the next ground
  creatures share; Worms runs on it (tested in game, its behaviour unchanged). **Creeping** (§3.16, `creature::creep`):
  a crawl moved by hand that climbs, for the Ladybugs and the Phasmids, and the Dung Beetles' that doesn't.
- **Glass jar and territories** (§4): catching, releasing, settling on a flower patch or in a garden, the jar on
  display (spared by blasts and pistons), and the tree (the Phasmids) and pasture (the Dung Beetles) rules; a jar
  holding each catchable creature in the creative tab. The jar's absence from the creative tab and recipe when no jar mod is
  installed, and the catch/release/display/name/TNT/piston behavior, are confirmed. The **sand** rule (§4.2, for
  Scorpions) is coded but not yet exercised in game.
- **Colours and patterns** (§5), used by the butterflies.
- **Feathers** (§3.8): the feather recipes with a mod's feather (the vulture's, the seagull's).
- **Glowing creatures' lights** (§3.11, `creature::glow`): the fireflies', the fairies' and the storm spirits' lights,
  sharing the game's 8.
- **Fighting back** (§3.12, `creature::anger`): the scorpions and the storm spirits.
- **Water creatures** (§3.10, `creature::aquatic`), **touches** (§3.13, `creature::touch`) and **hunters** (§3.7,
  `creature::prey`, the sea turtles, the dolphins and the seagulls; chickens: the ladybugs and the dung beetles): the jellyfish and
  the flying fish (and the storm spirits' touch). How much water is over a point (`aquatic::water_over`): the
  jellyfish's and the flying fish's depth, the seagull's surface.
- **Food on the ground** (§3.15, `creature::food`): the vulture's meat, the seagull's fish and stolen food.
- **Mob helpers** (`creature::mob`): `ours` (muted, own box, own goals, spared), `resize`, `Muted`, `despawn_far` (small
  wild creatures far from players, as vanilla's ambient mobs), `nearest_player`, `player_of`, `give_effect` (an effect
  on a mob or a player; `give_effect_level` at a level above the first: the ladybugs' Luck II), `is_boat` (a boat or a raft: the jellyfish, the flying fish, the seagull). Particles:
  `particles::puff` (a puff of particles around a point); friends: `friend::sign` (hearts or smoke after a try to
  befriend: the fairies, the seagull).
- **Body helpers** (`creature::body`): `distance`, `flat_distance` (across, heights left out: the seagull, the dung
  beetle), `turned` and `unit` (a heading turned, made a unit: the flying fish, the dung beetle), `Rand::between` (a
  number of ticks in a range).
- **The item in hand** (`creature::hand`): items whose right-click only the server's hooks handle (the jar, the guide,
  the mods' throwables) skip the off hand's turn, `use_one` uses one up, `stack` makes a stack of an item by its id.
- **Creature guide book and bestiary** (§7).

Limits: single player only (the drawing reads render state from statics, kept on the server's side); the systems of §6
stay in the vulture until a second mod needs them, except friends (§6.2) and the flight (§6.1).

## 1. Role and rules ✅

- Every mod of the series **requires** the library; the library depends on no mod.
- A system belongs here when **two or more mods use it or are specified to use it** (the rule of two), or when it ties
  the series together (the tab). Otherwise it stays in its mod until a second mod needs it.
- The library holds **mechanisms**, the mods hold **species**: no entity type in `creature:`. A mod gives the library its
  species and numbers (registration at start, or parameters).
- A library feature that only makes sense with some mods is **switched on by them** (registration, conditional recipe),
  e.g. the glass jar (§4). The same goes for optional content between mods: a recipe that uses another mod's item when
  it is installed and a vanilla item otherwise is two conditional recipes, never a requirement.
- A system moved here keeps working for the mod it came from, in the same commit.

## 2. Technical context

- **Mod id / namespace**: `creature`. Name: "Sunrise Creatures".
- **Its code**: the crate `mod-creature` (`#[mod_api::sunrise_mod]` on its `Hooks`), which the mods reach as the
  `creature` crate. The game runs its hooks before theirs (dependency order), so a mod registers its species from its
  own `register` hook. It publishes functions, types and statics; shared state is behind `Mutex`es.
- **Its content**: the series' tab, shared blocks and items (`creature:glass_jar`, `creature:guide`, the jar blocks), their recipes
  (conditional when a feature depends on a mod), textures and translations (`en_us`, `fr_fr`).
- **Pinned** to the series' Sunrise revision (docs/SUNRISE-ISSUES.md). Version `0.3.4`; a breaking API change bumps
  the minor version while in `0.x`, and the mods' `[dependencies] creature` with it.

## 3. Systems in place

### 3.1 Model drawing (`creature::model`) ✅

- `Part` / `Cube`: a Blockbench model as data. `cargo creature model <mod> [model]` converts
  `mods/<mod>/assets/models/<model>.bbmodel` into a `pub static <MODEL>: Part` in the mod's crate.
- `draw(host, draw, part, texture, pose)`: draws the tree with the mod's texture; `pose(group)` gives each group's
  animation for the frame. Called from the mod's `mob_submit` hook, which then cancels the template's model.
- Users: every mob of the series.

### 3.2 Mob sounds (`creature::sounds`) ✅

- Our mobs copy a vanilla mob whose sounds they must not play, and the game prefixes a mob's sounds with `minecraft:`. The mod
  mutes its mob and the library plays the mod's own sounds with `Host::play_sound`.
- `queue(mob, sound, volume, pitch)` from goals; `tick_post(host, event, voice, source)` from the mod's
  `entity_tick_post`: plays the queue, and the `Voice`'s hurt and death sounds by watching the mob's health.
  `play_queued(host, mob, source)` plays only the queue (a mob without a `Voice`).
- Users: every mob of the series.

### 3.3 Creative tab ✅

- One tab, "Sunrise Creatures" (`itemGroup.creature`), generated from the modules' `module.toml`
  (docs/ARCHITECTURE.md §4). Order ✅: every spawn egg first, then the library's items, then each mod's other items;
  mods in alphabetical order, each one's items in its own order.
- Icon ✅: the creature guide book (`creature:guide`, §7).
- The glass jar is **hidden when no mod uses it** ✅ (none of the eight jar mods of §4 installed): not in the tab,
  not craftable. A tab file can't be conditional, so the library's native half takes it out of the tab
  (`build_creative_mode_tab_contents`). With a jar mod, a **jar holding each creature it catches** ✅ follows the empty
  jar in the tab (in its usual look), so a creative player can display them or let them out.

### 3.4 Monsters leave our creatures alone (`creature::mob`) ✅

- Our mobs copy the wandering trader, which zombies, husks and illagers hunt. The library's `living_change_target`
  cancels a mob's change of target onto one of our creatures (and the blow its tick dealt with it): `mob::spared`
  (tested with zombies).
- `mob::ours(mob, width, height, eye)` from the mod's `register_goals`: muted, its own box, its template's goals cleared,
  and spared. Users: every mob of the series.
- `mob::resize(mob, width, height, eye)`: a mod mob's size other than its template's (`ours` calls it), from tiny
  (butterflies, worms) to big (the cyclops).

### 3.5 Small fliers (`creature::flutter`) ✅

The flight of butterflies, moths, fireflies and fairies: short erratic flights and landing (on flowers), resting, out
by day, by night or at any hour but in the rain. A mod gives a `Kind` (its `Hours`, speed, height, flight and rest lengths, drawn to light, landing on
flowers, an optional take-off sound played at each take-off) and calls `register` (size, muted, the flight goal) and
`tick_post` (territory, despawning far from players); what the library keeps of one is dropped when it leaves the
level (`creature::state`). `steer` flies one toward a point (a fairy following its friend).

**Spawning** ✅ (`spawns`): the game's natural spawning adds creatures only every 400 ticks, under a cap the farm animals
fill, so small fliers would only appear with new chunks and be gone once their players left. A mod registers a
`Spawning` (kind, biomes, group size, cap near a player, chance); every 10 s, for each player, one chance in its
`chance` while fewer than its `cap` are within 48 blocks, a group comes out 16 to 40 blocks away over the ground, in one
of its biomes, at its hours. Wild ones far from players despawn, so they come and go as bats do. Users: Butterflies
(butterflies, moths), Fireflies, Fairies, and the **Phasmids** ✅, whose groups are placed **on the top of leaves**
instead of over the ground (`spawns_on_leaves`, mods/phasmids/SPECS.md §3), and the **Dung Beetles** ✅, on grass, dirt
or coarse dirt, or sand next to grass (`spawns_on`, the blocks a mod's function accepts, mods/dung_beetles/SPECS.md §2),
and the **Sporelins** ✅, on their soil in the dark (mods/sporelins/SPECS.md §2); **Ladybugs** ✅ (their short flights
and spawning, mods/ladybugs/SPECS.md §3). A kind of given blocks comes out on the highest of them in the column with
air over it, under leaves too: a sporelin on the soil under a dark forest's canopy, a phasmid on the canopy's top.

### 3.6 Small crawlers (`creature::crawl`) ✅

The slow crawl on the ground of worms, scorpions and the other ground creatures: the worm's `Crawl` goal moved here as
`creature::crawl::Crawl`, driven by a `Cfg` (speed, how long a crawl and a pause last, how far a wild one wanders, how
its settled area is found, a pause hook for the worm's dig). A wild one crawls around where it first crawled; a settled
one crawls only over its area (its mod's territory rule). Users: **Worms** (in place), **Scorpions**
(mods/scorpions/SPECS.md §3); the crawl of the **Phasmids**, the **Ladybugs** and the **Dung Beetles** is creeping
(§3.16).

### 3.7 Hunters eat small creatures (`creature::prey`) ✅

A hunter that sees one of its prey within its range goes over and eats it: the creature is gone (no drop), wild or
settled. A mod registers its kinds as prey of a hunter, a vanilla mob or **another mod's** (by its entity id): a hunter
whose mod is not installed is skipped, so neither mod requires the other. Not on creatures in a jar, on display or in a
bucket. A mod can also add its creature as a hunter of another mod's prey (`add_hunter`: the seagull, of the flying
fish) ✅; the prey's fact and its mod's `when_caught` stay for its own hunters. A hunter of the series' mods is not
walked to its prey: it goes for it itself (its flight), asking which one is its own (`quarry`) ✅.

- **Chickens** peck small creatures within 6 blocks, so a player keeps chickens out of a garden or a pasture. Users:
  **Worms**, **Dung Beetles** ✅ (mods/dung_beetles/SPECS.md §2), **Ladybugs** ✅ (mods/ladybugs/SPECS.md §3).
- **Sea turtles** swim to jellyfish within 8 blocks and eat them ✅ (the hunter sent with `MobRef::move_to`). User:
  **Jellyfish** (mods/jellyfish/SPECS.md §3).
- **Dolphins** chase flying fish within 12 blocks and eat them ✅; the fish flee by gliding. User: **Flying Fish**
  (mods/flying_fish/SPECS.md §3).
- After a catch, the prey's mod may do something to the hunter (`prey::when_caught`, given its id) ✅: the flying fish's
  dolphin is fed, as by a player.
- **Seagulls** (a mod's hunter, `add_hunter`) swoop on gliding flying fish within 16 blocks and snatch them in the air,
  when both mods are installed ✅. Users: **Flying Fish** and **Seagulls** (mods/seagulls/SPECS.md §2).

### 3.8 Feathers (`creature:feathers`) ✅

A mod's own feather works as a feather: the item tag `creature:feathers` lists vanilla's feather and each mod's (as
`required: false` entries, so a missing mod's is left out), and the guide's recipe (§7) takes any of it ✅. The
recipes for arrows, the brush and the book and quill with a mod's feather are the library's ✅ (moved from the Vulture
with the Seagulls): they take the tag `creature:mod_feathers` (the vulture's and the seagull's feathers, not vanilla's,
whose recipes are vanilla's), with conditions on either mod being installed. Users: **Vulture**, **Seagulls**.

### 3.9 Thrown things (`creature::projectile`) ✅

A projectile is a mob of its mod (a copy of the wandering trader that can't be hurt, its own box and drawing),
launched with a velocity and flown with its own gravity until it reaches its target, lands, hits a wall or flies too
long; the mod reads how it ended and does what its projectile does there. One with no target finds what its box meets
on its own (`hits`). Users: **Cyclops** (its boulder, `cyclops:boulder`, hurled at a target on an arc, which tears up
the blocks where it lands) and **Scorpions** (the thrown scorpion dagger, mods/scorpions/SPECS.md §5); the **Magma
Golem** (planned) for its magma glob (mods/magma_golem/SPECS.md §2).

### 3.10 Water creatures (`creature::aquatic`) ✅

The life in water of jellyfish and flying fish, which copy the land-living wandering trader:

- **Breathing**: they never drown (the library's `living_breathe`); they lose the template's Float goal (`mob`), which
  would carry them to the surface.
- **In water** they float (no gravity), their mod's behaviours moving them. **Beached**, they fall, flop now and then
  and dry out after the mod's delay (a point of `minecraft:dry_out` damage a second), dropping as usual.
- **The bucket**: a water bucket used on one catches it into the mod's bucket item (`Bucket`, its look and name kept as
  in the glass jar's item data), which lets it out in water on the block face clicked and gives the empty bucket back (a bucket with no creature in it,
  from the creative tab, lets out one of its mod's kind);
  one let out is tagged `creature.kept` and never despawns (`mob::despawn_far`).
- **Spawning** (`spawns`): groups under the surface of their biomes' water, out of sight of the players, under a cap.

A mod gives its kind, its bucket item and its drying delay (`Aquatic`), and how it spawns (`Spawning`: a kind with looks
spawns in groups of one look, rolled once for the group's biome). Users:
**Jellyfish** (mods/jellyfish/SPECS.md §3, §5) and **Flying Fish** (mods/flying_fish/SPECS.md §3, §5).

### 3.11 Lights of glowing creatures (`creature::glow`) ✅

A soft coloured light around a glowing creature, seen only (it lights nothing in the world). The game draws at most 8
such lights from every mod together, within 32 blocks of the player: the library hands them out, to the creatures
nearest a player. A mod gives each creature's light (colour, reach, strength, height, whether it shines now) from its
server tick. Users: **Fireflies** (the lit lanterns), **Fairies** (their night light, wider around one following its
friend), **Storm Spirits** and **Jellyfish** (the glow jelly).

### 3.12 Fighting back (`creature::anger`) ✅

A creature of a kind that fights back, hit, takes whoever hit it as its target (the library's damage hook notes them),
until it lets them go: dead, gone from its level, a creative player or out of the mod's reach. The mod's own behaviour
does the fighting, from `pursued`. Users: **Scorpions** (a pinch or a sting) and **Storm Spirits** (a dash and a
shock).

### 3.13 Touches (`creature::touch`) ✅

What touches a creature is hurt by it, at most once every so many ticks each while it stays in touch (`due`), the mod
doing its own harm: **Storm Spirits** (a zap) and **Jellyfish** (its kind's sting).

### 3.14 Big fliers (`creature::flight`) ✅

The flight of the vulture and the seagull, moved from the vulture with the Seagulls (§6.1). `Flight` is flown like a
plane: always along its heading, turning toward the wanted one (banking), nosing up or down toward the wanted pitch,
trading speed for height like a glider, hovering on beating wings for a strike or a snatch (`hover`); it is steered each
tick by `Controls` (a rider's keys, or the creature's own behaviour). What differs between fliers is their `Wings`
(cruising and diving speeds, wing-beat and take-off sounds, their own pace). `Follow` follows its friend (the
library's friends, §6.2) as a `Following` says: flying to them past some distance, landing near them (on the water for
one that floats), coming to them when far, when the mod lets it. `Soar` is soaring on its own, as a
`Soaring` says: a take-off now and then, a climb (given up after 10 s held down, under leaves say), circling (its
circle drifting along a random way for a roaming flier, the seagull: `Soar::roaming`; toward water for one that
floats), then a glide down onto a high perch near it, one of those within a block of the highest at random
(`landing_spot`, its `perch` rule; over a flat sea, any way); one that floats lands only on the water or within 4
blocks of it when there is water around, so a seagull keeps to the coast.
Riding stays the vulture's. Users: **Vulture**, **Seagulls**.

### 3.15 Food on the ground (`creature::food`) ✅

A creature's tick looks for the nearest item it wants lying near it (`nearest`: its entity, where it is, its id and
who threw it, for a thrown food that may befriend), its behaviour goes to it, and its tick eats one of the stack
(`take_one`). Users: **Vulture** (raw meat, a thrown raw chicken may tame it), **Seagulls** (fish and food, a thrown
raw fish may befriend it).

### 3.16 Creeping (`creature::creep`) ✅

A small creature crawling by hand over the tops of blocks and up their sides, as a beetle or a stick insect does. Unlike
the small crawlers' goal (§3.6, walked by the engine's path finding), it moves itself, so it climbs, and it runs among a
creature's behaviours (a small flier's crawl between its flights). A mod gives a `Creeping` (speeds on a top and up a
side, crawl and pause lengths, how far around it creeps with no area, its area of block tops, when it may, a sound it now and then makes as it moves) and adds
`Creep::new(&HOW)` to its behaviours. A crawl picks a top of its area (or a point around it), creeps straight toward it,
and climbs while its target is above: up the block in its way, or, under its target, up any side next to it (a trunk),
through leaves as an insect goes through foliage; a solid ceiling ends the crawl. While it pauses it claims nothing.
A creature that doesn't climb (a `climb` of 0) gives up at a wall. `too_tall` says a block is taller than a step (a
fence, a wall, a gate): the dung beetle's roll and the sporelin's run turn aside at it.

A **tree climber** (`Creep::climber`, the Phasmids) holds on to the bark it is on (a log, leaves), with nothing solid
under its feet, when it pauses or keeps still, its belly to the bark and its middle 0.3 block from it, instead of dropping; under a target more than a block and
a half above it with nothing to climb beside it, it first goes to the nearest log within 4 blocks. While it clings its
renderer draws it upright, head up (`creep::clinging`); broken bark lets it fall. Users: **Ladybugs** (between
flowers), **Phasmids** (over the canopy and up the trunks, clinging), **Dung Beetles** (about their pasture, no
climbing).

## 4. Glass jar and territories ✅

Catching small creatures and settling them in a place they won't leave. Users, the eight current jar mods: **Butterflies**
✅, **Worms** ✅, **Fireflies** ✅, **Fairies** ✅, **Ladybugs** ✅, **Phasmids** ✅ (the tree rule), **Scorpions** ✅ (the
sand rule), **Dung Beetles** ✅ (the pasture rule). The jar is the library's ✅ and is **active only if one of them is
installed** ✅ (hidden otherwise, §3.3).

### 4.1 The jar

- Item `creature:glass_jar`. Recipe ✅ conditional (`neoforge:conditions`, `mod_loaded` butterflies **or** worms **or**
  fireflies **or** fairies **or** ladybugs **or** phasmids **or** dung_beetles **or** scorpions): 3 glass in a U + 1 oak
  slab lid (gives 2 ✅).
- **Catch** ✅: right-click a catchable creature with an empty jar → a **filled jar** holding it (its colours, pattern and
  name are kept).
- **One jar for every creature** ✅: a filled jar is the same `creature:glass_jar`, carrying the creature it holds as
  item data (`minecraft:custom_data`, its `creature` compound): its kind, colours, pattern and name. No mod
  has a jar item of its own. Its name says what is inside ("Glass Jar of Butterfly" ✅), and its icon shows the
  creature behind the glass: the mod registers the icon of a jar holding it ✅. Filled jars don't stack ✅.
- Catching and releasing are the library's. A mod registers its catchable kinds:

  ```rust
  creature::jar::register(Catchable { kind: "worms:worm", icon: "worms:item/worm_in_jar", territory: Some(GARDEN) });
  ```

- **Release** ✅: right-click a block with a filled jar → the creature comes out there and the jar is empty again. If the
  creature has a territory rule (§4.2) it **settles** there. A jar whose creature's mod is no longer installed releases
  nothing and stays full.
- **Display** ✅: sneak + right-click places the filled jar as a small block (a creature visibly inside, glowing faintly
  for a firefly or a fairy), broken back into the item. Decorative.
- **Befriended and named creatures** ✅: a befriended one (§6.2) is only caught by its friend, and the jar keeps its
  friend ("Befriended" under its name): let out, it is still theirs. A named one keeps its name ("Name: ..." under the
  jar's), restored when released.

### 4.2 Territories ✅

A settled creature **stays in an area** around one or several blocks, and never despawns ✅. The area is computed from
the block it was released on, by the mod's rule:

| Rule | User | Area |
|---|---|---|
| **Garden** ✅ | Worms | The farmland connected to the release block (4 directions, same level ±1); **at least 10 blocks** ✅, otherwise the worm won't settle |
| **Flower patch** ✅ | Butterflies, Fireflies ✅ | Released on a flower (its anchor): the flowers **within 8 blocks** of the anchor, plus 2 blocks around them and 4 blocks up. The 8-block cap keeps it to at most ~20 × 20 blocks, even in a meadow or a flower forest where every flower touches the next; in a sparse garden it is just the flower beds |
| **Tree** ✅ | Phasmids | Released on leaves or a log: the leaves and logs connected to it within 8 blocks (a trunk counts straight up); **at least 10 leaves** ✅, otherwise it won't settle (mods/phasmids/SPECS.md §5) |
| **Sand** ✅ (rule in place, its mod to come) | Scorpions | Released on sand, red sand, sandstone or terracotta: those blocks connected to it within 8 blocks; **at least 10** ✅, otherwise it won't settle (mods/scorpions/SPECS.md §6) |
| **Pasture** ✅ | Dung Beetles | Released with **at least 3 grazing animals** (cows, mooshrooms, sheep, horses, donkeys, mules, goats, camels, llamas: `territory::GRAZERS`) **within 8 blocks** of where it comes out: 8 blocks around that block; fewer than 3 animals left around and the creature is unsettled (mods/dung_beetles/SPECS.md §4) |

- The creature remembers its anchor (saved on the entity); every few seconds ✅ it checks the area still holds (the
  garden is still ≥ 10 farmland, the flowers are still there). If not, it becomes unsettled (wanders, may despawn ✅).

## 5. Colours and patterns ✅

One species with **several colours and patterns** ✅. Users: **Butterflies**, the **Flying Fish** and the **Jellyfish**
(their kinds as patterns with textures of their own), the **Ladybugs** (their kinds as patterns, drawn by the mod: a
tinted shell and its spots, mods/ladybugs/SPECS.md §1).

- A creature rolls a **base colour**, a **pattern** and a **pattern colour** at spawn (from lists the mod gives, each
  weighted, per biome ✅). Saved on the entity, kept in jars.
- Drawn as two layers: a base texture tinted by the base colour, then the pattern texture tinted by the pattern colour
  (`draw_cube` takes a colour), like vanilla's tropical fish. So 8 colours × 6 patterns × 8 colours need only 7
  textures.

## 6. Systems that could move from the vulture

Each moves here when a second mod needs it. Until then it stays in the vulture.

| # | System | Today |
|---|---|---|
| 6.1 | **Riding and flight**: mount by right-click, **dynamic ridden flight** (steered by the look, banking into arc turns like a plane, vulture/SPECS.md §6.2), dismount, mount hearts (vanilla's), a saddle | the flight itself (glide, soaring, landing) moved here with the Seagulls ✅ (`creature::flight`, §3.14); riding and the wing poses stay in the mods |
| 6.2 | **Taming and ownership**: owner on the entity, sit / follow, defend the owner, healing food, only the owner rides | following in flight ✅ in the library (`creature::flight::Follow`, §3.14: the vulture and the seagull); **friends** ✅ in the library (`creature::friend`: the friend's UUID and staying, saved on the creature as `creature:friend`, kept in the glass jar), used by the **Fairies**, the **Seagulls** (befriended with a fish), the **Sporelins** (with a mushroom; following on
foot, which stays theirs while no other walker follows) and the **Vulture** (its owner and sitting; a save from before, which held them in `vulture:state`, is moved over once, tested in game); defending and riding come with the Living Flowers (planned) |
| 6.3 | **Babies and growth**: smaller model, growth time, faster when fed | in the vulture |
| 6.4 | **Ridden special attack**: a key that makes the mount attack (the vulture's talons) | in the vulture |

## 7. Creature guide book ✅

A book about the series' creatures, given to every player.

- **Item** `creature:guide` ("Creature Guide" ✅), unstackable, with **its own texture** ✅ (a worn leather field guide,
  a creature's footprint on the cover ✅).
- **Given at a player's first spawn** in a world ✅ (once per player, remembered on the player; `native_player_logged_in`
  ✅). Craftable again ✅ to replace a lost one: a book, a feather and leather (shapeless).
- **The bestiary** ✅: right-click opens it. On the right, the species met as small
  icons; picking one opens its page on the left: the creature turning in 3D (as Create's Ponder), the biomes it was met
  in, what was found out (how it is tamed, what makes it breed, a jar catches it, it settles on a flower...) and, for
  a species with looks, the morphs met. Nothing about a species not met yet, not even how many there are.
- Each mod registers its species and the facts a player can learn about them (`creature::bestiary::register`); the
  library draws the book. A page opens when a player first meets a wild one, and each fact when they find it out.
- **For Minecraft's own creatures** ✅ (the Vanilla Bestiary, mods/vanilla_bestiary/SPECS.md): a species with no model
  of ours draws its icon, large and turning (`bestiary::draw_icon`); a species' biomes may be a whole dimension
  (`@minecraft:the_nether`); a fact with no word is a drop, its item alone, on its own lines below the other facts under
  the heading "Drops", and the facts wrap onto as many lines as they need; `bestiary::players_near` gives the players
  who saw something happen. The right page lists the series' species, then, below a line, Minecraft's own
  (`minecraft:` ids), and the wheel scrolls it when they don't fit.
- **Operators' commands** ✅ (level 2), to check the pages: `/bestiary unlock <species>` (its id, or the part after the
  colon when one species has it: `cow`) writes down everything about it for the player who runs it: every biome it
  spawns in, every fact, every look its varieties can roll; `/bestiary unlock all` does it for every species;
  `/bestiary reset` forgets everything. The chat completes the species' ids.
- **Fixed** (was: after `/bestiary unlock all` the Creature Guide no longer opened): the book's data (every species,
  biome, fact and morph the player has met) is one custom payload, which overflowed the then payload limit (32767 bytes; Sunrise's is 4096 per string)
  once all 92 species were unlocked and was silently dropped, so the book never opened. It is now sent in chunks under
  the limit, split on species boundaries, and the client opens on the last chunk (`book.rs`, `send_chunked`). Tested in
  game: `/bestiary unlock all` then the guide opens, and `/bestiary reset` then it opens.

- **Morphs** ✅: the morphs are the looks of the library's colours and patterns (§5), each shown at the bottom of the
  page by a swatch and drawn by the species when picked. A **kind with a model of its own** is a pattern its species
  draws with that model: the Sporelins' red and brown caps ✅ (and the Phasmids' stick and leaf kinds, each a texture).
  Planned with the Living Flowers ✅: a kind shown by its item rather than a swatch (the poppy, the dandelion, the
  cornflower).

### 7.1 Tabs ✅

The book has **tabs**, as ribbons above its right page ✅ (the open one in the paper's colour): **Creatures** (the
bestiary above) and **Items**. The book opens on the tab last read (while the game runs).

- **Creatures**: the series' creatures, then, below a line, the vanilla ones when the Vanilla Bestiary is installed
  (mods/vanilla_bestiary/SPECS.md).
- **Items** (§7.2).
- **Scrolling** ✅: the wheel scrolls the list it is over, a row at a time, with a thin scrollbar when there is more than
  shown: the right page's icons, a species' morph swatches, and the left page's details (a species' biomes, facts and
  drops; an item's recipes and uses), so a long page (a butterfly met in many biomes) is read whole. A species with
  looks has a shorter 3D view, for its details to have room above its swatches.

### 7.2 The Items tab ✅

What each item of the series is for, so a player knows how to make it and what to do with it.

- **One page per item** of the series (spawn eggs left out) ✅: on the right page the items, as icons; on the left
  the one picked: the item turning in the 3D view, with its name; **how to get it**: its recipes on a 3 × 3 grid of
  item icons (crafting, smelting, smoking, campfire cooking, brewing; the stations drawn beside it, one grid for the
  same recipe in several stations, a slot that takes several items, a tag's, showing each in turn), what drops it (a
  creature's icon: "Dropped by a cyclops"), or what gives it (a befriended fairy's dust); **its uses**, a few lines
  each with an icon ("Held: the farm animals within 12 blocks follow you"); and what it is **used to make** (the
  results of the recipes it goes into: a vulture feather makes arrows, the brush, the book and quill, the guide).
- **Every item shown from the start** ✅ (unlike the creatures): the tab is a reference to plan a craft, so a player can
  look up the shepherd's crook before meeting a cyclops. Only the items of installed mods are listed.
- Each mod registers its items' pages (`creature::bestiary::register_item`, `creature::items::ItemPage`) ✅: the item,
  the creatures that drop it, what else gives it and its uses, as language keys with icons. The glass jar's page shows
  only with a mod that uses the jar.
- **The recipes are the modules' own files** ✅: `cargo creature tab` reads every module's `data/<id>/recipe/` into the
  library's `recipes.rs` (the series' tags expanded, Minecraft's drawn by an item of theirs), and `cargo creature check`
  fails when it is out of date. A page
  can't show a recipe the game doesn't have.
- Users: the library (the glass jar, the guide), Vulture (feather, raw and cooked meat), Butterflies (dust, confetti),
  Cyclops (eye, club, shepherd's crook), Fairies (fairy dust), Dung Beetles (dung ball).

## 8. Out of scope for v1

- Multiplayer (LAN, dedicated server): v2, for the whole series. Server logic is kept apart from rendering.
- A configuration file.
- Compatibility with mods outside the series.

## 9. Testing in game

Creative, a test world; `cargo creature install` first.

- The creative tab "Sunrise Creatures": every installed mod's items, spawn eggs first; the glass jar only with a jar mod, then a filled jar per catchable creature.
- `/bestiary unlock all`: every installed species' page, full; then open the Creature Guide (regression for the known
  bug in §7). `/bestiary reset` empties the book and it still opens. The wheel scrolls a long page (a butterfly's) and
  the icons; the Items tab lists every installed mod's items, each with its recipe.
- A new world: the creature guide given once; its bestiary pages turn in 3D, list the biomes met, the facts and the
  rarity; a craft of the guide with a vulture feather.
- Monsters (a zombie, a skeleton) ignore a butterfly, a worm, a vulture.
- The glass jar, confirmed: it is hidden when no jar mod is installed; catch, release, display and break it; names are
  kept; TNT and pistons leave placed jars alone; flower settling works. Scorpions' sand settling rule still needs an
  in-game check.
- The recipe book lists the series' recipes once one ingredient is held.
- Small fliers come out near a player within a minute or two, in their biomes and hours.

## 10. Open questions

- ~~Q12: The Items tab's pages: found little by little, or all shown?~~ All shown from the start (§7.2).

- ~~Q8: Filled jars: one item per creature mod, or one `creature:glass_jar` holding anything?~~ One jar for every
  creature (§4.1).
- ~~Q9: Can a filled jar be placed as a decorative block?~~ Yes, sneak + right-click (§4.1).
- ~~Q10: The jar when no mod uses it?~~ Hidden: not in the tab, not craftable (§3.3).
- ~~Q11: Can a lost guide be crafted again?~~ Yes: a book, a feather and leather (§7).
- ~~Q5: The tab's order?~~ Spawn eggs first, then the library's items, then each mod's, mods in alphabetical order (§3.3).
