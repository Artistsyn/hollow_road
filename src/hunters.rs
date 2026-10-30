//! Krix and Lurz in the hollow, the book's chapter 8 and side quest 3 (`docs/story.md`). Once the
//! Blackridge chief is down they go east with Olekeen to the hollow, where the fog keeps a void
//! horror. Reached while they hold out, the fight is to save them: each time the hero falls there,
//! the horror takes one, Lurz first (dragged into the dark), then Krix (torn open). If Stonehollow's
//! court falls first, they were left to it. Killed before they ever got there, it never had them.
//! Whoever lives turns up later with the weapon the book gave them; whoever died returns much
//! later as a Taint echo.

use crate::atlas::Boss;
use crate::world::{Note, World};

/// The fog hollow off the Bandit Hollows, where the horror waits.
pub const FOG: &str = "fog_hollow";
/// The boss whose fall sends the hunters east.
const CHIEF: &str = "chief_hold";
/// Stonehollow's court: fallen while the hunters are still in the fog, they are past saving.
const COURT: &str = "magistrate";

/// A Taint echo of a hunter the Taint had: where it waits (place, metres in), the duel, what it
/// leaves (the weapon he would have handed over alive), and whether the story has made it.
pub struct Echo { pub place: &'static str, pub at: f32, pub boss: &'static Boss, pub leaves: &'static str, pub made: fn(&World) -> bool }

pub static LURZ_ECHO: Boss = Boss { breed: &crate::side::LURZ_ECHO, souls: 3600, key: None, mini: true, remnant: None, sought: false };
pub static KRIX_ECHO: Boss = Boss { breed: &crate::side::KRIX_ECHO, souls: 4400, key: None, mini: true, remnant: None, sought: false };

/// Lurz on the Violet Road, Krix at the Blackwood's edge: far from the fog, much later. Bounty in
/// the Void-Twisted Forest, at the end of his trail (`bounty`).
pub const ECHOES: [Echo; 3] = [
    Echo { place: "violet_road", at: 40.0, boss: &LURZ_ECHO, leaves: "Lurz's Spear", made: |w| w.hunters.settled && !w.hunters.lurz_lives() },
    Echo { place: "blackwood", at: 42.0, boss: &KRIX_ECHO, leaves: "Krix's Notched Axe", made: |w| w.hunters.settled && !w.hunters.krix_lives() },
    Echo { place: crate::bounty::FOREST, at: 34.0, boss: &crate::bounty::BOUNTY_ECHO, leaves: "Bounty's Longbow", made: World::bounty_waits },
];

/// How Krix and Lurz stand: how many the horror has taken (Lurz first, then Krix), and whether
/// that is settled (the horror dead, or the hunters left to it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hunters { pub lost: u8, pub settled: bool }

impl Hunters {
    pub fn lurz_lives(self) -> bool { self.lost == 0 }
    pub fn krix_lives(self) -> bool { self.lost <= 1 }

    pub fn saved(self) -> serde_json::Value { serde_json::json!({ "lost": self.lost, "settled": self.settled }) }

    pub fn restored(v: &serde_json::Value) -> Hunters {
        Hunters { lost: v["lost"].as_u64().map_or(0, |n| n.min(2) as u8), settled: v["settled"].as_bool().unwrap_or(false) }
    }
}

impl World {
    /// Whether Krix and Lurz are out in the fog with the horror: gone east after the chief, and
    /// nothing settled yet.
    pub fn hunters_in_the_fog(&self) -> bool { self.beaten.contains(&CHIEF) && !self.hunters.settled }

    /// The hero fell: if it was in the hollow, with the horror standing and the hunters in it,
    /// the horror takes the next of them.
    pub fn hunters_fall_with_the_hero(&mut self) {
        if self.here().id == FOG && self.hunters_in_the_fog() && !self.beaten.contains(&FOG) {
            self.hunters.lost = (self.hunters.lost + 1).min(2);
        }
    }

