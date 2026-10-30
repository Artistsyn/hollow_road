//! Vren's relic trade (`docs/story.md`, side quest 4): at the Broken Spire, once he has seen the
//! rune, he asks after what the hero's kills left behind (the Screamer's voice, the cage's bone,
//! the Mother's finger) and pays for each in echoes and in what he knows. A relic sold is gone,
//! and the smith can no longer forge it: what Vren tells is the price of the weapon. The last he
//! buys earns his loupe as well.

use crate::summons::{Answer, Ask};
use crate::visitors::{Visit, Visitor};
use crate::world::{Note, World};

/// Vren's crooked tower down the Warrens' side path.
const SPIRE: &str = "broken_spire";
/// The relics Vren buys: the answer that sells each, the remnant, and what he pays.
const BUYS: [(&str, &str, u32); 3] = [
    ("vren_buys_voice", "Screamer's Drowned Voice", 900),
    ("vren_buys_bone", "Violet-Veined Bone", 800),
    ("vren_buys_finger", "The Mother's Hooked Finger", 1400),
];

pub const VISITS: &[Visit] = &[
    Visit { id: "vren_wants_voice", who: Visitor::Vren, place: SPIRE, at: 20.0, when: |w| w.vren_would_buy(0), pages: &[
        "That smell. You've a piece of the Hollowmere thing on you.",
        "Still murmurs, don't it? I'll take it. Coin, and what the last one like it told me.",
    ] },
    Visit { id: "vren_wants_bone", who: Visitor::Vren, place: SPIRE, at: 20.0, when: |w| w.vren_would_buy(1), pages: &[
        "The cage's rib. You kept it. Clever, or greedy. I like both.",
        "Sell it me and I'll tell you what the violet in it is.",
    ] },
    Visit { id: "vren_wants_finger", who: Visitor::Vren, place: SPIRE, at: 20.0, when: |w| w.vren_would_buy(2), pages: &[
        "The Mother's finger. Somebody at the guild'll want to know she's dead. I'd rather know what she held.",
        "I'll pay more than it's worth, and throw in something worth more than coin. Don't ask me why. You won't like the answer.",
    ] },
];

pub const ASKS: &[Ask] = &[
    Ask { visit: "vren_wants_voice", answers: &[
        Answer { id: BUYS[0].0, say: "Sell him the Drowned Voice.", reply: &[
            "...There. Hear it? Not the well. Under the well. Something down there taught it to talk.",
            "Somebody wanted Hollowmere listening, hunter. Ask yourself who gains from a village that hears voices.",
        ], gift: None },
        Answer { id: "vren_keeps_off_voice", say: "Keep it.", reply: &["Suit yourself. The smith'll hammer it into something that screams on every swing."], gift: None },
    ] },
    Ask { visit: "vren_wants_bone", answers: &[
        Answer { id: BUYS[1].0, say: "Sell him the bone.", reply: &[
            "Taint. Not rot, not magic. It's what bleeds through where the Veil runs thin.",
            "Bones that drink it start to hum. Your stone hums too. Think on that, some night you can't sleep.",
        ], gift: None },
        Answer { id: "vren_keeps_off_bone", say: "Keep it.", reply: &["Then mind it don't start humming in your pack."], gift: None },
    ] },
    Ask { visit: "vren_wants_finger", answers: &[
        Answer { id: BUYS[2].0, say: "Sell him the finger.", reply: &[
            "She kept them because something taught her to keep. Same teacher as the well, I'd wager.",
            "Here. A loupe. Look at any relic through it and you'll see what I see in everything: its price.",
        ], gift: Some("Vren's Loupe") },
        Answer { id: "vren_keeps_off_finger", say: "Keep it.", reply: &["Your trophy. Don't come crying when it scratches at the inside of your pack."], gift: None },
    ] },
];

impl World {
    /// Whether Vren asks for relic `k` of `BUYS`: he has seen the rune, it is carried, and he
    /// has not been answered about it.
    fn vren_would_buy(&self, k: usize) -> bool {
        let (_, relic, _) = BUYS[k];
        let knows = self.met("vren_shows_blade") || self.met("vren_asks_blade");
        let asked = ASKS[k].answers.iter().any(|a| self.answers.contains(&a.id));
        knows && !asked && self.remnants.contains(&relic)
    }

    /// What selling to Vren under answer `id` does, if it sold anything: the relic gone, the echoes paid.
    pub(crate) fn sell_relic(&mut self, id: &str) -> Vec<Note> {
        let Some(&(_, relic, pay)) = BUYS.iter().find(|(sold, _, _)| *sold == id) else { return Vec::new() };
        let Some(at) = self.remnants.iter().position(|r| *r == relic) else { return Vec::new() };
        self.remnants.remove(at);
        self.souls += pay;
        vec![Note::Paid(Visitor::Vren.name(), pay)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vren_buys_a_relic_once_he_knows_the_rune_and_the_smith_loses_it() {
        let mut w = World::new(0.0);
        w.remnants.push("Screamer's Drowned Voice");
        assert!(!w.vren_would_buy(0), "not before he has seen the rune");
        w.meetings.push("vren_shows_blade");
        assert!(w.vren_would_buy(0) && !w.vren_would_buy(1), "only what is carried");
        let souls = w.souls;
        let notes = w.answer(BUYS[0].0);
        assert_eq!(notes, vec![Note::Paid("Vren", 900)]);
        assert_eq!(w.souls, souls + 900);
        assert!(!w.remnants.contains(&"Screamer's Drowned Voice") && !w.vren_would_buy(0), "sold, and not asked again");
        // Kept: he does not ask again, and the smith still has it.
        w.remnants.push("Violet-Veined Bone");
        w.answer("vren_keeps_off_bone");
        assert!(!w.vren_would_buy(1) && w.remnants.contains(&"Violet-Veined Bone"));
        // The last of them earns his loupe.
        w.remnants.push("The Mother's Hooked Finger");
        let notes = w.answer(BUYS[2].0);
        assert!(notes.contains(&Note::Found("Vren's Loupe")) && notes.contains(&Note::Paid("Vren", 1400)), "{notes:?}");
    }
}
