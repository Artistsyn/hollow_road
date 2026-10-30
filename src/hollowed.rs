//! The Hollowed Shrine's heart mural (`docs/story.md`, section 2): the way down to the Wraith
//! Lord's sanctum is a carved heart that opens to the hero's blood. At the Vine Halls' fork,
//! leaning toward it, E gives blood: a share of their whole health, never the last of it, and
//! the heart stays open after (`Key::Heart`, kept with the keys).

use crate::atlas::Key;
use crate::fight::Fight;
use crate::world::{Note, World, HOLD_M};

/// What the heart takes, of the hero's whole health.
pub const BLOOD: f32 = 0.4;

impl World {
    /// Whether the fork ahead has a way the heart bars, and nothing else does.
    fn heart_ahead(&self) -> bool {
        !self.keys.contains(&Key::Heart) && self.gates_ahead().iter().any(|&(b, keyed, open)| keyed && !open && self.wants_only_blood(b))
    }

    /// Whether E gives blood to the heart now: at the fork, close enough to choose, nothing on
    /// the hero, and the heart ahead.
    pub fn can_bleed(&self, fight: &Fight, walked: f64, choose_within: Option<f32>) -> bool {
        self.heart_ahead() && choose_within.is_some_and(|m| m <= 2.0 * HOLD_M) && !fight.hero.dead && Self::quiet(fight, walked)
    }

    /// The blood given: the heart opens, and stays open.
    pub fn bleed(&mut self, fight: &mut Fight) -> Vec<Note> {
        let hero = &mut fight.hero;
        let take = BLOOD * hero.stats.max_hp();
        hero.hp = (hero.hp - take).max(1.0);
        if !self.keys.contains(&Key::Heart) { self.keys.push(Key::Heart); }
        vec![Note::Bled]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_heart_bars_the_sanctum_until_it_is_paid_in_blood_and_never_kills() {
        assert!(crate::atlas::barred("vine_halls", "wraith_sanctum", &[Key::Lantern], &[]).is_some());
        assert!(crate::atlas::barred("vine_halls", "wraith_sanctum", &[Key::Heart], &[]).is_none());
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        f.hero.hp = 5.0;
        assert_eq!(w.bleed(&mut f), vec![Note::Bled]);
        assert!(f.hero.hp >= 1.0 && w.keys.contains(&Key::Heart));
        assert!(World::restored(0.0, &w.saved()).is_some_and(|r| r.keys.contains(&Key::Heart)));
    }
}
