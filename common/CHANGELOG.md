# Changelog

Each version's section is its registry changelog (`cargo creature release`).

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
