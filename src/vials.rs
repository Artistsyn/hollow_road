//! Shrine glass (`docs/story.md`, section 8): the flasks' own clouded glass, melted at a rest into
//! one more flask. The story's road gives some, from the great foes it cannot go round; the rest
//! wait for the thorough, in chests at the ends of roads no one has to walk, and for the brave, on
//! the bodies of foes no one has to fight. Three flasks to start, fifteen with every piece.
//! **[new]**

use crate::atlas::Key;
use crate::world::{Note, World};

/// What every piece is called when found and listed.
pub const SHRINE_GLASS: &str = "Shrine Glass";

/// How a piece is come by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum By { Boss, Chest }

/// One piece: the name it is saved by, where it lies, how it is won, and whether the story's road
/// hands it over.
pub struct Glass { pub name: &'static str, pub place: &'static str, pub by: By, pub story: bool }

impl Glass {
    const fn new(name: &'static str, place: &'static str, by: By, story: bool) -> Glass { Glass { name, place, by, story } }

    /// Which piece is called `name`.
    pub fn named(name: &str) -> Option<usize> { GLASS.iter().position(|g| g.name == name) }

    /// Flasks a hero who walked the story's road, and found half of what lies off it, carries into
    /// a region of `tier`: what bots and tuning expect of a player by then.
    pub fn expected_at(tier: u32) -> u32 {
        let before = |g: &&Glass| crate::atlas::PLACES.iter().find(|p| p.id == g.place).is_some_and(|p| crate::atlas::REGIONS[p.region].tier < tier);
        let story = GLASS.iter().filter(before).filter(|g| g.story).count();
        let off = GLASS.iter().filter(before).filter(|g| !g.story).count();
        crate::fight::FLASKS + (story + off / 2) as u32
    }
}

/// Every piece, by its `Key::Glass` index: saves keep them by name, so names never change.
pub const GLASS: [Glass; 12] = [
    // The story's: the great foes on the road.
    Glass::new("Shrine Glass of the Chief's Hold", "chief_hold", By::Boss, true),
    Glass::new("Shrine Glass of the Magistrate's Court", "magistrate", By::Boss, true),
    Glass::new("Shrine Glass of the Wraith Sanctum", "wraith_sanctum", By::Boss, true),
    Glass::new("Shrine Glass of the Tainted Cathedral", "cathedral", By::Boss, true),
    Glass::new("Shrine Glass of the Statue's Court", "statue_court", By::Boss, true),
    // The thorough's: chests at roads' ends.
    Glass::new("Shrine Glass of the Drowned Mill", "drowned_mill", By::Chest, false),
    Glass::new("Shrine Glass of the Bandits' Hole", "bandit_cave", By::Chest, false),
    Glass::new("Shrine Glass of the Void-Twisted Forest", "void_wood", By::Chest, false),
    // The brave's: foes no one is sent after.
    Glass::new("Shrine Glass of the Black Water", "black_water", By::Boss, false),
    Glass::new("Shrine Glass of the Fog Hollow", "fog_hollow", By::Boss, false),
    Glass::new("Shrine Glass of the Rune Vault", "rune_vault", By::Boss, false),
    Glass::new("Shrine Glass of the Whispering Altars", "whisper_altars", By::Boss, false),
];

impl World {
    /// The pieces of shrine glass carried: a flask more each.
    pub fn glass(&self) -> u32 { self.keys.iter().filter(|k| matches!(k, Key::Glass(_))).count() as u32 }

    /// The piece left here, won `by` its boss falling or its chest opened, if not yet carried.
    pub(crate) fn glass_won(&mut self, by: By) -> Option<Note> {
        let here = self.here().id;
        let n = GLASS.iter().position(|g| g.place == here && g.by == by)?;
        let key = Key::Glass(u8::try_from(n).ok()?);
        if self.keys.contains(&key) { return None; }
        self.keys.push(key);
        Some(Note::Found(SHRINE_GLASS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrine_glass_lies_where_a_boss_or_a_chest_is_and_each_piece_is_one_more_flask() {
        for (n, g) in GLASS.iter().enumerate() {
            let p = crate::atlas::PLACES.iter().find(|p| p.id == g.place).unwrap_or_else(|| panic!("{} lies nowhere", g.name));
            match g.by {
                By::Boss => assert!(p.boss.is_some(), "{}: no boss at {}", g.name, p.id),
                By::Chest => assert!(p.cache.is_some(), "{}: no chest at {}", g.name, p.id),
            }
            assert_eq!(Key::Glass(n as u8).name(), g.name);
            assert_eq!(Glass::named(g.name), Some(n));
        }
        let mut w = World::new(0.0);
        assert_eq!(crate::fight::Hero::with(crate::fight::Stats { kit: w.kit(), ..Default::default() }).flasks, crate::fight::FLASKS);
        w.keys.extend([Key::Glass(0), Key::Glass(7)]);
        assert_eq!(crate::fight::Hero::with(crate::fight::Stats { kit: w.kit(), ..Default::default() }).flasks, crate::fight::FLASKS + 2);
        assert_eq!(Glass::expected_at(0), crate::fight::FLASKS);
        assert_eq!(Glass::expected_at(9), crate::fight::FLASKS + 5 + 3, "the story's five, and half the seven off it");
    }

    #[test]
    fn a_chest_or_a_boss_with_glass_gives_it_once() {
        let mut w = World::new(0.0);
        w.lingering = None;
        w.arrive(&crate::atlas::stop("drowned_mill", "hollowmere"), 100.0, 0.0, 0);
        let notes = w.open_cache();
        assert!(notes.contains(&Note::Found(SHRINE_GLASS)), "{notes:?}");
        assert_eq!(w.glass(), 1);
        assert_eq!(w.glass_won(By::Chest), None, "once");
        let mut fight = crate::fight::Fight::new(Vec::new());
        assert_eq!(fight.hero.flasks, crate::fight::FLASKS, "not before a rest");
        w.rest(&mut fight);
        assert_eq!(fight.hero.flasks, crate::fight::FLASKS + 1, "melted in at the rest");
    }
}
