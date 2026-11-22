# Publishing the series

The series is published on Sunrise's registry ([playsunrise.net/mods](https://playsunrise.net/mods)), as Sunrise's
guide says ([Publishing a mod](https://playsunrise.net/docs/publish-a-mod)): each module is its own project, a
`native-mod`, whose entry lives in [sunrise-org/sunrise-mods](https://github.com/sunrise-org/sunrise-mods)
(`mods/<id>.toml`, `icons/<id>.png`) and whose files are pinned there by their SHA-256. Every change to an entry is a
pull request a person reviews. What a listed project promises is that registry's
[MOD-POLICY.md](https://github.com/sunrise-org/sunrise-mods/blob/main/MOD-POLICY.md): the right to publish everything
in it under its licence, no Minecraft game files (our packages hold only our own namespaces), nothing hidden. Ours:
the code under PolyForm Noncommercial 1.0.0, the artwork the author's under its own terms, the sounds CC0 (the
registry accepts noncommercial licences); the manifests' `license` is
`PolyForm-Noncommercial-1.0.0 AND LicenseRef-SunriseCreatures-Art`, and each package carries `LICENSE` and
`LICENSE-ART.md`.

## What a published module has

- **`module.toml`'s `[registry]`**: its one-line `summary` (at most 160 characters), 1 to 3 `categories` from a mod's
  list (`tools/src/release.rs`, `CATEGORIES`), up to 10 `tags` (lowercase letters, digits and `-`, at most 24 characters), and its `icon`: one of its models (its
  picture in `docs/models/`) or a PNG by its path (the library's and the Vanilla Bestiary's: the guide). A module
  without the table is not published (the planned mods).
- **`sunrise-mod.toml`**: its `name`, `version`, `authors`, `license`, `description`, `homepage` and `issues`, shown by
  the game's Mods screen; the package also carries the icon (`icon.png`, made by `cargo creature pack`).
- **`README.md`**: its registry page's description (50 to 10,000 characters; its title left out, its links made
  absolute to this repository on GitHub at the version's tag). The registry's pages show no image in their text (an
  image comes out as `![](...)`), so image lines are left out: the pictures are the screenshots below.
- **`[registry] screenshots`**: up to 8 pictures for the page, by their paths (PNG, JPEG, or a WebP the tool turns into
  a JPEG), written to `registry/screenshots/<id>/`: the in-game panel of the README when there is one, then the
  models' pictures. A 0.x version's page opens with a "Beta" line: the series stays in
  beta until 1.0.
- **`CHANGELOG.md`**: a section per version (`## 0.1.0`), its changelog in the registry and its GitHub release's notes.

`cargo creature check` holds every published module to these rules (the registry's own), so a page that would be
refused fails before release.

## Releasing a version

1. **Version.** Raise `version` in the module's `sunrise-mod.toml` and `Cargo.toml` (semver: a mod's
   `[dependencies] creature = "^0.3"` follows the library's), and write its `CHANGELOG.md` section. A published file
   never changes: any change, even one byte, is a new version.
2. **Check.** `cargo creature check` and `cargo creature native-check`, and try the module in game
   (`cargo creature install`).
3. **Release.** `cargo creature release [module...]` (default: every published module): it builds each package,
   checks it as the registry does (`cargo sunrise-mod check`), names it `target/release/<id>-<version>.smod`, signs it
   if the series has a key (below), and writes its entry `registry/mods/<id>.toml` (the new version first, the old ones
   kept) and its icon `registry/icons/<id>.png`. A version already published with another file is refused. Signing
   asks the key's password once, at the start of the run (a wrong one stops it before anything is built); the
   modules already signed are kept, so a stopped run goes on with `cargo creature release` again. Commit `registry/` with the version.
4. **GitHub release.** One release of this repository per module version, tag `<id>-<version>`, the `.smod` (and its
   `.minisig`) as its asset, marked a pre-release while the series is in beta (0.x): the command prints each
   `gh release create` line (add `--prerelease`).
5. **Registry pull request.** In a fork of sunrise-org/sunrise-mods, copy `registry/mods/<id>.toml` and
   `registry/icons/<id>.png`, commit with a sign-off (`git commit -s`: the registry's `dco` check), and open a pull
   request whose description links each file (its GitHub release asset) with its SHA-256 (the command prints both). A
   maintainer copies the file to the registry's staging release; its checks run; a reviewer merges. The submission
   form (playsunrise.net/mods/submit) opens the same pull request once its service runs. Questions come as comments on
   the pull request.

   Before opening it, run the registry's own checks on the fork's branch (`.github/workflows/check.yml` there): with
   `base` a checkout of sunrise-mods' `main`, `head` the branch and `files/` holding each new file named by its SHA-256,

   ```bash
   cargo sunrise-mod registry review base head --author EAnathos
   cargo sunrise-mod registry check base head --files files
   ```

   Sunrise's modding book still warns against submitting code mods (a note from when the mod API was the fork's); the
   registry's own README takes native mods, and lists some: go by the registry.

The library goes first: every mod requires it (`[dependencies] creature`). The registry reads that from the package
(each version's `requires` in its index), and the launcher installs the library with any of the mods.

A page's links and images point at the version's tag (`<id>-<version>`), which never moves, not at `main`: the
registry's review asked for it (the 0.1.x pages still point at `main`).

## The collection

The registry shows the series as one card too, the collection `sunrise_creatures` (its `collections/<id>.toml`, owned
by EAnathos, mirrored in `registry/collections/`): a player installs all its members at once, or picks some. It is no
package: each member keeps its own entry, versions and updates. A new published module joins it with a pull request
adding its id to `members` (after the module itself is listed; its commit signed off, `git commit -s`, as every
registry commit); owning both, the review marks it routine. The
collection's own page (name, summary, description, icon, defaults, member order) changes by a pull request a reviewer
reads.

## Built from source (Nightly)

Since launcher 1.1.4, the registry also lists native mods by their source (`sources/<id>.toml` in sunrise-mods,
published as the Nightly list `sources.json`), and a player on Nightly can build a mod straight from it on their
computer, after a warning that the code isn't reviewed. Every module of the series has such a listing (added by the
registry's maintainers): `repo = "EAnathos/sunrise-creatures"`, its manifest's path, its icon, and
`track = { branch = "main" }`. So:

- **`main` is what Nightly players build.** Push to it only code that builds and works: a module half done, or a mod
  whose code changed without its version raised, reaches them as it is. Raise a module's version when its code
  changes (the launcher offers an update by version), even before its registry release.
- **The pin follows the Nightly.** A source build compiles against the player's own game build, and the launcher shows
  whether a mod is made for it from the manifest's `[native]` pins: when Sunrise publishes a new Nightly, move the
  series' pin to it promptly (docs/SUNRISE-ISSUES.md, "Moving the pin"), or source builds fall out of step.
- **A new module** gets its listing with a pull request adding `sources/<id>.toml` (the format is in sunrise-mods'
  README, "Source mods"), beside its `mods/<id>.toml`; a collection appears in the Nightly list only when every member
  has a listing.
- The reviewed files of `mods/` stay what Stable players install, and what the launcher also offers on Nightly.

## Published

| Version | What | Registry |
|---|---|---|
| library 0.1.1, the eight mods 0.1.0 (2026-11-18) | First releases, signed; GitHub releases `<id>-<version>` (pre-releases) | [sunrise-org/sunrise-mods#10](https://github.com/sunrise-org/sunrise-mods/pull/10), merged 2026-11-18; the collection [#13](https://github.com/sunrise-org/sunrise-mods/pull/13), made by the registry's maintainers |
| library 0.2.0, Vulture 0.2.0, Storm Spirits 0.1.0 (new), the other mods 0.1.1 (2026-11-19) | Sunrise's 2026-11-19 Nightly, the vulture's screen, Storm Spirits; signed (one password per run) | [sunrise-org/sunrise-mods#19](https://github.com/sunrise-org/sunrise-mods/pull/19), merged 2026-11-19; Storm Spirits into the collection: [#20](https://github.com/sunrise-org/sunrise-mods/pull/20) |
| library 0.3.0, Jellyfish 0.1.0 (new), Storm Spirits 0.2.0, Vulture 0.2.1, the other mods 0.1.2 (2026-11-21) | Sunrise's 2026-11-20 Nightly, water creatures, the Jellyfish, packages with `uses.json` | [sunrise-org/sunrise-mods#34](https://github.com/sunrise-org/sunrise-mods/pull/34), merged; the Jellyfish into the collection: [#35](https://github.com/sunrise-org/sunrise-mods/pull/35) |
| library 0.3.1, Flying Fish 0.1.0 (new), Vulture 0.2.2 (2026-11-21) | `prey::when_caught`, the Flying Fish, the Vulture's fixes from #34's review | [sunrise-org/sunrise-mods#36](https://github.com/sunrise-org/sunrise-mods/pull/36), merged 2026-11-20; the Flying Fish into the collection: [#37](https://github.com/sunrise-org/sunrise-mods/pull/37) |
| library 0.3.2, Seagulls 0.1.0 (new), Vulture 0.2.3, Flying Fish 0.1.1, Jellyfish 0.1.1, Fairies 0.1.3 (2026-11-22) | The Seagulls; what two mods shared moved to the library (flight, food on the ground, feather recipes, helpers) | [sunrise-org/sunrise-mods#50](https://github.com/sunrise-org/sunrise-mods/pull/50), merged 2026-11-21; the Seagulls into the collection: [#51](https://github.com/sunrise-org/sunrise-mods/pull/51) |
| library 0.3.3 and 0.3.4, Ladybugs 0.1.0, Phasmids 0.1.0, Dung Beetles 0.1.0 and 0.1.1, Sporelins 0.1.0 (new), Flying Fish 0.1.2, Seagulls 0.1.1 (2026-11-22) | Four new mods; creeping, the tree and pasture rules, filled jars in the creative tab; the fish and the seagulls wander, the seagulls along the coast; in-game pictures on every creature mod's page (the review asked) | [sunrise-org/sunrise-mods#55](https://github.com/sunrise-org/sunrise-mods/pull/55), merged 2026-11-22; the four new mods added to the collection by the registry's maintainers |
| The new licence on every page (2026-11-22) | Code PolyForm Noncommercial 1.0.0, artwork the author's | [sunrise-org/sunrise-mods#67](https://github.com/sunrise-org/sunrise-mods/pull/67), merged 2026-11-22 |
| Worms 0.1.3, Cyclops 0.1.3, Dung Beetles 0.1.2, Sporelins 0.1.1 (2026-11-22) | Chickens eat worms; a beetle's ball to the player who took it; a sporelin's stew day saved; the licence files in the packages | [sunrise-org/sunrise-mods#68](https://github.com/sunrise-org/sunrise-mods/pull/68), in review |

## Signing (recommended)

A signed project can only be updated with its key. Once:

```bash
minisign -G -p registry/minisign.pub -s ~/.minisign/sunrise-mods.key
```

`registry/minisign.pub` is committed (key `EBB079C007FD597B`, made 2026-11-18); the secret key
(`~/.minisign/sunrise-mods.key`) and its password stay private and backed up (never in the repository): without them,
no version of these projects can be published again. From then on `cargo creature release` signs every file: set `SUNRISE_MODS_KEY` to the secret key's path.
Every version of a project with a key must be signed, by the same key.

## Changing a page

A page (its text, icon, screenshots, summary, categories, tags) belongs to the project, not to a version: it changes
without a new version. `cargo creature pages [module...]` writes the published modules' entries again from their
README and `module.toml`, every version as it is, then a pull request to sunrise-mods carries them as for a release
(no file to stage). The README inside a released package doesn't change: the next version takes the new one.

## Updates and takedowns

An update is a new version through the same steps: its `[[versions]]` table is added on top of the entry. A file
can't be replaced or removed; a bad one is taken down by the registry (`revoked.toml`), which turns it off in players'
launchers.
