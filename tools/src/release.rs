//! Releases on Sunrise's registry (docs/PUBLISHING.md; the registry is sunrise-org/sunrise-mods). A module is
//! published when its `module.toml` has a `[registry]` table: it is a project registered there by its package folder.
//!
//! A release is a tag, `<id>/v<version>`, pushed to GitHub: the registry's shared workflow (called by
//! `.github/workflows/sunrise-release-<id>.yml`) builds the package from the tagged folder, as it is in git, attests it
//! and publishes it as a GitHub release; the registry's bot then opens its pull request on sunrise-mods, which a person
//! reviews. So the folder holds, committed, what a release needs beside the code (`files`), written by
//! `cargo creature pages` and checked by `cargo creature check`:
//! - `page.toml`: `[registry]`'s summary and categories, the README as the description (`description`), its pictures;
//! - `page/`: the pictures, `[registry]`'s screenshots in their order (`<n>-<name>.png|.jpg`);
//! - `icon.png`: its icon (`icon.rs`), on its page and in the game's Mods screen.
//!
//! The licences travel in its README instead of copies of the repository's files: it ends with `LICENCE`, the code's
//! Required Notice and the links to its licence and the artwork's (PolyForm Noncommercial asks only that a copy comes
//! with those terms or their URL, and the Required Notice), which the package carries and the page shows.
//!
//! Its `Cargo.lock` is committed too, as Cargo writes it.
//!
//! `cargo creature release [module...]` checks each one as the workflow will (`cargo sunrise-mod check --release`),
//! tags it, and prints the pushes: one tag at a time (GitHub starts no workflow for more than three tags pushed at
//! once), the library first.
use std::path::{Path, PathBuf};

use crate::modules::{self, LIBRARY, Module, Registry};

/// The repository, for the page's links.
pub const SOURCE: &str = "https://github.com/EAnathos/sunrise-creatures";
/// Its files, for the README's images (followed by a revision: a tag).
const RAW: &str = "https://raw.githubusercontent.com/EAnathos/sunrise-creatures";

/// A mod's categories (sunrise-mods' `registry/categories.toml`, those for a `native-mod`).
pub const CATEGORIES: &[&str] = &[
    "adventure",
    "cursed",
    "decoration",
    "economy",
    "equipment",
    "food",
    "game-mechanics",
    "library",
    "magic",
    "management",
    "minigame",
    "mobs",
    "optimization",
    "social",
    "storage",
    "technology",
    "transportation",
    "utility",
    "worldgen",
];

/// Most pictures a page shows (ours: the registry takes more), and the biggest it takes (bytes).
const SCREENSHOTS: usize = 8;
const SCREENSHOT_BYTES: u64 = 5 * 1024 * 1024;

/// The published modules named (all of them for none), the library first.
fn published(names: &[String]) -> crate::Result<Vec<Module>> {
    let chosen: Vec<Module> = std::iter::once(modules::library()?)
        .chain(modules::mods()?)
        .filter(|m| m.registry.is_some() && (names.is_empty() || names.contains(&m.id)))
        .collect();
    if chosen.is_empty() {
        return Err("no module named has a [registry] table".into());
    }
    Ok(chosen)
}

/// `cargo creature pages [module...]`: the published modules' release files (`files`) written in their folders, the
/// pictures in `page/` that are no longer listed removed.
pub fn pages(names: &[String]) -> crate::Result {
    for m in published(names)? {
        let files = files(&m)?;
        for (path, bytes) in &files {
            let to = m.dir.join(path);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            if std::fs::read(&to).ok().as_ref() != Some(bytes) {
                std::fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
            }
        }
        for extra in unlisted(&m, &files) {
            std::fs::remove_file(&extra).map_err(|e| format!("{}: {e}", extra.display()))?;
        }
        println!(
            "{}: page written ({} pictures)",
            m.id,
            m.registry.as_ref().map_or(0, |r| r.screenshots.len())
        );
    }
    Ok(())
}

