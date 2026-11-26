# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.1.4

- Drops vanilla feathers instead of a Seagull-specific item. Needs Sunrise Creatures 0.4.2.

## 0.1.3

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.

## 0.1.2

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md` in the
  package).

## 0.1.1

- A seagull roams the coast: its circle drifts along a random way and it lands on any side (they all went the same
  way).
- Needs Sunrise Creatures 0.3.3.

## 0.1.0

- First release on Sunrise: seagulls on beaches and stony shores that soar along the coast, float on the water, follow
  boats and snatch food from hands; befriended with a raw fish, one follows you, dives for fish for you and circles over
  a lucky fishing spot; over warm seas they catch gliding flying fish (with the Flying Fish); feathers and meat; their
  own sounds. Needs Sunrise Creatures 0.3.2.
