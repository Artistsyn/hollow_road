//! Save profiles: each pilgrimage kept in its own slot, so one can be carried on while another
//! calling walks the road another way. A slot is a JSON file in the player's own data folder
//! (`dir`), written whole to a side file and renamed over the last, so a save cut off half way
//! leaves the one before it. What is kept is what the road cannot give back: the hero's levels
//! and calling, the time played, and the world's progress (`World::saved`). Where they stood is
//! not: a pilgrimage carried on wakes at the shrine last rested at, as after a death.

use crate::fight::Stats;
use crate::shrine::Calling;
use crate::world::World;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Slots on the title's list.
pub const SLOTS: usize = 5;
/// What a save is written as; a file of a later version is left alone, not misread.
const VERSION: u64 = 1;

/// Where the slots are kept: `HOLLOW_ROAD_SAVES` when set, else the platform's own place for an
/// application's data (Application Support on macOS, `%APPDATA%` on Windows, `$XDG_DATA_HOME` or
/// `~/.local/share` elsewhere). Never beside the game itself, which may not be writable.
pub fn dir() -> PathBuf {
    if let Some(d) = std::env::var_os("HOLLOW_ROAD_SAVES") { return PathBuf::from(d); }
    let home = || std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let base = if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(home)
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".local/share"))
    };
    base.join("Hollow Road").join("profiles")
}

fn file(dir: &Path, slot: usize) -> PathBuf { dir.join(format!("slot{}.json", slot + 1)) }

/// A slot as the title lists it: free, holding a pilgrimage, or holding a file this game cannot
/// read (a later version's, or broken), which is listed but never chosen or written over.
#[derive(Clone, Debug, PartialEq)]
pub enum Slot { Empty, Kept(Summary), Unreadable }

/// A kept pilgrimage as the title lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub calling: Calling,
    pub level: u32,
    /// The place of the shrine they will wake at.
    pub place: &'static str,
    /// Seconds played.
    pub played: u64,
    /// Times the last boss has fallen (a pilgrimage begun again).
    pub pilgrimages: u32,
    /// When it was last written, seconds since 1970: the newest is offered first.
    pub saved: u64,
}

impl Summary {
    /// One line of the list: `Warden  level 12  ·  Hollowmere Crossroads  ·  3 h 20 m`.
    pub fn line(&self) -> String {
        let (h, m) = (self.played / 3600, self.played / 60 % 60);
        let time = if h > 0 { format!("{h} h {m} m") } else { format!("{m} m") };
        let again = if self.pilgrimages > 0 { format!("  ·  pilgrimage {}", self.pilgrimages + 1) } else { String::new() };
        format!("{}  level {}  ·  {}  ·  {time}{again}", self.calling.name(), self.level, self.place)
    }
}

/// A slot read back: who, how far, and the world to set out into.
pub struct Saved {
    pub calling: Calling,
    pub stats: Stats,
    pub played: u64,
    pub world: Value,
}

/// The save of a pilgrimage: `calling`, the levels in `stats`, `played` seconds, and `world`.
pub fn record(calling: Calling, stats: &Stats, played: u64, world: &World) -> Value {
    let saved = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    json!({
        "version": VERSION,
        "saved": saved,
        "calling": calling.name(),
        "played": played,
        "hero": {
            "vigor": stats.vigor, "endurance": stats.endurance, "might": stats.might, "dexterity": stats.dexterity,
            "focus": stats.focus, "weapon": stats.weapon, "armour": stats.armour,
        },
        "world": world.saved(),
    })
}

/// Write `record` into `slot`: whole to a side file first, then over the old one.
pub fn write(dir: &Path, slot: usize, record: &Value) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let side = dir.join(format!("slot{}.json.part", slot + 1));
    std::fs::write(&side, serde_json::to_string_pretty(record).map_err(std::io::Error::other)? + "\n")?;
    std::fs::rename(&side, file(dir, slot))
}

