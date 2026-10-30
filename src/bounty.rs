//! Bounty's trail (`docs/story.md`, side quest 9). Bounty was the fox-folk archer who slipped
//! through the Warrens like a ghost, left bread and hope for Olekeen's kin, and vanished before
//! Olekeen was grown. Old notices across the world carry his fox mark, each read one pointing on
//! to the next: the Warrens, the marsh, under Stonehollow, and Eryndor, where he followed the
//! void-wraiths that took a Warrens girl into the forest past the Blackwood. There his Taint echo
//! waits, and his longbow.
//! Olekeen, met after, learns through the hero what became of the one he idolised. **[new]**

use crate::atlas::Boss;
use crate::visitors::{Visit, Visitor};
use crate::world::World;

/// Where his last notice was left: the Ruins of Eryndor.
pub const ERYNDOR: &str = "angel_ruins";
/// Where the trail ends and his echo waits: the wood past the Blackwood's edge, gone wrong.
pub const FOREST: &str = "void_wood";
/// The last notice, read: his echo stands past it.
const LAST: &str = "fox_notice_eryndor";

pub static BOUNTY_ECHO: Boss = Boss { breed: &crate::side::BOUNTY_ECHO, souls: 7800, key: None, mini: true, remnant: None, sought: false };

pub const VISITS: &[Visit] = &[
    Visit { id: "fox_notice_lane", who: Visitor::Notice, place: "warrens_lane", at: 24.0, when: |_| true, pages: &[
        "(An old bounty notice under the eaves, curled with damp: RATS IN THE GRAIN STORE. TWO COPPER.)",
        "(Across it someone has burned a small fox's head, and scratched beneath: Paid in bread. Shared out along Stilt Lane.)",
        "(On the back, in the same hand: The marsh lights aren't lights. Going to see.)",
    ] },
    Visit { id: "fox_notice_marsh", who: Visitor::Notice, place: "vale_edge", at: 18.0, when: |w| w.met("fox_notice_lane"), pages: &[
        "(A notice on a leaning post, the ink nearly gone: LIGHTS IN THE MARSH. CHILDREN MISSING. ANY REWARD.)",
        "(The fox's head again, burned deep. Under it: Done. Brought the children home. Took no reward.)",
        "(Scratched below: Stonehollow pays better. I owe the Warrens a winter. Going.)",
    ] },
    Visit { id: "fox_notice_sewers", who: Visitor::Notice, place: "sewers", at: 22.0, when: |w| w.met("fox_notice_marsh"), pages: &[
        "(A registrar's notice, stamped and old: SOMETHING UNDER THE BRIDGES. TAKES THE NIGHT-SOIL MEN. FORTY SILVER.)",
        "(The fox's head, and: Done. Silver sent home to the Warrens.)",
        "(Under it, cramped and hurried: Wraiths out of Eryndor's trees. They took a Warrens girl, a goblin, yellow-eyed. Going after them. If I don't come back, someone tell her brother I tried.)",
    ] },
    Visit { id: LAST, who: Visitor::Notice, place: ERYNDOR, at: 20.0, when: |w| w.met("fox_notice_sewers"), pages: &[
        "(A notice that was never posted, pinned with an arrow to a fallen angel's open hand: VOID-WRAITHS. NO REWARD. NONE NEEDED.)",
        "(No fox's head on this one. The hand that wrote the rest was shaking: Followed them back past the Blackwood's edge, where the trees grow wrong. Found her. Too late. They're coming back.)",
        "(At the bottom: Whoever finds this, take the bow.)",
    ] },
    Visit { id: "olekeen_hears_of_bounty", who: Visitor::Olekeen, place: "sealed_slab", at: 18.0, when: |w| w.bounty_found() && w.met("olekeen_bound") && !w.met(crate::finale::STRUCK), pages: &[
        "That bow. Olekeen know that bow. Fox on the grip. Olekeen see it every night, before Olekeen sleep.",
        "Where? ...Twisted wood, past Blackwood. Wraiths. Olekeen's sister, wraiths take her. Long ago.",
        "Bounty not leave. Bounty go to get her. All these years Olekeen think: Bounty just gone. Bounty forget.",
        "(He holds the bow a long time, then pushes it back into your hands.) No. Note say: whoever find, take bow. Hunter find. Hunter keep. Olekeen keep the rest.",
    ] },
    // Told after the trap instead, once he is out of the Order's cells.
    Visit { id: "olekeen_hears_of_bounty_freed", who: Visitor::Olekeen, place: crate::finale::CELLS, at: 24.0, when: |w| w.bounty_found() && w.met(crate::finale::FREED) && !w.met("olekeen_hears_of_bounty"), pages: &[
        "That bow. Olekeen know that bow. Fox on the grip. Olekeen see it every night, before Olekeen sleep.",
        "Where? ...Twisted wood, past Blackwood. Wraiths. Olekeen's sister, wraiths take her. Long ago.",
        "Bounty not leave. Bounty go to get her. All these years Olekeen think: Bounty just gone. Bounty forget.",
        "(He holds the bow a long time, then pushes it back into your hands.) No. Note say: whoever find, take bow. Hunter find. Hunter keep. Olekeen keep the rest.",
    ] },
];

impl World {
    /// Whether Bounty's echo waits in Eryndor: his last notice read.
    pub fn bounty_waits(&self) -> bool { self.met(LAST) }

    /// Whether the end of Bounty's trail has been found: his echo laid down, or his bow carried
    /// (a new pilgrimage forgets the duel, not the bow).
    pub fn bounty_found(&self) -> bool {
        self.bounty_waits() && (self.beaten.contains(&FOREST) || self.gear.owns(crate::gear::find("Bounty's Longbow")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fox_marked_notices_lead_one_to_the_next_and_on_to_his_echo_and_his_bow() {
        let mut w = World::new(0.0);
        let at = |w: &mut World, stop: &str| { w.lingering = None; w.arrive(stop, 100.0, 0.0, 0); };
        at(&mut w, &crate::atlas::stop("vale_edge", "vale"));
        assert!(w.visit().is_none_or(|v| v.id != "fox_notice_marsh"), "the marsh notice waits for the first");
        for (stop, id) in [(crate::atlas::stop("warrens_lane", "warrens"), "fox_notice_lane"), (crate::atlas::stop("vale_edge", "vale"), "fox_notice_marsh"),
            (crate::atlas::stop("sewers", "lower_stairs"), "fox_notice_sewers"), (crate::atlas::stop(ERYNDOR, "blackwood"), LAST)] {
            at(&mut w, &stop);
            assert_eq!(w.visit().map(|v| v.id), Some(id), "{stop}");
            w.meet_visitor();
        }
        at(&mut w, &crate::atlas::stop(ERYNDOR, "blackwood"));
        assert!(w.boss_here().is_none(), "his echo is not in the ruins");
        at(&mut w, &crate::atlas::stop(FOREST, "blackwood"));
        assert!(w.boss_here().is_some_and(|(_, b)| std::ptr::eq(b, &BOUNTY_ECHO)), "his echo stands past the last");
        let notes = w.echo_leaves(&BOUNTY_ECHO);
        assert_eq!(notes, vec![crate::world::Note::Found("Bounty's Longbow")]);
        w.beaten.push(FOREST);
        assert!(w.bounty_found());
    }
}
