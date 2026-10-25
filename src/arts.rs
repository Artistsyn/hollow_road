//! Weapon arts: what a weapon the smith forges from a boss's remnant does besides its strikes,
//! each echoing the boss it came from (Q, or ART on a touch screen). An art is paid for from the
//! art meter, which blows landed, parries and blows slipped at the last moment fill; an art's own
//! blows never do, so the meter rewards fighting well rather than pressing Q.
//!
//! Both fights carry them out their own way (`fight::Fight::art_fires`, `side::SideFight::art_fires`);
//! what an art costs, how long it takes and how hard it strikes is the same in both, here.

use crate::fight::Spec;

/// The meter's top.
pub const ART_MAX: f32 = 100.0;

/// What fills the meter: a light, a heavy, a riposte landed; a parry; a roll or backstep that
/// slipped a blow at the last moment; a bulwark's last-moment block.
pub const GAIN_LIGHT: f32 = 7.0;
pub const GAIN_HEAVY: f32 = 12.0;
pub const GAIN_RIPOSTE: f32 = 25.0;
pub const GAIN_PARRY: f32 = 30.0;
pub const GAIN_DODGE: f32 = 15.0;
pub const GAIN_BLOCK: f32 = 15.0;

/// How long Stone Skin holds (ticks), and what of a blow then gets through.
pub const STONE_TICKS: u32 = 600;
pub const STONE_TAKEN: f32 = 0.7;

/// Ticks from Sentence's mark to the blow that falls on it.
pub const SENTENCE_DELAY: u32 = 50;

/// The thrown Hatchet: metres a second, and how far it flies before it turns back.
pub const HATCHET_SPEED: f32 = 15.0;
pub const HATCHET_RANGE: f32 = 9.0;

/// How far a blast throws what it catches back, metres, and how long what it staggers or
/// interrupts stays off balance (on the road; a yard foe is open for its own time, halved).
pub const SHOVE: f32 = 2.0;
pub const REELS: u32 = 70;

/// Light Lance's dash and Wingbeat's dive: metres covered.
pub const LANCE_DASH: f32 = 4.5;
pub const DIVE: f32 = 3.0;
/// When Wingbeat's dive lands, ticks from the start (the gust goes at the windup's end), and how
/// far ahead of where it comes down it strikes, metres (the gust having thrown foes back first).
pub const DIVE_AT: u32 = 52;
pub const DIVE_REACH: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponArt {
    /// The Bell-Hammer: a slam that rings through everything near, throwing it off balance (and
    /// the hammer rings the stunned harder).
    Toll,
    /// The Drowned Wail: a scream that breaks every windup within reach and drinks from all it
    /// catches; arrows in flight fall.
    Wail,
    /// The Chieftain's Cleaver: thrown down the lane, it bites the first foe and comes back,
    /// biting again on the way.
    Hatchet,
    /// The Gavel of Sentence: marks the nearest foe ahead; a moment later a blow no guard or
    /// shield turns falls on it.
    Sentence,
    /// The Reaper's Glaive: a sweep across every lane that drinks from all it cuts.
    Reap,
    /// The Lance of Light: a dash down the lane, untouchable, piercing what it meets.
    Lance,
    /// The Stoneheart Greatsword: the body turns to stone (no blow staggers it, less gets
    /// through) and a stomp throws back what is near.
    StoneSkin,
    /// The Feather of the Fallen Light: a wingbeat's gust throws everything ahead back, then a
    /// dive strikes down on the nearest.
    Wingbeat,
}

impl WeaponArt {
    #[cfg(test)]
    pub const ALL: [WeaponArt; 8] = [WeaponArt::Toll, WeaponArt::Wail, WeaponArt::Hatchet, WeaponArt::Sentence, WeaponArt::Reap, WeaponArt::Lance, WeaponArt::StoneSkin, WeaponArt::Wingbeat];

