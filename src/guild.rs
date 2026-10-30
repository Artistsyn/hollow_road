//! The Warrens guild, in the back room off the guild hall: Gruk behind his counter, one-eyed and snarling,
//! and the bounties on his board that send the hero on (`docs/story.md`, sections 2 and 5).
//! His words are the book's where it has them.

use crate::world::Note;

/// The place the guild keeps.
pub const HALL: &str = "guild_office";
/// Gruk's name, as the hero is told it.
pub const GRUK: &str = "Gruk";

/// His first words, before the first bounty: the Collector was the guild's own.
const FIRST: [&str; 4] = [
    "Shut that door. The hollowed don't knock.",
    "Put the Collector down, did ya? He collected for this guild twenty winters. Then the mist got in his head, same as the rest.",
    "Guild's still open. Bounties don't care who's hollow, and coin don't either.",
    "No proof, no pay.",
];

/// The east gate's beast, killed by a hero who went out before anyone posted it: paid for all the
/// same, once, whenever the hero next stands at his counter.
const GATE: [&str; 2] = [
    "What's that you're draggin'? Bone, violet in the cracks. That from the east gate?",
    "Nobody posted it. Guild pays for proof all the same. Board or no board, a dead thing's a dead thing.",
];
/// The same, with the bone sold or forged: the gate watch's word stands for it.
const GATE_WORD: [&str; 2] = [
    "Gate watch says the bone thing out east is dead, and that it was you did it. Watch don't lie for free.",
    "Nobody posted it. Guild pays for proof all the same, and a dead thing's a dead thing.",
];
/// A bounty posted that the hero has already done.
const ALREADY: &str = "...Hold on. Word's already in on that one. Done, ain't it? Then here.";
/// What he has heard of the east gate by the time the Hollowmere bounty is paid: Lurz's alarm,
/// and who answered it.
const GATE_HEARD: [&str; 2] = [
    "Lurz come runnin' in hollerin' 'bout somethin' with arms, out the east gate. Olekeen went after it. That yellow one don't wait for the board.",
    "Lurz come runnin' in hollerin' 'bout the east gate, then come back sayin' the thing's already dead. Olekeen went out anyway. He'll be sore.",
];
/// What the gate's beast pays, on its bone.
const GATE_PAY: u32 = 400;

/// What he has heard of the crew's hole under the ridge once their chief is paid for: emptied
/// before the chief fell (by someone he would rather not know of), or run off with by his crew.
const CAVE_HEARD: [[&str; 2]; 2] = [
    [
        "Old trader says there's more of 'em hidin' in a hole under the ridge, off the hollows. Ran there with the take when their chief went down.",
        "No bounty on it. Whatever they took's spoken for by now. Bandits ain't sentimental.",
    ],
    [
        "Tusk says the crew kept a hole under the ridge, off the hollows. Says it was picked clean 'fore their chief even fell.",
        "Somebody got there first. ...Don't tell me. Guild don't pay for what it don't know.",
    ],
];

/// What he says once the board is bare.
const DONE: [&str; 2] = [
    "Board's bare. Come back when the world's worse. Won't be long.",
    "Stonehollow's registrar pays for proof same as me. Asks fewer questions, too.",
];

/// The bounties Gruk posts, in the order the hero is handed them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bounty { Hollowmere, Blackridge }

impl Bounty {
    pub const ALL: [Bounty; 2] = [Bounty::Hollowmere, Bounty::Blackridge];

    /// The notice on the board, and how the hero is told it was taken.
    pub fn notice(self) -> &'static str {
        match self {
            Bounty::Hollowmere => "Beast in Hollowmere. Tentacles. Screaming. 50 silver.",
            Bounty::Blackridge => "The Blackridge crew. Ten silver a head, twenty for their chief.",
        }
    }

    /// The place whose boss falling is the proof.
    pub fn proof(self) -> &'static str {
        match self { Bounty::Hollowmere => "hollowmere", Bounty::Blackridge => "chief_hold" }
    }

    /// The echoes it pays, on the proof.
    pub fn pay(self) -> u32 {
        match self { Bounty::Hollowmere => 600, Bounty::Blackridge => 1200 }
    }

    /// Gruk posting it: where to go, said without a map.
    fn posted(self) -> &'static [&'static str] {
        match self {
            Bounty::Hollowmere => &[
                "Board's got one. \"Beast in Hollowmere. Tentacles. Screaming. 50 silver.\"",
                "Down the old road, past the vale, through the bell gate. Something's in their well, they say.",
                "Bring proof.",
            ],
            Bounty::Blackridge => &[
                "Fresh one, ink still wet. Blackridge crew. Hit three caravans, killed two.",
                "Ten silver a head, twenty for their chief. North of Hollowmere, up the Pine Road.",
                "Bring proof. Blade, helm, somethin' clear.",
            ],
        }
    }

    /// What he says while it is open.
    fn waiting(self) -> &'static [&'static str] {
        match self {
            Bounty::Hollowmere => &[
                "Hollowmere's past the vale. The beast's in the well. Bring proof.",
                "Proof? No? Then what're ya standin' at my counter for?",
                "No proof, no pay.",
            ],
            Bounty::Blackridge => &[
                "Blackridge is north, up the Pine Road. The chief holds the high ground.",
                "Mind the proof. No proof, no pay.",
            ],
        }
    }

    /// The proof laid on his counter.
    fn proven(self) -> &'static [&'static str] {
        match self {
            Bounty::Hollowmere => &[
                "Proof?",
                "...Good kill. Coin's yours.",
                "And that stone you pried off their well. Here, one more coin, for the lookin'. I ain't buyin' it.",
                "Traders babble 'bout funny stones, weird hunts gone sour. Watch yer back. Trouble follows stuff like that.",
            ],
            Bounty::Blackridge => &["That's the chief's, right enough. Twenty, and the rest."],
        }
    }
}

