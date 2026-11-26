//! Reads Minecraft's own data for the Vanilla Bestiary (mods/vanilla_bestiary/SPECS.md §2, §3).
//!
//! From the game's jar (the player's own, as the launcher prepared it), writes the mod's
//! `src/game_data.rs`: for each creature that has a spawn egg, the biomes it spawns in on its own (each
//! biome's `minecraft:gameplay/natural_mob_spawns`), what its loot table can drop (items, following the tables it
//! names) and the first item of its `<creature>_food` tag (what breeds it). Creatures that only come with structures,
//! raids or bosses have no biome here: the mod gives theirs.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::PathBuf;

use serde_json::Value;

/// The Minecraft version of the series' Sunrise pin (docs/SUNRISE-ISSUES.md).
const VERSION: &str = "26.3";
const OUT: &str = "mods/vanilla_bestiary/src/game_data.rs";

/// The jar the launcher prepared, when none is given.
fn default_jar() -> crate::Result<PathBuf> {
    let data = crate::launcher::data_dir()?;
    // Where the launcher keeps it now, or kept it before.
    let jar = |dir: &str| data.join(format!("{dir}/versions/{VERSION}/{VERSION}.jar"));
    Ok(Some(jar("launcher/minecraft"))
        .filter(|j| j.is_file())
        .unwrap_or_else(|| jar("minecraft")))
}

pub fn run(jar: Option<&str>) -> crate::Result {
    let jar = jar.map_or_else(default_jar, |j| Ok(PathBuf::from(j)))?;
    let file = std::fs::File::open(&jar).map_err(|e| format!("{}: {e}", jar.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("{}: {e}", jar.display()))?;
    // Every JSON file the data needs, by its path in the jar.
    let mut files: BTreeMap<String, Value> = BTreeMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.into_owned();
        let wanted = name == "assets/minecraft/lang/en_us.json"
            || name.starts_with("data/minecraft/worldgen/biome/")
            || name.starts_with("data/minecraft/loot_table/entities/")
            || name.starts_with("data/minecraft/tags/item/");
        if !wanted || !name.ends_with(".json") {
            continue;
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|e| format!("{name}: {e}"))?;
        files.insert(
            name.clone(),
            serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))?,
        );
    }
    let rust = render(&files)?;
    let out = crate::root().join(OUT);
    std::fs::write(&out, rust).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    shadows(&files)
}

/// Where the eggs' shadows go: one item definition per spawn egg.
const SHADOWS: &str = "mods/vanilla_bestiary/assets/vanilla_bestiary/items/shadow";

