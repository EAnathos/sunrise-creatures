//! `cargo creature bump <patch|minor|major> <module...|all>`: a new version of each module named (`all`: every module
//! with a package), as docs/PUBLISHING.md's first step: its `sunrise-mod.toml` version raised (the module's only
//! version: its crate's stays `check::CRATE_VERSION`), a section for it on top of its `CHANGELOG.md` (with
//! `PLACEHOLDER`, which `cargo creature check` refuses until it says what changed), and a published module's release
//! files written again (`cargo creature pages`: the page's links follow the version's tag).
use crate::modules::{self, Module};

/// What a new changelog section says until it is written.
pub const PLACEHOLDER: &str = "(what changed)";

pub fn run(args: &[String]) -> crate::Result {
    let usage = "usage: cargo creature bump <patch|minor|major> <module...|all>";
    let (part, names) = args.split_first().ok_or(usage)?;
    if names.is_empty() {
        return Err(usage.into());
    }
    let all: Vec<Module> = std::iter::once(modules::library()?).chain(modules::mods()?).collect();
    let chosen: Vec<&Module> = all
        .iter()
        .filter(|m| m.manifest().is_some() && (names.iter().any(|n| n == "all") || names.contains(&m.id)))
        .collect();
    for n in names.iter().filter(|n| *n != "all") {
        if !chosen.iter().any(|m| &m.id == n) {
            return Err(format!("{n}: no module of that id with a package"));
        }
    }
    for m in chosen {
        let old = crate::release::version(m)?;
        let new = next(&old, part)?;
        let manifest = m.dir.join("sunrise-mod.toml");
        let text = read(&manifest)?;
        let from = format!("version = \"{old}\"");
        if !text.contains(&from) {
            return Err(format!("{}: no line {from}", manifest.display()));
        }
        write(&manifest, &text.replacen(&from, &format!("version = \"{new}\""), 1))?;
        let changelog = m.dir.join("CHANGELOG.md");
        let text = read(&changelog)?;
        write(&changelog, &with_section(&text, &new))?;
        if m.registry.is_some() {
            crate::release::pages(std::slice::from_ref(&m.id))?;
        }
        println!("{}: {old} -> {new} (write its CHANGELOG.md section)", m.id);
    }
    Ok(())
}

/// `version` raised by `part`: `patch`, `minor` or `major` (semver: a 0.x minor resets the patch).
fn next(version: &str, part: &str) -> crate::Result<String> {
    let n: Vec<u64> = version
        .split('.')
        .map(|p| p.parse().map_err(|_| format!("{version}: not x.y.z")))
        .collect::<crate::Result<_>>()?;
    let [a, b, c] = n[..] else {
        return Err(format!("{version}: not x.y.z"));
    };
    Ok(match part {
        "patch" => format!("{a}.{b}.{}", c + 1),
        "minor" => format!("{a}.{}.0", b + 1),
        "major" => format!("{}.0.0", a + 1),
        _ => return Err(format!("{part}: patch, minor or major")),
    })
}

/// `changelog` with a section for `version` before its first one (or at its end).
fn with_section(changelog: &str, version: &str) -> String {
    let section = format!("## {version}\n\n- {PLACEHOLDER}\n\n");
    match changelog.find("\n## ") {
        Some(i) => format!("{}{section}{}", &changelog[..=i], &changelog[i + 1..]),
        None => format!("{}\n\n{section}", changelog.trim_end()),
    }
}

fn read(path: &std::path::Path) -> crate::Result<String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &std::path::Path, text: &str) -> crate::Result {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_raised_by_its_part() {
        assert_eq!(next("0.3.5", "patch").unwrap(), "0.3.6");
        assert_eq!(next("0.3.5", "minor").unwrap(), "0.4.0");
        assert_eq!(next("0.3.5", "major").unwrap(), "1.0.0");
        assert!(next("0.3", "patch").is_err());
        assert!(next("0.3.5", "huge").is_err());
    }

    #[test]
    fn a_new_section_goes_on_top() {
        let text = "# Changelog\n\nIntro.\n\n## 0.1.0\n\n- First.\n";
        assert_eq!(
            with_section(text, "0.1.1"),
            format!("# Changelog\n\nIntro.\n\n## 0.1.1\n\n- {PLACEHOLDER}\n\n## 0.1.0\n\n- First.\n")
        );
    }
}