/// What is out of date in published module `m`'s folder (for `cargo creature check`): each release file that differs
/// from what `pages` writes, each picture no longer listed, and a missing `Cargo.lock`.
pub fn stale(m: &Module) -> crate::Result<Vec<String>> {
    let files = files(m)?;
    let rel = |p: &Path| p.strip_prefix(crate::root()).unwrap_or(p).display().to_string();
    let mut out: Vec<String> = files
        .iter()
        .filter(|(path, bytes)| std::fs::read(m.dir.join(path)).ok().as_ref() != Some(bytes))
        .map(|(path, _)| format!("{}: out of date, run `cargo creature pages`", rel(&m.dir.join(path))))
        .collect();
    out.extend(
        unlisted(m, &files)
            .iter()
            .map(|p| format!("{}: not a listed picture, run `cargo creature pages`", rel(p))),
    );
    let readme = m.dir.join("README.md");
    if !std::fs::read_to_string(&readme).is_ok_and(|t| t.trim_end().ends_with(LICENCE.trim_end())) {
        out.push(format!(
            "{}: doesn't end with the series' licence section (`release::LICENCE`)",
            rel(&readme)
        ));
    }
    for copy in ["LICENSE", "LICENSE-ART.md"] {
        if m.dir.join(copy).exists() {
            out.push(format!(
                "{}: not needed, the README carries the licences",
                rel(&m.dir.join(copy))
            ));
        }
    }
    if m.native().is_some() && !m.dir.join("Cargo.lock").is_file() {
        out.push(format!(
            "{}: missing (a native release carries its lock: `cargo generate-lockfile` there)",
            rel(&m.dir.join("Cargo.lock"))
        ));
    }
    Ok(out)
}

/// How every published module's README ends: the licences, which its package carries (as the README) and its page
/// shows. The Required Notice stays a plain line, as PolyForm Noncommercial asks.
pub const LICENCE: &str = "## Licence

Required Notice: Copyright (c) 2026 EAnathos (https://github.com/EAnathos/sunrise-creatures)

The code is under the [PolyForm Noncommercial License 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0).
The artwork (models, textures, icons, pictures) is the author's, all rights reserved, under the terms of
[LICENSE-ART.md](https://github.com/EAnathos/sunrise-creatures/blob/main/LICENSE-ART.md). The sounds are CC0.
";

/// The files in `page/` that aren't among `files`.
fn unlisted(m: &Module, files: &[(PathBuf, Vec<u8>)]) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(m.dir.join("page")) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| !files.iter().any(|(f, _)| m.dir.join(f) == *p))
        .collect();
    out.sort();
    out
}

/// Published module `m`'s release files, by their paths in its folder.
pub fn files(m: &Module) -> crate::Result<Vec<(PathBuf, Vec<u8>)>> {
    let registry = m.registry.as_ref().ok_or_else(|| format!("{}: no [registry]", m.id))?;
    let version = version(m)?;
    let description = beta(&version) + &description(m, &tag(&m.id, &version))?;
    let problems = page_problems(registry, &description);
    if !problems.is_empty() {
        return Err(format!("{}: [registry]: {}", m.id, problems.join("; ")));
    }
    let pictures = pictures(registry)?;
    let names: Vec<String> = pictures.iter().map(|(n, _)| n.clone()).collect();
    let mut files = vec![(
        PathBuf::from("page.toml"),
        page(registry, &description, &names).into_bytes(),
    )];
    files.extend(pictures.into_iter().map(|(n, bytes)| (PathBuf::from(n), bytes)));
    let icon = crate::root().join(format!("target/icons/{}.png", m.id));
    crate::icon::make(&m.id, &registry.icon, &icon)?;
    let bytes = std::fs::read(&icon).map_err(|e| format!("{}: {e}", icon.display()))?;
    files.push((PathBuf::from("icon.png"), bytes));
    Ok(files)
}

/// The module's version, from its `sunrise-mod.toml`.
pub fn version(m: &Module) -> crate::Result<String> {
    let path = m
        .manifest()
        .ok_or_else(|| format!("{} has no sunrise-mod.toml", m.id))?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let t: toml::Table = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    t.get("version")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("{}: no version", path.display()))
}

