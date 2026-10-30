//! The book's later cast, met as the story reaches them (`docs/story.md`, sections 2 to 4), on
//! the same terms as `visitors`: each stands somewhere while the story is where it needs to be,
//! says their piece once and moves on. Every one points on to the next place without a marker.
//! Stonehollow: the registrar pays for proof as Gruk does and sends the hero to Steel; Steel reads
//! the rune as a trail and sends them under the city to Helena; Helena names the Hollowed Shrine.

use crate::visitors::{Visit, Visitor};
use crate::world::{Note, World};

/// Stonehollow: where the registrar keeps the ledger, Steel's shop, Helena's chapel below.
const STAIRS: &str = "lower_stairs";
const UPPER: &str = "stone_upper";
const UNDER: &str = "sewers";
const CHIEF: &str = "chief_hold";
const COURT: &str = "magistrate";
const SANCTUM: &str = "wraith_sanctum";
/// The Hollowed Shrine's halls, where Exodus waits for the hero with the rune.
const HALLS: &str = "vine_halls";
/// Varnholt: the village where Atrael burns the barns and Mira keeps the tavern's books; the
/// cathedral above it; the violet road in, with its lone tree.
const VILLAGE: &str = "varnholt";
const CATHEDRAL: &str = "cathedral";
const VIOLET: &str = "violet_road";
const BLACKWOOD: &str = "blackwood";
/// The thing that struck in the alley (`alley`), once it has.
pub const STRUCK: &str = "alley_struck";

