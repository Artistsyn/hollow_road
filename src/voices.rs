//! What the keeper and the smith say as the road goes on. Each has beats, one per boss felled:
//! the first word after a boss is that beat's news, told once and whole; after that, one of its
//! idle lines at a time. Their threads are the game's own (`docs/story.md`, "Hidden threads"):
//! hinted here, never told. Maren keeps her hands in her sleeves; Brannoc once forged for an
//! Order and carved before he forged.

/// What one of them says once the boss at `after` has fallen ("" from the start): its news, told
/// once, then its idle lines.
pub struct Beat { pub after: &'static str, pub news: &'static [&'static str], pub idle: &'static [&'static str] }

/// Who is spoken to at the shrine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker { Keeper, Smith }

const KEEPER: &[Beat] = &[
    Beat { after: "", news: &[], idle: &[
        "The Veil is thin past the marsh. If you hear singing in the reeds, do not answer it.",
        "Hollowmere was a town of bells. Now it has only the one, and something rings it at night.",
        "You ask what you are. The mist gives back what it cannot hold. I do not know what holds you.",
        "Every shrine remembers who rested at it. So do I.",
    ] },
    Beat { after: "east_gate", news: &[
        "Something came out of the eastern reeds, and you sent it back. Before anyone asked you to.",
        "The Warrens will not thank you. They never thank anyone. But the road noticed.",
    ], idle: &["The bone you carry hums. So does the stone, when you find it. They are kin, those two."] },
    Beat { after: "gate", news: &[
        "The Warden is down? Then Hollowmere's bells are silent at last.",
        "He rang them every night the mist came. He thought it kept the mist out. It only told it where they were.",
    ], idle: &["Past the gate lies the village. Do not drink from the well. Do not look into it for long."] },
    Beat { after: "hollowmere", news: &[
        "You carry something that hums. I can hear it from here.",
        "No, keep it covered. Old stones like that were cut to be sung over, and there is no one left who knows the song.",
        "There were women once who knew it. They wore grey, and cut the runes into their palms so they could never set them down. They kept the living safe. Then they did not.",
    ], idle: &[
        "The stone warms toward what it wants. Learn to feel it.",
        "The guild man will want to see it. Show him if you must. Never sell it.",
    ] },
    Beat { after: "chief_hold", news: &[
        "Stonehollow lies past the ridge. A city of locks, and not one of them holds.",
        "There is a chapel under its streets. Or there was. If you find a woman in grey there, do not tell her where I keep my fire.",
    ], idle: &["They say a suit of armour with no one inside keeps a shop in that city. The dead have always kept odd trades."] },
    Beat { after: "magistrate", news: &[
        "Under that city lies a shrine older than the city. It is not one of mine.",
        "Whatever is carved on its walls, remember it was carved by someone who wanted it believed.",
    ], idle: &["The Order will tell you they bring harmony. Ask them whose."] },
    Beat { after: "wraith_sanctum", news: &[
        "You went into the Hollowed Shrine and came out again. What did the walls show you?",
        "Cloaked figures, reaching for the stars. Yes. I have seen them in my sleep, and I have never been there.",
        "Those who tore the Veil wore cloaks. So do I. So does half this road. Remember that.",
    ], idle: &["Something has followed you since the Shrine. Not a wraith. Something with a mask."] },
    Beat { after: "cathedral", news: &[
        "Varnholt's priest swore he would guard it always. Did he?",
        "Always is a long time to keep a watch. Long enough to forget what you were watching for.",
    ], idle: &["You carry a piece of the Eye. It is colder than you are, and it is learning you."] },
    Beat { after: "statue_court", news: &[
        "Eryndor's angels kept their oath in stone. Stone forgets nothing, and forgives nothing.",
        "You are nearly whole. The Eye, I mean. Not you. You were never going to be whole.",
    ], idle: &["Go north, where starlight meets stone. I will keep the fire. It is what I am for."] },
    Beat { after: "spire_summit", news: &[
        "You went to the top of the world and came back down. The mist gave you back again.",
        "Each time, I wonder whether it is you that it gives back.",
    ], idle: &["Rest. The road is the same road. You are the one who is not."] },
    // Off the story's road: added last, so the beats heard before keep their bits.
    Beat { after: "black_water", news: &[
        "The black water is still tonight. I had forgotten it could be.",
        "Whatever you carried out of there, someone carried it in first. Remember them, even if no one else does.",
    ], idle: &["Some of those taken are only kept. It is not the same as gone. It is worse."] },
    Beat { after: "fog_hollow", news: &[
        "There was a thing in the fog past the ridge that knew the names of the men it hunted.",
        "It did not come up out of anything. It came through. That is a different kind of door, and doors are not shut by killing what walks in.",
    ], idle: &["When the fog speaks your name, do not answer. It is only learning how you sound."] },
    Beat { after: "starlit_wastes", news: &[
        "The thing on the plain was made to keep the pilgrims' road. Ask yourself who it was made to let by.",
    ], idle: &["Stars that close are not a kindness. They are watching what comes up the road."] },
    Beat { after: "sky_peaks", news: &[
        "Where the sky broke, things came through and stayed. Like the mist. Like you, perhaps.",
        "The stair is near now. Whatever went up ahead of you is waiting at the top of it.",
    ], idle: &["Rest. The slab will open to what you carry, or it will not."] },
];

const SMITH: &[Beat] = &[
    Beat { after: "", news: &[], idle: &[
        "Shieldbearers carry the good iron. Lancers too, if you get past the point.",
        "Every edge I forge, the road blunts. That's the trade.",
        "I forged for an Order once. Don't ask which.",
        "Wood listens better than iron. Forgets better, too. I carved, once, before I took up the hammer.",
    ] },
    Beat { after: "east_gate", news: &["That bone. Violet in the cracks. Bring it here; I've a mind to make something of those antlers."],
        idle: &["Antler and bone. Wood's cousin. It listens, a little."] },
    Beat { after: "gate", news: &["The Warden's clapper? Bring it. Bell-metal remembers every note it ever rang. Makes a mean hammer."],
        idle: &["The Warden's men forged their own. Bad steel, good hearts. The steel lasted longer."] },
    Beat { after: "hollowmere", news: &[
        "Let me see that stone you're hiding. Huh. I've seen marks like those.",
        "Cut into the wrists of something I made, a long time ago. Forget I said it.",
    ], idle: &["Don't let me near that stone again. My hands remember things I'd sooner they didn't."] },
    Beat { after: "chief_hold", news: &["Ridge bandits used to buy their steel off me. Then the Order paid better, and I stopped asking what for."],
        idle: &["Brutes swing heavy and forget their feet. You know that by now."] },
    Beat { after: "magistrate", news: &["Stonehollow. I had a forge in its north end once. Gone now? Course it is."],
        idle: &["That city's headsman bought his edge off me. I never asked whose necks. I'm good at not asking."] },
    Beat { after: "wraith_sanctum", news: &[
        "Been under the city, have you? Then you've seen what's carved down there. Cloaked figures, reaching for the stars.",
        "I've seen that picture before. Stamped at the top of every order I was paid to fill.",
        "I made their masks. Beaks, to keep the gas out. I never asked what the gas was for. I think I know now.",
    ], idle: &["If you see a beak in the crowd, walk the other way. Slowly."] },
    Beat { after: "cathedral", news: &[
        "If you ever meet a carving that talks, tell it the old man says to live free.",
        "No, I'm not drunk. Just tell it.",
    ], idle: &["Wood forgets. That's the kindest thing about it. That's why I chose it."] },
    Beat { after: "statue_court", news: &["A stone heart, eh? I can make something heavy of it. The angels never could make anything light."],
        idle: &["Angels. Built to last, and look at them."] },
    Beat { after: "spire_summit", news: &["Back again. The road blunts everything. Sit, let me see that edge."],
        idle: &["Same road, same dead. Same old man, too. Some things the mist can't improve."] },
    // Off the story's road: added last, so the beats heard before keep their bits.
    Beat { after: "black_water", news: &["Grey fingers, hooked, and a hunter's rope still knotted on one. Bring me that. I'll make it pull the other way."],
        idle: &["Hunters go into the mire in sixes and come out in ones. Always have."] },
    Beat { after: "fog_hollow", news: &["An eye that won't shut. Ugly little sliver. I can stiffen what it came off into an edge, if you can stand it looking at you."],
        idle: &["Razors that think. I've forged worse. Can't remember when."] },
    Beat { after: "starlit_wastes", news: &["Starglass. I set glass like that once, for an Order. Red, though. They wanted the world one colour."],
        idle: &["Whoever cut that construct cut it to last. I'd shake their hand, if I trusted where it had been."] },
    Beat { after: "sky_peaks", news: &["A drake's fang, warped through. Takes the hammer and gives nothing back. I can put a haft to it, though."],
        idle: &["Wind like that strips a blade. Let me see it before you climb."] },
];

impl Speaker {
    fn beats(self) -> &'static [Beat] { match self { Speaker::Keeper => KEEPER, Speaker::Smith => SMITH } }

    /// A word when spoken to, given the bosses `beaten`: the news of the first beat whose boss has
    /// fallen and whose news has not been `heard` (a bit a beat, set as told), every page of it;
    /// else idle line `k` of the latest story beat reached, with those of the beats reached off the
    /// story's road (a boss no one is sent after).
    pub fn talk(self, beaten: &[&str], heard: &mut u32, k: u32) -> Vec<String> {
        let reached = |b: &Beat| b.after.is_empty() || beaten.contains(&b.after);
        let beats = self.beats();
        if let Some(i) = (0..beats.len()).find(|&i| reached(&beats[i]) && !beats[i].news.is_empty() && *heard & (1 << i) == 0) {
            *heard |= 1 << i;
            return beats[i].news.iter().map(|s| (*s).to_string()).collect();
        }
        let latest = beats.iter().rev().find(|b| reached(b) && Self::on_the_story(b)).map_or(&[][..], |b| b.idle);
        let aside = beats.iter().filter(|b| reached(b) && !Self::on_the_story(b)).flat_map(|b| b.idle);
        let lines: Vec<&str> = latest.iter().chain(aside).copied().collect();
        lines.get(k as usize % lines.len().max(1)).map(|s| vec![(*s).to_string()]).unwrap_or_default()
    }

    /// Whether a beat follows a boss the story sends the hero after (or none: the start).
    fn on_the_story(beat: &Beat) -> bool {
        crate::atlas::place_of(beat.after).and_then(|p| p.boss).is_none_or(|(_, boss)| boss.sought)
    }
}

impl crate::side::Breed {
    /// What a boss says as it dies, the book's words where it has them; the beasts say nothing.
    pub fn last_words(&self) -> Option<&'static str> {
        Some(match self.name {
            "The Debt Collector" => "Paid... in full...",
            "The Bell Warden" => "Who will ring... the bells...",
            "The Hollowmere Screamer" => "Frail... yet...",
            "The Mother of the Black Water" => "Don't... leave us... down here...",
            "The Horror in the Hollow" => "More of us... are waking...",
            "The Echo of Lurz" => "Ain't... runnin'... twice...",
            "The Echo of Krix" => "Dead don't mean... gone...",
            "The Bandit Chief" => "The ridge... was mine...",
            "The Order Agent" => "The Order... claims its due...",
            "The Hollow Magistrate" => "Guilty... all of you... guilty...",
            "The Wraith Lord" => "The dark... was mine to keep...",
            "The Angel Priest" => "I guarded... too long...",
            "The Void Stalker" => "Unmade... undone...",
            "The Statue Archangel" => "Oath... broken...",
            "The Radiant Construct" => "No sign... shown... The road... is closed...",
            "The Archangel" => "The absent heart's taint... consumes...",
            "The Crimson Captain" => "The gate... was to hold...",
            "Exodus Lightfoot" => "The Prophet... will have... his door...",
            // Exodus's own voice, at the last, out of what wore him; Fin-S's, out of his.
            "Voryn's Vessel" => "Harmony... was... his... word...",
            "Voryn Made Whole" => "Fiiin... ess... un... finished...",
            _ => return None,
        })
    }

    /// What a boss says into the hero's head as it is named, for those that speak so in the book.
    pub fn whisper(&self) -> Option<&'static str> {
        Some(match self.name {
            "The Hollowmere Screamer" => "Little thing... so small... so frail...",
            // In the voice of one it took.
            "The Mother of the Black Water" => "Krix...? Krix, is that you? It's so cold down here...",
            "The Horror in the Hollow" => "Small hunters... in the fog... all break...",
            "The Echo of Lurz" => "They're still there... they're still there...",
            "The Echo of Krix" => "Rippers... in me... in me...",
            "The Angel Priest" => "Seeker. You tread fallen ground.",
            "The Void Stalker" => "You fail... all break...",
            "The Archangel" => "Intruder. The hunger beyond consumes.",
            // The book's hint made plain: the cloaked ones who tore the Veil were the Order.
            "Exodus Lightfoot" => "We tore it once, in cloaks, and the angels broke our Eye. This time it is done properly.",
            "Voryn's Vessel" => "They called me Prophet, little light, and opened a door for me. I have worn their Order for an age. Now I wear him.",
            "Voryn Made Whole" => "The door was never the Eye, little light. The door was always him. And now it is open.",
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn news_is_told_once_in_order_then_the_latest_idle_lines_turn() {
        let mut heard = 0;
        let first = Speaker::Keeper.talk(&[], &mut heard, 0);
        assert_eq!(first.len(), 1, "nothing beaten: an idle line");
        let beaten = ["gate", "hollowmere"];
        let news = Speaker::Keeper.talk(&beaten, &mut heard, 0);
        assert!(news[0].contains("Warden"), "{news:?}");
        let next = Speaker::Keeper.talk(&beaten, &mut heard, 0);
        assert!(next[0].contains("hums") && next.len() == 3, "{next:?}");
        let idle: Vec<_> = (0..2).map(|k| Speaker::Keeper.talk(&beaten, &mut heard, k)).collect();
        assert!(idle.iter().all(|l| l.len() == 1) && idle[0] != idle[1]);
        assert!(idle[0][0].contains("stone"), "{idle:?}");
    }

    #[test]
    fn every_region_boss_has_last_words_and_the_speakers_whisper() {
        for p in crate::atlas::PLACES {
            let Some((_, boss)) = p.boss else { continue };
            // The beasts that never spoke die without a word.
            let beast = ["The Keening Cage", "The Warped Drake"].contains(&boss.name());
            if !boss.mini && !beast { assert!(boss.breed.last_words().is_some(), "{} dies silent", boss.name()); }
        }
        assert!(crate::side::SCREAMER.whisper().is_some_and(|w| w.contains("frail")));
    }

    #[test]
    fn every_beat_follows_a_boss_place_and_says_something() {
        for who in [Speaker::Keeper, Speaker::Smith] {
            assert!(who.beats().len() <= 32);
            for b in who.beats() {
                assert!(!b.idle.is_empty());
                let boss = crate::atlas::PLACES.iter().any(|p| p.id == b.after && p.boss.is_some_and(|(_, boss)| !boss.mini));
                assert!(b.after.is_empty() || boss, "{} is no region boss's place", b.after);
            }
        }
    }
}
