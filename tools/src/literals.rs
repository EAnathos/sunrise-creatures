//! The library's settings structs in the mods' code (docs/CONVENTIONS.md, "Library structs"): every literal of a type
//! the library gives a `DEFAULT` (`pub const DEFAULT: <Type>` in `common/src/`) ends with `..<Type>::DEFAULT`, so a
//! field the library adds later doesn't break the mods built before it. `cargo creature check` lists those that don't.
//!
//! Read from the source text, as a person would: a literal is `Type {` or `path::Type {` (a field list, not a
//! declaration, a pattern or a function's body), the type the library's when it is named through `creature` (its path,
//! or a `use creature::...` naming it or its module).
use std::path::{Path, PathBuf};

/// The library's types with a `DEFAULT`.
pub fn defaulted(library: &Path) -> Vec<String> {
    let mut files = Vec::new();
    rust_files(&library.join("src"), &mut files);
    let mut types = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("pub const DEFAULT: ") {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !types.contains(&name) {
                    types.push(name);
                }
            }
        }
    }
    types
}

/// The literals of `types` in `text` (a mod's source file) that don't end with `..<Type>::DEFAULT`: their lines and
/// types.
pub fn missing(text: &str, types: &[String]) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    for t in types {
        let needle = format!("{t} {{");
        let mut from = 0;
        while let Some(at) = text[from..].find(&needle).map(|i| i + from) {
            from = at + needle.len();
            // A whole name, not the end of another (`MyKind {`).
            if at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_') {
                continue;
            }
            let path = path_before(text, at);
            if !is_library(text, &path, t) || !is_value(text, at - path.len()) {
                continue;
            }
            let open = at + needle.len() - 1;
            let Some(close) = matching(bytes, open) else {
                continue;
            };
            let top = top_level(&text[open + 1..close]);
            // A pattern or a shorthand-only literal names no `field: value`; one with `..` is fine.
            if top.contains("..") || !top.contains(':') {
                continue;
            }
            out.push((text[..at].lines().count().max(1), t.clone()));
        }
    }
    out.sort();
    out
}

/// The `a::b::` before position `at`.
fn path_before(text: &str, at: usize) -> String {
    let before = &text[..at];
    let start = before
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .map_or(0, |i| i + 1);
    before[start..].to_owned()
}

/// Whether `Type` at `path` is the library's: named through `creature`, or imported from it (itself or its module).
fn is_library(text: &str, path: &str, t: &str) -> bool {
    if path.starts_with("creature::") {
        return true;
    }
    let name = path
        .trim_end_matches("::")
        .split("::")
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(t);
    text.lines().any(|l| {
        let l = l.trim();
        l.starts_with("use creature::")
            && (l.ends_with(&format!("::{name};"))
                || l.contains(&format!("{{{name},"))
                || l.contains(&format!(" {name},"))
                || l.contains(&format!(" {name}}}"))
                || l.contains(&format!("{{{name}}}"))
                || l.contains(&format!("::{name}::")))
    })
}

/// Whether the text before `at` makes what follows a value (not a declaration, an impl, or a function's body).
fn is_value(text: &str, at: usize) -> bool {
    let line = text[..at].rsplit('\n').next().unwrap_or("");
    let words = ["struct ", "enum ", "impl ", "fn ", "-> ", "trait "];
    !words.iter().any(|w| line.contains(w)) || line.trim_end().ends_with('=')
}

/// The `}` closing the `{` at `open`.
fn matching(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `body` without what is nested in brackets.
fn top_level(body: &str) -> String {
    let mut depth = 0;
    body.chars()
        .filter(|c| {
            match c {
                '{' | '(' | '[' => depth += 1,
                '}' | ')' | ']' => depth -= 1,
                _ => return depth == 0,
            }
            false
        })
        .collect()
}

pub fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_literal_ends_with_its_default() {
        let types = vec!["Light".to_string(), "Kind".to_string()];
        let text = "use creature::glow::Light;\n\
                    static A: Light = Light {\n    rgb: 1,\n};\n\
                    static B: Light = Light {\n    rgb: 1,\n    ..Light::DEFAULT\n};\n\
                    static C: creature::flutter::Kind = creature::flutter::Kind {\n    speed: 1.0,\n};\n\
                    struct Kind {\n    spots: u8,\n}\n\
                    static D: Kind = Kind {\n    spots: 2,\n};\n\
                    fn f(l: Light) -> Light {\n    l\n}\n";
        let lines: Vec<usize> = missing(text, &types).into_iter().map(|(l, _)| l).collect();
        assert_eq!(
            lines,
            vec![2, 9],
            "A and C, not the mod's own Kind nor a function's body"
        );
    }
}
