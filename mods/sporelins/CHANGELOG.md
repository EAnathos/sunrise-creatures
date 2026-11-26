# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.1.3

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.
- It comes out under a dark forest's canopy, where it is dark, as its spec says (the spawning stopped at the leaves).
- It comes out at night, in the rain too, and never by day.

## 0.1.2

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.

## 0.1.1

- A sporelin remembers the day it last filled a bowl across a reload (it was forgotten, so a reload gave a second
  stew that day).
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md`).

## 0.1.0

- First release on Sunrise: sporelins, red and brown caps, in dark forests, mushroom fields, old growth taigas and
  swamps, in the dark; they hide as mushrooms from a player who comes near, puff spores when startled and plant
  mushrooms; befriended with a mushroom of their colour, one follows you or stays and tends a mushroom patch, fills an
  empty bowl with stew once a day and glows softly at night; killed, a mushroom of its colour.
- Needs Sunrise Creatures 0.3.4.
