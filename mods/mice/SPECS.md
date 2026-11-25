# Mice: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): small field mice that flee from players
and are hunted by cats.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the approved first model and design questions, no mod implementation yet.

- **The mouse model** (`sources/models/mouse.bbmodel`; [front](../../docs/models/mice-mouse-front.png) and
  [back](../../docs/models/mice-mouse-back.png) previews): a small, low field mouse with a warm brown coat, cream belly,
  rounded pink ears, dark eyes, short whiskers and a long, thin tail. The model uses a 64 × 64 pixel texture; painted
  faces use one texture pixel per model unit and the other faces use flat-colour swatches.
- The model's appearance and visible proportions are approved from the preview. Texture details can be refined later.

The v1 design is specified; the pixel texture can be refined during the art pass. Next: implement Mice.

## 1. Technical context

- **Mod id**: `mice` ✅. Entity id: `mice:mouse` ✅, so Elephant can optionally recognize it. Items: `mice:raw_mouse`
  and `mice:cooked_mouse`.
- **Dependency**: the series' library `creature` ([common/SPECS.md](../../common/SPECS.md)). Elephant must not become a
  dependency of Mice, and Mice must remain optional for Elephant.
- **Languages**: source text in English; in-game text in English and French.
- **Model**: Blockbench free-format cube model, converted with `cargo creature model mice mouse` when implementation
  begins.
- Elephant's approved response is already specified in [elephant/SPECS.md](../elephant/SPECS.md): when a mouse from this
  separate mod comes within 8 blocks, an elephant trumpets and flees for 5 seconds; its riders stay mounted. The mouse
  should expose a stable entity id for that optional detection, but the elephant feature must still work when this mod
  is absent.

## 2. The mouse

- **Appearance** ✅: one natural field-mouse look for v1, with brown fur, a pale underside, pink ears and nose, dark eyes,
  short whiskers and a long tail. The model's shape and proportions are approved from the Blockbench preview.
- **Visible size** ✅: about 0.56 blocks tall at the ears and 1.25 blocks long including the tail; the ears span about
  0.75 blocks.
- **Hitbox** ✅: 0.3 blocks wide × 0.3 blocks tall, so mice can fit through small openings.
- **Health** ✅: 4 HP (two hearts).
- **Spawning** ✅: uncommon groups of 2 to 4, with spawn weight 3 relative to other natural spawns, in plains, forest,
  meadow and savanna biomes, including savanna plateau; they spawn outdoors on grass or dirt.
- **Behaviour** ✅: peaceful; when a player comes within 6 blocks, the mouse scurries away. It has no special ability
  in v1. Vanilla cats hunt and kill mice. The mouse should not need to be present for Elephant to function.

## 3. Player interactions ✅

Mice are ambient creatures in v1. Players cannot catch or carry, tame, feed or breed them.

## 4. Drops and additional mechanics ✅

- **Mouse meat** ✅: the base drop is 1 `mice:raw_mouse`. If a mouse burns to death, it drops `mice:cooked_mouse`
  instead. Raw and cooked mouse meat use the same food values as the series' ordinary meat: raw restores 2 hunger with a
  chance of Hunger; cooked restores 6 hunger.
- **Looting** ✅: Looting increases the amount of mouse meat, following the series' existing meat drops.
- **Predators** ✅: vanilla cats hunt and kill mice.
- **Sounds** ✅: short, quiet squeaks play occasionally nearby and when a mouse flees; they do not make a continuous
  sound.

## 5. Bestiary ✅

The page records the biomes where a wild mouse was met. It has no behaviour facts. Its Items tab lists the raw and cooked
mouse meat.

## 6. In-game validation

After implementation, test in a new survival world with the series library and Mice enabled:

- Find uncommon groups of 2 to 4 (spawn weight 3) outdoors on grass or dirt in plains, forest, meadow and savanna biomes,
  including savanna plateau; confirm they do not spawn indoors.
- Check the 0.3 × 0.3-block hitbox against small openings and confirm it follows the visible body.
- Check that the mouse has 4 HP.
- Confirm mice never attack and scurry away when a player approaches within 6 blocks; check that they have no special
  ability in v1.
- Confirm that mice have no capture, taming, feeding or breeding interactions in v1.
- Check that vanilla cats hunt and kill mice.
- Confirm the base drop of 1 raw mouse meat, the cooked drop when a mouse burns to death, and that Looting increases the
  amount.
- Bring a mouse within 8 blocks of an elephant with both mods enabled; confirm that the elephant trumpets and flees for 5
  seconds without dismounting its riders. Then disable Mice and confirm that Elephant loads and behaves normally.
- Check that the bestiary records encountered biomes, lists raw and cooked mouse meat, and has no behaviour facts.
- Confirm short, quiet squeaks play occasionally nearby and when mice flee, with no continuous sound.

## 7. Out of scope for v1

- Catching, carrying, taming, feeding and breeding mice.
- Mouse behaviour facts in the bestiary; the page records biomes only.

## 8. Open questions

- ~~Q1: Is the field-mouse look in the preview right?~~ The model's shape and proportions are approved; texture details
  can be refined later.
- ~~Q2: Where should mice spawn, how common should they be, and in what group sizes?~~ Uncommon groups of 2 to 4
  (weight 3) outdoors on grass or dirt in plains, forest, meadow and savanna biomes, including savanna plateau.
- ~~Q3: Should they simply flee from players, or also hide in grass or small holes?~~ Peaceful; they flee from players
  within 6 blocks and have no special ability in v1.
- ~~Q4: Should players catch, carry, tame, feed or breed mice?~~ No player interactions in v1; mice remain ambient and wild.
- ~~Q5: Should cats hunt mice?~~ Yes; vanilla cats hunt and kill them.
- ~~Q6: Should mice drop anything when killed?~~ Yes; mice drop mouse meat.
- ~~Q7: How much meat should they drop, and should it cook when they burn?~~ Base drop 1 raw mouse meat, increased by
  Looting; it is replaced by cooked mouse meat if the mouse burns to death.
- ~~Q8: Should Looting increase the amount of mouse meat?~~ Yes; it increases the amount, following existing meat drops.
- ~~Q9: Should mice squeak?~~ Yes; short, quiet squeaks play occasionally nearby and when a mouse flees, with no
  continuous sound.
- ~~Q10: Should the bestiary record a fact when a cat hunts a mouse?~~ No; record biomes only.
- ~~Q11: Is a hitbox about 0.3 × 0.3 blocks right for this model?~~ Yes; 0.3 blocks wide × 0.3 blocks tall.
- ~~Q12: What natural spawn weight should mice use?~~ 3, a little more common than Elephant's weight 2.
- ~~Q13: Is 2 HP (one heart) right for the mouse?~~ 4 HP (two hearts).
