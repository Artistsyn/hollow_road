//! Who can be called to a boss, and where: the story's half of `allies`. The game never says how.
//! What the hunters say on the road hints at it, and a choice in what the hero says back decides
//! it (`ASKS`); a prompt at the veil is all that shows it worked. Krix, if he lived, was let keep
//! his axe ("not done adding to it"), and Mara's whistle is carried: blown twice where the dark is
//! thickest, he finds the trail. Olekeen, asked to hunt together (at Stonehollow's gate, or once
//! cut free under the Spire): the humming stone raised, he hears it. Exodus, accepted in the
//! Shrine's halls, for its guardian alone: the stone let sing. None goes everywhere:
//! Krix never back into the fog or the black water, nor past the north road; Olekeen only from
//! Stonehollow on, and not while the Order holds him.

use crate::allies::Summoner;
use crate::world::{Note, World};

/// One thing the hero can say back: its name (kept once said), the words, the answer, and
/// what is handed over for it.
#[derive(Debug, PartialEq)]
pub struct Answer {
    pub id: &'static str,
    pub say: &'static str,
    pub reply: &'static [&'static str],
    pub gift: Option<&'static str>,
}

/// A choice at the end of a meeting (`visitors::VISITS`): asked until one of its answers is given.
#[derive(Debug, PartialEq)]
pub struct Ask { pub visit: &'static str, pub answers: &'static [Answer] }

/// Krix's axe, taken or left with him: his to keep, and his way to be found.
const AXE: &str = "Krix's Notched Axe";
const KEPT: &str = "krix_keeps_axe";
const TAKEN: &str = "krix_gives_axe";
const HUNT: &str = "olekeen_hunts_with";
/// Exodus's help, given for the one fight he offers it for: the guardian beneath the Shrine.
const EXODUS: &str = "exodus_joins";
const SANCTUM: &str = "wraith_sanctum";
/// Where Krix will not go again: the hollow and the black water.
const NEVER_KRIX: [&str; 2] = [crate::hunters::FOG, "black_water"];
/// The last region Krix walks to, and the first Olekeen answers in (Stonehollow).
const KRIX_REGIONS: usize = 4;
const OLEKEEN_FROM: usize = 3;

pub const ASKS: &[Ask] = &[
    Ask { visit: "krix_home", answers: &[
        Answer { id: TAKEN, say: "Take the axe.", reply: &["Swing it like you mean it. It knows how."], gift: Some(AXE) },
        Answer { id: KEPT, say: "Tell him to keep it.", reply: &[
            "...Keep it. Ha. Not done adding to it, you reckon.",
            "Mara blew twice when she'd found the trail. Twice, sharp.",
            "Where the dark's thickest, that's where the trail goes.",
        ], gift: None },
    ] },
    Ask { visit: "krix_home_alone", answers: &[
        Answer { id: TAKEN, say: "Take the axe.", reply: &["Swing it like you mean it. Lurz would've."], gift: Some(AXE) },
        Answer { id: KEPT, say: "Tell him to keep it.", reply: &[
            "...Keep it. Lurz would've laughed at that.",
            "Mara blew twice when she'd found the trail. Twice, sharp.",
            "Where the dark's thickest, that's where the trail goes.",
        ], gift: None },
    ] },
    Ask { visit: "olekeen_stone_saved", answers: &[
        Answer { id: HUNT, say: "Hunt with me.", reply: &[
            "With you? Ha! Olekeen hunt alone.",
            "...Mostly.",
            "Your stone hum. Olekeen's stone hum. Like they talk. Loud stone, Olekeen hear.",
        ], gift: None },
        Answer { id: "olekeen_parts", say: "Good hunting.", reply: &["Good hunting. Olekeen first next time."], gift: None },
    ] },
    Ask { visit: "exodus_meets", answers: &[
        Answer { id: EXODUS, say: "Fight beside me.", reply: &[
            "Good. When your stone sings beneath the shrine, I will be there.",
            "I am always there.",
        ], gift: None },
        Answer { id: "exodus_refused", say: "I walk alone.", reply: &[
            "Then you walk toward a grave. The Order will collect the stone from what is left.",
            "...I hope that I am wrong. I am rarely wrong.",
        ], gift: None },
    ] },
    Ask { visit: "fins_sleeps", answers: &[
        Answer { id: "fins_woken", say: "Touch the rune to its chest.", reply: &[
            "Fiiin... ess. Unit... zero. Awake. Awake?",
            "You hum. Fin-S hum. Same song. Fin-S follow. Fin-S carve.",
            "(It unfolds, creaking, and keeps to the road behind you, at a distance.)",
        ], gift: None },
        Answer { id: "fins_left", say: "Let it sleep.", reply: &["(You leave it in the roots. Its wrists flicker once, violet, as you go.)"], gift: None },
    ] },
    Ask { visit: "atrael_burns", answers: &[
        Answer { id: "atrael_stopped", say: "Stop him.", reply: &[
            "...You would stand between me and the fire? For them?",
            "Then keep them. When they turn, remember that you chose it.",
            "(His wings open, gold gone to red, and he is gone in a burst of ravens. The barn doors stay shut. Inside, someone is weeping.)",
        ], gift: None },
        Answer { id: "atrael_burned", say: "Let it burn.", reply: &[
            "Wise. Or cold. In this place they are the same.",
            "(The barns go up. The screaming stops sooner than you would have thought.)",
        ], gift: None },
    ] },
    Ask { visit: "olekeen_bound", answers: &[
        Answer { id: HUNT, say: "Hunt with me.", reply: &[
            "Olekeen owe you. Olekeen hate owing.",
            "Olekeen find bow first. Then... loud stone, Olekeen hear.",
        ], gift: None },
        Answer { id: "olekeen_goes_ahead", say: "Go. Find your bow.", reply: &["Olekeen go. Olekeen come behind. Always behind, now."], gift: None },
    ] },
];

