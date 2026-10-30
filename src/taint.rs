//! The Taint on the hero (`docs/story.md`, section 8: the fifth thing the ending answers to). The
//! book's Taint twists flesh and minds, can grow, and can be cleansed; Helena heals it. Two things
//! let it into the hero: the heart mural paid in blood, and the shards of the Eye held out for
//! Steel to touch. Each lends strength and spoils the flasks a little, and Helena can draw each
//! back out once, if the hero lets her. Kept, enough of it and the three guardians will not
//! answer a call through the Eye.

use crate::atlas::Key;
use crate::summons::{Answer, Ask};
use crate::visitors::{Visit, Visitor};
use crate::world::World;

/// Helena's chapel under Stonehollow.
const CHAPEL: &str = "sewers";
/// What the heart mural's blood price leaves in the hero, and what holding the shards out does.
const HEART: u32 = 1;
const EYE: u32 = 2;
/// Taint the three guardians will not answer through.
pub const UNHEARD: u32 = 2;
/// Each point's strength (more damage) and blight (flasks heal less).
const STRENGTH: f32 = 0.05;
const BLIGHT: f32 = 0.06;

pub const VISITS: &[Visit] = &[
    Visit { id: "helena_sees_the_heart", who: Visitor::Helena, place: CHAPEL, at: 44.0, when: |w| w.met_helena() && w.keys.contains(&Key::Heart), pages: &[
        "(She takes your wrist and turns it to the lamp. Under the skin, faint, a violet line runs toward the heart.)",
        "You paid that shrine in blood. It kept more than it took.",
        "I can draw it out. It will hurt, and you will be weaker for it after. Or keep it. It is strength, of a kind. My order thought so too.",
    ] },
    Visit { id: "helena_sees_the_eye", who: Visitor::Helena, place: CHAPEL, at: 44.0, when: |w| w.met_helena() && w.answers.contains(&"steel_touches"), pages: &[
        "(She stops halfway across the chapel. The frost on her sleeves rises on its own.)",
        "You held them out. The pieces of the Eye, for that tin fool to touch. Something looked back through you while he did. I can smell it.",
        "I can draw it out, or you can keep it. But hear me: whatever names you mean to call with that Eye, they will not come to a voice the Taint speaks through.",
    ] },
];

pub const ASKS: &[Ask] = &[
    Ask { visit: "helena_sees_the_heart", answers: &[
        Answer { id: "helena_draws_heart", say: "Let her draw it out.", reply: &[
            "(Frost along your arm, then fire. Something thin and violet slides out at your wrist and dies in the cold air.)",
            "There. Do not go paying shrines in blood again.",
        ], gift: None },
        Answer { id: "helena_heart_kept", say: "Keep it.", reply: &["As you like. When it starts to sing, I will not be able to reach it."], gift: None },
    ] },
    Ask { visit: "helena_sees_the_eye", answers: &[
        Answer { id: "helena_draws_eye", say: "Let her draw it out.", reply: &[
            "(It takes longer this time. When it is done she sits down on the chapel floor and does not speak for a while.)",
            "Gone. Tell Steel he owes me a candle. Several.",
        ], gift: None },
        Answer { id: "helena_eye_kept", say: "Keep it.", reply: &["Then I hope you never need the old names. They will not hear you."], gift: None },
    ] },
];

impl World {
    /// Whether Helena has been met in her chapel.
    pub(crate) fn met_helena(&self) -> bool { self.met("helena_reads") || self.met("helena_struck") || self.met("helena_reads_late") }

    /// The Taint in the hero: what let it in, less what Helena drew back out.
    pub fn taint(&self) -> u32 {
        let said = |id: &str| self.answers.contains(&id);
        let heart = if self.keys.contains(&Key::Heart) && !said("helena_draws_heart") { HEART } else { 0 };
        let eye = if said("steel_touches") && !said("helena_draws_eye") { EYE } else { 0 };
        heart + eye
    }

    /// What everything worn does, with the Taint's strength and blight on top.
    pub fn kit(&self) -> crate::gear::Kit {
        let mut kit = self.gear.kit();
        let taint = self.taint() as f32;
        kit.damage *= 1.0 + STRENGTH * taint;
        kit.heal *= (1.0 - BLIGHT * taint).max(0.0);
        kit.flasks = self.glass();
        kit
    }

    /// The Taint for the road's status line, when there is any.
    pub fn taint_line(&self) -> String {
        match self.taint() { 0 => String::new(), taint => format!("  taint {taint}") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_taint_comes_with_the_blood_and_the_shards_and_helena_draws_each_out_once() {
        let mut w = World::new(0.0);
        assert_eq!(w.taint(), 0);
        let (damage, heal) = (w.kit().damage, w.kit().heal);
        w.keys.push(Key::Heart);
        w.answers.push("steel_touches");
        assert_eq!(w.taint(), HEART + EYE);
        assert!(w.kit().damage > damage && w.kit().heal < heal, "strength, and the flasks spoiled");
        assert!(w.visit().is_none(), "nobody to see it before Helena is met");
        w.meetings.push("helena_reads");
        w.arrive(&crate::atlas::stop(CHAPEL, "lower_stairs"), 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("helena_sees_the_heart"));
        w.meet_visitor();
        w.answer("helena_draws_heart");
        assert_eq!(w.taint(), EYE);
        assert!(w.asking("helena_sees_the_heart").is_none(), "drawn once, not asked again");
        w.answer("helena_eye_kept");
        assert_eq!(w.taint(), EYE, "kept is kept");
        assert!(w.asking("helena_sees_the_eye").is_none());
    }
}