    pub fn name(self) -> &'static str {
        match self {
            WeaponArt::Toll => "Toll", WeaponArt::Wail => "Wail", WeaponArt::Hatchet => "Hatchet", WeaponArt::Sentence => "Sentence",
            WeaponArt::Reap => "Reap", WeaponArt::Lance => "Light Lance", WeaponArt::StoneSkin => "Stone Skin", WeaponArt::Wingbeat => "Wingbeat",
        }
    }

    /// What it takes from the meter (out of `ART_MAX`), paid when it goes off: one cut off before
    /// then costs nothing.
    pub fn cost(self) -> f32 {
        match self {
            WeaponArt::Toll => 50.0, WeaponArt::Wail => 40.0, WeaponArt::Hatchet => 35.0, WeaponArt::Sentence => 55.0,
            WeaponArt::Reap => 45.0, WeaponArt::Lance => 40.0, WeaponArt::StoneSkin => 50.0, WeaponArt::Wingbeat => 60.0,
        }
    }

    /// Its timing and weight on the road (the yard's is the same): windup, live and recovery
    /// ticks, stamina, untouchable ticks, reach, damage and stance damage (times the hero's
    /// strike damage, as a strike's), the lanes beside it reaches, and armour against a flinch.
    pub fn spec(self) -> Spec {
        let s = |frames, stamina, reach, damage, stance, wide, armour| Spec { frames, stamina, iframes: None, reach, damage, stance, wide, armour };
        match self {
            // A long haul up, then the slam: armoured until it lands.
            WeaponArt::Toll => s((34, 6, 30), 30.0, 3.2, 30.0, 45.0, 1, 40),
            WeaponArt::Wail => s((20, 8, 24), 15.0, 7.0, 10.0, 10.0, 2, 0),
            WeaponArt::Hatchet => s((16, 2, 22), 18.0, HATCHET_RANGE, 26.0, 22.0, 0, 0),
            WeaponArt::Sentence => s((24, 4, 26), 20.0, 8.0, 60.0, 70.0, 0, 0),
            WeaponArt::Reap => s((28, 8, 30), 32.0, 3.4, 24.0, 20.0, 2, 30),
            WeaponArt::Lance => Spec { iframes: Some((4, 30)), ..s((10, 14, 20), 25.0, 1.6, 32.0, 30.0, 0, 0) },
            WeaponArt::StoneSkin => s((30, 4, 26), 20.0, 2.6, 16.0, 30.0, 1, 60),
            // The gust at the windup's end, the dive (untouchable in the air) at `DIVE_AT`.
            WeaponArt::Wingbeat => Spec { iframes: Some((24, DIVE_AT)), ..s((22, 34, 20), 30.0, 5.0, 40.0, 40.0, 1, 0) },
        }
    }

    /// Ticks from the start that it goes off (and is paid for).
    pub fn fires(self) -> u32 { self.spec().frames.0 }

    /// Of what it deals, what comes back as health (besides what the weapon drinks).
    pub fn drinks(self) -> f32 { match self { WeaponArt::Wail => 1.0, WeaponArt::Reap => 0.5, _ => 0.0 } }

    /// Whether it is drawn with the weapon's light swing (a throw, a scream) rather than its heavy.
    pub fn light(self) -> bool { matches!(self, WeaponArt::Hatchet | WeaponArt::Wail) }

    /// What it does, as each forged weapon's line in the shrine's gear panel says (`gear::ITEMS`,
    /// whose lines are constants; a test holds them to these).
    #[cfg(test)]
    pub fn text(self) -> &'static str {
        match self {
            WeaponArt::Toll => "a slam that rings through all near, throwing them off balance",
            WeaponArt::Wail => "a scream that breaks every windup in reach, drinks from all it catches and drops arrows",
            WeaponArt::Hatchet => "thrown down the lane, bites the first foe and comes back, biting again",
            WeaponArt::Sentence => "marks the foe ahead; a moment later a blow no guard turns falls on it",
            WeaponArt::Reap => "a sweep across every lane that drinks from all it cuts",
            WeaponArt::Lance => "a dash down the lane, untouchable, piercing what it meets",
            WeaponArt::StoneSkin => "ten seconds of stone (no blow staggers you, 30% less gets through) and a stomp that throws back what is near",
            WeaponArt::Wingbeat => "a gust throws everything ahead back, then a dive strikes down on the nearest",
        }
    }
}

/// What the meter takes from a fight's events this tick (an art's own blows give nothing).
pub fn gained(events: &[crate::fight::Event]) -> f32 {
    use crate::fight::{Event, Move};
    events.iter().map(|e| match e {
        Event::Hit { mv: Move::Light, .. } => GAIN_LIGHT,
        Event::Hit { mv: Move::Heavy, .. } => GAIN_HEAVY,
        Event::Hit { mv: Move::Riposte, .. } => GAIN_RIPOSTE,
        Event::Parried { .. } => GAIN_PARRY,
        Event::JustDodge { .. } => GAIN_DODGE,
        Event::PerfectBlock => GAIN_BLOCK,
        _ => 0.0,
    }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_art_costs_less_than_a_full_meter_and_goes_off_before_it_ends() {
        for a in WeaponArt::ALL {
            let s = a.spec();
            assert!(a.cost() > 0.0 && a.cost() <= ART_MAX, "{a:?}");
            assert!(a.fires() < s.frames.0 + s.frames.1 + s.frames.2, "{a:?}");
            assert!(!a.text().is_empty() && !a.name().is_empty());
        }
        assert!(WeaponArt::Wingbeat.fires() < DIVE_AT && DIVE_AT < { let f = WeaponArt::Wingbeat.spec().frames; f.0 + f.1 });
    }

    #[test]
    fn the_meter_fills_from_fighting_well_and_not_from_arts() {
        use crate::fight::{Event, Move};
        let ev = [Event::Hit { enemy: 0, mv: Move::Light }, Event::Parried { enemy: 0 }, Event::Hit { enemy: 0, mv: Move::Art }];
        assert_eq!(gained(&ev), GAIN_LIGHT + GAIN_PARRY);
        // A full meter is a handful of blows and a parry or two, not a single exchange.
        assert!(ART_MAX / GAIN_LIGHT > 8.0 && ART_MAX / GAIN_PARRY > 3.0);
    }
}