pub const LATER: &[Visit] = &[
    // The registrar, down where the trade is: paid for the Blackridge helms if Gruk has not paid
    // them yet, then the stone told to stay hidden, and Steel's named.
    Visit { id: "registrar_pays", who: Visitor::Registrar, place: STAIRS, at: 12.0, when: |w| w.blackridge_unpaid() && !w.met("registrar_first"), pages: &[
        "Guild ledger. You'll be the Warrens hunter. Gruk's word travels faster than his feet.",
        "That a chief's helm? Spiked. Blackridge.",
        "...Solid take, blue-eyes. Guild pays the same here as anywhere. Ledger don't care which door you came in.",
        "That stone at your belt. Don't wave it about in Stonehollow; there's folk here pay for whispers.",
        "Want it read? North, the Upper City. Steel's Occult Research and Pawn. Knock loud.",
    ] },
    Visit { id: "registrar_first", who: Visitor::Registrar, place: STAIRS, at: 12.0, when: |w| !w.blackridge_unpaid() && !w.met("registrar_pays"), pages: &[
        "Guild ledger. You'll be the Warrens hunter. Gruk's word travels faster than his feet.",
        "Proof's paid here same as the Warrens. Bring me heads, helms, whatever's left of 'em.",
        "That stone at your belt. Don't wave it about in Stonehollow; there's folk here pay for whispers.",
        "Want it read? North, the Upper City. Steel's Occult Research and Pawn. Knock loud.",
    ] },
    Visit { id: "registrar_court", who: Visitor::Registrar, place: STAIRS, at: 12.0, when: |w| w.beaten.contains(&COURT) && (w.met("registrar_first") || w.met("registrar_pays")), pages: &[
        "The court's quiet. First time in years the bells've rung for something other than a hanging.",
        "No bounty on a magistrate. Never was. Wouldn't have known who to pay it to.",
        "...Here. Off the books. Don't tell Gruk.",
    ] },
    // Steel, a soul in empty armour: the rune read as a trail, pointing under the city.
    Visit { id: "steel_reads", who: Visitor::Steel, place: UPPER, at: 18.0, when: |w| w.carries_rune() && (w.met("registrar_first") || w.met("registrar_pays")), pages: &[
        "WELCOME to Steel's Occult Research and Pawn! ...Ha! Jumped, didn't you? Gets 'em every time!",
        "No, no, nobody in here. Just the armour. Me. I'm the armour. Long story. I'll tell it you, for a price.",
        "Now. That stone. Let Steel have a look... Not a ward. Not a curse. A trail. Every mark on it points at the next.",
        "This one points down. Under the city, the Underborough: the Old Sewers, off the Lower Stairs. There's a chapel down there, and a woman in it. Helena Thornheart. Reads runes better than me. Don't tell her I said so.",
        "And mind the alleys down there. Things walk the Underborough that cast no shadow. Ha! Neither do I, mind.",
    ] },
    // Helena in her chapel under the city: the Hollowed Shrine named, and the thing in the alley
    // known, if it struck.
    Visit { id: "helena_struck", who: Visitor::Helena, place: UNDER, at: 44.0, when: |w| w.met("steel_reads") && w.met(STRUCK) && !w.beaten.contains(&SANCTUM), pages: &[
        "Steel sent you. Of course he did. He sends me everything he can't carry.",
        "Something went for you in the alley. Hooded, quick, gone. It's circled this chapel for weeks. It wants what you carry.",
        "Show me the stone. ...Old work. Older than the Order's prayers, older than this city's stones.",
        "Your blight's root, your rune's maker: the Hollowed Shrine, in the Veil's heart. Over the Bridge Ward, down the Sunken Steps.",
        "Go, then. And if the dark starts speaking in your own voice, come back. I've cured worse. Not many.",
    ] },
    Visit { id: "helena_reads", who: Visitor::Helena, place: UNDER, at: 44.0, when: |w| w.met("steel_reads") && !w.met(STRUCK) && !w.beaten.contains(&SANCTUM), pages: &[
        "Steel sent you. Of course he did. He sends me everything he can't carry.",
        "Sit. No, not there; that's where the last one bled out. ...You've the look of a mind half-eaten. The mist does that.",
        "Show me the stone. ...Old work. Older than the Order's prayers, older than this city's stones.",
        "Your blight's root, your rune's maker: the Hollowed Shrine, in the Veil's heart. Over the Bridge Ward, down the Sunken Steps.",
        "Go, then. And if the dark starts speaking in your own voice, come back. I've cured worse. Not many.",
    ] },
    Visit { id: "helena_reads_late", who: Visitor::Helena, place: UNDER, at: 44.0, when: |w| w.met("steel_reads") && w.beaten.contains(&SANCTUM) && !w.met("helena_reads") && !w.met("helena_struck"), pages: &[
        "Steel sent you. Of course he did. He sends me everything he can't carry.",
        "Show me the stone. ...Old work. Older than the Order's prayers, older than this city's stones.",
        "And you've been to the Hollowed Shrine already. I can smell the vines on you. Most don't come back from it.",
        "Whatever made your stone, you're further down its road than I ever went. If the dark starts speaking in your own voice, come back. I've cured worse. Not many.",
    ] },
    // Exodus Lightfoot of the Order, cold and helpful, speaking in absolutes: an ally for the
    // guardian below (`summons`), and the Order's eyes on the rune.
    Visit { id: "exodus_meets", who: Visitor::Exodus, place: HALLS, at: 14.0, when: |w| w.carries_rune() && !w.beaten.contains(&SANCTUM), pages: &[
        "Stop. You carry a stone of the old Veil. Do not deny it. It sings.",
        "I am Exodus Lightfoot, of the Order of the Radiant Veil. The Order mends what was broken. Always.",
        "The guardian below has fixed on your stone. It lets no one pass. It has never let anyone pass.",
        "Together, it falls. Alone, you fall. There is no third way.",
    ] },
    Visit { id: "exodus_thanks", who: Visitor::Exodus, place: HALLS, at: 14.0, when: |w| w.beaten.contains(&SANCTUM) && w.answers.contains(&"exodus_joins") && !w.met(crate::finale::STRUCK), pages: &[
        "The guardian is unmade. The Order thanks you.",
        "The carving behind the heart: a cathedral, three bells. The Order knows it. Varnholt.",
        "The Order remembers everyone who helps it. Everyone.",
    ] },
    Visit { id: "exodus_notes", who: Visitor::Exodus, place: HALLS, at: 14.0, when: |w| w.beaten.contains(&SANCTUM) && w.answers.contains(&"exodus_refused") && !w.met(crate::finale::STRUCK), pages: &[
        "You lived. Alone. The Order notes it.",
        "The Order notes everything.",
    ] },
    // Fin-S, Unit 0, asleep in a lone tree's roots on the road to Varnholt: woken by the rune
    // (`summons`), he follows, and carves a likeness of each friend the hero has made.
    Visit { id: "fins_sleeps", who: Visitor::FinS, place: VIOLET, at: 18.0, when: |w| w.carries_rune() && w.beaten.contains(&SANCTUM) && !w.met(crate::finale::STRUCK), pages: &[
        "(A wooden figure sits in the roots of a lone tree, moss in its joints. The lights at its wrists are dark.)",
        "(Cut into its chest, in careful letters: MUST REMAIN UNFINISHED.)",
        "(The rune at your belt hums. Something in the wood hums back.)",
    ] },
    Visit { id: "fins_carves_olekeen", who: Visitor::FinS, place: BLACKWOOD, at: 10.0, when: |w| w.answers.contains(&"fins_woken") && w.olekeen_friend() && (!w.met(crate::finale::STRUCK) || w.fins_saved()), pages: &[
        "Fin-S... carve. While you sleep. Look.",
        "Little hunter. Big ears. Bow. He hums like you hum. Fin-S hear him far off.",
        "For you. Keep. Fin-S carve more when Fin-S meet more.",
    ] },
    Visit { id: "fins_carves_krix", who: Visitor::FinS, place: CATHEDRAL, at: 6.0, when: |w| w.answers.contains(&"fins_woken") && w.beaten.contains(&CATHEDRAL) && w.hunters.krix_lives() && w.hunters_home() && (!w.met(crate::finale::STRUCK) || w.fins_saved()), pages: &[
        "Fin-S carve. This one... sad. Ear gone. Listens all night.",
        "Fin-S hear of him from you. You hum his name in sleep. Krix.",
        "For you. Keep.",
    ] },
    // Atrael at Varnholt's edge, burning barns with the tainted hiding in them: the first mercy
    // choice (`summons`), which follows the hero to the end.
    Visit { id: "atrael_burns", who: Visitor::Atrael, place: VILLAGE, at: 10.0, when: |w| w.beaten.contains(&SANCTUM) && !w.beaten.contains(&CATHEDRAL), pages: &[
        "Stand aside, rune-bearer. These barns are full of what Varnholt became.",
        "They hide in the straw and pray to a cathedral that stopped listening. The Taint is in them. Fire is the only mercy left.",
        "My mistress gave me fire and ravens for this. Do not make me spend them on you.",
    ] },
    // Mira, who keeps what Varnholt remembers: the cathedral first, and once its shard is taken,
    // her map to Eryndor; what she saw of the barns colours both.
    Visit { id: "mira_bells", who: Visitor::Mira, place: VILLAGE, at: 30.0, when: |w| w.met("atrael_burns") && !w.beaten.contains(&CATHEDRAL), pages: &[
        "Mira. I keep the Broken Bell's books, and what Varnholt remembers. Somebody has to.",
        "The bells stopped a month ago. The priest swore to guard something up there. Then the singing turned to screaming.",
        "If you're going up, go now. Whatever he's become, it's getting louder.",
    ] },
    Visit { id: "mira_map_saved", who: Visitor::Mira, place: VILLAGE, at: 30.0, when: |w| w.beaten.contains(&CATHEDRAL) && w.answers.contains(&"atrael_stopped"), pages: &[
        "You stood in front of Atrael. Half the village watched you do it. The other half was in the barns.",
        "And you came down from the cathedral with something cold. The old books call it a piece of the Eye that Sees Beyond.",
        "Three guardians held its pieces. One was here. The second keeps Eryndor, past the Blackwood.",
        "Here. My map, the old pilgrims' road through the wood. The wolves don't keep to it. Mind that you do.",
        "And this. It's not much. It's what they had. They asked me to give it to whoever stood in the road.",
    ] },
    Visit { id: "mira_map", who: Visitor::Mira, place: VILLAGE, at: 30.0, when: |w| w.beaten.contains(&CATHEDRAL) && w.answers.contains(&"atrael_burned"), pages: &[
        "The barns. I watched from the bell tower.",
        "...I still have to give you the map. I don't have to like you.",
        "That cold thing you carry is a piece of the Eye that Sees Beyond. Three guardians held its pieces. The second keeps Eryndor, past the Blackwood.",
        "The old pilgrims' road goes through the wood. The wolves don't keep to it. Go.",
    ] },
    Visit { id: "mira_map_plain", who: Visitor::Mira, place: VILLAGE, at: 30.0, when: |w| w.beaten.contains(&CATHEDRAL) && !w.answers.contains(&"atrael_stopped") && !w.answers.contains(&"atrael_burned"), pages: &[
        "Mira. I keep the Broken Bell's books, and what Varnholt remembers. The bells are quiet now. Quiet's better than what they were.",
        "That cold thing you brought down is a piece of the Eye that Sees Beyond. Three guardians held its pieces. The second keeps Eryndor, past the Blackwood.",
        "Here. My map, the old pilgrims' road through the wood. The wolves don't keep to it. Mind that you do.",
    ] },
    Visit { id: "helena_after_shrine", who: Visitor::Helena, place: UNDER, at: 44.0, when: |w| w.beaten.contains(&SANCTUM) && !w.beaten.contains(&CATHEDRAL) && (w.met("helena_reads") || w.met("helena_struck")), pages: &[
        "You came back. Most don't. The Shrine keeps what walks into it.",
        "A heart that opens for blood. A carving of a cathedral. ...That's Varnholt. Three days' ride east, past the court. You'd hear its bells from the hills.",
        "Its bells stopped a month ago. Nobody's gone to ask why. Go and ask why.",
    ] },
];

