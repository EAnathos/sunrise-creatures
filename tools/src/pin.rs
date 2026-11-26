//! `cargo creature pin [<commit>]`: moves the whole series to another Sunrise build (the current Nightly without one),
//! as docs/SUNRISE-ISSUES.md's "Moving the pin" says. Sunrise's own `cargo sunrise-mod pin` repins the library and
//! reports what of the API it uses changed; its new `[native]` (`game`, `sunrise`, `mixins`) is then written in every
//! other module's `sunrise-mod.toml`, CI's `SUNRISE_REV` and CONTRIBUTING's `REV`, which `cargo creature check` holds
//! to the library's.
use std::path::Path;

use crate::install::run_in;
use crate::modules;

pub fn run(build: Option<&str>) -> crate::Result {
    let library = modules::library()?;
    let mut args = vec!["sunrise-mod", "pin"];
    args.extend(build);
    run_in(&library.dir, "cargo", &args)?;
    let manifest = read(&library.dir.join("sunrise-mod.toml"))?;
    let pins: Vec<(&str, String)> = ["game", "sunrise", "mixins"]
        .into_iter()
        .filter_map(|k| Some((k, native(&manifest, k)?)))
        .collect();
    let sunrise = pins
        .iter()
        .find(|(k, _)| *k == "sunrise")
        .map(|(_, v)| v.clone())
        .ok_or("common/sunrise-mod.toml: no [native] sunrise after the pin")?;
    for m in modules::mods()? {
        let Some(path) = m.manifest() else {
            continue;
        };
        let mut text = read(&path)?;
        if native(&text, "sunrise").is_none() {
            continue;
        }
        for (k, v) in &pins {
            text = set_native(&text, k, v);
        }
        write(&path, &text)?;
    }
    let root = crate::root();
    let ci = root.join(".github/workflows/ci.yml");
    write(&ci, &after(&read(&ci)?, "SUNRISE_REV: ", &sunrise))?;
    let contributing = root.join("CONTRIBUTING.md");
    write(&contributing, &after(&read(&contributing)?, "REV=", &sunrise))?;
    println!(
        "\nThe series is pinned to {sunrise}. Next (docs/SUNRISE-ISSUES.md, \"Moving the pin\"): fix what the report \
         above names, `cargo creature native-check` (it fetches the new SDK), install sunrise-stage at it \
         (CONTRIBUTING.md), put the test instance on that build, `cargo creature install` and try it in game, update \
         docs/SUNRISE-ISSUES.md's pin line, then `cargo creature bump` the modules to release them for it."
    );
    Ok(())
}

/// The value of `key` in `text`'s `[native]` table.
fn native(text: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            inside = l == "[native]";
        } else if inside && let Some(rest) = l.strip_prefix(&format!("{key} = \"")) {
            return rest.strip_suffix('"').map(str::to_owned);
        }
    }
    None
}

/// `text` with `key` of its `[native]` table set to `value` (its line kept where it is).
fn set_native(text: &str, key: &str, value: &str) -> String {
    let mut inside = false;
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let l = line.trim();
            if l.starts_with('[') {
                inside = l == "[native]";
            } else if inside && l.starts_with(&format!("{key} = \"")) {
                return format!("{key} = \"{value}\"");
            }
            line.to_owned()
        })
        .collect();
    lines.join("\n") + if text.ends_with('\n') { "\n" } else { "" }
}

/// `text` with the 40-hex commit after each `marker` replaced by `commit`.
fn after(text: &str, marker: &str, commit: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(marker) {
        let start = i + marker.len();
        out.push_str(&rest[..start]);
        let old = &rest[start..];
        let len = old.chars().take_while(char::is_ascii_hexdigit).count();
        if len == 40 {
            out.push_str(commit);
            rest = &old[40..];
        } else {
            rest = old;
        }
    }
    out.push_str(rest);
    out
}

fn read(path: &Path) -> crate::Result<String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, text: &str) -> crate::Result {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "94256510f74e6843754e0a93fd9e4da7e45b1056";
    const NEW: &str = "ba1f24e4ad41246b558230aa665539dc768b1287";

    #[test]
    fn the_native_table_s_pins_are_read_and_set_in_place() {
        let text = format!(
            "id = \"x\"\nsunrise = \"no\"\n\n[native]\ngame = \"26.3\"\nsunrise = \"{OLD}\"\n\n[dependencies]\ncreature = \"^0.3\"\n"
        );
        assert_eq!(native(&text, "sunrise").as_deref(), Some(OLD));
        let moved = set_native(&text, "sunrise", NEW);
        assert_eq!(native(&moved, "sunrise").as_deref(), Some(NEW));
        assert!(
            moved.contains("id = \"x\"\nsunrise = \"no\""),
            "outside [native], left alone"
        );
        assert!(moved.ends_with("creature = \"^0.3\"\n"));
    }

    #[test]
    fn a_commit_after_its_marker_is_replaced() {
        let ci = format!("  SUNRISE_REV: {OLD}\n  key: x-${{{{ env.SUNRISE_REV }}}}\n");
        assert_eq!(
            after(&ci, "SUNRISE_REV: ", NEW),
            format!("  SUNRISE_REV: {NEW}\n  key: x-${{{{ env.SUNRISE_REV }}}}\n")
        );
    }
}
