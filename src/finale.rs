//! The last act (`docs/story.md`, sections 1 and 8). At the Summit the Eye is made whole and the
//! Order springs its trap: gas, and the Eye, Olekeen's party and Fin-S (if he was woken) carried
//! off; the hero wakes at the Sealed Slab's fire. Down the Spire's far side is the Order's
//! stronghold: Olekeen in its cells, Fin-S on a plinth in its foundry as Unit 0 (taken from the
//! Summit if he was woken, from his roots if not), and Exodus with the Eye. The Order made Fin-S
//! as the Eye's conduit, to bring the world of Voryn (whom it knows as the Prophet) wholly into
//! this one. Freed before Exodus falls, he is no door, and Voryn takes Exodus instead; left on his
//! plinth, he is finished, and Voryn comes through him whole. Once that is beaten, Olekeen hands
//! the hero the Eye, and what they say decides the ending.
//! Calling the three guardians is only offered to a hero who sealed enough of the tears (the
//! spirits say so at the Summit, and nothing else does). What became of everyone follows from
//! the hero's choices on the road.

use crate::atlas;
use crate::steel::Steel;
use crate::summons::{Answer, Ask};
use crate::visitors::{Visit, Visitor};
use crate::world::{Go, World};

/// The trap sprung at the Summit, and Fin-S taken with the rest.
pub const STRUCK: &str = "order_struck";
const TAKEN: &str = "fins_taken";
pub const FREED: &str = "olekeen_freed";
pub const CELLS: &str = "penitent_cells";
const FOUNDRY: &str = "unit_foundry";
const SLAB: &str = "sealed_slab";
/// The tears in the Veil the hero can close: the things that came through them, beaten.
const TEARS: [&str; 5] = ["east_gate", "black_water", "fog_hollow", "whisper_altars", "sky_peaks"];
const TEARS_TO_BE_HEARD: usize = 3;
/// The meetings and answers of the last act, forgotten when the pilgrimage begins again.
const ACT: [&str; 9] = [STRUCK, TAKEN, FREED, "fins_unit_zero", "fins_unit_zero_waking", "eye_offered", "eye_offered_sealed", "eye_offered_unfreed", "eye_offered_unfreed_sealed"];

/// What the hero did with the Eye.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending { Seal, Fall, Order, Break }

/// What became of Fin-S: freed, following, or finished as Voryn's door and broken in the chamber.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit { Free, Follows, Fell }

/// What the road left behind it, told after the ending: Atrael stopped or not (None if never
/// met), how many of Krix and Lurz the hollow took (None if never settled), Fin-S, Steel, the
/// Taint Helena saw the hero carry out (0 if she never met them), and Bounty's trail followed to
/// its end (Some), told to Olekeen or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fates { pub atrael: Option<bool>, pub lost: Option<u8>, pub fin_s: Unit, pub steel: Steel, pub taint: u32, pub bounty: Option<bool> }

/// Olekeen, left in his cell, in the chamber all the same.
const OLEKEEN_CHEWED_FREE: &str = "(Olekeen limps in, rope still round one wrist.) Olekeen have teeth. Olekeen chew. Olekeen follow the noise.";

const OLEKEEN_HANDS_IT_OVER: [&str; 3] = [
    "Down. Thing in it gone. Olekeen see it go. Like smoke, back into Eye.",
    "Here. Eye. Olekeen not hold it long. Olekeen look at it. It look back.",
    "Hunter choose. Olekeen just hunter. Olekeen not choose world.",
];

