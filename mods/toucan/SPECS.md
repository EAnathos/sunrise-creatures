# Toucan: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): a peaceful tropical bird of the jungle
canopy, with a large colourful bill. Tame one with melon slices.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable (0.1.0, validated in game, not released yet: its registry page waits for its in-game picture): the toucan as an entity (`toucan:toucan`, its spawn egg),
0.8 × 1.2 blocks, 8 health, drawn with its model (feet on the ground; on a perch its tail bobbing and its head, the bill its child in the model, turned about; in flight its body
pitched and banked, its wings beating or held out, its legs tucked back); its spawning by day on the top of the leaves
(with two open blocks over them) of the three jungles, alone or in pairs, at most 2 within 48 blocks of a player (the library's spawning of small
creatures, `creature::flutter::spawns_on_leaves`), despawning far from players unless tamed or named; its behaviours
(`canopy.rs`): hops about the branches (never off the canopy's edge), short flights to the leaves of a tree 5 to 14
blocks off, each after a rest of 5 to 10 seconds on its perch (hopping about, not fleeing a player meanwhile) (only a perch with a clear way there, flown on beating wings straight to the point over it
then down onto it; a flight that stops getting nearer for 1.5 s is given up), a flight to the nearest perch when it falls or lands in water, fleeing 8 to 16 blocks to a
perch away from a monster within 8 blocks or, wild, a player within 6 (3 when they sneak), all on the library's
flight (`creature::flight`); a melon slice on the ground within 10 blocks, flown straight down to and eaten, one thrown by a
player taming it for them one time in four (hearts or smoke), the tamed one following (the library's `Follow`) or
staying on its friend's right-click: staying, it sits back on its rump and keeps still (not fleeing, not eating)
until told to follow, when it gets up; its drops (0 to 2 feathers, a raw toucan, cooked by fire or a smelting weapon,
more with Looting) and the cooked toucan's recipes; its guide page (rare, the `Feed` fact) and its two items' pages.
Its sounds are cut from CC0 recordings (`sources/sounds.toml`: a keel-billed toucan's croaks, a calling pair and a
chestnut-mandibled toucan; the vulture's wing beats); its item icons are our drawing. Drawn the right way round: the
model faces -z where the series' face +z, so it is turned half round when drawn.

- **The model** (`sources/models/toucan.bbmodel`; [front](../../docs/models/toucan-toucan-front.png) and
  [back](../../docs/models/toucan-toucan-back.png) previews): the approved 48-cube design has a dark body, pale chest,
  colourful bill, layered wings and a raised, pale-tipped tail. Its 64 × 64 texture is a flat-colour blockout; every
  face maps at one texel per model unit.

Next: its in-game picture and its first release; the pixel texture (a flat-colour blockout for now).

## 1. Technical context

- **Mod id**: `toucan` ✅. Entity `toucan:toucan`, spawn egg `toucan:toucan_spawn_egg`, and items `toucan:raw_toucan`
  and `toucan:cooked_toucan`.
- **Dependency**: the series' library `creature` (`common/SPECS.md`).
- **Languages**: source text in English; in-game text in English and French.
- **Model**: Blockbench free-format cube model, converted with `cargo creature model toucan` when implementation begins.

## 2. The toucan ✅

- **Size** ✅: hitbox 0.8 blocks wide and 1.2 blocks tall. At 16 model units per block, the visible model is about
  1.06 blocks wide, 1.19 tall and 2.5 long, including its bill and fan tail.
- **Appearance** ✅: one natural colour variant for v1; a blue-black body and layered wings, an ivory chest, a gold and
  red throat, turquoise eye markings, and a long yellow bill with an orange base and black tip.
- **Spawning** ✅: rare in jungle, sparse jungle and bamboo jungle, alone or in pairs, preferably above ground among
  leaves.
- **Behaviour** ✅: peaceful; hops on branches, makes short flights between nearby trees, lands on leaves and flees
  from nearby players and hostile mobs. It does not attack.

## 3. Taming ✅

- Feed a wild toucan a `minecraft:melon_slice` to tame it. Following the Vulture and Seagulls, drop the melon slice
  nearby; the toucan flies down and eats it. Each slice has a one-in-four chance to tame it for the player who threw
  it.
- A tamed toucan follows its friend or stays on right-click. Use the library's friend state and flight-follow goal.
  It cannot be ridden.

## 4. Drops and items ✅

- A toucan drops 0–2 `minecraft:feather` items and 1 `toucan:raw_toucan`. Looting increases the quantity. The raw meat
  drops as `toucan:cooked_toucan` if the toucan burns or is killed with a smelting weapon.

## 5. Bestiary ✅

The page records the biomes where a wild toucan was met. A `Feed` fact reveals the melon slice. Its Items tab lists
the raw and cooked meat. The page uses the library bestiary described in [common/SPECS.md](../../common/SPECS.md)
§7.

## 6. In-game validation

After implementation, test in a new survival world with the series library and Toucan enabled:

- Find toucans in the three jungle biomes, including on leaves.
- Check the bill, chest and eye colours from the front and both sides, and readability in the bestiary's 3D view.
- Observe hopping, short flights, landing and fleeing from players and hostile mobs.
- Drop melon slices near wild toucans; check the taming chance, ownership, following and staying.
- Check vanilla feather and meat drops, including cooked meat when killed by fire or a smelting weapon.

## 7. Out of scope for v1

- Breeding and riding.

## 8. Open questions

- ~~Q1: Its look?~~ Approved from the Blockbench previews (§0).
- ~~Q2: Where and how does it live?~~ Rare in the three jungle biomes; it hops, flies short distances and flees (§2).
- ~~Q3: How is it tamed?~~ Melon slices, following the Vulture and Seagulls' thrown-food pattern (§3).
- ~~Q4: Its size?~~ A 0.8 × 1.2-block hitbox; bill and tail extend beyond it visually (§2).
- ~~Q5: Its drops?~~ 0–2 feathers and 1 meat, cooked by fire or a smelting weapon (§4).
- ~~Q6: Breeding?~~ Out of scope for v1 (§7).
