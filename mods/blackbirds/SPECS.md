# Blackbirds: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): the Eurasian blackbird (*Turdus
merula*), a lively songbird of forests, meadows and gardens. It hops along the grass, tilts its head to listen for
wild earthworms, perches in trees and bushes, and fills dawn and dusk with a flute-like song.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the approved specification and blackbird model (male and female textures), no mod implementation yet.

- **The blackbird model** (`sources/models/blackbird.bbmodel`; [front](../../docs/models/blackbirds-blackbird-front.png)
  and [back](../../docs/models/blackbirds-blackbird-back.png) previews): a compact songbird with a plump chest, a
  slightly cocked tail, a slender beak (separate lower mandible `beak_lower` to open while singing), an orbital eye-ring
  around dark eyes, folded layered wings (`wing_left`, `wing_left_tip`, `wing_right`, `wing_right_tip`) and slender
  dark legs (`leg_left`, `leg_right`).
- **Textures**: two 64 × 64 textures sharing the model:
  - **Male** (`assets/blackbirds/textures/entity/blackbird/male.png`): jet-black plumage with subtle slate-blue feather
    shading, bright orange-yellow beak and eye-ring.
  - **Female** (`assets/blackbirds/textures/entity/blackbird/female.png`): warm dark-brown plumage with a paler
    speckled breast, dull horn-brown beak and subtle pale eye-ring.

Next: implement Blackbirds on Sunrise (including CC0 song recordings in `sources/sounds.toml`).

## 1. Technical context

- **Mod id**: `blackbirds` ✅. Entity `blackbirds:blackbird` ✅, spawn egg `blackbirds:blackbird_spawn_egg` ✅.
- **Items**: `blackbirds:feather` ✅, `blackbirds:raw_blackbird` ✅, `blackbirds:cooked_blackbird` ✅.
- **Dependency**: the series' library `creature` ([common/SPECS.md](../../common/SPECS.md)). No hard dependency on
  Worms (`mods/worms/SPECS.md`): earthworm hunting is optional via `creature::prey::add_hunter` (`common/SPECS.md` §3.7).
- **Rule of two** (`docs/ARCHITECTURE.md` §3):
  - **Small-bird hopping and perching**: Blackbirds (like [Toucan](../toucan/SPECS.md) §2) hops on the ground and
    branches, makes short fluttering flights between bushes and trees, lands on leaves and flees from nearby players.
    When both are implemented, the shared perching and short-flight movement either uses or extends `creature::flutter`
    (`common/SPECS.md` §3.5) in the library rather than duplicating it.
  - **Feathers**: `blackbirds:feather` registers into `creature:feathers` and `creature:mod_feathers` (`common/SPECS.md`
    §3.8) so it crafts arrows, brushes, books and quills, and the creature guide.
  - **Wild-only prey** (`creature::prey`): hunting wild earthworms while sparing settled garden worms uses a `wild_only`
    option on `creature::prey::Hunter` (default `false` in `Hunter::DEFAULT`) or checks `creature::territory` before
    targeting.
  - **Thrown food and friends**: befriending uses `creature::food` and `creature::friend` (`common/SPECS.md` §3.15,
    §6.2).
- **Languages**: source text in English; in-game text in English and French (`en_us.json`, `fr_fr.json`).
- **Model**: Blockbench free-format cube model, facing `+z`, converted with
  `cargo creature model blackbirds blackbird` into `src/model_blackbird.rs`.

## 2. The blackbird ✅

- **Appearance** ✅: two natural looks rolled 50/50 at spawn as morphs (`common/SPECS.md` §5, §7):
  - **Male** (`blackbirds:entity/blackbird/male`): jet-black plumage with subtle slate-blue feather shading, bright
    orange-yellow beak, yellow-orange orbital eye-ring, and dark brown-grey legs.
  - **Female** (`blackbirds:entity/blackbird/female`): warm dark-brown plumage, paler speckled breast, dull horn-brown
    beak and subtle pale eye-ring.
- **Size** ✅: hitbox `0.4 × 0.5` blocks; at 16 texels per block the visible model is about 0.33 blocks wide (wings
  folded), 0.61 blocks tall and 1.0 block long from beak tip to tail tip.
- **Health** ✅: 6 HP (3 hearts).
- **Spawning** ✅: common in forest, birch forest, flower forest, plains, meadow, taiga and cherry grove biomes, alone
  or in pairs, on grass or among tree leaves. Wild ones despawn far from players (`creature::mob::despawn_far`).
- **Movement and fleeing** ✅: peaceful; never attacks. On the ground it hops briskly on grass, dirt, moss, podzol and
  farmland, pausing now and then with its head cocked slightly to one side to listen. Between trees and bushes it makes
  short fluttering flights, lands on leaves, logs and fences, and flicks its tail. It flees from sprinting or close
  players (within 5 blocks) and hostile mobs, taking off with an alarm chatter.