pub const VISITS: &[Visit] = &[
    Visit { id: FREED, who: Visitor::Olekeen, place: CELLS, at: 40.0, when: |w| w.met(STRUCK), pages: &[
        "Hunter! Olekeen think, this time nobody come. Olekeen wrong. Good wrong.",
        "Gas again. Same beak men. Leader the rabbit. White fur, black beak. Olekeen smell everything. Olekeen not smell rabbit.",
        "They take Eye down, deep. Big room. Rabbit say: do again what the cloaks did. Open it. Olekeen not know what that mean. Olekeen think: bad.",
        "Other way, they carry things. Wooden things. Sawdust smell.",
        "Rabbit say: wooden one is door. Door for Prophet. Olekeen not know Prophet. Olekeen not like door.",
        "Bow gone. Again. Olekeen have teeth. Olekeen come behind.",
    ] },
    Visit { id: "fins_unit_zero", who: Visitor::FinS, place: FOUNDRY, at: 18.0, when: |w| w.met(TAKEN), pages: &[
        "(Rows of wooden figures stand on plinths, faceless, unfinished. At the end of the rows one has a face, and moss in its joints.)",
        "Fiiin... ess. Unit... zero. They say: finish Fin-S. Tomorrow. Make Fin-S... whole.",
        "Then Prophet come. Into Fin-S. Fill Fin-S up, all the way. Fin-S not want.",
        "On Fin-S, cut in: MUST REMAIN UNFINISHED. Fin-S not know who cut it. Fin-S remember... a voice. Old. A fire behind it.",
    ] },
    Visit { id: "fins_unit_zero_waking", who: Visitor::FinS, place: FOUNDRY, at: 18.0, when: |w| w.met(STRUCK) && !w.fins_with_you(), pages: &[
        "(Rows of wooden figures stand on plinths, faceless, unfinished. At the end of the rows one has a face, and moss in its joints: the figure from the lone tree's roots.)",
        "Fiiin... ess. Unit... zero. Awake. They wake Fin-S, not you. But you... hum. Same song.",
        "They say: finish Fin-S. Tomorrow. Then Prophet come. Into Fin-S. Fill Fin-S up, all the way. Fin-S not want.",
        "On Fin-S, cut in: MUST REMAIN UNFINISHED. Fin-S not know who cut it. Fin-S remember... a voice. Old. A fire behind it.",
    ] },
    Visit { id: "eye_offered_sealed", who: Visitor::Olekeen, place: atlas::FINALE, at: 36.0,
        when: |w| w.beaten.contains(&atlas::FINALE) && w.met(FREED) && w.heard_by_the_three(), pages: &[
        OLEKEEN_HANDS_IT_OVER[0], OLEKEEN_HANDS_IT_OVER[1],
        "It hum like your stone. Louder. Like many stones. Like... voices. Three voices.",
        OLEKEEN_HANDS_IT_OVER[2],
    ] },
    Visit { id: "eye_offered", who: Visitor::Olekeen, place: atlas::FINALE, at: 36.0,
        when: |w| w.beaten.contains(&atlas::FINALE) && w.met(FREED) && !w.heard_by_the_three(), pages: &OLEKEEN_HANDS_IT_OVER },
    Visit { id: "eye_offered_unfreed_sealed", who: Visitor::Olekeen, place: atlas::FINALE, at: 36.0,
        when: |w| w.beaten.contains(&atlas::FINALE) && !w.met(FREED) && w.heard_by_the_three(), pages: &[
        OLEKEEN_CHEWED_FREE, OLEKEEN_HANDS_IT_OVER[0], OLEKEEN_HANDS_IT_OVER[1],
        "It hum like your stone. Louder. Like many stones. Like... voices. Three voices.",
        OLEKEEN_HANDS_IT_OVER[2],
    ] },
    Visit { id: "eye_offered_unfreed", who: Visitor::Olekeen, place: atlas::FINALE, at: 36.0,
        when: |w| w.beaten.contains(&atlas::FINALE) && !w.met(FREED) && !w.heard_by_the_three(), pages: &[
        OLEKEEN_CHEWED_FREE, OLEKEEN_HANDS_IT_OVER[0], OLEKEEN_HANDS_IT_OVER[1], OLEKEEN_HANDS_IT_OVER[2],
    ] },
];

const SEAL: Answer = Answer { id: "eye_sealed", say: "Call the three names.", reply: &[
    "(You say them into the Eye: Lirathar. Sylvara. Korath. It grows warm in your hands, then bright.)",
], gift: None };
const BREAK: Answer = Answer { id: "eye_broken", say: "Break it.", reply: &["Break? ...Good. Olekeen like break. Break simple."], gift: None };
const ORDER: Answer = Answer { id: "eye_to_order", say: "Give it to the Order.", reply: &[
    "(Masked figures kneel in the doorway as you hold it out. Olekeen spits on the floor, and does not look at you.)",
], gift: None };
const FALL: Answer = Answer { id: "eye_let_fall", say: "Open the Veil.", reply: &[
    "Hunter. No. Hunter—",
    "(The Eye opens wider than an eye can.)",
], gift: None };

const LIVE_FREE: Answer = Answer { id: "fins_free", say: "Live free.", reply: &[
    "Liiive... free.",
    "Old voice say that. Old voice, and a hammer, and a fire. Fin-S remember now.",
    "(Its wrists light violet. It steps down off the plinth, and is gone between the rows.)",
], gift: None };
const COME: Answer = Answer { id: "fins_follows", say: "Come with me.", reply: &["Fin-S come. Fin-S carve. Fin-S stay... unfinished. With you."], gift: None };

