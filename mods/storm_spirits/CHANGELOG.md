# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.2.2

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.

## 0.2.1

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md` in the
  package).

## 0.2.0

- Hit, it calls lightning down on you every few seconds (sparks over your head first, a second before the bolt).
- Tougher: 60 health (was 20), and its shock and touch hurt twice as much (6 and 2).
- Its own sounds: zaps, the buzz of its dashes, a crackle at its birth and fading, sparks before a bolt.
- Built for Sunrise's 2026-11-20 Nightly and Sunrise Creatures 0.3.

## 0.1.0

- First release on Sunrise: the storm spirit, born of weather lightning, and its spark.
