# Toucan: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): a peaceful tropical bird of the jungle
canopy, with a large colourful bill. Tame one with melon slices.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and model, no code yet.

- **The model** (`sources/models/toucan.bbmodel`; [front](../../docs/models/toucan-toucan-front.png) and
  [back](../../docs/models/toucan-toucan-back.png) previews): the approved 48-cube design has a dark body, pale chest,
  colourful bill, layered wings and a raised, pale-tipped tail. Its 64 × 64 texture is a flat-colour blockout; every
  face maps at one texel per model unit.

Next: finish the pixel texture, then the code, on Sunrise.

## 1. Technical context

- **Mod id**: `toucan` ✅. Entity `toucan:toucan`, spawn egg `toucan:toucan_spawn_egg`, and items `toucan:feather`,
  `toucan:raw_toucan` and `toucan:cooked_toucan`.
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

- A toucan drops 0–2 `toucan:feather` items and 1 `toucan:raw_toucan`. Looting increases the quantity. The raw meat
  drops as `toucan:cooked_toucan` if the toucan burns or is killed with a smelting weapon.
- The feather works in the library's feather recipes (`creature:feathers`).

## 5. Bestiary ✅

The page records the biomes where a wild toucan was met. A `Feed` fact reveals the melon slice. Its Items tab lists
the feather and raw/cooked meat. The page uses the library bestiary described in [common/SPECS.md](../../common/SPECS.md)
§7.

## 6. In-game validation

After implementation, test in a new survival world with the series library and Toucan enabled:

- Find toucans in the three jungle biomes, including on leaves.
- Check the bill, chest and eye colours from the front and both sides, and readability in the bestiary's 3D view.
- Observe hopping, short flights, landing and fleeing from players and hostile mobs.
- Drop melon slices near wild toucans; check the taming chance, ownership, following and staying.
- Check feather and meat drops, including cooked meat when killed by fire or a smelting weapon.

## 7. Out of scope for v1

- Breeding and riding.

## 8. Open questions

- ~~Q1: Its look?~~ Approved from the Blockbench previews (§0).
- ~~Q2: Where and how does it live?~~ Rare in the three jungle biomes; it hops, flies short distances and flees (§2).
- ~~Q3: How is it tamed?~~ Melon slices, following the Vulture and Seagulls' thrown-food pattern (§3).
- ~~Q4: Its size?~~ A 0.8 × 1.2-block hitbox; bill and tail extend beyond it visually (§2).
- ~~Q5: Its drops?~~ 0–2 feathers and 1 meat, cooked by fire or a smelting weapon (§4).
- ~~Q6: Breeding?~~ Out of scope for v1 (§7).