/// Each spawn egg's shadow (`shadow/<egg>.json`), for the book's species not met yet (`Species::shadow`): an item
/// definition drawing Minecraft's own egg model in black (a constant tint over its one layer); the book greys it. It
/// names the game's model and copies nothing of it. The eggs no longer in the game are removed.
fn shadows(files: &BTreeMap<String, Value>) -> crate::Result {
    let dir = crate::root().join(SHADOWS);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for egg in eggs(files)? {
        let path = dir.join(format!("{egg}_spawn_egg.json"));
        std::fs::write(&path, shadow(egg)).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    println!("{}", dir.display());
    Ok(())
}

/// The spawn eggs in the game, by creature (`cow`), from the English names.
fn eggs(files: &BTreeMap<String, Value>) -> crate::Result<BTreeSet<&str>> {
    let lang = files
        .get("assets/minecraft/lang/en_us.json")
        .and_then(Value::as_object)
        .ok_or("the jar has no assets/minecraft/lang/en_us.json")?;
    Ok(lang
        .keys()
        .filter_map(|k| k.strip_prefix("item.minecraft.")?.strip_suffix("_spawn_egg"))
        .collect())
}

/// The shadow of creature `egg`'s spawn egg: its model, tinted black.
fn shadow(egg: &str) -> String {
    format!(
        "{{\n  \"model\": {{\n    \"type\": \"minecraft:model\",\n    \"model\": \"minecraft:item/{egg}_spawn_egg\",\n    \
         \"tints\": [{{ \"type\": \"minecraft:constant\", \"value\": -16777216 }}]\n  }}\n}}\n"
    )
}

/// The creatures (by spawn egg), their biomes, drops and breeding food, as Rust.
fn render(files: &BTreeMap<String, Value>) -> crate::Result<String> {
    let lang = files
        .get("assets/minecraft/lang/en_us.json")
        .and_then(Value::as_object)
        .ok_or("the jar has no assets/minecraft/lang/en_us.json")?;
    let creatures: BTreeSet<&str> = lang
        .keys()
        .filter_map(|k| k.strip_prefix("item.minecraft.")?.strip_suffix("_spawn_egg"))
        .collect();
    let mut biomes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (path, biome) in files {
        let Some(name) = path
            .strip_prefix("data/minecraft/worldgen/biome/")
            .and_then(|n| n.strip_suffix(".json"))
        else {
            continue;
        };
        let spawns = &biome["attributes"]["minecraft:gameplay/natural_mob_spawns"]["argument"]["spawns_by_category"];
        for list in spawns.as_object().into_iter().flat_map(|c| c.values()) {
            for spawn in list.as_array().into_iter().flatten() {
                if let Some(kind) = spawn["type"].as_str() {
                    biomes
                        .entry(kind.to_owned())
                        .or_default()
                        .insert(format!("minecraft:{name}"));
                }
            }
        }
    }
    let mut rust = format!(
        "// Generated by `cargo creature vanilla` from Minecraft {VERSION}'s data (natural spawns, loot tables, \
         food tags): do not edit.\n\
         use crate::GameData;\n\n\
         #[rustfmt::skip]\n\
         pub static GAME_DATA: &[GameData] = &[\n"
    );
    for creature in creatures {
        let id = format!("minecraft:{creature}");
        let list = |items: &mut dyn Iterator<Item = String>| -> String {
            items.map(|s| format!("{s:?}")).collect::<Vec<_>>().join(", ")
        };
        let spawn = list(&mut biomes.get(&id).into_iter().flatten().cloned());
        let mut drops = BTreeSet::new();
        loot_items(files, &format!("entities/{creature}"), &mut drops, 0);
        let drops = list(&mut drops.into_iter());
        let food =
            first_of_tag(files, &format!("{creature}_food"), 0).map_or("None".to_owned(), |f| format!("Some({f:?})"));
        writeln!(
            rust,
            "    GameData {{ id: {id:?}, biomes: &[{spawn}], drops: &[{drops}], food: {food} }},"
        )
        .unwrap();
    }
    rust.push_str("];\n");
    Ok(rust)
}

/// The items a loot table (`entities/cow`) can give, with those of the tables it names.
fn loot_items(files: &BTreeMap<String, Value>, table: &str, out: &mut BTreeSet<String>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Some(json) = files.get(&format!("data/minecraft/loot_table/{table}.json")) else {
        return;
    };
    let mut tables = Vec::new();
    walk(json, &mut |v| match v["type"].as_str() {
        Some("minecraft:item") => {
            if let Some(name) = v["name"].as_str() {
                out.insert(name.to_owned());
            }
        }
        Some("minecraft:loot_table") => {
            if let Some(name) = v["value"].as_str() {
                tables.push(name.trim_start_matches("minecraft:").to_owned());
            }
        }
        _ => {}
    });
    for t in tables {
        loot_items(files, &t, out, depth + 1);
    }
}

fn walk(v: &Value, f: &mut dyn FnMut(&Value)) {
    match v {
        Value::Object(map) => {
            f(v);
            map.values().for_each(|c| walk(c, f));
        }
        Value::Array(list) => list.iter().for_each(|c| walk(c, f)),
        _ => {}
    }
}

/// The first item of item tag `minecraft:<tag>`, following the tags it names.
fn first_of_tag(files: &BTreeMap<String, Value>, tag: &str, depth: usize) -> Option<String> {
    if depth > 4 {
        return None;
    }
    let json = files.get(&format!("data/minecraft/tags/item/{tag}.json"))?;
    let first = json["values"].as_array()?.first()?;
    let name = first.as_str().or_else(|| first["id"].as_str())?;
    match name.strip_prefix("#minecraft:") {
        Some(inner) => first_of_tag(files, inner, depth + 1),
        None => Some(name.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_spawns_drops_and_food() {
        let mut files = BTreeMap::new();
        files.insert(
            "assets/minecraft/lang/en_us.json".to_owned(),
            json!({"item.minecraft.cow_spawn_egg": "Cow Spawn Egg", "item.minecraft.stone": "Stone"}),
        );
        files.insert(
            "data/minecraft/worldgen/biome/plains.json".to_owned(),
            json!({"attributes": {"minecraft:gameplay/natural_mob_spawns": {"argument": {"spawns_by_category":
                {"creature": [{"type": "minecraft:cow", "weight": 8}]}}}}}),
        );
        files.insert(
            "data/minecraft/loot_table/entities/cow.json".to_owned(),
            json!({"pools": [{"entries": [{"type": "minecraft:item", "name": "minecraft:leather"},
                {"type": "minecraft:loot_table", "value": "minecraft:entities/cow_extra"}]}]}),
        );
        files.insert(
            "data/minecraft/loot_table/entities/cow_extra.json".to_owned(),
            json!({"pools": [{"entries": [{"type": "minecraft:item", "name": "minecraft:beef"}]}]}),
        );
        files.insert(
            "data/minecraft/tags/item/cow_food.json".to_owned(),
            json!({"values": ["#minecraft:grains"]}),
        );
        files.insert(
            "data/minecraft/tags/item/grains.json".to_owned(),
            json!({"values": ["minecraft:wheat"]}),
        );
        let rust = render(&files).unwrap();
        assert!(rust.contains(
            r#"GameData { id: "minecraft:cow", biomes: &["minecraft:plains"], drops: &["minecraft:beef", "minecraft:leather"], food: Some("minecraft:wheat") }"#
        ));
        assert!(!rust.contains("stone"));
    }
}