pub const ASKS: &[Ask] = &[
    Ask { visit: "fins_unit_zero", answers: &[LIVE_FREE, COME] },
    Ask { visit: "fins_unit_zero_waking", answers: &[LIVE_FREE, COME] },
    Ask { visit: "eye_offered_sealed", answers: &[SEAL, BREAK, ORDER, FALL] },
    Ask { visit: "eye_offered", answers: &[BREAK, ORDER, FALL] },
    Ask { visit: "eye_offered_unfreed_sealed", answers: &[SEAL, BREAK, ORDER, FALL] },
    Ask { visit: "eye_offered_unfreed", answers: &[BREAK, ORDER, FALL] },
];

impl Ending {
    /// The ending an answer chose, if it chose one.
    pub fn of(id: &str) -> Option<Ending> {
        match id { "eye_sealed" => Some(Ending::Seal), "eye_let_fall" => Some(Ending::Fall), "eye_to_order" => Some(Ending::Order), "eye_broken" => Some(Ending::Break), _ => None }
    }
}

impl World {
    /// Whether Fin-S walks with the hero to the Summit, to be taken there.
    pub fn fins_with_you(&self) -> bool { self.answers.contains(&"fins_woken") }

    /// Whether Fin-S is off his plinth, and so no door for Voryn when Exodus falls.
    pub fn fins_saved(&self) -> bool { self.answers.contains(&LIVE_FREE.id) || self.answers.contains(&COME.id) }

    /// Whether the three guardians would hear a call through the Eye: enough tears sealed, and not
    /// too much of the Taint in the voice calling.
    pub fn heard_by_the_three(&self) -> bool {
        TEARS.iter().filter(|t| self.beaten.contains(t)).count() >= TEARS_TO_BE_HEARD && self.taint() < crate::taint::UNHEARD
    }

    /// The trap sprung: everyone taken, the hero waking by the Sealed Slab's fire.
    pub fn taken(&mut self) -> Go {
        self.meetings.push(STRUCK);
        if self.fins_with_you() { self.meetings.push(TAKEN); }
        self.called = None;
        self.checkpoint = atlas::stop(SLAB, "spire_stair");
        self.wake()
    }

    /// Whether what Voryn rose in (`name`, the boss after Exodus) has been seen to rise before;
    /// seeing it now marks it so, so its entrance plays once.
    pub fn face_risen(&mut self, name: &'static str) -> bool {
        let before = self.faced.contains(&name);
        if !before { self.faced.push(name); }
        before
    }

    /// What became of everyone, as the road left them.
    pub fn fates(&self) -> Fates {
        let said = |id: &str| self.answers.contains(&id);
        let atrael = (said("atrael_stopped") || said("atrael_burned")).then(|| said("atrael_stopped"));
        let fin_s = if said("fins_free") { Unit::Free } else if said("fins_follows") { Unit::Follows } else { Unit::Fell };
        Fates { atrael, lost: self.hunters.settled.then_some(self.hunters.lost), fin_s, steel: self.steel_fate(), taint: if self.met_helena() { self.taint() } else { 0 },
            bounty: self.bounty_found().then(|| self.met("olekeen_hears_of_bounty") || self.met("olekeen_hears_of_bounty_freed")) }
    }

    /// The last act forgotten as the pilgrimage begins again: the Eye to be won, and chosen for,
    /// once more.
    pub(crate) fn forget_the_finale(&mut self) {
        self.meetings.retain(|m| !ACT.contains(m));
        self.answers.retain(|a| Ending::of(a).is_none() && !["fins_free", "fins_follows"].contains(a));
    }
}

