# Phasmids: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): stick insects and leaf
insects, masters of camouflage, hiding in the trees as twigs and leaves. Still by day and swaying like a twig in the
wind when they move, they are hard to spot: finding them is the game. Caught in a glass jar, they settle on a tree of
your own.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable (0.1.0), played in game: both insects in their kinds, coming out on top of leaves, still by day and with a
player near, crawling over the canopy at night, spotted by a look (the shimmer, the count saved on the player, the
facts), dropping through the leaves to play dead when hit and climbing back up their tree. The glass jar's catch works;
a release on a trunk was tried: the phasmid dropped off the bark and was drawn lying flat. It now clings to the trunk
upright (the library's tree climbers, common/SPECS.md §3.16), and an unsettled one on the ground under its tree goes to
the trunk to climb back to the canopy: to try again in game.

- **The stick insect** (`assets/models/stick_insect.bbmodel`, pictures in docs/models/): a long thin twig of a body
  (head, thorax, abdomen and a forked tip, with darker node bands like a twig's), long antennae, the front legs held
  straight forward along them as real stick insects do to look like a twig, and two pairs of long thin legs bent at the
  knee. Groups: `body`, `abdomen`, `abdomen_tip`, `antenna_*`, `leg_front_*`, `leg_mid_*`, `leg_hind_*` and their
  `*_lower`, so the game can sway it and walk it. A texture per kind (§2): oak, birch, spruce, jungle.
- **The leaf insect** (`assets/models/leaf_insect.bbmodel`): a flat leaf-shaped body (a broad abdomen tapering to a
  point, a midrib and veins painted on top, a darker margin), a small head with short antennae, and six legs with leafy
  flaps. Groups as the stick insect's, without the abdomen's tip. A texture per kind (§2): green and dry.

Since 0.1.0's first cut: its crawl is the library's creeping (§3.16), which climbs leaf steps and trunks and goes
through foliage; its jar and its guide page show it on a twig; its sounds (a rustle as it crawls, a click as it
drops).

## 1. Technical context

- **Mod id**: `phasmids` ✅. Entities `phasmids:stick_insect` and `phasmids:leaf_insect` (each with its kinds as morphs,
  common/SPECS.md §7), spawn eggs `phasmids:stick_insect_spawn_egg` and `phasmids:leaf_insect_spawn_egg`. No item of its
  own ✅: a caught one is in the library's `creature:glass_jar` ✅.
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4, tiny), monsters sparing it (§3.4),
  **creeping** (§3.16: its crawl over the canopy and up trunks and leaf steps, new with this mod and the Ladybugs), **spawning** (§3.5, on leaves: `flutter::spawns_on_leaves`, new
  with this mod), the **glass jar and territories** (§4, which this mod switches on; the rule "tree", `Rule::Tree`, new
  with this mod), the creature guide's pages and morphs (§7). Climbing back up after playing dead (to the trunk, up through the
  leaves) is its own.
- Spotting is counted on the player (`phasmids:spotted` on their saved data), each phasmid once per player.
- Both copy the wandering trader (goals only), with a tiny box: 0.6 × 0.2 blocks (stick insect), 0.5 × 0.2 (leaf
  insect) ✅.
- Its own sounds ✅: almost silent, a faint rustle of leaves now and then as it crawls (the library's creeping plays it,
  §3.16), a dry click when it drops to play dead, from freesound, CC0 (`sources/sounds.toml`).

## 2. Kinds ✅

Each kind matches the trees it lives in, so it is camouflaged where it spawns.

| Creature | Kind | Where |
| --- | --- | --- |
| Stick insect | **Oak** (brown) | forests, flower forests, dark forests, plains' and meadows' trees |
| Stick insect | **Birch** (white with black marks) | birch forests, old growth birch forests |
| Stick insect | **Spruce** (dark brown) | taigas, old growth taigas |
| Stick insect | **Jungle** (green) | jungles, bamboo jungles |
| Leaf insect | **Green** | jungles, bamboo jungles |
| Leaf insect | **Dry** (brown, dead-leaf spots) | sparse jungles, savannas |

## 3. Life in the trees ✅

- About two thirds of a block long (the stick insect, with its antennae and front legs), half a block (the leaf
  insect). 2 health. Never attacks.