- **Dawn and dusk song** ✅: its signature behaviour. Around sunrise, around sunset, and for a short while after rain,
  perched blackbirds sing a melodic fluting song (CC0 freesound recordings in `sources/sounds.toml`), opening `beak_lower`
  as phrases play (with subtle note particles). At night they roost quietly in leaves.
- **Foraging and earthworms** ✅:
  - **Berries**: pecks at fallen `minecraft:sweet_berries` via `creature::food`, and hops into sweet berry bushes
    without taking thorn damage.
  - **Worms** (when [Worms](../worms/SPECS.md) is installed): hunts **wild** earthworms (`worms:worm`) on grass and
    dirt within 8 blocks (`creature::prey::add_hunter`, `common/SPECS.md` §3.7), **sparing settled garden worms** so it
    never harms a player's garden.
- **Predators** ✅: vanilla cats and foxes stalk and hunt wild blackbirds (`creature::prey`).

## 3. Befriending ✅

- Drop `minecraft:sweet_berries` near a wild blackbird (`creature::food`): it hops over and eats; each berry has a 1 in
  4 chance to befriend the player who threw it (`creature::friend`, `common/SPECS.md` §6.2).
- A befriended blackbird stays or follows on right-click, flies along with its friend or perches on nearby leaves and
  fences, sings more readily near them, keeps its name with a name tag, and never despawns. It cannot be ridden.

## 4. Drops and items ✅

- **Feather** (`blackbirds:feather`): drops 0 to 2 feathers (increased by Looting); works in `creature:feathers` and
  `creature:mod_feathers` (`common/SPECS.md` §3.8).
- **Meat** (`blackbirds:raw_blackbird`, `blackbirds:cooked_blackbird`): drops 1 raw blackbird (2 hunger with a chance
  of Hunger), replaced by cooked blackbird (6 hunger) if it burns or is killed with a smelting weapon, increased by
  Looting, matching Vulture, Seagulls, Toucan and Mice.

## 5. Bestiary ✅

Uses `creature::bestiary` (`common/SPECS.md` §7):

- Records the biomes where a wild blackbird was met, its rarity (`common`), and both male and female morphs.
- **Items tab**: lists `blackbirds:feather`, `blackbirds:raw_blackbird` and `blackbirds:cooked_blackbird`.
- **Facts**:
  - `Song`: heard a blackbird sing at dawn or dusk.
  - `Worm`: saw a blackbird catch a wild earthworm (when Worms is installed).
  - `Befriend`: befriended a blackbird with sweet berries.

## 6. Out of scope for v1

- Breeding, nests and eggs.
- Catching in `creature:glass_jar` (birds in the series are not jarred).
- Riding.
- Other thrush species (song thrush, mistle thrush).

## 7. In-game validation

Once implemented, test in a survival world with `creature` and `blackbirds` enabled:

- Find blackbirds (50/50 male and female) spawning alone or in pairs in forest, birch forest, flower forest, plains,
  meadow, taiga and cherry grove biomes.
- Observe hopping, head-tilting pauses, tail flicking, short flights between trees/bushes, landing on leaves, and fleeing
  from sprinting players.
- Wait for dawn, dusk and rain clearing: verify melodic song playback, `beak_lower` animation and quiet roosting at
  night.
- Test with Worms enabled: verify blackbirds catch wild earthworms and spare settled garden worms; test with Worms
  disabled to confirm no hard dependency.
- Confirm vanilla cats and foxes hunt wild blackbirds.
- Test befriending with thrown sweet berries (stay/follow, singing near friend), feather recipes (`creature:mod_feathers`),
  raw/cooked meat drops and Bestiary morphs and facts.

## 8. Open questions

- ~~Q1: Female variant?~~ Yes, two natural morphs (50/50 at spawn): black male with orange-yellow beak and eye-ring, and
  warm brown female with a paler speckled breast and horn-brown beak (§2).
- ~~Q2: Health and biomes?~~ 6 HP (3 hearts), spawning in forest, birch forest, flower forest, plains, meadow, taiga
  and cherry grove (§2).
- ~~Q3: Meat drops?~~ Yes: 0 to 2 feathers and 1 raw blackbird (cooked if burned), matching Toucan, Seagulls and Mice
  (§4).
- ~~Q4: Befriending?~~ Yes: thrown `minecraft:sweet_berries` (1 in 4 chance), stay/follow and singing near its friend
  (§3).
- ~~Q5: Settled garden worms?~~ Hunt wild earthworms only; spare settled garden worms (§2).
- ~~Q6: Predators?~~ Vanilla cats and foxes hunt wild blackbirds (§2).
