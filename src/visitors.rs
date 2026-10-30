//! People met on the road who are not the shrine's, the way the Souls games keep theirs: each
//! stands somewhere while the story is where it needs to be, says their piece once, and moves on,
//! a step ahead. What they say answers what the hero has done, and in what order: a beast the
//! story would send the hero after may already be dead, by the hero's hand or someone else's.
//! Their questlines are in `docs/story.md` (section 4, and "Exploration rewarded").

use crate::world::{Note, World};

/// Who can be met on the road, drawn from `assets/pixellab/<folder>` (an idle facing the camera).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visitor { Olekeen, Krix, Rik, Skif, SkifGrown, Lurz, Tusk, Vren, Registrar, Steel, Helena, Exodus, Atrael, Mira, FinS, Notice, Mark }

impl Visitor {
    pub fn name(self) -> &'static str {
        match self {
            Visitor::Olekeen => "Olekeen", Visitor::Krix => "Krix", Visitor::Rik => "Rik", Visitor::Skif | Visitor::SkifGrown => "Skif", Visitor::Lurz => "Lurz",
            Visitor::Tusk => "Tusk", Visitor::Vren => "Vren", Visitor::Registrar => "The Registrar", Visitor::Steel => "Steel", Visitor::Helena => "Helena Thornheart",
            Visitor::Exodus => "Exodus Lightfoot", Visitor::Atrael => "Atrael", Visitor::Mira => "Mira", Visitor::FinS => "Fin-S",
            Visitor::Notice => "A Fox-Marked Notice", Visitor::Mark => "A Hunter's Mark",
        }
    }
    pub fn folder(self) -> &'static str {
        match self {
            Visitor::Olekeen => "olekeen", Visitor::Krix => "krix", Visitor::Rik => "rik", Visitor::Skif => "skif", Visitor::SkifGrown => "skif_grown", Visitor::Lurz => "lurz",
            Visitor::Tusk => "tusk", Visitor::Vren => "vren", Visitor::Registrar => "registrar", Visitor::Steel => "steel", Visitor::Helena => "helena",
            Visitor::Exodus => "exodus", Visitor::Atrael => "atrael", Visitor::Mira => "mira", Visitor::FinS => "fin_s", Visitor::Notice => "fox_notice", Visitor::Mark => "olekeen_mark",
        }
    }
    /// How tall they stand, metres: Steel's armour and winged Atrael the tallest, the goblin
    /// hunters shorter, Skif a youngling until Stonehollow; a notice, its post.
    pub fn height(self) -> f32 {
        match self {
            Visitor::Olekeen | Visitor::Krix | Visitor::Lurz | Visitor::SkifGrown | Visitor::Tusk => 1.3,
            Visitor::Rik => 1.75, Visitor::Skif => 1.0, Visitor::Vren | Visitor::Mira | Visitor::Registrar | Visitor::Helena => 1.65,
            Visitor::Steel | Visitor::Atrael => 2.0, Visitor::Exodus => 1.55, Visitor::FinS | Visitor::Notice => 1.45, Visitor::Mark => 1.3,
        }
    }
}

/// One meeting: its name (kept once told), who, at which place and how many metres in, the story
/// it waits for, and what is said.
pub struct Visit {
    pub id: &'static str,
    pub who: Visitor,
    pub place: &'static str,
    pub at: f32,
    pub when: fn(&World) -> bool,
    pub pages: &'static [&'static str],
}

/// The beast at the east gate, the book's first hunt after Hollowmere.
const GATE: &str = "east_gate";
/// Krix's beast, past the gate in the black water, which the book never kills.
const BLACK: &str = "black_water";
/// Where Krix drinks, once the Collector is down and the hunters come back in.
const HALL: &str = "guild_hall";
/// The Blackridge crew's hole under the ridge, and the boss whose fall sends them off with its stash.
pub const CAVE: &str = "bandit_cave";
const CHIEF: &str = "chief_hold";
/// The camp under the Chief's Hold, where Olekeen waits for dark once the Blackridge bounty is up.
const CAMP: &str = "ridge_camp";
/// The hollow where the void horror waits for Krix and Lurz.
const FOG: &str = crate::hunters::FOG;
/// The hamlet whose well drew up the first beast, and where Rik still listens to it.
const MERE: &str = "hollowmere";
/// Stonehollow's court, whose fall is how long Skif keeps his watch.
const COURT: &str = "magistrate";

/// What someone hands over the first time their words are heard: (meeting, piece of gear).
const GIFTS: &[(&str, &str)] = &[
    ("skif_watch", "Skif's Rusted Knife"),
    ("lurz_home", "Lurz's Spear"),
    ("vren_asks_blade", "Vren's Chipped Ring"),
    ("fins_carves_olekeen", "Fin-S's Carved Hunter"),
    ("fins_carves_krix", "Fin-S's Carved Listener"),
    ("mira_map_saved", "Varnholt Straw Charm"),
];
/// The Warrens market, Tusk's cart; Vren's crooked tower down its side path.
const MARKET_W: &str = "warrens";
const SPIRE_B: &str = "broken_spire";
/// The ruin the rune-marked blade lay in.
pub const FINGERS: &str = "drowned_fingers";
/// Stonehollow's gate, where Olekeen comes in from Blackridge.
const STONE: &str = "stone_gate";
/// Before the Hollowed Shrine, and the beast in its sanctum.
const STEPS: &str = "sunken_steps";
const SANCTUM: &str = "wraith_sanctum";
/// Under the Spire's sealed slab, where the Order leaves Olekeen bound; gone once the Summit falls.
const SLAB: &str = "sealed_slab";
const SUMMIT: &str = "spire_summit";
/// Olekeen's meetings at Stonehollow's gate, one of which is told.
const AT_STONE: [&str; 4] = ["olekeen_stone_fog", "olekeen_stone_saved", "olekeen_stone_one", "olekeen_stone_lost"];
/// Where Lurz turns up once home from the hollow: Stonehollow's lantern market.
const MARKET: &str = "stone_market";