/// A module's release tag: `<id>/v<version>`, as the shared workflow takes for a package in a subfolder.
pub fn tag(id: &str, version: &str) -> String {
    format!("{id}/v{version}")
}

/// `page.toml`: the summary, the description, the categories and the pictures (in `page/`).
fn page(r: &Registry, description: &str, pictures: &[String]) -> String {
    let list: Vec<String> = r.categories.iter().map(|c| basic(c)).collect();
    let mut out = String::from(
        "# Written by `cargo creature pages` from module.toml's [registry] and README.md: edit those, not this file.\n",
    );
    out.push_str(&format!("summary = {}\n", basic(&r.summary)));
    out.push_str(&format!("description = {}\n", multiline(description)));
    out.push_str(&format!("categories = [{}]\n", list.join(", ")));
    for p in pictures {
        out.push_str(&format!("\n[[screenshots]]\npath = {}\n", basic(p)));
    }
    out
}

/// The page's pictures, `[registry]`'s screenshots in their order: their paths in the folder (`page/<n>-<name>.png`,
/// or `.jpg` for a JPEG or a WebP made a JPEG) and their bytes.
fn pictures(r: &Registry) -> crate::Result<Vec<(String, Vec<u8>)>> {
    if r.screenshots.is_empty() || r.screenshots.len() > SCREENSHOTS {
        return Err(format!("1 to {SCREENSHOTS} screenshots"));
    }
    let mut out = Vec::new();
    for (k, path) in r.screenshots.iter().enumerate() {
        let from = crate::root().join(path);
        let fail = |e: String| format!("{}: {e}", from.display());
        let stem = from
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = from
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let (ext, bytes) = match ext.as_str() {
            "png" | "jpg" | "jpeg" => (ext.clone(), std::fs::read(&from).map_err(|e| fail(e.to_string()))?),
            "webp" => {
                let picture = image::open(&from).map_err(|e| fail(e.to_string()))?.to_rgb8();
                let mut bytes = std::io::Cursor::new(Vec::new());
                picture
                    .write_to(&mut bytes, image::ImageFormat::Jpeg)
                    .map_err(|e| fail(e.to_string()))?;
                ("jpg".into(), bytes.into_inner())
            }
            _ => return Err(fail("a screenshot is a PNG, a JPEG or a WebP".into())),
        };
        if bytes.len() as u64 > SCREENSHOT_BYTES {
            return Err(fail("over 5 MiB, the registry's most for a picture".into()));
        }
        out.push((format!("page/{}-{stem}.{ext}", k + 1), bytes));
    }
    Ok(out)
}

/// `cargo creature release [module...]`: each published module named (all for none) checked as the release workflow
/// will check it, then tagged; the pushes printed. A version already tagged is skipped (a released version never
/// changes: raise it).
pub fn run(names: &[String]) -> crate::Result {
    let clean = git(&["status", "--porcelain"])?;
    if !clean.trim().is_empty() {
        return Err("the checkout has changes: commit them first (a release builds the tagged commit)".into());
    }
    let mut tagged = Vec::new();
    for m in published(names)? {
        let version = version(&m)?;
        let tag = tag(&m.id, &version);
        if !git(&["tag", "--list", &tag])?.trim().is_empty() {
            println!("{tag}: already tagged, skipped (raise the version to release again)");
            continue;
        }
        changelog(&m, &version)?;
        let problems = stale(&m)?;
        if !problems.is_empty() {
            return Err(problems.join("\n"));
        }
        let lock = m.dir.join("Cargo.lock");
        if m.native().is_some() && git(&["ls-files", "--error-unmatch", &lock.display().to_string()]).is_err() {
            return Err(format!(
                "{}: not committed (a native release carries it)",
                lock.display()
            ));
        }
        crate::install::run_in(&m.dir, "cargo", &["sunrise-mod", "check", "--release", &tag])?;
        git(&["tag", "-a", &tag, "-m", &format!("{} {version}", m.name)])?;
        println!("{tag}: tagged");
        tagged.push(tag);
    }
    if tagged.is_empty() {
        println!("Nothing new to release.");
        return Ok(());
    }
    println!("\nPush main first, then each tag on its own (the library's first, the mods' once it is listed):");
    println!("  git push origin main");
    for t in &tagged {
        println!("  git push origin refs/tags/{t}");
    }
    if tagged.iter().any(|t| t.starts_with(&format!("{LIBRARY}/"))) && tagged.len() > 1 {
        println!(
            "The mods build against the library from the registry: push their tags once its release is merged on \
             sunrise-mods, or their workflows fail (push a failed one's tag again after deleting it)."
        );
    }
    Ok(())
}

