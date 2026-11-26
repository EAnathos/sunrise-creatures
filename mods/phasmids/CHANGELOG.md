# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.1.2

- What it keeps about each creature lives in the library's `creature::state`, emptied when the creature leaves the
  level; its settings of the library's systems end with `..Type::DEFAULT`, so a later library doesn't break it. Needs
  Sunrise Creatures 0.4.0.
- The licences are in the README (its last section) rather than as copies of `LICENSE` and `LICENSE-ART.md`.

## 0.1.1

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Released on a trunk, or stopped on one, it clings to the bark, drawn upright, head up, instead of dropping off; one
  on the ground under its tree walks to the trunk and climbs back to the canopy.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md` in the
  package).

## 0.1.0

- First release on Sunrise: stick insects (oak, birch, spruce, jungle) and leaf insects (green, dry) on the leaves of
  their trees, still by day and crawling over the canopy at night, up its steps and trunks; spot them by looking at
  them a moment, counted on your guide; hit, they drop and play dead, then climb back up their tree; caught in the
  glass jar, they settle on a tree, shown in it on a twig; a faint rustle as they crawl, a dry click as they drop.
- Needs Sunrise Creatures 0.3.3.
