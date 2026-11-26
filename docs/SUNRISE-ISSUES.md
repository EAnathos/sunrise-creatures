# Sunrise issues

The issues we raised or follow on [Sunrise](https://github.com/sunrise-org/sunrise), and the workaround each one left in
our code (marked there with its issue). Update this page when an issue moves, and before moving the series' pin
(below). Issues go to [sunrise-org/sunrise](https://github.com/sunrise-org/sunrise/issues): search it first, open
and closed, and file only with a clean repro and the pinned revision. A fixed issue whose workaround is gone leaves
this page (its history is in git).

Our pin: Nightly `nightly-2026-11-20-9425651`, source `94256510f7` (every `sunrise-mod.toml`'s `[native]` pins and
CI's `SUNRISE_REV`), still the newest Nightly. Last checked: 2026-11-26 (each issue's state on the tracker, whether
its fix is in our pin, and that each workaround is still in the code where this page says).

Status: **to open** (found, not filed), **open**, **fixed upstream** (merged, not yet in our pin).

## Open

| Issue | What | Status | Our workaround (where) | Drop it when fixed |
|---|---|---|---|---|
| [#1134](https://github.com/sunrise-org/sunrise/issues/1134) (not ours, we follow it) | `entity_size` isn't asked for a client's copies of entities: there, our creatures keep their template's box (a wandering trader's, 1.95 high) | fixed upstream (closed 2026-11-23), after our Nightly (9425651) | The book centres our creatures by their own box, offset from the template's height (`common/src/bestiary/book.rs`, `TEMPLATE_HEIGHT`); the cyclops and the magma golem get a client box through `minecraft:scale` (`creature::mob::scale_client_box`); the crosshair finds an outlined creature by its outline (`creature::pick`), which also covers parts reaching out of its box | Drop the offset and the scale once a pinned build has the fix; keep `creature::pick` for the parts out of the box |
| [#3815](https://github.com/sunrise-org/sunrise/issues/3815) | A cursor a mod changes from `menu_clicked` (`MenuRef::cursor`) isn't sent to the client, which shows a ghost stack until its next click | fixed upstream (#3821, `84730b8a47`, closed 2026-11-21), in no Nightly yet | The vulture's screen opens again in place after a throw (`mods/vulture/src/screen.rs`, `clicked`) | Drop the reopening |
| [#3816](https://github.com/sunrise-org/sunrise/issues/3816) | A mod's living type can't be disallowed in Peaceful: the egg's "Disabled in Peaceful", `/summon` and spawns follow the template | fixed upstream (#3824, `53df669995`: `"allowed_in_peaceful": false` beside `template`, closed 2026-11-21), in no Nightly yet | The cyclops goes in Peaceful by itself, and its egg's line comes from a static its server tick keeps (`mods/cyclops/src/lib.rs`, `PEACEFUL`), single player only | Use the new key, drop the static |
| [#6099](https://github.com/sunrise-org/sunrise/issues/6099) | A living entity's renderer gets no synced attachments (`EntityRenderState::attachments` is filled for general entity types only), so a creature can't show its server's state on another computer's client | open (filed 2026-11-26; the same on `main` `d20e96bdbaf`) | Single player: the renderers read the server's statics by entity id (`common/src/state.rs`; docs/ARCHITECTURE.md §8) | Move the renderers' reads to a synced attachment (§8) |
| [#6100](https://github.com/sunrise-org/sunrise/issues/6100) | A cast fishing bobber is no entity a mod can find (commands, `entities_in`), and nothing reads a player's hook (`Player.fishing`) | open (filed 2026-11-26) | The seagull guesses the bobber from the cast, about 6 blocks ahead along the player's look on the water (`mods/seagulls/src/lib.rs`, `CASTS`, `bobber`) | Read the real bobber |

## Limits of our workarounds

| Workaround | Its limits |
|---|---|
| The book's offset (#1134) | Reads our box from the library's table (`creature::mob::size_of`), not from the entity |
| The vulture's screen opened again after a throw (#3815) | What is left on the cursor after a right-click throw goes back to the inventory |

## To open (candidates)

Found, not filed: check the tracker and the newest Nightly first. None today.

## Moving the pin

1. Pick the build and its commit: the newest Nightly or Dev build (`sunrise-launcher versions`, or the `nightly`
   and `dev-*` releases of sunrise-org/sunrise), or the first with a fix we wait for (`git log <old>..<new>` in the
   Sunrise sources).
2. `cargo creature pin <commit>` (without a commit: the current Nightly). Sunrise's `cargo sunrise-mod pin` repins
   the library and lists the API items it uses that changed; the tool then writes its `[native]` in every module, CI's
   `SUNRISE_REV` and CONTRIBUTING's `REV`. Install `sunrise-stage` at it (CONTRIBUTING.md; `cargo sunrise-mod` is
   the newest signed tool, whatever the pin); `cargo creature native-check` fetches the new SDK by itself (it compares
   `target/sunrise-sdk/sdk.json` with the library's pin) and shows what fails to build.
3. Install that build and keep the instance on it (`sunrise-launcher install <build>`,
   `sunrise-launcher instance version <instance> <build>`), or `cargo creature install` refuses the mods.
4. `cargo creature check`, `cargo creature install`; drop the workarounds it fixes, update the tables and this page's
   pin line; test in game.
5. A released package is made for the build it pins (the registry accepts a release only for a build Sunrise
   published, and offers it for the builds it fits): the modules reach players on the new pin with new versions
   (`cargo creature bump`, docs/PUBLISHING.md).
