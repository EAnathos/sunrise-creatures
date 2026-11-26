# Publishing the series

The series is published on Sunrise's registry ([playsunrise.net/mods](https://playsunrise.net/mods), its data in
[sunrise-org/sunrise-mods](https://github.com/sunrise-org/sunrise-mods)), as Sunrise's guide says
([Publishing your mod](https://playsunrise.net/docs/publish-a-mod)): each module is its own project, a `native-mod`,
registered by its package folder (`common`, `mods/<id>`; the repository's registration is sunrise-mods'
`registry/repositories/1449407242.toml`). What a listed project promises is that registry's
[MOD-POLICY.md](https://github.com/sunrise-org/sunrise-mods/blob/main/MOD-POLICY.md): the right to publish everything
in it under its licence, no Minecraft game files (our packages hold only our own namespaces), nothing hidden. Ours:
the code under PolyForm Noncommercial 1.0.0, the artwork the author's under its own terms, the sounds CC0; the
manifests' `license` is `PolyForm-Noncommercial-1.0.0 AND LicenseRef-SunriseCreatures-Art`. The registry asks for
nothing more, and PolyForm Noncommercial only that a copy comes with its terms or their URL and the Required Notice:
so each published module's README, which its package carries and its page shows, ends with the series' licence section
(the Required Notice and the two licences' links, `tools/src/release.rs`, `LICENCE`; `cargo creature check` holds it),
rather than copies of `LICENSE` and `LICENSE-ART.md` in every folder.

## How a release works

A release is a tag, `<id>/v<version>` (`creature/v0.3.5`, `vulture/v0.2.4`), pushed to GitHub. It runs the module's
workflow, `.github/workflows/sunrise-release-<id>.yml` (written by `cargo sunrise-mod release setup`, which rewrites it
when the registry's shared workflow moves: never edit it by hand), which calls the registry's shared release workflow:

1. it checks out the tagged commit and runs `cargo sunrise-mod check --release <tag>` then `build` in the module's
   folder, as it is in git (our `cargo creature pack` plays no part): the `.smod` holds the source, its `Cargo.lock`
   and `uses.json`, never a compiled library, and a page bundle comes beside it;
2. a separate job attests both files (a GitHub artifact attestation naming this repository and the shared workflow)
   and publishes them as a GitHub release of this repository, named after the tag;
3. the registry's bot sees the release within the hour and opens a pull request on sunrise-mods (the release and the
   project's page); its checks run (formats, hashes, attestations, a scan of the source, a build on Linux and a
   compile check on Windows and macOS), a reviewer reads the code for malware, and once merged the release is listed
   for the game builds it fits. Players' launchers build it from its source for their own game.

A mod's build fetches the library from the registry (`[dependencies] creature`): **a mod is released only once the
library version it needs is listed**, or its workflow fails at the check (`creature ^0.3.5: no project in the Sunrise
mod registry provides it`). Release the library first, wait for its pull request to merge, then the mods.

## What a published module has

Everything the workflow needs is committed in the module's folder:

- **`module.toml`'s `[registry]`** (the repository's, not packaged): its one-line `summary` (25 to 160 characters),
  1 to 8 `categories` from a mod's list (`tools/src/release.rs`, `CATEGORIES`, as sunrise-mods'
  `registry/categories.toml`), its `icon` (one of its models, its picture in `docs/models/`, or a PNG by its path) and
  its `screenshots` (1 to 8, PNG, JPEG, or a WebP the tool turns into a JPEG; at most 5 MiB each). A module without
  the table is not published (the planned mods).
- **Its release files**, written from those by `cargo creature pages` (never by hand; `cargo creature check` fails
  when one is stale): `page.toml` (the summary, the categories, the pictures, and the README as the description: its
  title and its image lines left out, its links made absolute at the version's tag; a 0.x version's page opens with a
  "Beta" line, the series being in beta until 1.0), `page/<n>-<name>.png|.jpg` (the pictures) and `icon.png` (its
  page's and the game's Mods screen's).
- **`README.md`** ending with the series' licence section (above).
- **`Cargo.lock`**, committed: a native release carries it, and a stale one fails the registry's scan. Cargo writes it
  (`cargo generate-lockfile` in the folder, once `cargo sunrise-mod check` has fetched the SDK).
- **`sunrise-mod.toml`**: its `name`, `version`, `authors`, `license`, `description`, `homepage` and `issues`, its
  `[native]` pin (the registry accepts a release only for a build Sunrise published) and its `[dependencies]`.
- **`CHANGELOG.md`**: a section per version (`## 0.1.0`).
- **Its release workflow** (above): `cargo creature check` fails without it.

## Releasing a version

You need the newest `cargo sunrise-mod` (the `mod-tool` release: "Newest tool from main" in Sunrise's
[Install the mod tool](https://github.com/sunrise-org/sunrise/blob/main/docs/modding/src/start/install-tool.md);
`cargo sunrise-mod self-update` keeps it current).

1. **Version.** `cargo creature bump <patch|minor|major> <module...>`: it raises the version in the module's
   `sunrise-mod.toml` (its only version: every crate's `Cargo.toml` stays `0.0.0`), adds its `CHANGELOG.md` section
   and writes its release files again (the page's links follow the version's tag). Write the section (`cargo creature
   check` refuses the placeholder the command leaves), and raise a mod's `[dependencies] creature` to the library
   version whose API it uses. A released version never changes: any change, even one byte, is a new version.
2. **Check.** `cargo creature check` and `cargo creature native-check`, and try the module in game
   (`cargo creature install`). Commit, and push `main`.
3. **Tag.** `cargo creature release [module...]` (default: every published module): on a clean checkout, for each
   version not tagged yet, it requires the changelog section, the release files up to date and the lock committed,
   runs `cargo sunrise-mod check --release <tag>` (the workflow's first step), and tags. It prints the pushes.
4. **Push the tags one at a time** (GitHub starts no workflow for more than three tags pushed at once, nor for a tag
   pushed with the commit that adds its workflow): the library's first; the mods' once the library's release is
   listed. Watch each run under the repository's Actions, then the registry's pull request on sunrise-mods.

Sunrise's own `cargo sunrise-mod release patch` does steps 1, 3 and 4 for a single package, but writes no changelog
section nor page, and pushes the tag with the branch: use `cargo creature bump` and `cargo creature release`.

A release whose workflow failed can be run again: delete its tag (`git tag -d <tag>`,
`git push origin :refs/tags/<tag>`) and its draft GitHub release if one was made, fix, commit, then tag again. Once
the release is published (the GitHub release made), its version is spent.

## The page

A project's page (its summary, description, categories, pictures, icon) comes with each release: the registry's pull
request for a release updates it. Change it in `module.toml`, the README or the pictures, run `cargo creature pages`,
and it is shown with the next version.

## The collection

The collection is maintained in sunrise-mods' `registry/collections/<id>/` (`collection.toml` names its member
projects, alongside an icon). All 16 mods and the library are listed in the registry. The Sunrise Creatures
collection (`4pggl9jF`) is present in the signed index; registry maintainers added it in [#325](https://github.com/sunrise-org/sunrise-mods/pull/325),
temporarily removed it in [#329](https://github.com/sunrise-org/sunrise-mods/pull/329) while expecting our collection
PR to merge, then restored it in [#330](https://github.com/sunrise-org/sunrise-mods/pull/330). Our duplicate PR
[#328](https://github.com/sunrise-org/sunrise-mods/pull/328) was closed without merging.

## Updates and takedowns

An update is a new version through the same steps. A released file can't be replaced or removed; a bad one is taken
down by the registry (`registry/revoked.toml` on sunrise-mods), which turns it off in players' launchers.

## Released

On the new registry (since 2026-11-24): each release is the GitHub release `<id>/v<version>` of this repository.

| Version | What | Registry |
|---|---|---|
| library 0.3.5, Butterflies 0.1.3, Cyclops 0.1.4, Dung Beetles 0.1.3, Fairies 0.1.4, Fireflies 0.1.3, Flying Fish 0.1.3, Jellyfish 0.1.2, Ladybugs 0.1.1, Phasmids 0.1.1, Scorpions 0.1.3, Seagulls 0.1.2, Sporelins 0.1.2, Storm Spirits 0.2.1, Vanilla Bestiary 0.1.3, Vulture 0.2.4, Worms 0.1.4 | The first releases on the new registry: locks, tree climbers, the licence in every package | [#269](https://github.com/sunrise-org/sunrise-mods/pull/269) (library), [#278](https://github.com/sunrise-org/sunrise-mods/pull/278) (Butterflies), [#306–#320](https://github.com/sunrise-org/sunrise-mods/pull/306) (other mods), all merged; collection [#330](https://github.com/sunrise-org/sunrise-mods/pull/330), merged |

On the old registry (2026-11-18 to 2026-11-24, deleted with the new launcher; its entries, signed with the series'
minisign key, are in this repository's history, `registry/` before 2026-11-24):

| Version | What | Registry |
|---|---|---|
| library 0.1.1, the eight mods 0.1.0 (2026-11-18) | First releases, signed; GitHub releases `<id>-<version>` (pre-releases) | [sunrise-org/sunrise-mods#10](https://github.com/sunrise-org/sunrise-mods/pull/10), merged 2026-11-18; the collection [#13](https://github.com/sunrise-org/sunrise-mods/pull/13), made by the registry's maintainers |
| library 0.2.0, Vulture 0.2.0, Storm Spirits 0.1.0 (new), the other mods 0.1.1 (2026-11-19) | Sunrise's 2026-11-19 Nightly, the vulture's screen, Storm Spirits; signed (one password per run) | [sunrise-org/sunrise-mods#19](https://github.com/sunrise-org/sunrise-mods/pull/19), merged 2026-11-19; Storm Spirits into the collection: [#20](https://github.com/sunrise-org/sunrise-mods/pull/20) |
| library 0.3.0, Jellyfish 0.1.0 (new), Storm Spirits 0.2.0, Vulture 0.2.1, the other mods 0.1.2 (2026-11-21) | Sunrise's 2026-11-20 Nightly, water creatures, the Jellyfish, packages with `uses.json` | [sunrise-org/sunrise-mods#34](https://github.com/sunrise-org/sunrise-mods/pull/34), merged; the Jellyfish into the collection: [#35](https://github.com/sunrise-org/sunrise-mods/pull/35) |
| library 0.3.1, Flying Fish 0.1.0 (new), Vulture 0.2.2 (2026-11-21) | `prey::when_caught`, the Flying Fish, the Vulture's fixes from #34's review | [sunrise-org/sunrise-mods#36](https://github.com/sunrise-org/sunrise-mods/pull/36), merged 2026-11-20; the Flying Fish into the collection: [#37](https://github.com/sunrise-org/sunrise-mods/pull/37) |
| library 0.3.2, Seagulls 0.1.0 (new), Vulture 0.2.3, Flying Fish 0.1.1, Jellyfish 0.1.1, Fairies 0.1.3 (2026-11-22) | The Seagulls; what two mods shared moved to the library (flight, food on the ground, feather recipes, helpers) | [sunrise-org/sunrise-mods#50](https://github.com/sunrise-org/sunrise-mods/pull/50), merged 2026-11-21; the Seagulls into the collection: [#51](https://github.com/sunrise-org/sunrise-mods/pull/51) |
| library 0.3.3 and 0.3.4, Ladybugs 0.1.0, Phasmids 0.1.0, Dung Beetles 0.1.0 and 0.1.1, Sporelins 0.1.0 (new), Flying Fish 0.1.2, Seagulls 0.1.1 (2026-11-22) | Four new mods; creeping, the tree and pasture rules, filled jars in the creative tab; the fish and the seagulls wander, the seagulls along the coast; in-game pictures on every creature mod's page (the review asked) | [sunrise-org/sunrise-mods#55](https://github.com/sunrise-org/sunrise-mods/pull/55), merged 2026-11-22; the four new mods added to the collection by the registry's maintainers |
| The new licence on every page (2026-11-22) | Code PolyForm Noncommercial 1.0.0, artwork the author's | [sunrise-org/sunrise-mods#67](https://github.com/sunrise-org/sunrise-mods/pull/67), merged 2026-11-22 |
| Worms 0.1.3, Cyclops 0.1.3, Dung Beetles 0.1.2, Sporelins 0.1.1 (2026-11-22) | Chickens eat worms; a beetle's ball to the player who took it; a sporelin's stew day saved; the licence files in the packages | [sunrise-org/sunrise-mods#68](https://github.com/sunrise-org/sunrise-mods/pull/68), merged 2026-11-22 |
