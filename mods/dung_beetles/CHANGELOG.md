# Changelog

Each version's section says what changed in it (`cargo creature release` needs one before it tags the version).

## 0.1.3

- Released through Sunrise's new registry: built from this repository by the registry's release workflow, attested by
  GitHub, and reviewed before it is listed.
- Its shared state is locked the library's way (`creature::sync`), never held while it calls the game. Needs Sunrise
  Creatures 0.3.5.

## 0.1.2

- A beetle's ball goes to the player who took it (it went to the nearest one).
- The code is under PolyForm Noncommercial 1.0.0, the artwork the author's (`LICENSE`, `LICENSE-ART.md`).

## 0.1.1

- Built on the library's `creep::too_tall`, unchanged.
- Needs Sunrise Creatures 0.3.4.

## 0.1.0

- First release on Sunrise: dung beetles in savannas, plains, meadows and at the edges of deserts that follow the
  grazing herds, shape balls where the animals stood, roll them backwards in straight lines (perfectly straight under a
  clear night sky) and bury them, turning coarse dirt into dirt, dirt into grass and grass into flowers; take a
  rolling beetle's dung ball to do it by hand or fill a composter; chickens peck them up; caught in the glass jar, they
  settle in a pasture.
- Needs Sunrise Creatures 0.3.3.
