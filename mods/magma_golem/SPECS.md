# Magma Golem: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): a giant of living magma,
six blocks tall, that sleeps in the Nether as a heap of glowing rock until someone comes too close. A rare mini-boss:
found, not summoned, fought with a boss bar, and cooled with snow.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

In development (0.1.0, not released): everything of §2 to §5 is written, built and checked (Clippy, unit tests).
Tried in game: it spawns in the Nether; the rest is being tried, its numbers the starting values below.

- **Done**: the entity (3 × 6 blocks, 250 health, armour 10, no knockback, immune to fire and lava), its drawing
  (`draw.rs`) and every pose of §2.1 but the hurt flinch and the death (vanilla's tilt): the mound, rising with its
  roar, lying down, walking, the fist slam, the ground pound, the glob, the leap, crusted at half pace, the jaw ajar enraged;
  its seams full-bright over it (`magma_golem_glow.png`, the texture's glowing pixels), pulsing asleep with its eyes
  dark, brighter enraged, dark crusted; its light (the library's coloured light, `creature::glow`: seen only, it
  lights no block). Its sleep and fight (`golem.rs`): it wakes when a player comes within 6 blocks, hits it or mines
  within 4 blocks of it; its boss bar (the players within 48 blocks; bars left by a closed game are swept); its chase
  (straight at its target over steps up to 2 blocks, jumping when a wall holds it; a path round when the target is
  far above or below or the straight way stops bringing it nearer; the nearest player in its sight when the game
  gives it no target); the fist slam, the ground pound (fire left around, small magma cubes
  shed enraged), the leap, the magma glob (`glob.rs`, the library's lobbed projectile, its blast and fire); it heals in lava and fire, is enraged under half health (faster, shorter wind-ups), crusted by 5
  snowballs within 4 seconds or a splash water bottle (slowed, double damage taken) and lies down again after
  2 minutes without a target, its health restored. Its spawning (`spawn.rs`): in the Nether's basalt deltas and wastes
  near lava, on a floor with room for it, never within 256 blocks of another (where each appeared is saved with the
  Nether until it dies), never despawning, none in Peaceful. Its drops (its heart, magma cream and blocks, 50 experience); its guide
  page and facts ("Awake", "Crust", "Lava bath", "Heart": learnt by its killer); the magma heart and the molten pickaxe
  (`pickaxe.rs`: raw ores give ingots, sand glass, cobblestone stone, netherrack nether bricks, ancient debris
  netherite scrap, with the furnace's experience), its smithing recipe and its recipe book unlock.
- **Placeholders**: its sounds are vanilla's until its own are cut from freesound (§1): its hurt, death and steps
  (`assets/magma_golem/sounds.json`: the ravager's and basalt's, pitched down, named, no file copied) and its
  attacks' (the ravager's roar, the iron golem's blow, an explosion, lava hissing, basalt breaking).
- **Left**: the hurt flinch and the death pose (§2.1), its own sounds; trying and balancing it all in game.
- **The model** (`sources/models/magma_golem.bbmodel`, pictures in docs/models/): our own design, about 6 blocks
  tall, built from boulders of dark grey-brown rock split by glowing orange seams, lava showing at the belly, the
  chest, the elbows, the wrists and the knees. Hunched forward like an ape: a massive back and huge shoulders rise
  above its head, held low in front: plain dark stone, a heavy overhanging brow, two glowing eyes in dark sockets, a
  nose ridge, cheekbones, and a jaw with a glowing mouth that opens on a glowing maw. Long thick arms end in enormous
  fists that rest on the ground before it; short heavy legs set wide. Every moving part has its own pivot for the
  animations (§2.1): `body` (at the hips), `head` (the neck), `jaw`, `arm_*` (the shoulders), `forearm_*` (the
  elbows), `fist_*` (the wrists), `leg_*` (the hips), `shin_*` (the knees).

## 1. Technical context

- **Mod id**: `magma_golem` ✅. Entity `magma_golem:magma_golem`, spawn egg `magma_golem:magma_golem_spawn_egg`, items
  `magma_golem:magma_heart`, `magma_golem:molten_pickaxe` ✅ (§5), projectile `magma_golem:magma_glob`.
- Uses from the library: model drawing and sounds (§3.1, §3.2), **hitbox size** (§3.4, a big mob), monsters sparing it
  (§3.4), the creature guide's pages (§7), and, moved from the Cyclops with this mod (rule of two), **the lobbed
  projectile** (the cyclops's boulder: lifted, thrown on an arc, breaking blocks where it lands or not;
  common/SPECS.md §3.9) ✅.
- It copies the wandering trader (goals only), a box of 3 × 6 blocks ✅ (6 blocks tall). Sunrise: the boss bar
  (`create_boss_bar`, shown to the players within 48 blocks).
- Its own sounds (grinding rock, a deep rumble, the hiss of lava) from freesound, CC0, cut by `cargo creature sounds`.

## 2. The magma golem ✅

- **Its look** (our own design): a hunched giant built from heavy boulders of dark rock, lava glowing orange in the
  seams between them; a small head held low in front, below its massive back and shoulders, two glowing eyes, long
  thick arms ending in enormous fists that rest on the ground, short heavy legs set wide. Its seams are
  drawn full-bright, and it lights its surroundings (light 12).
- **Dormant**: it sleeps **curled up as a mound of glowing rock** (a pose of the model: knees drawn up, arms around,
  head tucked, about 3 blocks high), still and silent, only its cracks slowly pulsing. The Nether has no day: it wakes
  when a player comes within 6 blocks, hits it or mines next to it, rising with a rumble (2 seconds) before it attacks.
  It lies down again where it stands after 2 minutes without a target, its health restored.
- **Spawning**: rare, in the **Nether**: basalt deltas and the nether wastes near lava seas, always dormant, on a
  floor with room for it (3 × 6 × 3), never within 256 blocks of another. It never despawns.
- **Fighting** (a mini-boss, its boss bar while awake): 250 health, armor 10, no knockback, immune to fire and lava.
  Heavy steps, yet it runs at its target (a little faster than a walking player); it wades through lava and **heals
  in lava and fire** (2 health a second).
  - **Fist slam**: in reach (4 blocks), 14 damage and sets on fire.
  - **Ground pound**: when players are within 6 blocks, both fists on the ground: a shockwave (8 damage, thrown up and
    back, set on fire), fire left around it.
  - **Leap**: a player 9 to 18 blocks away (never nearer), once every 15 seconds at most: it crouches and springs,
    bending its fall toward them in the air to come down on them; its landing hurts the players within 4 blocks (10 damage, thrown up and back). It
    takes no fall damage.
  - **Magma glob**: at range (12 to 24 blocks, while it can't leap), it tears a glob of magma (a block wide) from its
    body, holds it overhead, and throws it fast on a slight arc (6 damage, sets on fire, a small blast and fire where
    it lands, breaks no block).
  - **Enraged** under half health: its cracks blaze brighter, it moves faster, and each ground pound sheds 1 or 2
    small magma cubes.
- **Cooled by snow**: snowballs hurt it (3 damage) and a few in a row (5 within 4 seconds) **crust it over**: its cracks
  go dark for 6 seconds, it is slowed and takes double damage. A splash water bottle crusts it at once. The way to beat
  it in the Nether: bring snow.
- **Drops**: always its **magma heart** (§5), 4 to 8 magma cream, 2 to 6 magma blocks, 50 experience.

### 2.1 Animation ✅

Every movement is animated by the code from the model's parts (`creature::model::draw`'s `pose`), smoothed between
ticks, slow and heavy, the ground shaking under it (dust and rock particles, a deep thud).

- **Dormant**: the mound pose (legs folded under, back rounded high, arms wrapped around, head tucked under the
  brow), its eyes dark; only a slow breath lifts the back now and then.
- **Waking** (2 s): the back heaves first, the fists push on the ground, the legs unfold, the arms come out, the head
  rises last and its eyes light up, the jaw opening on a roar. **Lying down** is the same, backwards.
- **Idle**: a slow breath (the back and shoulders rise and fall), the head turning toward the nearest player, the
  fists shifting on the ground.
- **Walking**: heavy steps, one leg then the other with the knee bending, the body rolling from side to side and
  dipping on each footfall; the arms swing opposite the legs, its fists brushing the ground like an ape's.
- **Fist slam**: one arm rises high behind it (0.3 s), then smashes down in front, the body following the
  blow; it straightens again.
- **Ground pound**: both arms rise over its head (1 s, the warning), the jaw opens, both fists come down together, the
  body dropping with them.
- **Magma glob**: one hand tears a glob from its body (the glob growing in its fist), lifts it over the head and holds
  it there (1.6 s in all), then throws it.
- **Leap**: a crouch (0.7 s, the body sinking, the arms swung back), then in the air the arms thrown up and the legs
  tucked, the jaw open; it lands on its feet.
- **Hurt**: a short flinch back, the head jerking.
- **Crusted** (cooled by snow): stiff and slow, every movement at half speed, its eyes dim.
- **Enraged**: a roar (the jaw wide open, the head thrown back, the arms spread), then every movement faster.
- **Death**: it drops to its knees, slumps forward on its fists and stays still as its glow fades.

## 3. Boss or not ✅

A mini-boss, found rather than summoned: rare and dormant in the Nether, a boss bar while it fights, a sure trophy
(its heart). Not a true boss like the Wither (no summoning, no altar, no world event): finding one sleeping is the
event. Summoning one (a ritual with magma blocks, as the iron golem or the Wither) could come later (§6).

## 4. Bestiary page ✅

The biomes met in, and the facts: "Awake" (woke one), "Crust" (cooled one with snow or water), "Lava bath" (saw one
heal in lava), "Heart" (took its heart).

## 5. Items ✅

- **Magma heart** (`magma_golem:magma_heart`): a glowing core of magma, warm in the hand. Its use:
- **Molten pickaxe** (`magma_golem:molten_pickaxe`): a netherite pickaxe and a magma heart in a smithing table. Mines
  as netherite, and **smelts what it mines** (iron, gold and copper ores give ingots, sand gives glass, stone gives
  stone, cobblestone gives stone), with the furnace's experience. Fireproof. Its durability is the netherite pickaxe's.

## 6. Out of scope for v1

- Summoning one, taming or riding it, smaller magma golems, a golem of another rock (an obsidian or a basalt golem),
  armour from its heart.

## 7. Open questions

- ~~Q1: The name?~~ Magma Golem, `magma_golem` (§1).
- ~~Q2: Where and how rare?~~ Basalt deltas and nether wastes near lava seas, 256 blocks apart (§2).
- ~~Q3: A boss?~~ A found mini-boss with a boss bar, no summoning in v1 (§3).
- ~~Q4: Its attacks and weakness?~~ Fist slam, ground pound, magma glob, enraged under half health; snow cools it (§2).
- ~~Q5: Its loot?~~ The magma heart, which makes the molten pickaxe (§5).
- ~~Q6: The thrown boulder?~~ Moved from the Cyclops to the library, shared with the magma glob (§1).
- ~~Q7: The animations?~~ Every movement animated, slow and heavy (§2.1).
