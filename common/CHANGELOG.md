# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.4.3

- `creature::stats`: a creature's own stats (its maximum health and its pace), rolled within its kind's ranges, saved
  on it (`creature:stats`) and inherited by its babies; moved from the Vulture for the Elephant.
- `creature::friend::Follow`: following a friend on foot, moved from the Sporelins for the Elephant.
- `creature::ride`: a ridden creature's rider and the keys they hold (the server's `player_input`, kept by the library
  for any of the series' creatures), moved from the Vulture for the Elephant.
- `creature::pack`: a creature's packs (its storage, saved as `creature:pack`) and its owner's screen, in pages a mod
  describes (`Screens`), every vanilla click of a chest's slots handled; moved from the Vulture for the Elephant.
- `creature::growth`: babies, growth and breeding (a creature's age, saved as `creature:age`; love; `Mate`; the babies
  born tamed with inherited stats); moved from the Vulture for the Elephant.
- `creature::pick`: a young creature's outline shrinks with its box.
- `creature::hand::button`: a screen's button named, with a line saying what a click does (the Vulture's and the
  Elephant's screens); a page changed closes the old screen first.
- A creature's box follows its `minecraft:scale` beyond its client box's (a young elephant's stage).
- `creature::anger::provoke`: a creature set on someone who didn't hit it (whoever hit its rider: the Elephant).

## 0.4.2

- Creatures' feathers are vanilla `minecraft:feather` items. The guide recipe uses a vanilla feather; the library no longer
  registers custom feather tags or duplicate arrow, brush and book-and-quill recipes.

## 0.4.1

- The bestiary shows every installed species: how many were met of how many ("Species met: 12 / 99"), and the ones not
  met yet as a grey shadow of their spawn egg; picked, one not met yet shows only its name, its rarity and a question
  mark turning in place of the creature.
- `Species::shadow`: an item model drawing a species' icon in black, for an egg the GUI can't draw as a shadow from its
  texture (Minecraft's, the Vanilla Bestiary's).
- `creature::model::living_pose_upright`: a creature turned and scaled without the tip onto its side as it dies, for a
  death pose of its own (the Magma Golem's).
- `creature::pick`: a creature is hit where its outline is, parts reaching out of its box too (the Magma Golem's
  arms), through the client's `client_pick`.
- `creature::model::draw_attached`: a model drawn with what it holds following its hand (the Magma Golem's glob).
- `creature::particles` hands out flames, lava drops, explosions and large smoke too (the Magma Golem's).

## 0.4.0

- What the series keeps about each creature lives in `creature::state` (`EntityMap`, `EntitySet`), which the library
  empties of an entity when it leaves the level: no module keeps anything about a creature that is gone.
- Each settings struct the mods build (`Creeping`, `Soaring`, `Light`, `Species`, `Spawning`...) has a `DEFAULT`, for
  their literals to end with `..Type::DEFAULT`: a field added later no longer breaks the mods built before it.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.
- Breaking: the cleanup functions `creature::state` made redundant are gone (`body::forget`, `bestiary::forget`,
  `aquatic::forget`, `colours::forget`, `flight::forget`, `friend::forget`, `projectile::forget`,
  `territory::forget`, `flutter::forget`, `flutter::gone`): a mod leaves a gone creature to the library. A dying
  creature keeps its look through its death animation.
- Spawning on given blocks: a kind comes out on the highest of its blocks in the column, under leaves too (it stopped
  at a forest's canopy, so a sporelin almost never came out in a dark forest), and on a spot nothing collides in
  (grass, ferns, mushrooms or leaf litter on the soil no longer rule it out).
- A tree climber clings only to bark or leaves with nothing solid under its feet (a phasmid just come out on the
  ground no longer stands upright there).

## 0.3.5

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Shared state: every lock taken one way (`creature::sync::Locked`) and none held while calling the game, a mod's
  callback or another module, so a behaviour can call back into the library (`prey::quarry`, `body::forget`) without a
  deadlock; a lock left poisoned by a panic the game contained is taken as it is and counted in crash reports, rather
  than breaking every later tick.
- Tree climbers (`creep::Creep::climber`): a creature that stops on a trunk holds on there, its belly to the bark
  (`creep::clinging` for its drawing), and lets go when the bark is broken; under a target well above it, it walks to
  the nearest trunk first.
- A dead creature's behaviours stay stopped through its death animation; what the library keeps per creature (its
  behaviours, queued sounds, wing beats, clinging) is dropped when it leaves the level.
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md` in the
  package).

## 0.3.4

- Soaring (`flight::Soar`), for a flier that floats (the seagull): its circle drifts toward water and it lands on the
  water or its shore, so it keeps to the coast rather than wandering inland.
- `creep::too_tall`: a fence, a wall or a gate, taller than a step (the Dung Beetles, the Sporelins); the spore
  particle for the Sporelins.

## 0.3.3

- Body helpers shared by the mods: `body::flat_distance`, `body::turned`, `body::unit`, `Rand::between`.
- The glass jar's pasture rule (`territory::Rule::Pasture`: a creature released among at least 3 grazing animals
  settles there) and its recipe with the Dung Beetles installed; small creatures that come out on the blocks a mod
  chooses (`flutter::spawns_on`); creeping that doesn't climb (a `climb` of 0).
- A jar holding each creature a mod catches follows the empty glass jar in the creative tab.
- Soaring (`flight::Soar`): a flier lands on one of the high points near it at random (over a flat sea it always went
  the same way), and a roaming one's circle drifts along a random way (`Soar::roaming`: the seagull).
- Creeping (`creature::creep`): a crawl moved by hand that climbs up the block in its way or a trunk, through leaves
  (the Ladybugs, the Phasmids).
- `mob::give_effect_level`: an effect at a level above the first (the ladybugs' Luck II).
- The glass jar's recipe also comes with the Scorpions (which it catches: a fix), the Ladybugs or the Phasmids
  installed.
- Small creatures that come out on top of leaves (`flutter::spawns_on_leaves`), and the glass jar's tree rule
  (`territory::Rule::Tree`: a creature released on leaves or a log settles in that tree): the Phasmids.

## 0.3.2

- Big fliers (`creature::flight`): the vulture's flight and soaring, moved from the Vulture for the Seagulls; a climb
  held down (under leaves) gives up after 10 s and glides down to a perch.
- Hunters (`creature::prey`): a mod can add its creature as a hunter of another mod's prey (`add_hunter`); a hunter of
  the series' mods goes for its prey itself (`quarry`).
- The feather recipes (arrows, the brush, the book and quill) with a mod's feather, moved from the Vulture: they take
  the vulture's and the seagull's feathers (`creature:mod_feathers`).
- Following a friend in flight (`creature::flight::Follow`), from the Vulture, shared with the Seagulls.
- Food on the ground (`creature::food`): the nearest wanted item near a creature, and one of it eaten (the Vulture's
  meat, the Seagulls' fish).
- Small helpers several mods share: `hand::stack`, `particles::puff`, `friend::sign` (hearts or smoke after a try to
  befriend), `mob::is_boat`, `aquatic::water_over`.

## 0.3.1

- Built for Sunrise's 2026-11-20 Nightly (9425651), as 0.3.0.
- Hunters (`creature::prey`): after a catch, the prey's mod may do something to the hunter (`prey::when_caught`): the
  flying fish's dolphin is fed.

## 0.3.0

- Built for Sunrise's 2026-11-20 Nightly (9425651); the mods built on it need this version (`^0.3`). Each package
  lists what it uses of the game's API (`uses.json`).
- Water creatures (`creature::aquatic`): they never drown and don't float to the surface, are beached out of water
  (flopping, drying out), caught in a water bucket into their mod's bucket and let out again, kept for good, and spawn
  in groups of one look under the water of their biomes.
- Touches (`creature::touch`): what touches a creature is hurt by it at most once every so many ticks.
- Hunters (`creature::prey`): a hunter within range goes for its prey and eats it (sea turtles and jellyfish).
- `mob::player_of`; the creative tab edits its contents through the game's new edit handle.

## 0.2.0

- Built for Sunrise's 2026-11-19 Nightly (520fc48); the mods built on it need this version (`^0.2`).
- The lights of glowing creatures (fireflies, fairies, storm spirits) share the game's 8 fairly: the creatures
  nearest a player get them, instead of one mod taking every slot.
- Fighting back is the library's (`creature::anger`), shared by the scorpions and the storm spirits; with
  `give_effect`, `nearest_player` and `distance`.
- The glass jar places each creature itself (`Species::draw_scale`, `draw_lift`).

## 0.1.1

- First release on Sunrise. (0.1.0 was never published: 0.1.1 is the library's first version players get.)
