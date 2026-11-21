# Ladybugs: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): ladybugs crawling on the flowers and
leaves of meadows and forests, red, yellow, orange or black, with their spots. As the old saying goes, one that lands
on you brings luck: for as many minutes as it has spots. Counting them is the game, and the rare kinds are the luckiest.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable (0.1.0), played in game: the five kinds, drawn in their colours with their spots, spawning in their biomes,
flying short hops between flowers and resting on them, gathering in clusters at night, landing on a player who stands
still (seated on top of their head) and giving them Luck for as many minutes as it has spots, chickens pecking them up,
the glass jar, the guide's page and facts.

- **The model** (`sources/models/ladybug.bbmodel`, pictures in docs/models/): a domed ladybug about seven texels long:
  its two elytra (a shell and a two-step dome each, so it looks round), the black pronotum with its white side patches,
  a small black head with white cheeks, short antennae, six thin legs, and the hind wings folded under the elytra.
  Groups: `body` (with the pronotum, the head, the antennae and the legs), `elytron_left`, `elytron_right` (opened up
  and out by the pose to fly), `wing_left`, `wing_right` (spread from under them); the elytra and wings sit beside the
  body, so the shell is drawn tinted on its own. Its embedded texture is the seven-spot's, for the pictures; the game
  draws `textures/entity/ladybug/base.png` (the shell grey) and each kind's spots (`<kind>.png`, white).
- Not yet (❓ below): crawling (it rests where it lands), clusters by day in cold biomes, sheltering under leaves in
  the rain, the "All the spots" fact, flying off when hurt, its own sound.

## 1. Technical context

- **Mod id**: `ladybugs` ✅. Entity `ladybugs:ladybug` (one entity, its kinds as morphs, common/SPECS.md §7), spawn egg
  `ladybugs:ladybug_spawn_egg`. No item of its own ✅: a caught one is in the library's `creature:glass_jar`.
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size and monsters sparing it (§3.4), **small
  fliers** (§3.5: its short flights between flowers, spawning), **colours and patterns** (§5: its kind is its look's
  pattern, rolled by biome), **hunters** (§3.7: chickens), the **glass jar and territories** (§4, which this mod switches on; rule "flower"), the creature guide's
  pages and morphs (§7).
- It copies the wandering trader (goals only), with a tiny box: 0.25 × 0.15 blocks ✅.
- Drawn as the butterfly ✅: a greyscale shell tinted by its kind's colour, its spots a texture of their own over it
  in theirs, the body and wings as they are (the library's drawing layers). A kind fixes its colours and its spots
  together, so the mod draws it itself rather than through `colours::draw`, which rolls them apart.
- Crawling ❓: the library's crawl is a goal the small fliers' rest holds still, so v1 flies and rests only.
- Its own sounds ❓: almost silent; a soft buzz of wings as it takes off, from freesound, CC0, cut by
  `cargo creature sounds` (none yet).

## 2. Kinds ✅

| Kind | Shell | Spots | Where | Rarity |
| --- | --- | --- | --- | --- |
| **Seven-spot** | red | 7 black | plains, meadows, flower forests, forests, birch forests | common |
| **Two-spot** | red | 2 black | plains, meadows, forests, cherry groves | common |
| **Twenty-two-spot** | yellow | 22 small black | meadows, flower forests, sunflower plains | uncommon |
| **Orange** | orange | 16 white | birch forests, dark forests, old growth birch forests | uncommon |
| **Harlequin** | black | 4 red | swamps, mangrove swamps, dark forests | rare |

Each one is its kind for good (saved, kept in the glass jar). The guide shows the kinds met as morphs.

## 3. Life on the flowers ✅

- About a sixth of a block long. 1 health. Never attacks; a monster never targets it (the library).
- **Spawning**: on flowers, grass and leaves in its biomes, by day, 1 to 3 together, through the library's spawning of
  small fliers (§3.5): a chance every 10 s for each player, up to 6 within 32 blocks. Wild ones despawn far from
  players.
- **By day** it rests on a flower or the ground (crawling about ❓, §1), and now and then opens its
  elytra and **flies** a short way to another flower (the library's flight of small fliers): its elytra lift, its hind
  wings unfold and buzz. It rests in the rain (under leaves if there are any ❓).
- **At night** ladybugs near each other **gather in a cluster** ✅ (and by day in cold biomes, snowy plains, snowy
  taigas, groves ❓): they fly to the same
  block and rest packed together (a sight: up to a dozen on one log), until the morning.
- **Hunted** (the library's prey, §3.7): a chicken within 6 blocks pecks it up. A seagull doesn't (it eats fish).
- **No drop** ✅.

## 4. Luck, its signature ✅

- A wild ladybug near a player who stands still (or sits, or sneaks) for a few seconds may **fly over and land on
  them** (one chance in 20 a minute ✅): on top of their head, seated there as a passenger of the player
  (`EntityRef::ride_player`, which Sunrise draws as vanilla draws a mob on a player) ✅.
- While it rests there, and after it leaves, the player has **Luck** for **as many minutes as it has spots**: 2 for a
  two-spot, 7 for a seven-spot, 16 for an orange, 22 for a twenty-two-spot; the rare harlequin gives **Luck II** for 4.
  Luck makes fishing and loot chests luckier, as vanilla's Luck does (with a seagull's lucky spot, a fishing trip pays).
- It stays 30 to 90 s, then flies off on its own; sprinting makes it (and getting hurt ❓)
  fly off at once, the luck kept. A ladybug lands on a player at most once a day.
- A right-click on the ladybug on you lets it go (it flies off), as the rhyme says.
- A player who got luck from a kind learns the "Luck" fact; one who got it from every kind, the "All the spots" fact ❓
  (not yet: it needs the kinds each player got luck from, saved).

## 5. The glass jar ✅

A ladybug is caught in the library's glass jar (common/SPECS.md §4) and released where the player wants it; released
on a flower it stays around that flower patch (the library's "flower" rule, as the butterflies). On display, the jar
shows it crawling on the glass. A settled ladybug never despawns, and lands on its own players as a wild one does
(§4).

## 6. Bestiary page ✅

The biomes met in, the kinds met as its morphs, and the facts: "Luck" (one landed on you), "Cluster" (saw a cluster of
at least five), "Chicken" (saw a chicken peck one up), "Jar" (caught one in a glass jar), "Flower" (let one out on a
flower, where it settles); "All the spots" (luck from every kind) ❓ (§4). Its rarity: common.

## 7. Items

None of its own ✅: the jar is the library's, the spawn egg the series'.

## 8. Out of scope for v1

- Aphids and crops protected from them (Minecraft has no pests), larvae and eggs, breeding, other beetles.

## 9. Open questions

- ~~Q1: Drawn as the butterfly, or a texture per kind?~~ As the butterfly: a tinted shell and a spot pattern (§1).
- ~~Q2: Landing on the player: how often?~~ One chance in 20 a minute while still, once a day each, so luck stays a
  small surprise (§4).
- ~~Q3: How it shows on the player?~~ Seated on top of their head (`ride_player`, §4).
- Q4: Crawling: a library change so the small fliers' rest lets a crawl move them, or a flier that walks (§1)?