/// What Gruk has handed the hero and paid for, kept with the world's save.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Guild {
    /// Bounties posted to the hero (`Bounty::ALL`, in order), and how many of them paid.
    pub posted: u8,
    pub paid: u8,
    /// Times talked to with nothing new to say.
    pub talks: u32,
    /// Whether the east gate's beast has been paid for.
    pub gate: bool,
    /// Whether he has told what became of the crew's stash under the ridge.
    pub cave: bool,
}

impl Guild {
    /// A word with Gruk: his pages, and what it came to (a bounty taken, echoes paid). Whatever
    /// the hero has proven since is paid, and the next bounty posted; one already done is paid
    /// as it is posted. `bone`: whether the east gate's bone (or what was forged of it) is carried.
    pub fn meet(&mut self, beaten: &[&'static str], bone: bool) -> (Vec<String>, Vec<Note>) {
        let mut pages: Vec<&'static str> = Vec::new();
        let mut notes = Vec::new();
        if self.posted == 0 { pages.extend(FIRST.iter().take(3)); }
        let gate = beaten.contains(&"east_gate");
        if gate && !self.gate {
            pages.extend(if bone { GATE } else { GATE_WORD });
            notes.push(Note::Paid(GRUK, GATE_PAY));
            self.gate = true;
        }
        loop {
            self.pay_proven(beaten, &mut pages, &mut notes);
            let Some(&next) = Bounty::ALL.get(usize::from(self.posted)).filter(|_| self.paid == self.posted) else { break };
            pages.extend(next.posted());
            notes.push(Note::Bounty(next.notice()));
            self.posted += 1;
            if !beaten.contains(&next.proof()) { break; }
            pages.push(ALREADY);
        }
        if self.posted == 1 && self.paid == 0 && pages.len() > 3 { pages.push(FIRST[3]); }
        if pages.is_empty() {
            let open = Bounty::ALL.get(usize::from(self.paid)).filter(|_| self.paid < self.posted);
            let lines = open.map_or(&DONE[..], |b| b.waiting());
            pages.push(lines[self.talks as usize % lines.len()]);
            self.talks += 1;
        }
        (pages.into_iter().map(str::to_owned).collect(), notes)
    }

    /// Every posted bounty the hero has proven, paid in order.
    fn pay_proven(&mut self, beaten: &[&'static str], pages: &mut Vec<&'static str>, notes: &mut Vec<Note>) {
        while let Some(&due) = Bounty::ALL.get(usize::from(self.paid)).filter(|b| self.paid < self.posted && beaten.contains(&b.proof())) {
            pages.extend(due.proven());
            notes.push(Note::Paid(GRUK, due.pay()));
            if due == Bounty::Hollowmere { pages.push(GATE_HEARD[usize::from(beaten.contains(&"east_gate"))]); }
            self.paid += 1;
        }
    }

    /// Once the Blackridge bounty is paid, and once only, what he has heard of the crew's stash:
    /// `by_hero` whether the hero emptied it (None: still there, and nothing to tell).
    pub fn cave_word(&mut self, by_hero: Option<bool>) -> Vec<String> {
        let Some(hero) = by_hero.filter(|_| usize::from(self.paid) >= Bounty::ALL.len() && !self.cave) else { return Vec::new() };
        self.cave = true;
        CAVE_HEARD[usize::from(hero)].iter().map(|&s| s.to_owned()).collect()
    }

    /// The bounty the hero has taken and not yet proven.
    pub fn open(&self) -> Option<Bounty> {
        Bounty::ALL.get(usize::from(self.paid)).copied().filter(|_| self.paid < self.posted)
    }

    pub fn saved(&self) -> serde_json::Value {
        serde_json::json!({ "posted": self.posted, "paid": self.paid, "talks": self.talks, "gate": self.gate, "cave": self.cave })
    }

    pub fn restored(v: &serde_json::Value) -> Guild {
        let count = |k: &str| v[k].as_u64().unwrap_or(0);
        let posted = u8::try_from(count("posted")).unwrap_or(0).min(Bounty::ALL.len() as u8);
        let talks = u32::try_from(count("talks")).unwrap_or(0);
        let told = |k: &str| v[k].as_bool().unwrap_or(false);
        Guild { posted, paid: u8::try_from(count("paid")).unwrap_or(0).min(posted), talks, gate: told("gate"), cave: told("cave") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gruk_posts_the_hollowmere_bounty_then_pays_for_proof_and_posts_blackridge() {
        let mut g = Guild::default();
        let (pages, notes) = g.meet(&[], true);
        assert!(pages[0].contains("hollowed") && pages.iter().any(|p| p.contains("Beast in Hollowmere")) && pages.last().is_some_and(|p| p == "No proof, no pay."));
        assert_eq!(notes, vec![Note::Bounty(Bounty::Hollowmere.notice())]);
        assert_eq!(g.open(), Some(Bounty::Hollowmere));
        // Nothing proven: one hint at a time, round and round.
        let (pages, notes) = g.meet(&["guild_hall"], true);
        assert_eq!((pages.len(), notes.len()), (1, 0));
        assert!(pages[0].contains("Hollowmere"));
        // The beast down: paid, and the next posted, in one word with him.
        let (pages, notes) = g.meet(&["guild_hall", "hollowmere"], true);
        assert_eq!(pages[0], "Proof?");
        assert_eq!(notes, vec![Note::Paid(GRUK, 600), Note::Bounty(Bounty::Blackridge.notice())]);
        assert_eq!(g.open(), Some(Bounty::Blackridge));
        // Both proven at once (the chief down before the first was cashed in) pays both.
        let mut late = Guild { posted: 1, ..Guild::default() };
        let (pages, notes) = late.meet(&["hollowmere", "chief_hold"], true);
        assert_eq!(notes, vec![Note::Paid(GRUK, 600), Note::Bounty(Bounty::Blackridge.notice()), Note::Paid(GRUK, 1200)]);
        assert!(pages.contains(&ALREADY.to_owned()), "posted and paid at once: {pages:?}");
        let (pages, notes) = late.meet(&["hollowmere", "chief_hold"], true);
        assert!(notes.is_empty() && pages[0].contains("bare"), "{pages:?}");
        assert_eq!(late.open(), None);
        assert_eq!(Guild::restored(&late.saved()), late);
    }

    #[test]
    fn the_east_gates_beast_is_paid_for_unposted_and_the_hollowmere_pay_tells_who_took_it() {
        let mut g = Guild::default();
        let (pages, _) = Guild::default().meet(&["east_gate"], false);
        assert!(pages.iter().any(|p| p.contains("Gate watch")), "the bone gone, the watch's word: {pages:?}");
        let (_, notes) = g.meet(&["east_gate"], true);
        assert_eq!(notes, vec![Note::Paid(GRUK, GATE_PAY), Note::Bounty(Bounty::Hollowmere.notice())]);
        let (pages, notes) = g.meet(&["east_gate", "hollowmere"], true);
        assert!(pages.iter().any(|p| p.contains("He'll be sore")) && !notes.contains(&Note::Paid(GRUK, GATE_PAY)), "paid once: {pages:?}");
        let mut late = Guild::default();
        late.meet(&[], true);
        let (pages, _) = late.meet(&["hollowmere"], true);
        assert!(pages.iter().any(|p| p.contains("Olekeen went after it")), "{pages:?}");
    }

    #[test]
    fn once_the_chief_is_paid_for_gruk_tells_once_who_emptied_the_crews_hole() {
        let mut g = Guild { posted: 2, paid: 1, ..Guild::default() };
        assert!(g.cave_word(Some(true)).is_empty(), "not before the Blackridge pay");
        g.paid = 2;
        assert!(g.cave_word(None).is_empty(), "nothing to tell while the stash is there");
        assert!(g.cave_word(Some(true)).iter().any(|p| p.contains("got there first")));
        assert!(g.cave_word(Some(true)).is_empty(), "told once");
        let mut late = Guild { posted: 2, paid: 2, ..Guild::default() };
        assert!(late.cave_word(Some(false)).iter().any(|p| p.contains("Ran there with the take")));
        assert!(Guild::restored(&late.saved()).cave);
    }
}
