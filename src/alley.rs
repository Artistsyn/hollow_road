//! The Underborough alley (`docs/story.md`, section 2, Stonehollow): once the hero carries the
//! rune under the city, the rune flares and a hooded thing strikes out of the dark, with no veil
//! and no name, tests them, and melts away when hurt. It is the Void Stalker, met again at the
//! Whispering Altars; Helena knows it struck.

use crate::atlas::Boss;
use crate::fight::Fight;
use crate::world::{Note, World};

/// Where it strikes: the Old Sewers, the Underborough's alleys, before Helena's chapel.
const UNDER: &str = "sewers";
const AT: f32 = 24.0;
/// The share of its health left when it melts away.
pub const MELTS: f32 = 0.6;

pub static HOODED_THING: Boss = Boss { breed: &crate::side::HOODED, souls: 0, key: None, mini: true, remnant: None, sought: false };

impl World {
    /// The hooded thing, waiting in the alley for whoever carries the rune: metres in, and it.
    pub(crate) fn alley_here(&self) -> Option<(f32, &'static Boss)> {
        (self.here().id == UNDER && self.carries_rune() && !self.met(crate::cast::STRUCK)).then_some((AT, &HOODED_THING))
    }

    /// Whether the boss here strikes unannounced rather than waiting behind a veil.
    pub fn strikes(&self) -> bool {
        self.boss_here().is_some_and(|(_, b)| std::ptr::eq(b, &HOODED_THING))
    }

    /// The strike, now: the hero has walked into the alley's dark, nothing else on them.
    pub fn strike_due(&self, fight: &Fight, walked: f64) -> Option<&'static Boss> {
        self.boss_due(fight, walked).filter(|_| self.strikes())
    }

    /// It melted away: known to have struck (Helena hears of it), and never again here.
    pub(crate) fn alley_fled(&mut self) -> Vec<Note> {
        self.meetings.push(crate::cast::STRUCK);
        vec![Note::Fled(HOODED_THING.name())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hooded_thing_strikes_once_without_a_veil_for_whoever_carries_the_rune() {
        let mut w = World::new(0.0);
        w.arrive("sewers@lower_stairs", 100.0, 0.0, 0);
        assert!(w.boss_here().is_none(), "nothing to draw it without the rune");
        w.keys.push(crate::atlas::Key::Rune);
        assert!(w.strikes() && w.veil().is_none(), "no veil before it");
        let f = Fight::new(Vec::new());
        let due = (0..400).map(|k| 100.0 + k as f64 * 0.25).find(|&m| w.strike_due(&f, m).is_some());
        assert!(due.is_some(), "it never struck");
        assert_eq!(w.boss_felled(), vec![Note::Fled("A Hooded Thing")]);
        assert!(w.met(crate::cast::STRUCK) && w.boss_here().is_none() && !w.beaten.contains(&UNDER));
    }
}