pub const VISITS: &[Visit] = &[
    // In the story's order: the Hollowmere bounty paid, Olekeen took the beast, and he is still
    // standing over it when the hero comes.
    Visit { id: "olekeen_gate_his_kill", who: Visitor::Olekeen, place: GATE, at: 24.0, when: |w| w.claimed.contains(&GATE) && !w.beaten.contains(&CHIEF), pages: &[
        "Hunter! You come for bone thing too?",
        "Too late. Olekeen kill. Big boom. Dead now.",
        "Lurz see it first, run to guild, shout. Olekeen run here. Olekeen shoot good.",
        "Your stone hum? Olekeen's hum at bone thing. Like they talk.",
        "Bad thing leave mark. Olekeen go north now, Blackridge. Olekeen first.",
    ] },
    // The hero went out the gate before anyone sent them: Olekeen came for his bounty and found it
    // dead.
    Visit { id: "olekeen_gate_beaten_to_it", who: Visitor::Olekeen, place: GATE, at: 24.0, when: |w| w.beaten.contains(&GATE) && w.guild.paid > 0 && !w.beaten.contains(&CHIEF), pages: &[
        "Bone thing... dead?",
        "Lurz shout at guild, big shadow thing at east gate. Olekeen run all way here. Bone thing already dead.",
        "...You kill? You?",
        "Hmph. Good kill. Olekeen not say twice.",
        "Was Olekeen's bounty. Olekeen not done. Next one, Olekeen first.",
    ] },
    // Olekeen at Ridge Camp once the Blackridge bounty is posted: waiting for dark to go up at the
    // chief, or finding the hero already did. Krix and Lurz go with him, east, to the hollow.
    Visit { id: "olekeen_camp_waiting", who: Visitor::Olekeen, place: CAMP, at: 20.0, when: |w| w.guild.posted >= 2 && !w.beaten.contains(&CHIEF), pages: &[
        "You! Hunter from Warrens. Olekeen here first.",
        "Chief up top, in his hold. Many bows. Olekeen wait for dark. Then big boom.",
        "Krix come too. Lurz too. Krix twitchy. Lurz scared of own feet.",
        "Your stone hum here? Olekeen's quiet. Means bad thing not here. Means bad thing somewhere else.",
    ] },
    Visit { id: "olekeen_camp_beaten", who: Visitor::Olekeen, place: CAMP, at: 20.0, when: |w| w.guild.posted >= 2 && w.beaten.contains(&CHIEF) && !w.met("olekeen_camp_waiting") && !w.hunters.settled && !AT_STONE.iter().any(|id| w.met(id)), pages: &[
        "Chief dead?! Olekeen wait all night for dark!",
        "...You. Hunter from Warrens. And now you first to chief.",
        "Hmph. Olekeen keep score. Olekeen behind. Not for long.",
        "Krix, Lurz with Olekeen now. We go east, to hollow. Krix say shadows there. Krix always say shadows.",
    ] },
    Visit { id: "olekeen_camp_after", who: Visitor::Olekeen, place: CAMP, at: 20.0, when: |w| w.beaten.contains(&CHIEF) && w.met("olekeen_camp_waiting") && !w.hunters.settled && !AT_STONE.iter().any(|id| w.met(id)), pages: &[
        "Chief dead. Olekeen see from rocks. Good kill.",
        "Olekeen wait for dark. You not wait for nothing. Hmph!",
        "Krix, Lurz with Olekeen now. We go east, to hollow. Krix say shadows there. Krix always say shadows.",
    ] },
    // Olekeen left the youngling Skif to guard the husk, the book's "No touch, no take", once he had
    // been out to it; after Stonehollow Skif is still there, grown into a hunter.
    Visit { id: "skif_guard", who: Visitor::Skif, place: GATE, at: 28.0, when: |w| w.olekeen_went_out() && !w.beaten.contains(&COURT), pages: &[
        "No touch! No take!",
        "...Olekeen say guard. Skif guard good.",
        "Olekeen gone north. Olekeen say Olekeen come back. Skif wait.",
        "No touch, no take.",
    ] },
    Visit { id: "skif_watch", who: Visitor::SkifGrown, place: GATE, at: 28.0, when: |w| w.olekeen_went_out() && w.beaten.contains(&COURT), pages: &[
        "You. The one with the humming stone. Skif remember.",
        "Skif still guard. Long time. Bone thing gone to dust. Skif guard dust.",
        "Skif hunter now. Three hounds out of the reeds, on Skif's spear.",
        "Olekeen come back? ...Olekeen say Olekeen come back.",
        "Here. Skif's old knife. Skif not need it now. Olekeen say no touch, no take. Skif say: take.",
    ] },
    // Rik, at Hollowmere once its beast is dead, hearing what the village will not. After Blackridge
    // the well stirs: Krix's beast calling up it while it lives, Mara's whistle once it is still.
    Visit { id: "rik_hears", who: Visitor::Rik, place: MERE, at: 38.0, when: |w| w.beaten.contains(&MERE) && !w.beaten.contains(&CHIEF), pages: &[
        "You hear it still?",
        "The beast's dead. I watched it die. But there's something... soft. Like it's in the wind.",
        "Gran called the well cursed. Said it drew things up from below.",
        "I keep thinking it hasn't finished drawing.",
    ] },
    Visit { id: "rik_well_stirs", who: Visitor::Rik, place: MERE, at: 38.0, when: |w| w.beaten.contains(&CHIEF) && !w.mother_down(), pages: &[
        "You came back. Good. Listen. ...No. Listen.",
        "Frail... yet... That's what the wind says now. Every night.",
        "Last night the well stirred. The water came up black. Not mud. Black, like they say the mire goes past the Warrens' east gate.",
        "Something down there was calling a man's name. Krix, it said. Krix, it's so cold.",
        "It isn't ours, whatever it is. It's reaching up from somewhere else. Out past that gate.",
    ] },
    Visit { id: "rik_well_still", who: Visitor::Rik, place: MERE, at: 38.0, when: |w| w.beaten.contains(&CHIEF) && w.mother_down() && w.met("rik_well_stirs"), pages: &[
        "The well's gone still. The water's clear again, near enough.",
        "Last night it whistled. Twice, sharp, like someone who'd found a trail. Then nothing.",
        "You went to where it was coming from. Didn't you.",
    ] },
    Visit { id: "rik_whistle", who: Visitor::Rik, place: MERE, at: 38.0, when: |w| w.beaten.contains(&CHIEF) && w.mother_down() && !w.met("rik_well_stirs"), pages: &[
        "Since the beast died, the wind says Frail... yet... every night. Gran would've blamed the well.",
        "But last night the well whistled. Twice, sharp, like someone who'd found a trail.",
        "Then it went quiet. Quieter than it's been since before the beast.",
    ] },
    // In the fog hollow while Krix and Lurz hold out (`hunters`): Lurz at its mouth, or Krix
    // alone once the horror has had Lurz. After it falls, whoever lived says so there.
    Visit { id: "lurz_in_the_fog", who: Visitor::Lurz, place: FOG, at: 8.0, when: |w| w.hunters_in_the_fog() && w.hunters.lurz_lives() && !w.beaten.contains(&FOG), pages: &[
        "They're still there! They're still there!",
        "Olekeen set a trap. Ropes, stones. Got four of the hounds. Then the fog came in, and something in it.",
        "Krix went in after it. Shouting. Rippers, he said. Rippers, in me.",
        "Don't go in. Don't... You're going in.",
        "Ain't runnin' twice. I'm behind you. Right behind you.",
    ] },
    Visit { id: "krix_in_the_fog", who: Visitor::Krix, place: FOG, at: 8.0, when: |w| w.hunters_in_the_fog() && !w.hunters.lurz_lives() && w.hunters.krix_lives() && !w.beaten.contains(&FOG), pages: &[
        "It took Lurz. Dragged him into the dark while you were gone. I heard his bones go.",
        "It talks. Like the black water did. It knows my name.",
        "Go on, then. I'm behind you. I'm always behind someone.",
    ] },
    Visit { id: "hollow_both_live", who: Visitor::Krix, place: FOG, at: 34.0, when: |w| w.beaten.contains(&FOG) && w.hunters_home() && w.met("lurz_in_the_fog") && w.hunters.lurz_lives(), pages: &[
        "You came. Into the fog, for two men you barely know.",
        "Lurz, get up. It's over. ...It's over.",
        "We're going home. Warrens for me. Lurz says Stonehollow, somewhere with walls.",
        "Find us. We owe you, and I pay what I owe.",
    ] },
    Visit { id: "hollow_krix_lives", who: Visitor::Krix, place: FOG, at: 34.0, when: |w| w.beaten.contains(&FOG) && w.hunters_home() && !w.hunters.lurz_lives() && w.hunters.krix_lives(), pages: &[
        "It's dead. And Lurz is still in there somewhere. In the dark.",
        "He said he wasn't running twice. He didn't.",
        "Dead don't mean gone. You'll hear him too.",
        "I'm going home. Find me at the guild, if you want. I'll be the one not sleeping.",
    ] },
    // Home from the hollow, with the weapons the book gave them.
    Visit { id: "krix_home", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.hunters_home() && w.hunters.lurz_lives(), pages: &[
        "Back at the guild. Done with fog.",
        "Lurz sends word from Stonehollow. Walls, he says. Good thick walls.",
        "Here. My axe. Every notch is a hunter I outlived. It's yours now; I'm done adding to it.",
    ] },
    Visit { id: "krix_home_alone", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.hunters_home() && !w.hunters.lurz_lives() && w.hunters.krix_lives(), pages: &[
        "Gruk let me cut Lurz's mark over the door. Next to the others.",
        "I don't sleep. I listen. Mara whistles. Lurz just screams.",
        "Here. My axe. Every notch is a hunter I outlived. Take it before I cut another.",
    ] },
    Visit { id: "lurz_home", who: Visitor::Lurz, place: MARKET, at: 14.0, when: |w| w.hunters_home() && w.hunters.lurz_lives(), pages: &[
        "Hey! Hey, it's you. The one who killed the fog thing.",
        "Walls. Good thick walls. And lamps, all night. I like it here.",
        "I ain't hunting no more. Here. My spear. Always finds the crack in things. Find a few more.",
        "Ain't runnin' twice. But I ain't walkin' back in neither.",
    ] },
    // Olekeen a step ahead, after Blackridge: at Stonehollow's gate, answering the hollow (and
    // sending the hero back while the hunters are still in the fog); before the Hollowed Shrine,
    // first or beaten to it; bound under the Spire's slab by the Order's masked men.
    Visit { id: "olekeen_stone_fog", who: Visitor::Olekeen, place: STONE, at: 6.0, when: |w| w.olekeen_at_stone() && w.hunters_in_the_fog(), pages: &[
        "Hunter. Stonehollow. Big walls, many eyes.",
        "Krix, Lurz... still in hollow. Fog came. Olekeen ran for help. Olekeen ran.",
        "Olekeen's plan. Olekeen's trap. Fog not in plan.",
        "You go back? You fast. Faster than Olekeen. Go back. East, off the hollows.",
    ] },
    Visit { id: "olekeen_stone_saved", who: Visitor::Olekeen, place: STONE, at: 6.0, when: |w| w.olekeen_at_stone() && w.hunters_home() && w.hunters.lurz_lives(), pages: &[
        "Hunter! Krix, Lurz... alive. Both. Olekeen hear it on road.",
        "Fog thing dead before it take them. Hunter kill it. Olekeen not forget.",
        "Olekeen go to registrar. Sack full of helms. Then Olekeen find relic-hunter. One who knows old shadows.",
    ] },
    Visit { id: "olekeen_stone_one", who: Visitor::Olekeen, place: STONE, at: 6.0, when: |w| w.olekeen_at_stone() && w.hunters_home() && !w.hunters.lurz_lives() && w.hunters.krix_lives(), pages: &[
        "Krix live. Lurz... not.",
        "Lurz say ain't running twice. Lurz not run. Olekeen ran.",
        "Olekeen's plan. Olekeen's failure. You not carry it. Olekeen carry.",
    ] },
    Visit { id: "olekeen_stone_lost", who: Visitor::Olekeen, place: STONE, at: 6.0, when: |w| w.olekeen_at_stone() && w.hunters_home() && !w.hunters.krix_lives(), pages: &[
        "Krix. Lurz. Both gone.",
        "Olekeen see Krix die. Every night. In mud. Wrong mud. Hollowmere mud. Why Hollowmere?",
        "...What's wrong with Olekeen?",
        "Olekeen's plan, Olekeen's failure. Rune's tied to what broke them. Olekeen gotta know why.",
    ] },
    Visit { id: "olekeen_steps_lost", who: Visitor::Olekeen, place: STEPS, at: 6.0, when: |w| w.beaten.contains(&COURT) && w.hunters.settled && !w.hunters.krix_lives() && !w.beaten.contains(&FOG) && !w.beaten.contains(&"statue_court"), pages: &[
        "Hunter. Krix. Lurz. Fog keep them both.",
        "Olekeen go back to hollow with help. Too late. Hollow quiet now. Too quiet.",
        "Olekeen's plan. Olekeen's failure. Rune's tied to what broke them. Olekeen go down to Shrine. Olekeen need to know why.",
    ] },
    Visit { id: "olekeen_shrine_first", who: Visitor::Olekeen, place: STEPS, at: 6.0, when: |w| w.beaten.contains(&COURT) && !w.beaten.contains(&SANCTUM) && !w.beaten.contains(&"statue_court") && !w.met("olekeen_steps_lost"), pages: &[
        "Olekeen here first! Ha! Score better now.",
        "Shrine down there. Old. Black stone. Vines that move when nobody look.",
        "Your stone hum here? Olekeen's sing. Loud. Like Shrine calling it home.",
        "Olekeen go down soon. You go first, Olekeen not mind. This once.",
    ] },
    Visit { id: "olekeen_shrine_beaten", who: Visitor::Olekeen, place: STEPS, at: 6.0, when: |w| w.beaten.contains(&SANCTUM) && !w.met("olekeen_shrine_first") && !w.beaten.contains(&"statue_court"), pages: &[
        "Shrine beast... dead. Again you. Always you.",
        "Olekeen stop keeping score. Score bad.",
        "Olekeen go east now. Varnholt, then north. Tower that touch sky. Olekeen first there. Olekeen promise.",
    ] },
    Visit { id: "olekeen_bound", who: Visitor::Olekeen, place: SLAB, at: 18.0, when: |w| w.beaten.contains(&"statue_court") && !w.beaten.contains(&SUMMIT), pages: &[
        "Hunter! Here! Olekeen here. Tied. Like boar for spit.",
        "Mask men. Beak masks, red glass eyes. Gas from little pots. Olekeen breathe once, wake up tied.",
        "They take Olekeen's bow. They say Order. They say Eye. They go up.",
        "Cut rope. ...Good. Olekeen owe you. Again. Olekeen stop counting that too.",
        "Olekeen still have teeth. You go up. Olekeen come behind. This time Olekeen not first. This time good.",
    ] },
    // Tusk at his cart once the hero carries the rune, whose mark he knows from a blade a shaken
    // hunter sold him, out of the Drowned Fingers; or, if the hero was there first, the hunter
    // came back with nothing. Vren at the Broken Spire reads the rune, and the bone after it.
    Visit { id: "tusk_sold_it", who: Visitor::Tusk, place: MARKET_W, at: 30.0, when: |w| w.carries_rune() && w.blade_found_by() == Some(false), pages: &[
        "Oi, hunter! Got silver to spend, or just sniffin' round my wares?",
        "...Funny mark, that. On your stone. Seen it before. On a blade.",
        "Shaken hunter sold it me. Said it came from a ruin up north, past the vale. Place called the Drowned Fingers.",
        "Muttered about whispers in the dark. Sold it cheap. Wanted it gone fast.",
        "I flipped it to some brainy type with a hood. Lives at the Broken Spire, edge of the Warrens. Vren. Watch yer step round him; he don't trade straight.",
    ] },
    Visit { id: "tusk_beaten_to_it", who: Visitor::Tusk, place: MARKET_W, at: 30.0, when: |w| w.carries_rune() && w.blade_found_by() == Some(true), pages: &[
        "Oi, hunter! ...Funny mark, that. On your stone.",
        "Hunter came by a while back, shaking. Been up to a ruin past the vale, the Drowned Fingers. Said there was meant to be a blade there, with marks like that.",
        "Said somebody'd beat him to it. Box pried open, mud everywhere, nothing in it.",
        "...That on your belt? Ha! Don't show Vren, out at the Broken Spire. He'll want it. He don't trade straight.",
    ] },
    Visit { id: "vren_shows_blade", who: Visitor::Vren, place: SPIRE_B, at: 20.0, when: |w| w.carries_rune() && w.blade_found_by() == Some(false) && !w.met("vren_asks_blade"), pages: &[
        "Trader sent you? ...Show me. Old, yes. Very old.",
        "Hollowmere? You slew it? Then you've stirred something, hunter.",
        "This blade came to me with the same marks. Pre-Shatter, maybe. Not a ward, not a blessing... a call.",
        "A hunter called Krix sketched me something like your beast. Lost his crew to it in the mire past the east gate. He drinks at the guild. Ask him.",
        "Bring me what you learn. I'll pay. Knowledge ain't free, and neither am I.",
    ] },
    Visit { id: "vren_asks_blade", who: Visitor::Vren, place: SPIRE_B, at: 20.0, when: |w| w.carries_rune() && w.blade_found_by() == Some(true) && !w.met("vren_shows_blade"), pages: &[
        "Trader sent you? ...That stone. Old, very old. Not a ward, not a blessing... a call.",
        "And Tusk says someone got to the Drowned Fingers before his hunter did. You. Show me the blade.",
        "...The same hand cut these. Whoever made your stone made this.",
        "Here. For the look. A ring I've no more use for. It pays well. It costs a little too, but everything does.",
    ] },
    Visit { id: "vren_reads_bone", who: Visitor::Vren, place: SPIRE_B, at: 20.0, when: |w| w.carries_rune() && (w.met("vren_shows_blade") || w.met("vren_asks_blade")) && w.beaten.contains(&GATE), pages: &[
        "Back so soon? ...Another one. The bone from the east gate's thing. Look how they flare together.",
        "Older than the Shatter. These marks are calls, bindings, maybe worse. Tied to places. Old places.",
        "Past the Blackridge: Stonehollow. Someone there hoards relics and pays for whispers. Driven, they say.",
    ] },
    // Krix, at the guild until he signs on for Blackridge. He tells of the black water to whoever
    // will listen, and once it is still, answers whether he told them first and whether they
    // brought anything back.
    Visit { id: "krix_tale", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.krix_drinks() && !w.mother_down(), pages: &[
        "You've got the look. Like you've heard something you shouldn't have.",
        "Past the east gate the mire goes black. Old stones. Black water. Something lives in it.",
        "Seven of us went in for it. Jek went under first. Then Mara. It didn't eat them. It kept them.",
        "I hear them at night. Mara whistles, the way she did on a hunt. Twice, when she'd found the trail.",
        "Dead don't mean gone. You'll hear yours too.",
    ] },
    Visit { id: "krix_told_whistle", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.krix_drinks() && w.mother_down() && w.met("krix_tale") && w.has_whistle() && !w.met("krix_told"), pages: &[
        "You went out there. Into the black.",
        "...Did it talk? In her voice?",
        "Don't. Don't tell me.",
        "That's her whistle. Keep it. She'd want it blown by someone still walking.",
        "It's quiet tonight. First time in a year.",
    ] },
    Visit { id: "krix_told", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.krix_drinks() && w.mother_down() && w.met("krix_tale") && !w.has_whistle() && !w.met("krix_told_whistle"), pages: &[
        "You went out there. Into the black. And it's quiet now.",
        "...Did it talk? In her voice? Don't. Don't tell me.",
        "She had a whistle. Bone, on a cord. If it's still out there, I can't go back for it. I can't.",
    ] },
    // The hero killed it before anyone told them of it: a stranger did what Krix could not.
    Visit { id: "krix_beaten_whistle", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.krix_drinks() && w.mother_down() && !w.met("krix_tale") && w.has_whistle() && !w.met("krix_beaten"), pages: &[
        "Heard someone went out past the gate. Into the black water. And came back.",
        "...That whistle. Where'd you get that?",
        "Mara's. That's Mara's. She'd blow it twice when she'd found the trail.",
        "It's dead, then. The thing. Wasn't me. Should've been me.",
        "Keep it. She'd want it blown by someone still walking.",
    ] },
    Visit { id: "krix_beaten", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.krix_drinks() && w.mother_down() && !w.met("krix_tale") && !w.has_whistle() && !w.met("krix_beaten_whistle"), pages: &[
        "Heard someone went out past the gate. Into the black water. And came back.",
        "Heard the thing's dead. ...Wasn't me. Should've been me.",
        "Seven of us went in. Jek. Mara. Four more. I came out with one ear and their voices.",
        "Dead don't mean gone. Maybe now it does.",
    ] },
    // Told it was quiet, then the whistle brought back after all.
    Visit { id: "krix_whistle_brought", who: Visitor::Krix, place: HALL, at: 16.0, when: |w| w.krix_drinks() && w.has_whistle() && (w.met("krix_told") || w.met("krix_beaten")), pages: &[
        "...You went back for it. Her whistle.",
        "Keep it. She'd want it blown by someone still walking.",
    ] },
];