- **Spawning**: on the top of leaves in its biomes, 1 or 2 together, through the library's spawning (§3.5), placed on
  leaves instead of over the ground: a chance every 10 s for each player, up to 4 within 32 blocks. Wild ones despawn far
  from players.
- **Moving**: it crawls slowly on leaves and climbs the sides of logs and leaves (the library's small crawlers, with
  climbing). It **clings** to a trunk it stops on, upright, head up, rather than dropping off (common/SPECS.md §3.16).
**By day it stays still**, and so does a wild one with a player within 6 blocks who hasn't spotted it; at dusk and at
night it walks about its tree. When it walks it **sways** side to
  side, as a twig in the wind (real stick insects do). It never comes down to the ground on its own.
- **Camouflage**, its signature ✅: it looks like its tree, still most of the time. A player
  **spots** one by looking at it (the crosshair on it) from within 6 blocks for a second: a faint shimmer shows it, and
  the bestiary counts it (§4). Wild ones keep still while a player is within 6 blocks, unless spotted.
- **Plays dead** (thanatosis): hit, or a player running into its tree, it **drops** to the ground with its legs folded
  and lies still for 10 to 20 s, then climbs back up to the nearest leaves. A dropped one is easy to catch.
- **Drops**: a stick insect drops 1 stick ✅; a leaf insect nothing.

## 4. Bestiary page ✅

The kinds met as its morphs, and the facts: "Spotted" (spotted one), "Keen eye" (spotted 10), "Master spotter"
(spotted 50), "Play dead" (saw one drop and play dead), "Night walker" (saw one walking at night), "Jar" (caught one in
a jar), "Settled" (settled one on a tree).

## 5. Glass jar and territory ✅

- **Caught** in the library's glass jar ✅ (common/SPECS.md §4.1), its kind and name kept. A glass jar on display
  shows it on a twig.
- **Released on a tree it settles** (the library's rule **tree**, §4.2, new with this mod): released on leaves or a log,
  its tree is the leaves and logs connected to that block (within 8 blocks of it); the tree
  must have **at least 10 leaves**, otherwise it won't settle. A settled one stays on its tree and never despawns. Leaves
  cut down below 10, it becomes unsettled (wanders, may despawn).
- The jar's recipe condition and the creative tab get `phasmids` with this mod (library §3.3, §4.1).

## 6. Out of scope for v1

- Eggs, nymphs and breeding, predators (wild parrots eating them), eating leaves (no blocks are touched), other phasmids (the giant spiny stick insect, the
  flying ones), a terrarium block, a phasmid as a pet that follows the player.

## 7. Testing in game

In a forest or a jungle, in the test instance (`/time set night` to see them move, `/time set day` to see them keep
still):

- Spawn eggs on top of leaves: still by day, and at night while you stand within 6 blocks without having spotted it;
  looked at for a second from within 6 blocks, a shimmer and the "Spotted" fact; at night from farther away, it crawls
  over the canopy, swaying, and climbs leaf steps.
- Hit one: it drops to the ground on its back, lies 10 to 20 s, then walks to the trunk and climbs it upright, back to
  the canopy.
- Catch one in the glass jar, release it on a trunk's side (a tree of at least 10 leaves): it settles ("Settled"),
  clings there upright by day, and climbs to its tree's leaves at night. Break the log it clings to: it falls.
- Released on the ground beside a tree at night: it goes to the trunk and climbs.

## 8. Open questions

- ~~Q1: Which creature?~~ Stick insects (French "phasmes"), not a ghost.
- ~~Q2: Which kinds?~~ A stick insect and a leaf insect, two models (§0).
- ~~Q3: What does the player do with them?~~ Spot them in their camouflage, and catch and settle them with the glass
  jar (§3, §5).
- ~~Q4: The mod id?~~ `phasmids`, for stick and leaf insects alike (§1).
- ~~Q5: The kinds?~~ Oak, birch, spruce and jungle stick insects, green and dry leaf insects, each matching its trees
  (§2).
- ~~Q6: Spotting?~~ Looking at one from within 6 blocks for a second, counted in the bestiary (§3, §4).
- ~~Q7: Breeding on a settled tree?~~ Out of scope for v1 (§6).
- ~~Q8: Parrots eating them?~~ Out of scope for v1 (§6).
- ~~Q9: Drops?~~ A stick from a stick insect, nothing from a leaf insect (§3).