impl World {
    /// Whether the hero has met Olekeen a step ahead, past the East Gate's first meeting: enough
    /// for Fin-S to have heard them hum his name.
    fn olekeen_friend(&self) -> bool { self.meetings.iter().any(|m| m.starts_with("olekeen_stone") || m.starts_with("olekeen_shrine") || *m == "olekeen_bound") }

    /// Whether the Blackridge bounty is proven and not yet paid: the registrar pays it as well as
    /// Gruk would.
    fn blackridge_unpaid(&self) -> bool {
        self.guild.open() == Some(crate::guild::Bounty::Blackridge) && self.beaten.contains(&CHIEF)
    }

    /// What the registrar pays at meeting `visit`: the Blackridge bounty, as the guild's ledger
    /// keeps it; a purse off the books once the court is quiet. None for anyone else's meeting.
    pub(crate) fn registrar_pays(&mut self, visit: &str) -> Option<Vec<Note>> {
        match visit {
            "registrar_pays" if self.blackridge_unpaid() => {
                self.guild.paid += 1;
                Some(self.paid_by_registrar(crate::guild::Bounty::Blackridge.pay()))
            }
            "registrar_court" => Some(self.paid_by_registrar(COURT_PURSE)),
            _ => None,
        }
    }

    fn paid_by_registrar(&mut self, n: u32) -> Vec<Note> {
        self.souls += n;
        vec![Note::Paid(Visitor::Registrar.name(), n)]
    }
}

