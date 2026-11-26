# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.2.6

- Drops vanilla feathers instead of a Vulture-specific item.
- Its stats (its health and flight speed) are the library's (`creature::stats`, saved as `creature:stats`): a vulture
  from before keeps its own, moved over once; its rider and their keys are the library's too (`creature::ride`).
  Needs Sunrise Creatures 0.4.3.

## 0.2.5

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.

## 0.2.4

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its state is saved without holding its lock while the game is called, and dropped once it leaves the level.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md` in the
  package).

## 0.2.3

- Its flight and soaring are the library's now (shared with the Seagulls), unchanged; a climb held down under leaves
  gives up after 10 s and glides down to a perch instead of hanging there.
- Its feather's recipes (arrows, the brush, the book and quill) are the library's now. Needs Sunrise Creatures 0.3.2.
- Following its owner and finding meat on the ground are the library's now, unchanged.

## 0.2.2

- The server keeps the riding keys only of a player riding a vulture, forgotten when they get off or leave; a talons
  press moves only the vulture its sender rides (both from the registry's review, sunrise-org/sunrise-mods#34).

## 0.2.1

- Built for Sunrise's 2026-11-20 Nightly (9425651) and Sunrise Creatures 0.3; the package lists what it uses of the game's API (`uses.json`).

## 0.2.0

- Its screen works as a chest's: pick up, put down, half and one with a right-click, swap, shift-click, number keys
  and the off hand's key, the drop key, double-click to gather, dragging, the creative middle-click, throwing outside
  the window. Before, a click moved a whole stack across, with no cursor.
- Its screen closes when the vulture dies.
- Built for Sunrise's 2026-11-19 Nightly and Sunrise Creatures 0.2.

## 0.1.0

- First release on Sunrise.