impl Fates {
    /// What is told of each, in order: the journey stop it is told over, metres in, and the words.
    pub fn told(&self) -> Vec<(&'static str, f32, &'static str)> {
        let mut out = Vec::new();
        match self.atrael {
            Some(true) => out.push(("varnholt@violet_road", 10.0, "In Varnholt, the barns still stand. The people in them do not always stay people, and nobody burns them for it.")),
            Some(false) => out.push(("varnholt@violet_road", 10.0, "In Varnholt they still sweep ash from where the barns stood. Ravens sit on the cathedral roof, and watch.")),
            None => {}
        }
        match self.lost {
            Some(0) => out.push(("guild_hall@warrens", 16.0, "At the Warrens guild, Krix still argues, by letter to Stonehollow, over who owes whom a drink. Nobody has ever paid.")),
            Some(1) => out.push(("guild_hall@warrens", 16.0, "At the guild, Krix drinks for two, and keeps the stool beside him empty.")),
            Some(_) => out.push(("guild_hall@warrens", 16.0, "Nobody sits in the corner of the guild hall that Krix and Lurz kept. Gruk has never struck their names from his book.")),
            None => {}
        }
        let fin_s = match self.fin_s {
            Unit::Free => "On the roads, travellers find small carvings left at the shrines, of faces from the road. No one sees who leaves them.",
            Unit::Follows => "By every fire you rest at, a wooden figure sits and carves, and never quite finishes.",
            Unit::Fell => "Under a lone tree on the violet road the roots have closed over an empty hollow. The Order finished what once slept there. It did not come back from the Chamber of the Eye.",
        };
        out.push(("violet_road@varnholt", 18.0, fin_s));
        let steel = match self.steel {
            Steel::Tin => Some("In Stonehollow, Steel tells the story of the Eye to anyone who pays, and gets it wrong every time, and laughs loudest at the wrong parts."),
            Steel::Refused => Some("In Stonehollow, Steel still tells the story of the Eye to anyone who pays. He leaves out the part at the Sealed Slab. Tin doesn't get cold."),
            Steel::Flesh => Some("Steel's shop is shut. A grey man with a laugh like a dropped kettle drinks down on the Lower Stairs now, and complains of the cold. Some nights his eyes go violet."),
            Steel::Unmet => None,
        };
        if let Some(steel) = steel { out.push(("stone_upper@stone_market", 18.0, steel)); }
        if self.taint > 0 { out.push(("sewers@lower_stairs", 20.0, "Under Stonehollow, Helena lights a candle each night for a hunter who carried the Taint out of her chapel. She does not say whether it is for them, or against them.")); }
        match self.bounty {
            Some(true) => out.push(("warrens_lane@warrens", 16.0, "In the Warrens, bread is left on doorsteps again, under a small fox's head burned into each lintel. Olekeen never says it is him.")),
            Some(false) => out.push(("warrens_lane@warrens", 16.0, "In the Warrens, bread is left on doorsteps again, under a small fox's head burned into each lintel. No one sees who leaves it.")),
            None => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trap_takes_fin_s_only_if_woken_and_wakes_the_hero_at_the_slab() {
        let mut w = World::new(0.0);
        w.arrive("spire_summit@sealed_slab", 100.0, 0.0, 0);
        w.answers.push("fins_woken");
        assert!(matches!(w.taken(), Go::Warp(s) if s == "sealed_slab@spire_stair"));
        assert!(w.met(STRUCK) && w.met(TAKEN));
        let mut w = World::new(0.0);
        w.answers.push("fins_left");
        w.taken();
        assert!(w.met(STRUCK) && !w.met(TAKEN));
        assert_eq!(w.fates().fin_s, Unit::Fell, "left in his roots, the Order took him there");
        w.arrive(&atlas::stop(FOUNDRY, CELLS), 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("fins_unit_zero_waking"), "he is on his plinth all the same");
        assert!(!w.fins_saved());
        w.answer("fins_follows");
        assert!(w.fins_saved() && w.asking("fins_unit_zero").is_none(), "asked once, whichever plinth");
    }

    #[test]
    fn olekeen_hands_over_the_eye_and_only_a_hero_who_sealed_the_tears_can_call_the_three() {
        let mut w = World::new(0.0);
        w.meetings.push(STRUCK);
        w.beaten.push(atlas::FINALE);
        w.arrive(&atlas::stop(atlas::FINALE, CELLS), 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("eye_offered_unfreed"), "left in his cell, he chews free and comes all the same");
        w.meetings.push(FREED);
        assert_eq!(w.visit().map(|v| v.id), Some("eye_offered"));
        assert!(w.asking("eye_offered").is_some_and(|a| a.answers.iter().all(|x| x.id != SEAL.id)));
        w.beaten.extend(["east_gate", "black_water", "fog_hollow"]);
        assert_eq!(w.visit().map(|v| v.id), Some("eye_offered_sealed"));
        assert_eq!(w.asking("eye_offered_sealed").map(|a| a.answers[0].id), Some(SEAL.id));
        w.answer("eye_broken");
        assert_eq!(Ending::of(w.answers[0]), Some(Ending::Break));
        w.forget_the_finale();
        assert!(w.answers.is_empty() && w.meetings.is_empty(), "the last act is played again");
    }

    #[test]
    fn what_became_of_everyone_follows_the_road() {
        let mut w = World::new(0.0);
        assert_eq!(w.fates().told().len(), 1, "nobody met, only Fin-S told: the Order finished him");
        w.answers.extend(["atrael_stopped", "fins_woken", "fins_free"]);
        w.hunters = crate::hunters::Hunters { lost: 1, settled: true };
        w.meetings.push("steel_reads");
        let told = w.fates().told();
        assert_eq!(told.len(), 4);
        assert!(told[0].2.contains("still stand") && told[1].2.contains("drinks for two") && told[2].2.contains("carvings"));
    }
}
