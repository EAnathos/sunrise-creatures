# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.1.5

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.

## 0.1.4

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.

## 0.1.3

- Chickens peck worms up: a chicken within 6 blocks walks over and eats one, wild or settled in a garden; a new fact
  in the guide, "Chicken".
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md`).

## 0.1.2

- Built for Sunrise's 2026-11-20 Nightly (9425651) and Sunrise Creatures 0.3; the package lists what it uses of the game's API (`uses.json`).

## 0.1.1

- Built for Sunrise's 2026-11-19 Nightly and Sunrise Creatures 0.2.

## 0.1.0

- First release on Sunrise.
