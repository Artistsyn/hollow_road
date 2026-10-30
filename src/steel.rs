//! Steel's flesh (`docs/story.md`, side quest 7, from his wish in the book's chapter 16). Once he
//! has read the rune, Steel asks to hear how each region's great foe fell, and tells his story a
//! piece for each: Richald Steel, the experiment to cheat death, the circle that took his
//! flesh. The last piece is his wish, to feel flesh again through the Eye. At the Sealed Slab,
//! with two of its shards carried, he asks to hold them. Letting him is a temptation and a cost:
//! he gives the hero his signet, the shards look back through the hero who held them out (`taint`),
//! and what he becomes is told at the end.

use crate::atlas::Key;
use crate::summons::{Answer, Ask};
use crate::visitors::{Visit, Visitor};
use crate::world::World;

/// Steel's shop in the Upper City, and the Spire's sealed door.
const UPPER: &str = "stone_upper";
const SLAB: &str = "sealed_slab";

/// What became of Steel: never met, still tin, made flesh by the Eye, or refused it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Steel { Unmet, Tin, Flesh, Refused }

pub const VISITS: &[Visit] = &[
    Visit { id: "steel_wants_relics", who: Visitor::Steel, place: UPPER, at: 18.0, when: |w| w.met("steel_reads") && w.beaten.contains(&"magistrate"), pages: &[
        "Red! You put the Magistrate down? Steel can smell his seal on you, or Steel's a teapot. Ha! Things like him leave a mark on whoever ends them.",
        "Here's the trade. You put down the big ones and come tell Steel how they fell, and Steel tells you what Steel was before the tin. A piece for a piece.",
        "Piece one. The name was Richald. Richald Steel. I had a body once, and I was not very careful with it.",
    ] },
    Visit { id: "steel_flesh_crown", who: Visitor::Steel, place: UPPER, at: 18.0, when: |w| w.met("steel_wants_relics") && w.beaten.contains(&"wraith_sanctum"), pages: &[
        "The Wraith Lord, gone? Empty inside, that one, like some helms Steel could mention. Gets 'em every time!",
        "Piece two. There were a few of us. Scholars, if you're generous. We sought the soul's secrets, to cheat death itself. Good intentions. Bad bet.",
        "One of us laughed like a goblin at a hanging. Laughed right up until the circle closed.",
    ] },
    Visit { id: "steel_flesh_halo", who: Visitor::Steel, place: UPPER, at: 18.0, when: |w| w.met("steel_flesh_crown") && w.beaten.contains(&"cathedral"), pages: &[
        "Varnholt's priest, down at last. He guarded too long, that one. Steel knows the feeling.",
        "Piece three. The circle took the others whole. It took Steel's flesh and left the rest of me stuck in this armour. I woke up hollow, and talking.",
        "Azrael, who keeps the shop with Steel, came later. Found her in a dig site, half-dead. Rebuilt her. Something ought to outlast me.",
    ] },
    Visit { id: "steel_flesh_heart", who: Visitor::Steel, place: UPPER, at: 18.0, when: |w| w.met("steel_flesh_halo") && w.beaten.contains(&"statue_court"), pages: &[
        "Eryndor's stone angel, broken. Stone keeps an oath better than flesh, they say. Steel would settle for the flesh.",
        "Last piece. Steel protects innocents from the Taint now. But Steel wonders. That Eye you're after: might it show what lies beyond? A chance to feel flesh again. To be whole.",
        "You carry two of its pieces now. Steel will be waiting at the Spire's door. Don't tell Helena. She worries.",
    ] },
    Visit { id: "steel_at_the_slab", who: Visitor::Steel, place: SLAB, at: 30.0, when: |w| w.steel_waits_at_the_slab(), pages: &[
        "Red! Thought Steel had missed you. So this is the door. Starlit stone, runes locked tight.",
        "You carry two pieces of it. Steel can hear them from here, humming like your stone, only deeper.",
        "Let Steel hold them. Only a moment. Only to feel... anything at all.",
    ] },
];

pub const ASKS: &[Ask] = &[
    Ask { visit: "steel_at_the_slab", answers: &[
        Answer { id: "steel_touches", say: "Let him hold the shards.", reply: &[
            "(He takes them in his gauntlets. The runes on his armour go dark. Inside the helm, for a moment, there is a face.)",
            "Ha... HA! Red, I can feel the cold. I'd forgotten cold.",
            "(Then the shards are back in your hands, warm. Something in you is listening that was not before.)",
            "Take this. It was mine when I had fingers for it. Steel won't need it now.",
        ], gift: Some("Richald's Signet") },
        Answer { id: "steel_refused", say: "No. Not with these.", reply: &[
            "...Ha. Right. Course not. Bad bet, Steel. Always was.",
            "Don't look at me like that, Red. Steel's fine. Tin doesn't get cold.",
        ], gift: None },
    ] },
];

impl World {
    /// Whether Steel waits at the Spire's sealed door: his story told, two shards carried, and the
    /// Order not yet sprung its trap.
    fn steel_waits_at_the_slab(&self) -> bool {
        self.met("steel_flesh_heart") && self.keys.contains(&Key::ShardI) && self.keys.contains(&Key::ShardII) && !self.met(crate::finale::STRUCK)
    }

    /// What became of Steel, as the road left him.
    pub fn steel_fate(&self) -> Steel {
        if self.answers.contains(&"steel_touches") { Steel::Flesh }
        else if self.answers.contains(&"steel_refused") { Steel::Refused }
        else if self.met("steel_reads") { Steel::Tin }
        else { Steel::Unmet }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(w: &mut World, stop: &str) { w.lingering = None; w.arrive(stop, 100.0, 0.0, 0); }

    #[test]
    fn steel_tells_his_story_a_piece_a_region_and_asks_for_the_shards_at_the_slab() {
        let mut w = World::new(0.0);
        w.meetings.push("steel_reads");
        assert_eq!(w.steel_fate(), Steel::Tin);
        at(&mut w, &crate::atlas::stop(UPPER, "lower_stairs"));
        assert!(w.visit().is_none(), "nothing to show him yet");
        for (beaten, id) in [("magistrate", "steel_wants_relics"), ("wraith_sanctum", "steel_flesh_crown"), ("cathedral", "steel_flesh_halo"), ("statue_court", "steel_flesh_heart")] {
            w.beaten.push(beaten);
            at(&mut w, &crate::atlas::stop(UPPER, "lower_stairs"));
            assert_eq!(w.visit().map(|v| v.id), Some(id));
            w.meet_visitor();
        }
        at(&mut w, &crate::atlas::stop(SLAB, "spire_stair"));
        assert!(w.visit().is_none_or(|v| v.id != "steel_at_the_slab"), "not without two shards");
        w.keys.extend([Key::ShardI, Key::ShardII]);
        w.meetings.push("olekeen_bound");
        if let Some(ask) = w.asking("olekeen_bound") { w.answer(ask.answers[0].id); }
        at(&mut w, &crate::atlas::stop(SLAB, "spire_stair"));
        assert_eq!(w.visit().map(|v| v.id), Some("steel_at_the_slab"));
        let notes = w.answer("steel_touches");
        assert!(notes.contains(&crate::world::Note::Found("Richald's Signet")), "{notes:?}");
        assert_eq!((w.steel_fate(), w.taint()), (Steel::Flesh, 2));
    }
}
