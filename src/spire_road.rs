//! The way to the Celestial Spire (`docs/story.md`, section 2.7): the Starlit Wastes, where the
//! angels' construct keeps the old pilgrims' road, and the Shattered Sky Peaks, where the sky broke
//! and a drake nests in the tear. Olekeen went ahead, as he promised at the Sunken Steps, and cut
//! his mark into the stones; the Order's boots followed his arrow. On the Peaks his last mark
//! breaks off where their gas took him, before they tied him under the Sealed Slab. **[new]**

use crate::visitors::{Visit, Visitor};

/// The Starlit Wastes and the Shattered Sky Peaks.
pub const WASTES: &str = "starlit_wastes";
pub const PEAKS: &str = "sky_peaks";

pub const VISITS: &[Visit] = &[
    Visit { id: "olekeen_mark_wastes", who: Visitor::Mark, place: WASTES, at: 20.0, when: |_| true, pages: &[
        "(Scratched big and proud into a fallen obelisk: OLEKEEN FIRST. Under it, an arrow points on, north, toward peaks under a broken sky.)",
        "(Around the stone the glassy ground is scuffed by many heavy boots, all going the same way. Someone is following his arrow.)",
    ] },
    Visit { id: "olekeen_mark_peaks", who: Visitor::Mark, place: PEAKS, at: 24.0, when: |_| true, pages: &[
        "(Another of his marks, cut into the rock of the pass: OLEKEEN FI... The last letters are one long scrape, as if the hand cutting them was pulled away.)",
        "(At the foot of the rock, broken clay: little round pots, smashed. The air here still smells sweet, like rot.)",
        "(Drag marks lead on up the pass, toward the Spire's stair.)",
    ] },
];

#[cfg(test)]
mod tests {
    use crate::world::World;

    #[test]
    fn olekeens_marks_lead_up_the_road_to_where_the_order_took_him() {
        let mut w = World::new(0.0);
        for (place, from, id) in [(super::WASTES, "statue_court", "olekeen_mark_wastes"), (super::PEAKS, super::WASTES, "olekeen_mark_peaks")] {
            w.lingering = None;
            w.arrive(&crate::atlas::stop(place, from), 100.0, 0.0, 0);
            assert_eq!(w.visit().map(|v| v.id), Some(id));
            w.meet_visitor();
        }
        assert!(crate::atlas::edge("spire_stair", "sealed_slab").is_some_and(|e| matches!(e.gate, crate::atlas::Gate::Keys(_))), "the slab opens to the shards");
    }
}
