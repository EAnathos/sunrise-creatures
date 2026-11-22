# Sporelins: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): small, shy mushroom sprites of
dark forests and mushroom fields. They pass for a mushroom when you come near, puff spores when startled, and plant
mushrooms where they walk; befriended, one follows you home and tends a mushroom patch there.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable on Sunrise (0.1.1), tested in game: its two looks, hiding from a player and coming out again, the spore puff
and running off when hit (along a fence too), befriending with a mushroom of its colour (one chance in 3), following its
friend and coming to them when far (onto the ground by them, never in the air), staying and tending a patch (mushrooms
of its colour planted around it), the stew once a day (a second bowl the same day refused, a reload included: the day is
saved on it), the glow of a friend's cap at night, the mushroom it drops.

- **Two models**, one per look (`sources/models/`, pictures in docs/models/), with the same groups so the same code
  poses both:
  - the red cap (`sporelin.bbmodel`): about two thirds of a block tall, a rounded red cap with white spots over beige
    gills (`head`), a pale stem for a body with two dark eyes and pink cheeks (`body`), two stubby arms and two short
    legs;
  - the brown cap (`sporelin_brown.bbmodel`): a wide, flat tan cap (9 texels across, as the brown mushroom's) on a
    thicker, taller stem, the same face.

  Hiding, it sits down: drawn lower by its legs' height, its arms and legs tucked under its cap.
- **The code**: `src/lib.rs` (spawning, looks, right-clicks, planting, the glow, the drawing), `src/life.rs` (its
  behaviours). From the library: friends (§6.2), colours, the spawning of small creatures on given blocks, glow,
  `creep::too_tall` (common/SPECS.md).

## 1. Technical context

- **Mod id**: `sporelins` ✅. Entity `sporelins:sporelin`, spawn egg `sporelins:sporelin_spawn_egg`. No item of its own
  ✅ (§4).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4), the
  creature guide's pages (§7), and **friends** (§6.2, already the library's: the fairies, the seagulls, the vulture).
- It copies the wandering trader (goals only), with one box for both looks ✅: 0.5 × 0.65 blocks.
- **Two looks** ✅: a red cap or a brown cap, like the two mushrooms, rolled at spawn and saved on the entity. A model
  and a texture each (`entity/sporelin/red`, `entity/sporelin/brown`), the brown one wider and flatter (§0).

## 2. The sporelin ✅

- A small mushroom creature: a wide cap for a head, a pale stem for a body, two dark eyes, stubby arms and legs. It
  waddles. Ambient and harmless, 6 health.
- **Spawning** ✅: dark forests, mushroom fields, old growth taigas and swamps, where it is dark (light ≤ 7 on its
  block, as mushrooms want), 1 to 3 together; red caps and brown caps alike, red ones more common in dark forests and
  mushroom fields. Wild ones despawn far from players.
- **Shy** ✅: a player who comes within 6 blocks (not sneaking) makes it **hide**: it sits down and pulls its cap over
  its stem, and passes for a mushroom until the player is 10 blocks away or stands still a few seconds. A sneaking
  player can come close.
- **A spore puff when startled** ✅: hit, or a player sprinting at it, it bursts a small cloud of spores (particles,
  a soft "pff") and runs off. Harmless.
- **Plants mushrooms** ✅: about once every 5 minutes, it plants a mushroom of its cap's colour on a dark block next to
  it where a mushroom can grow, never more than 4 within 5 blocks of it.
- Monsters leave it alone (library §3.4).

## 3. Befriending ✅

- **With a mushroom of its colour** ✅: a sporelin takes a red (or brown) mushroom from a player's hand (right-click;
  a hiding one lets a sneaking player come close); one chance in 3 it befriends that player (hearts), otherwise it eats it (a puff).
- **Befriended** (the library's taming and ownership, §6.2): it follows its friend, waddling, and teleports to them
  when far, like a tamed wolf; right-click makes it stay or follow. Named with a name tag. It no longer hides from its
  friend and never despawns.
- **Tending a patch** ✅: told to stay, it plants and tends mushrooms around where it stays (8 blocks), in the dark:
  planting as in §2, and now and then it makes a mushroom there spread to a next block.
- **Mushroom stew** ✅: give it an empty bowl (right-click) and it hands back a **mushroom stew**, once per in-game day;
  asked again the same day, it shakes its cap and the bowl stays in your hand.
- **Glows at night** ✅: a befriended one's cap spots glow softly in the dark (a full-bright layer, `emissive`, and a
  faint visual light around it, as the fireflies' `visuals`), lighting nothing in the world: a small night companion.
  Wild ones don't glow.
- Fed a mushroom of its colour, a befriended one heals (+2).

## 4. Drops ✅

- When killed: 1 mushroom of its cap's colour ✅. No item of its own ✅.

## 5. Bestiary page ✅

The biomes met in, its looks (red and brown caps), and the facts: "Hides" (seen hiding), "Mushroom" (befriended with a
mushroom), "Patch" (a mushroom it planted), "Stew" (a bowl of stew it gave).

## 6. Out of scope for v1

- Breeding, riding, other mushrooms (crimson or warped fungi in the Nether), a jar for it (it is too big).

## 7. Open questions

- ~~Q1: Shy?~~ Yes: it hides as a mushroom, and its spore puff is harmless (§2).
- ~~Q2: Befriending?~~ With a mushroom of its colour, one chance in 3 (§3).
- ~~Q3: What a befriended one does?~~ Tends a mushroom patch, gives a mushroom stew once a day, glows softly at night (§3).
- ~~Q4: Its drop?~~ A mushroom of its cap's colour (§4).
- ~~Q5: Its looks?~~ Two: a red cap and a brown cap (§1).