/// The registrar's purse for the court, off the books.
const COURT_PURSE: u32 = 600;

#[cfg(test)]
mod tests {
    use super::*;

    fn in_stonehollow() -> World {
        let mut w = World::new(0.0);
        w.beaten.push(CHIEF);
        w.keys.push(crate::atlas::Key::Rune);
        w
    }

    #[test]
    fn the_registrar_pays_the_blackridge_helms_once_and_points_to_steel_who_points_under_the_city() {
        let mut w = in_stonehollow();
        w.guild.posted = 2;
        w.guild.paid = 1;
        w.arrive("lower_stairs@stone_gate", 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("registrar_pays"));
        assert_eq!(w.gift_from("registrar_pays"), vec![Note::Paid("The Registrar", crate::guild::Bounty::Blackridge.pay())]);
        assert_eq!(w.guild.paid, 2, "Gruk will not pay it again");
        assert_eq!(w.souls, crate::guild::Bounty::Blackridge.pay(), "into the purse");
        w.meet_visitor();
        w.arrive("stone_upper@stone_market", 200.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(who, p)| who == "Steel" && p[3].contains("Helena")));
        w.arrive("sewers@lower_stairs", 300.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(who, p)| who == "Helena Thornheart" && p[3].contains("Hollowed Shrine")));
    }

    #[test]
    fn helena_knows_the_alley_struck_and_after_the_shrine_sends_the_hero_to_varnholt() {
        let mut w = in_stonehollow();
        w.meetings.extend(["steel_reads", STRUCK]);
        w.arrive("sewers@lower_stairs", 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("helena_struck"));
        w.meet_visitor();
        w.beaten.push(SANCTUM);
        w.arrive("lower_stairs@sewers", 200.0, 0.0, 0);
        w.arrive("sewers@lower_stairs", 300.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p[1].contains("Varnholt")));
    }

    #[test]
    fn exodus_offers_his_help_for_the_shrines_guardian_and_is_summoned_there_alone() {
        let mut w = in_stonehollow();
        w.arrive("vine_halls@sunken_steps", 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| (v.id, v.who)), Some(("exodus_meets", Visitor::Exodus)));
        assert!(w.asking("exodus_meets").is_some());
        w.answer("exodus_joins");
        w.arrive("wraith_sanctum@vine_halls", 200.0, 0.0, 0);
        assert_eq!(w.summons(), Some(crate::allies::Summoner::Exodus));
        w.arrive("magistrate@stone_upper", 300.0, 0.0, 0);
        assert_eq!(w.summons(), None, "for the Shrine's guardian only");
    }

    #[test]
    fn at_varnholt_atrael_asks_and_mira_answers_what_was_chosen_and_fin_s_carves_once_woken() {
        let mut w = in_stonehollow();
        w.beaten.push(SANCTUM);
        w.arrive("varnholt@violet_road", 100.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("atrael_burns"));
        w.meet_visitor();
        w.answer("atrael_stopped");
        w.beaten.push(CATHEDRAL);
        w.arrive("violet_road@varnholt", 200.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("fins_sleeps"));
        w.meet_visitor();
        w.answer("fins_woken");
        w.arrive("varnholt@violet_road", 300.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("mira_map_saved"));
        assert_eq!(w.gift_from("mira_map_saved"), vec![Note::Found("Varnholt Straw Charm")]);
        w.meetings.push("olekeen_stone_saved");
        w.arrive("blackwood@cathedral", 400.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("fins_carves_olekeen"));
    }

    #[test]
    fn meetings_follow_what_the_hero_did_not_only_where_they_stand() {
        let at = |w: &mut World, place: &str| { w.lingering = None; w.arrive(&crate::atlas::stop(place, crate::atlas::trunk(place)), 100.0, 0.0, 0); };
        // The registrar's purse is paid once, however often the hero talks to him.
        let mut w = World::new(0.0);
        w.beaten.push(COURT);
        w.meetings.push("registrar_first");
        at(&mut w, STAIRS);
        let before = w.souls;
        crate::shrine::Meeting::visitor(&mut w);
        crate::shrine::Meeting::visitor(&mut w);
        assert_eq!(w.souls - before, COURT_PURSE);
        // Mira does not blame a hero who never stood before Atrael either way.
        let mut w = World::new(0.0);
        w.beaten.push(CATHEDRAL);
        at(&mut w, VILLAGE);
        assert_eq!(w.visit().map(|v| v.id), Some("mira_map_plain"));
        // Fin-S's tree is empty once the Order has been through.
        let mut w = World::new(0.0);
        w.keys.push(crate::atlas::Key::Rune);
        w.beaten.push(SANCTUM);
        w.meetings.push(crate::finale::STRUCK);
        at(&mut w, VIOLET);
        assert!(w.visit().is_none_or(|v| v.id != "fins_sleeps"));
        // Helena, met after the Shrine, does not send the hero to it.
        let mut w = World::new(0.0);
        w.meetings.push("steel_reads");
        w.beaten.push(SANCTUM);
        at(&mut w, UNDER);
        assert_eq!(w.visit().map(|v| v.id), Some("helena_reads_late"));
        w.meet_visitor();
        assert!(w.met_helena());
    }
}