    /// The story moving on in the hollow: its horror dead settles what it took; Stonehollow's
    /// court falling while the hunters are still out there leaves them to it.
    pub fn hunters_move_on(&mut self) {
        if self.hunters.settled { return; }
        if self.beaten.contains(&FOG) {
            self.hunters.settled = true;
        } else if self.hunters_in_the_fog() && self.beaten.contains(&COURT) {
            self.hunters = Hunters { lost: 2, settled: true };
        }
    }

    /// Whether the hunters came home from the hollow (the chief down, the hollow settled), and so
    /// can be met again: Krix if he lived, Lurz if he did.
    pub fn hunters_home(&self) -> bool { self.beaten.contains(&CHIEF) && self.hunters.settled }

    /// The boss of the place the hero is in, metres in: its own, or a hunter's echo the hollow made.
    pub fn boss_here(&self) -> Option<(f32, &'static Boss)> {
        let id = self.here().id;
        self.here().boss.or_else(|| ECHOES.iter().find(|e| e.place == id && (e.made)(self)).map(|e| (e.at, e.boss))).or_else(|| self.alley_here())
    }

    /// What a felled echo leaves: the weapon its hunter carried, if the hero lacks it.
    pub fn echo_leaves(&mut self, boss: &Boss) -> Vec<Note> {
        let Some(e) = ECHOES.iter().find(|e| std::ptr::eq(e.boss, boss)) else { return Vec::new() };
        let i = crate::gear::find(e.leaves);
        if self.gear.gain(i) { vec![Note::Found(crate::gear::ITEMS[i].name)] } else { Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_the_fog() -> World {
        let mut w = World::new(0.0);
        w.beaten.push(CHIEF);
        w.arrive("fog_hollow@bandit_hollows", 100.0, 0.0, 0);
        w
    }

    #[test]
    fn each_fall_in_the_fog_costs_lurz_then_krix_and_the_horrors_death_settles_it() {
        let mut w = in_the_fog();
        assert!(w.hunters_in_the_fog());
        w.died(120.0);
        w.arrive("fog_hollow@bandit_hollows", 200.0, 0.0, 0);
        assert!(!w.hunters.lurz_lives() && w.hunters.krix_lives(), "Lurz first");
        w.boss_felled();
        assert!(w.hunters.settled && w.hunters_home());
        w.died(220.0);
        assert_eq!(w.hunters.lost, 1, "settled: nothing more to lose");
        assert_eq!(World::restored(0.0, &w.saved()).map(|b| b.hunters), Some(w.hunters));
    }

    #[test]
    fn the_hunters_the_fog_kept_come_back_as_echoes_that_leave_their_weapons() {
        let mut w = in_the_fog();
        w.hunters = Hunters { lost: 1, settled: true };
        w.arrive("violet_road@magistrate", 300.0, 0.0, 0);
        assert!(w.boss_here().is_some_and(|(_, b)| b.name() == "The Echo of Lurz"));
        let notes = w.boss_felled();
        assert!(notes.contains(&Note::Found("Lurz's Spear")), "{notes:?}");
        w.arrive("blackwood@cathedral", 400.0, 0.0, 0);
        assert!(w.boss_here().is_none(), "Krix lived");
        w.hunters.lost = 2;
        assert!(w.boss_here().is_some_and(|(_, b)| b.name() == "The Echo of Krix"));
    }

    #[test]
    fn left_until_stonehollows_court_falls_they_are_lost_and_killed_early_it_never_had_them() {
        let mut late = in_the_fog();
        late.beaten.push(COURT);
        late.story_moves();
        assert_eq!(late.hunters, Hunters { lost: 2, settled: true });
        let mut early = World::new(0.0);
        early.arrive("fog_hollow@bandit_hollows", 100.0, 0.0, 0);
        early.died(110.0);
        early.arrive("fog_hollow@bandit_hollows", 200.0, 0.0, 0);
        early.boss_felled();
        early.beaten.push(CHIEF);
        assert_eq!(early.hunters, Hunters { lost: 0, settled: true }, "the hunters were never there to lose");
        assert!(early.hunters_home());
    }
}