/// Runs git at the repository's root, its output.
fn git(args: &[&str]) -> crate::Result<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(crate::root())
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The module's README as its page's description: its title left out (the page shows the name), its image lines too
/// (the page's pictures are its screenshots), its links made absolute (to the repository on GitHub, at revision `rev`:
/// the version's tag, which never moves).
pub fn description(m: &Module, rev: &str) -> crate::Result<String> {
    let path = m.dir.join("README.md");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let body: Vec<&str> = text
        .lines()
        .skip_while(|l| l.starts_with("# ") || l.trim().is_empty())
        .filter(|l| !is_image(l))
        .collect();
    let text = body.join("\n").replace("\n\n\n", "\n\n");
    let dir = m.dir.strip_prefix(crate::root()).unwrap_or(&m.dir).to_path_buf();
    Ok(absolute_links(&text, &dir, rev).trim().to_owned() + "\n")
}

/// A line made only of images (`![alt](path)`, one or more).
fn is_image(line: &str) -> bool {
    let mut rest = line.trim();
    if rest.is_empty() {
        return false;
    }
    while !rest.is_empty() {
        let Some(after) = rest.strip_prefix("![") else {
            return false;
        };
        let Some(close) = after.find("](") else {
            return false;
        };
        let Some(end) = after[close..].find(')') else {
            return false;
        };
        rest = after[close + end + 1..].trim_start();
    }
    true
}

/// A 0.x version's page opens with this: the series is in beta until 1.0.
fn beta(version: &str) -> String {
    if version.starts_with("0.") {
        BETA.to_owned()
    } else {
        String::new()
    }
}

const BETA: &str = "> **Beta.** Played and tested in single player; multiplayer and balance come later. Reports are \
                    welcome on the [issues](https://github.com/EAnathos/sunrise-creatures/issues).\n\n";

/// `markdown` with its relative links (`[x](../../README.md)`) and images (`![](../../docs/models/a.png)`), relative
/// to `dir` in the repository, made absolute at revision `rev`.
fn absolute_links(markdown: &str, dir: &Path, rev: &str) -> String {
    let mut out = String::new();
    let mut rest = markdown;
    while let Some(at) = rest.find("](") {
        let image = rest[..at].rfind('[').is_some_and(|i| rest[..i].ends_with('!'));
        out.push_str(&rest[..at + 2]);
        rest = &rest[at + 2..];
        let end = rest.find(')').unwrap_or(rest.len());
        let target = &rest[..end];
        if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
            out.push_str(target);
        } else {
            let path = normalise(&dir.join(target));
            let base = if image {
                format!("{RAW}/{rev}")
            } else {
                format!("{SOURCE}/blob/{rev}")
            };
            out.push_str(&format!("{base}/{path}"));
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// `path` with its `.` and `..` resolved, `/`-separated.
fn normalise(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    parts.join("/")
}

/// What the registry would refuse on the page (`cargo sunrise-mod check --release`'s rules): the summary (25 to 160
/// characters: ours stays one line for the lists), the description (at least 125), the categories (1 to 8, distinct,
/// a mod's).
pub fn page_problems(r: &Registry, description: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !(25..=160).contains(&r.summary.trim().chars().count()) {
        problems.push("the summary must be 25 to 160 characters".into());
    }
    let length = description.trim().chars().count();
    if length < 125 {
        problems.push(format!(
            "the description (its README) is {length} characters, not at least 125"
        ));
    }
    if r.categories.is_empty() || r.categories.len() > 8 {
        problems.push("1 to 8 categories".into());
    }
    for (k, c) in r.categories.iter().enumerate() {
        if !CATEGORIES.contains(&c.as_str()) {
            problems.push(format!("{c} is not a mod's category"));
        }
        if r.categories[..k].contains(c) {
            problems.push(format!("category {c} twice"));
        }
    }
    problems
}

/// The section of the module's `CHANGELOG.md` for `version` (`## 0.1.0`), without its heading.
pub fn changelog(m: &Module, version: &str) -> crate::Result<String> {
    let path = m.dir.join("CHANGELOG.md");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let heading = format!("## {version}");
    let start = text
        .lines()
        .position(|l| l.trim() == heading)
        .ok_or_else(|| format!("{}: no section {heading}", path.display()))?;
    let section: Vec<&str> = text
        .lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with("## "))
        .collect();
    Ok(section.join("\n").trim().to_owned() + "\n")
}

/// A one-line TOML string.
fn basic(s: &str) -> String {
    let escaped: String = s
        .chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            c => vec![c],
        })
        .collect();
    format!("\"{escaped}\"")
}

