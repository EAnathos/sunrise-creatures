# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.1.4

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.

## 0.1.3

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md` in the
  package).

## 0.1.2

- A shoal wanders about: each fish's heading veers and turns now and then (they all swam the same way).
- A frightened fish takes off only along open water, turns along the water as a shore nears and drops back in before
  land: it no longer glides onto the shore and dies there.
- Needs Sunrise Creatures 0.3.3.

## 0.1.1

- A seagull (with the Seagulls) frightens a shoal, as a dolphin does.
- Built on the library's shared helpers (Sunrise Creatures 0.3.2), unchanged.

## 0.1.0

- First release on Sunrise: blue wing, spotted wing and sunset wing flying fish in shoals of one kind in lukewarm and
  warm oceans; they burst out of the water and glide over the waves when frightened; the bucket, raw and cooked flying
  fish, fished with a rod in their oceans and eaten by cats; dolphins hunt them and are fed by a
  catch; a glider bounces off what it runs into; its own sounds. Needs Sunrise Creatures 0.3.1.