/// `slot`'s save, None when it is empty or cannot be read as one.
pub fn read(dir: &Path, slot: usize) -> Option<Saved> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(file(dir, slot)).ok()?).ok()?;
    if v["version"].as_u64()? > VERSION { return None; }
    let calling = Calling::ALL.into_iter().find(|c| Some(c.name()) == v["calling"].as_str())?;
    let h = &v["hero"];
    let n = |k: &str| h[k].as_u64().unwrap_or(0) as u32;
    let stats = Stats { vigor: n("vigor"), endurance: n("endurance"), might: n("might"), dexterity: n("dexterity"), focus: n("focus"), weapon: n("weapon"), armour: n("armour"), ..Stats::default() };
    Some(Saved { calling, stats, played: v["played"].as_u64().unwrap_or(0), world: v["world"].clone() })
}

/// Every slot as the title lists it.
pub fn summaries(dir: &Path) -> Vec<Slot> {
    (0..SLOTS).map(|slot| {
        if !file(dir, slot).exists() { return Slot::Empty; }
        let kept = || {
            let s = read(dir, slot)?;
            let saved = std::fs::read_to_string(file(dir, slot)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v["saved"].as_u64()).unwrap_or(0);
            let world = World::restored(0.0, &s.world)?;
            Some(Summary { calling: s.calling, level: s.stats.level(), place: world.checkpoint_title(), played: s.played, pilgrimages: world.pilgrimages, saved })
        };
        kept().map_or(Slot::Unreadable, Slot::Kept)
    }).collect()
}

/// The slot last written, to offer first.
pub fn newest(slots: &[Slot]) -> Option<usize> {
    slots.iter().enumerate().filter_map(|(i, s)| match s { Slot::Kept(s) => Some((i, s.saved)), _ => None }).max_by_key(|&(_, t)| t).map(|(i, _)| i)
}

/// Empty `slot` for good (the player asked twice).
pub fn forget(dir: &Path, slot: usize) -> std::io::Result<()> {
    match std::fs::remove_file(file(dir, slot)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        r => r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder of its own under the target dir, for one test.
    fn scratch(name: &str) -> PathBuf {
        let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/profile_tests").join(name);
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_pilgrimage_written_to_a_slot_reads_back_as_it_was_and_forgetting_empties_it() {
        let d = scratch("roundtrip");
        let mut w = World::new(0.0);
        w.souls = 1234;
        w.shards = 3;
        let stats = Stats { vigor: 4, might: 7, weapon: 2, ..Stats::default() };
        write(&d, 2, &record(Calling::Warden, &stats, 3725, &w)).unwrap();
        let s = read(&d, 2).expect("slot 3 reads back");
        assert_eq!((s.calling, s.stats.vigor, s.stats.might, s.stats.weapon, s.played), (Calling::Warden, 4, 7, 2, 3725));
        let back = World::restored(0.0, &s.world).unwrap();
        assert_eq!((back.souls, back.shards), (1234, 3));
        let list = summaries(&d);
        assert_eq!(list.len(), SLOTS);
        assert!(list[0] == Slot::Empty && matches!(list[2], Slot::Kept(_)));
        let Slot::Kept(kept) = &list[2] else { unreachable!() };
        assert_eq!(kept.line(), format!("Warden  level {}  ·  {}  ·  1 h 2 m", stats.level(), back.checkpoint_title()));
        assert_eq!(newest(&list), Some(2));
        forget(&d, 2).unwrap();
        assert!(read(&d, 2).is_none() && summaries(&d).iter().all(|s| *s == Slot::Empty));
        // Nothing left half written beside it.
        assert!(std::fs::read_dir(&d).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".part")));
    }

    #[test]
    fn a_slot_from_a_later_game_or_a_broken_file_is_left_alone() {
        let d = scratch("unreadable");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(file(&d, 0), r#"{"version": 99, "calling": "Warden"}"#).unwrap();
        std::fs::write(file(&d, 1), "not json").unwrap();
        assert!(read(&d, 0).is_none() && read(&d, 1).is_none());
        assert_eq!(&summaries(&d)[..3], &[Slot::Unreadable, Slot::Unreadable, Slot::Empty]);
        // Still there: a slot is not cleared just for being unreadable.
        assert!(file(&d, 0).exists() && file(&d, 1).exists());
    }
}