impl Ask {
    /// Every choice the road asks, the last act's with the rest.
    pub fn every() -> impl Iterator<Item = &'static Ask> { ASKS.iter().chain(crate::finale::ASKS).chain(crate::relics::ASKS).chain(crate::steel::ASKS).chain(crate::taint::ASKS) }
}

impl Summoner {
    /// What E does at the veil to call them.
    pub fn call_line(self) -> &'static str {
        match self { Summoner::Krix => "Blow Mara's whistle twice", Summoner::Olekeen => "Raise the humming stone", Summoner::Exodus => "Let the stone sing", Summoner::Atrael => "" }
    }
    /// What the hero is told once they are called.
    pub fn answered(self) -> &'static str {
        match self {
            Summoner::Krix => "Two sharp notes go out into the dark. Somewhere behind the veil, something answers.",
            Summoner::Olekeen => "The stone hums loud. Somewhere near, another hums back.",
            Summoner::Exodus => "The stone sings. A white shape in a black beaked mask is beside you. You never heard him come.",
            Summoner::Atrael => "Ravens pour down into the square, a black storm of them, and Atrael comes out of it burning.",
        }
    }
}

impl World {
    /// The choice to put at the end of meeting `visit`, if one is still to be made: none of its
    /// answers given yet (Olekeen's two share theirs, so asked once only).
    pub fn asking(&self, visit: &str) -> Option<&'static Ask> {
        Ask::every().find(|a| a.visit == visit && !a.answers.iter().any(|x| self.answers.contains(&x.id)))
    }

    /// `id` said: kept, and what is handed over for it.
    pub fn answer(&mut self, id: &str) -> Vec<Note> {
        let Some(given) = Ask::every().flat_map(|a| a.answers).find(|x| x.id == id) else { return Vec::new() };
        if !self.answers.contains(&given.id) { self.answers.push(given.id); }
        let mut notes = self.sell_relic(given.id);
        let Some(name) = given.gift else { return notes };
        let i = crate::gear::find(name);
        if self.gear.gain(i) { notes.push(Note::Found(crate::gear::ITEMS[i].name)); }
        notes
    }

    /// An answer's name as kept in a save, if it is one.
    pub fn answer_named(id: &str) -> Option<&'static str> {
        Ask::every().flat_map(|a| a.answers).find(|x| x.id == id).map(|x| x.id)
    }

    /// Who would answer a call at this place's boss, if anyone, before it is faced: Olekeen
    /// where he goes (he is nearer), else Krix. Never to a hunter's echo.
    pub fn summons(&self) -> Option<Summoner> {
        let (_, boss) = self.boss_here()?;
        if self.boss_beaten() || self.called.is_some() || crate::hunters::ECHOES.iter().any(|e| std::ptr::eq(e.boss, boss)) { return None; }
        let (id, region) = (self.here().id, self.here().region);
        let olekeen = self.answers.contains(&HUNT) && region >= OLEKEEN_FROM && !self.olekeen_held() && id != crate::atlas::SUMMIT;
        let krix = self.answers.contains(&KEPT) && self.hunters.krix_lives() && self.taken.contains(&"black_water")
            && !NEVER_KRIX.contains(&id) && region <= KRIX_REGIONS;
        if id == SANCTUM && self.answers.contains(&EXODUS) && self.pilgrimages == 0 { Some(Summoner::Exodus) }
        else if olekeen { Some(Summoner::Olekeen) } else if krix { Some(Summoner::Krix) } else { None }
    }

    /// Whether the Order holds Olekeen: taken once the Spire's court falls until he is cut free
    /// under the slab, and taken again at the Summit until he is found in the stronghold's cells.
    fn olekeen_held(&self) -> bool {
        (self.beaten.contains(&"statue_court") && !self.meetings.contains(&"olekeen_bound"))
            || (self.meetings.contains(&crate::finale::STRUCK) && !self.meetings.contains(&"olekeen_freed"))
    }

    /// Whether E calls for help now: someone would answer, and the hero stands at the veil.
    pub fn can_call(&self, fight: &crate::fight::Fight, walked: f64) -> bool {
        self.summons().is_some() && self.at_veil(fight, walked)
    }

    /// Whether Atrael comes down on the Hollowmere Screamer in his ravens: once Gruk has posted the
    /// beast, so Olekeen is out hunting it (Atrael comes for him in the book), and never once the
    /// hero has stood against him at Varnholt.
    pub fn ravens_come(&self) -> bool { self.guild.posted > 0 && !self.answers.contains(&"atrael_stopped") }

    /// The call made: whoever answers comes into the fight behind the veil.
    pub fn call(&mut self) -> Vec<Note> {
        let Some(who) = self.summons() else { return Vec::new() };
        self.called = Some(who);
        vec![Note::Called(who)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Krix home from the hollow, Mara's whistle carried, standing at a boss he would come to.
    fn krix_ready() -> World {
        let mut w = World::new(0.0);
        w.beaten.push("chief_hold");
        w.hunters = crate::hunters::Hunters { lost: 0, settled: true };
        w.taken.push("black_water");
        w
    }

    #[test]
    fn krix_answers_only_if_he_kept_his_axe_lived_and_the_whistle_is_carried() {
        let mut w = krix_ready();
        w.arrive("magistrate@stone_upper", 100.0, 0.0, 0);
        assert_eq!(w.summons(), None, "nothing chosen yet");
        assert!(w.answer(TAKEN).contains(&Note::Found(AXE)));
        assert_eq!(w.summons(), None, "the axe taken, he hung it up");
        let mut w = krix_ready();
        assert!(w.answer(KEPT).is_empty(), "nothing handed over");
        assert!(w.asking("krix_home").is_none() && w.asking("krix_home_alone").is_none(), "asked once");
        w.arrive("magistrate@stone_upper", 100.0, 0.0, 0);
        assert_eq!(w.summons(), Some(Summoner::Krix));
        w.taken.retain(|t| *t != "black_water");
        assert_eq!(w.summons(), None, "no whistle, no call");
        w.taken.push("black_water");
        w.hunters.lost = 2;
        assert_eq!(w.summons(), None, "he died in the fog");
        w.hunters.lost = 0;
        assert_eq!(World::restored(0.0, &w.saved()).map(|b| b.answers), Some(vec![KEPT]));
        // Not back into the fog.
        w.arrive("fog_hollow@bandit_hollows", 200.0, 0.0, 0);
        w.beaten.retain(|b| *b != crate::hunters::FOG);
        assert_eq!(w.summons(), None);
    }

    #[test]
    fn olekeen_answers_from_stonehollow_on_unless_the_order_holds_him_and_a_call_is_one_a_fight() {
        let mut w = krix_ready();
        w.answer(HUNT);
        assert!(w.asking("olekeen_bound").is_none(), "asked at Stonehollow, not again under the Spire");
        w.arrive("chief_hold@ridge_camp", 100.0, 0.0, 0);
        w.beaten.retain(|b| *b != "chief_hold");
        assert_eq!(w.summons(), None, "Blackridge is before his road");
        w.beaten.push("chief_hold");
        w.arrive("magistrate@stone_upper", 200.0, 0.0, 0);
        assert_eq!(w.summons(), Some(Summoner::Olekeen));
        assert_eq!(w.call(), vec![Note::Called(Summoner::Olekeen)]);
        assert_eq!(w.summons(), None, "called already");
        w.died(210.0);
        assert_eq!(w.called, None, "a fall sends them home");
        w.arrive("magistrate@stone_upper", 300.0, 0.0, 0);
        w.beaten.push("statue_court");
        assert_eq!(w.summons(), None, "taken at the Spire");
        w.meetings.push("olekeen_bound");
        assert_eq!(w.summons(), Some(Summoner::Olekeen), "cut free");
        w.meetings.push(crate::finale::STRUCK);
        assert_eq!(w.summons(), None, "taken again at the Summit");
        w.meetings.push("olekeen_freed");
        assert_eq!(w.summons(), Some(Summoner::Olekeen), "found in the stronghold's cells");
    }
}