/// A multi-line TOML string.
fn multiline(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace("\"\"\"", "\"\"\\\"");
    format!("\"\"\"\n{escaped}\"\"\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_readme_s_relative_links_and_images_become_absolute() {
        let md = "![](../../docs/models/v-front.png) [series](../../README.md), [spec](SPECS.md), [web](https://x.y)";
        let out = absolute_links(md, Path::new("mods/vulture"), "vulture/v0.2.0");
        assert_eq!(
            out,
            format!(
                "![]({RAW}/vulture/v0.2.0/docs/models/v-front.png) [series]({SOURCE}/blob/vulture/v0.2.0/README.md), \
                 [spec]({SOURCE}/blob/vulture/v0.2.0/mods/vulture/SPECS.md), [web](https://x.y)"
            )
        );
    }

    #[test]
    fn a_0_x_version_says_it_is_in_beta() {
        assert!(beta("0.1.0").starts_with("> **Beta.**"));
        assert!(is_image("![](a.png) ![](b.png)"));
        assert!(!is_image("A [link](x) and ![](a.png)"));
        assert!(!is_image(""));
        assert!(beta("1.0.0").is_empty());
    }

    #[test]
    fn toml_strings_are_escaped() {
        assert_eq!(basic("a \"b\" \\c"), "\"a \\\"b\\\" \\\\c\"");
        assert_eq!(multiline("x \"\"\" y\n"), "\"\"\"\nx \"\"\\\" y\n\"\"\"");
    }

    #[test]
    fn a_page_is_checked_as_the_registry_does() {
        let r = Registry {
            summary: "A vulture.".into(),
            categories: vec!["mobs".into(), "bloom".into(), "mobs".into()],
            icon: "vulture".into(),
            screenshots: Vec::new(),
        };
        let problems = page_problems(&r, "short");
        assert_eq!(problems.len(), 4, "{problems:?}");
    }

    #[test]
    fn a_page_lists_its_pictures_and_a_tag_names_its_folder_s_package() {
        let r = Registry {
            summary: "A vulture you tame, ride and fly.".into(),
            categories: vec!["mobs".into()],
            icon: "vulture".into(),
            screenshots: Vec::new(),
        };
        let page = page(&r, "Text.\n", &["page/1-a.png".into()]);
        let t: toml::Table = toml::from_str(&page).unwrap();
        assert_eq!(t["summary"].as_str(), Some("A vulture you tame, ride and fly."));
        assert_eq!(t["description"].as_str(), Some("Text.\n"));
        assert_eq!(t["screenshots"][0]["path"].as_str(), Some("page/1-a.png"));
        assert_eq!(tag("vulture", "0.2.4"), "vulture/v0.2.4");
    }
}