impl Visit {
    /// Every meeting on the road: the first regions' here, the later cast's (`cast::LATER`).
    pub fn every() -> impl Iterator<Item = &'static Visit> { VISITS.iter().chain(crate::cast::LATER).chain(crate::finale::VISITS).chain(crate::relics::VISITS)
        .chain(crate::steel::VISITS).chain(crate::bounty::VISITS).chain(crate::taint::VISITS).chain(crate::spire_road::VISITS) }
}

impl World {
    /// The meeting waiting in the place the hero is in: one whose story has come and is untold, or
    /// the one just had, who stays until the hero leaves.
    pub fn visit(&self) -> Option<&'static Visit> {
        // Asked something not yet answered, they wait to hear it.
        let asked = |v: &&Visit| self.meetings.contains(&v.id) && (v.when)(self) && self.asking(v.id).is_some();
        let open = |v: &&Visit| self.lingering == Some(v.id) || (!self.meetings.contains(&v.id) && (v.when)(self)) || asked(v);
        Visit::every().filter(|v| v.place == self.here().id).find(open)
    }

    /// Where the one waiting here stands, road metres, and who.
    pub fn visitor_at(&self) -> Option<(f64, Visitor)> {
        self.visit().map(|v| (self.road_at(v.at), v.who))
    }

    /// Whether the hero stands by someone waiting to be spoken to, with no enemy close.
    pub fn can_meet_visitor(&self, fight: &crate::fight::Fight, walked: f64) -> bool {
        let Some((at, _)) = self.visitor_at() else { return false };
        (at - Self::standing(walked)).abs() <= crate::world::SHRINE_REACH as f64 && !fight.hero.dead && Self::quiet(fight, walked)
    }

    /// The meeting here, told: its words (only the last again, once heard), kept as heard.
    pub fn meet_visitor(&mut self) -> Option<(&'static str, Vec<String>)> {
        let v = self.visit()?;
        let heard = self.meetings.contains(&v.id);
        if !heard { self.meetings.push(v.id); }
        self.lingering = Some(v.id);
        let pages = if heard { &v.pages[v.pages.len().saturating_sub(1)..] } else { v.pages };
        Some((v.who.name(), pages.iter().map(|p| (*p).to_string()).collect()))
    }

    /// Whether Krix sits in the guild hall: once its Collector is down, until he goes out with the
    /// Blackridge bounty.
    fn krix_drinks(&self) -> bool { self.beaten.contains(&HALL) && !self.beaten.contains(&"chief_hold") && !self.hunters.settled }

    fn mother_down(&self) -> bool { self.beaten.contains(&BLACK) }

    /// Whether Mara's whistle was taken from among the Mother's dead.
    fn has_whistle(&self) -> bool { self.taken.contains(&BLACK) }

    pub(crate) fn met(&self, id: &str) -> bool { self.meetings.contains(&id) }

    pub(crate) fn carries_rune(&self) -> bool { self.keys.contains(&crate::atlas::Key::Rune) }

    /// Who found the rune-marked blade in the Drowned Fingers: the hero (true), the shaken hunter
    /// (false), or no one yet.
    pub fn blade_found_by(&self) -> Option<bool> {
        self.taken.contains(&FINGERS).then(|| !self.claimed.contains(&FINGERS))
    }

    /// Whether Olekeen waits at Stonehollow's gate: in from Blackridge, before its court falls,
    /// and not yet met there.
    fn olekeen_at_stone(&self) -> bool {
        self.beaten.contains(&CHIEF) && !self.beaten.contains(&COURT) && !AT_STONE[1..].iter().any(|id| self.met(id))
    }

    /// Whether Olekeen has been out to the east gate's beast, his kill or the hero's.
    fn olekeen_went_out(&self) -> bool { self.claimed.contains(&GATE) || (self.beaten.contains(&GATE) && self.guild.paid > 0) }

    /// What the one met in `visit` hands over, the first time: the piece, told.
    pub fn gift_from(&mut self, visit: &str) -> Vec<Note> {
        if let Some(paid) = self.registrar_pays(visit) { return paid; }
        let Some(&(_, name)) = GIFTS.iter().find(|(id, _)| *id == visit) else { return Vec::new() };
        let i = crate::gear::find(name);
        if self.gear.gain(i) { vec![Note::Found(crate::gear::ITEMS[i].name)] } else { Vec::new() }
    }

    /// The story moving on without the hero, once a beat has passed: with the Hollowmere bounty
    /// paid, Olekeen takes the east gate's beast, unless the hero took it first; with the Blackridge
    /// chief down, his crew empty their hole under the ridge, unless the hero emptied it first.
    pub fn story_moves(&mut self) -> Vec<Note> {
        if self.guild.paid > 0 && !self.beaten.contains(&GATE) && !self.claimed.contains(&GATE) {
            self.claimed.push(GATE);
        }
        if self.guild.paid > 0 && !self.taken.contains(&FINGERS) {
            self.taken.push(FINGERS);
            self.claimed.push(FINGERS);
        }
        if self.beaten.contains(&CHIEF) && !self.taken.contains(&CAVE) {
            self.taken.push(CAVE);
            self.claimed.push(CAVE);
        }
        self.hunters_move_on();
        Vec::new()
    }

    /// Whether the crew's stash under the ridge is gone, and whether the hero took it (true) or the
    /// crew did (false); None while it is still there.
    pub fn stash_taken_by_hero(&self) -> Option<bool> {
        self.taken.contains(&CAVE).then(|| !self.claimed.contains(&CAVE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_gate(w: &mut World) { w.arrive("east_gate@warrens_lane", 100.0, 0.0, 0); }

    #[test]
    fn in_the_storys_order_olekeen_has_the_gates_beast_and_says_so_once() {
        let mut w = World::new(0.0);
        at_gate(&mut w);
        assert!(w.visit().is_none() && !w.boss_beaten(), "nothing yet: the beast waits for whoever comes");
        w.guild.paid = 1;
        w.story_moves();
        assert!(w.boss_beaten(), "Olekeen took it");
        let (who, pages) = w.meet_visitor().expect("Olekeen stands over it");
        assert_eq!(who, "Olekeen");
        assert!(pages[1].contains("Olekeen kill"));
        assert!(w.visit().is_some() && w.meet_visitor().is_some_and(|(_, p)| p.len() == 1), "he stays while the hero does, with a word");
        w.arrive("warrens_lane@east_gate", 200.0, 0.0, 0);
        w.arrive("east_gate@warrens_lane", 300.0, 0.0, 0);
        assert!(w.visit().is_some_and(|v| v.who == Visitor::Skif), "told once, and he has moved on, leaving Skif over the husk");
        assert_eq!(World::restored(0.0, &w.saved()).map(|b| (b.claimed, b.meetings)), Some((vec![GATE, FINGERS], vec!["olekeen_gate_his_kill"])), "the shaken hunter found the blade at the same beat");
    }

    fn krix_says(w: &mut World) -> Vec<String> {
        w.arrive("guild_hall@warrens", 100.0, 0.0, 0);
        let (who, pages) = w.meet_visitor().expect("Krix is at the guild");
        assert_eq!(who, "Krix");
        w.arrive("warrens@guild_hall", 200.0, 0.0, 0);
        pages
    }

    #[test]
    fn krix_tells_of_the_black_water_and_answers_what_was_brought_back() {
        let mut w = World::new(0.0);
        w.arrive("guild_hall@warrens", 100.0, 0.0, 0);
        assert!(w.visit().is_none(), "the hall is the Collector's until he is down");
        w.beaten.push(HALL);
        assert!(krix_says(&mut w).iter().any(|p| p.contains("Jek went under")));
        w.arrive("guild_hall@warrens", 300.0, 0.0, 0);
        assert!(w.visit().is_none(), "told once");
        w.beaten.push(BLACK);
        assert!(krix_says(&mut w).iter().any(|p| p.contains("can't go back for it")));
        w.taken.push(BLACK);
        assert!(krix_says(&mut w).iter().any(|p| p.contains("went back for it")));
        w.arrive("guild_hall@warrens", 400.0, 0.0, 0);
        assert!(w.visit().is_none(), "nothing more to say");
    }

    #[test]
    fn a_hero_who_found_the_black_water_first_did_what_krix_could_not() {
        let mut w = World::new(0.0);
        w.beaten.extend([HALL, BLACK]);
        w.taken.push(BLACK);
        let pages = krix_says(&mut w);
        assert!(pages.iter().any(|p| p.contains("That's Mara's")) && pages.iter().any(|p| p.contains("Should've been me")), "{pages:?}");
        w.beaten.push("chief_hold");
        w.arrive("guild_hall@warrens", 300.0, 0.0, 0);
        assert!(w.visit().is_none(), "gone with the Blackridge bounty");
    }

    #[test]
    fn olekeen_waits_at_ridge_camp_for_dark_or_finds_the_chief_already_down() {
        let mut w = World::new(0.0);
        w.arrive("ridge_camp@pine_road", 100.0, 0.0, 0);
        assert!(w.visit().is_none(), "not before the Blackridge bounty is up");
        w.guild.posted = 2;
        assert!(w.meet_visitor().is_some_and(|(_, p)| p[1].contains("wait for dark")));
        w.beaten.push(CHIEF);
        w.arrive("pine_road@ridge_camp", 200.0, 0.0, 0);
        w.arrive("ridge_camp@pine_road", 300.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p[1].contains("not wait for nothing")));
        let mut early = World::new(0.0);
        early.beaten.push(CHIEF);
        early.guild.posted = 2;
        early.arrive("ridge_camp@pine_road", 100.0, 0.0, 0);
        assert!(early.meet_visitor().is_some_and(|(_, p)| p[1].contains("now you first to chief")));
    }

    #[test]
    fn the_crews_stash_is_the_heros_only_before_their_chief_falls() {
        let mut w = World::new(0.0);
        assert_eq!(w.stash_taken_by_hero(), None);
        w.taken.push(CAVE);
        w.beaten.push(CHIEF);
        w.story_moves();
        assert_eq!(w.stash_taken_by_hero(), Some(true), "the hero came first");
        let mut late = World::new(0.0);
        late.beaten.push(CHIEF);
        late.story_moves();
        assert_eq!(late.stash_taken_by_hero(), Some(false), "the crew ran off with it");
        late.arrive("bandit_cave", 100.0, 0.0, 0);
        assert!(late.cache_taken() && late.cache_at().is_some(), "the chest stands open and bare");
    }

    #[test]
    fn krixs_beast_waits_whatever_order_the_cage_krix_and_the_chief_come_in() {
        for hero_takes_the_cage in [true, false] {
            for krix_tells_first in [true, false] {
                let mut w = World::new(0.0);
                w.beaten.push(HALL);
                if krix_tells_first { krix_says(&mut w); }
                if hero_takes_the_cage { w.beaten.push(GATE); }
                w.guild.paid = 1;
                w.story_moves();
                if !krix_tells_first { krix_says(&mut w); }
                w.beaten.push(CHIEF);
                w.story_moves();
                at_gate(&mut w);
                assert!(w.boss_beaten(), "the gate is open, the hero's kill or Olekeen's");
                w.arrive("black_water@east_gate", 200.0, 0.0, 0);
                assert!(!w.boss_beaten() && !w.cache_taken(), "cage by hero {hero_takes_the_cage}, Krix first {krix_tells_first}: the Mother and her dead are still there");
            }
        }
    }

    #[test]
    fn skif_guards_the_husk_and_grown_gives_his_knife_once() {
        let mut w = World::new(0.0);
        w.guild.paid = 1;
        w.story_moves();
        at_gate(&mut w);
        assert!(w.visit().is_some_and(|v| v.who == Visitor::Olekeen), "Olekeen first");
        w.meet_visitor();
        w.arrive("warrens_lane@east_gate", 200.0, 0.0, 0);
        at_gate(&mut w);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p[0].contains("No touch")));
        w.beaten.push(COURT);
        w.arrive("warrens_lane@east_gate", 300.0, 0.0, 0);
        at_gate(&mut w);
        assert_eq!(w.visit().map(|v| (v.id, v.who)), Some(("skif_watch", Visitor::SkifGrown)));
        let knife = crate::gear::find("Skif's Rusted Knife");
        assert_eq!(w.gift_from("skif_watch"), vec![Note::Found("Skif's Rusted Knife")]);
        assert!(w.gear.owns(knife) && w.gift_from("skif_watch").is_empty(), "given once");
    }

    #[test]
    fn rik_hears_the_well_and_after_blackridge_it_calls_for_krix_or_whistles() {
        let mut w = World::new(0.0);
        let at_mere = |w: &mut World, t: f64| { w.arrive("gate@hollowmere", t, 0.0, 0); w.arrive("hollowmere@gate", t + 1.0, 0.0, 0); };
        at_mere(&mut w, 100.0);
        assert!(w.visit().is_none(), "not while the beast lives");
        w.beaten.push(MERE);
        at_mere(&mut w, 200.0);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p[1].contains("in the wind")));
        w.beaten.push(CHIEF);
        at_mere(&mut w, 300.0);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p.iter().any(|l| l.contains("Krix, it's so cold"))));
        w.beaten.push(BLACK);
        at_mere(&mut w, 400.0);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p.iter().any(|l| l.contains("You went to where"))));
        let mut early = World::new(0.0);
        early.beaten.extend([MERE, CHIEF, BLACK]);
        at_mere(&mut early, 100.0);
        assert!(early.meet_visitor().is_some_and(|(_, p)| p[1].contains("whistled. Twice")), "the hero was there first");
    }

    #[test]
    fn in_the_fog_a_fall_costs_lurz_and_krix_comes_home_alone_with_his_axe() {
        let mut w = World::new(0.0);
        w.beaten.push(CHIEF);
        w.arrive("fog_hollow@bandit_hollows", 100.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(who, p)| who == "Lurz" && p[0].contains("still there")));
        w.died(110.0);
        w.arrive("fog_hollow@bandit_hollows", 200.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(who, p)| who == "Krix" && p[0].contains("It took Lurz")));
        w.boss_felled();
        w.arrive("bandit_hollows@fog_hollow", 300.0, 0.0, 0);
        w.arrive("fog_hollow@bandit_hollows", 400.0, 0.0, 0);
        assert!(w.meet_visitor().is_some_and(|(_, p)| p.iter().any(|l| l.contains("You'll hear him too"))));
        w.arrive("guild_hall@warrens", 500.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("krix_home_alone"));
        // The axe is the hero's to take, or to leave with him (`summons`).
        assert!(w.gift_from("krix_home_alone").is_empty() && w.asking("krix_home_alone").is_some());
        assert_eq!(w.answer("krix_gives_axe"), vec![Note::Found("Krix's Notched Axe")]);
        w.arrive("stone_market@lower_stairs", 600.0, 0.0, 0);
        assert!(w.visit().is_none(), "Lurz is gone");
    }

    #[test]
    fn saved_from_the_fog_both_come_home_with_their_weapons() {
        let mut w = World::new(0.0);
        w.beaten.push(CHIEF);
        w.arrive("fog_hollow@bandit_hollows", 100.0, 0.0, 0);
        w.meet_visitor();
        w.boss_felled();
        assert!(w.meet_visitor().is_some_and(|(_, p)| p[1].contains("Lurz, get up")), "Lurz stood aside, and Krix speaks once it falls");
        w.arrive("stone_market@lower_stairs", 200.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| (v.id, v.who)), Some(("lurz_home", Visitor::Lurz)));
        assert_eq!(w.gift_from("lurz_home"), vec![Note::Found("Lurz's Spear")]);
        w.arrive("guild_hall@warrens", 300.0, 0.0, 0);
        assert_eq!(w.visit().map(|v| v.id), Some("krix_home"));
    }

    #[test]
    fn the_rune_marked_blade_is_the_heros_only_before_hollowmere_is_paid_and_tusk_and_vren_answer_it() {
        let rune = |w: &mut World| w.keys.push(crate::atlas::Key::Rune);
        let mut late = World::new(0.0);
        rune(&mut late);
        late.guild.paid = 1;
        late.story_moves();
        assert_eq!(late.blade_found_by(), Some(false));
        late.arrive("warrens@warrens_lane", 100.0, 0.0, 0);
        assert_eq!(late.visit().map(|v| v.id), Some("tusk_sold_it"));
        late.arrive("broken_spire", 200.0, 0.0, 0);
        assert!(late.meet_visitor().is_some_and(|(who, p)| who == "Vren" && p[2].contains("a call")));
        let mut early = World::new(0.0);
        early.arrive("drowned_fingers@vale_edge", 100.0, 0.0, 0);
        assert!(early.open_cache().contains(&Note::Found("Rune-Marked Blade")));
        rune(&mut early);
        early.guild.paid = 1;
        early.story_moves();
        assert_eq!(early.blade_found_by(), Some(true), "the hunter came back with nothing");
        early.arrive("broken_spire", 200.0, 0.0, 0);
        assert_eq!(early.visit().map(|v| v.id), Some("vren_asks_blade"));
        assert_eq!(early.gift_from("vren_asks_blade"), vec![Note::Found("Vren's Chipped Ring")]);
    }

    #[test]
    fn a_hero_who_went_out_the_gate_first_beat_olekeen_to_his_bounty() {
        let mut w = World::new(0.0);
        at_gate(&mut w);
        w.beaten.push(GATE);
        assert!(w.visit().is_none(), "he has not heard of it yet");
        w.guild.paid = 1;
        w.story_moves();
        assert!(!w.claimed.contains(&GATE));
        let (_, pages) = w.meet_visitor().expect("Olekeen comes for it");
        assert!(pages.iter().any(|p| p.contains("You kill")), "{pages:?}");
    }
}
