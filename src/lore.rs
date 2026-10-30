//! What things are, as the hero comes to read them: a few lines on every piece, remnant, key and
//! shard, in the way the Souls games tell their world through what is carried. Each is true to the
//! book (`docs/story.md`) or one of the threads it marks new; none names what the road has not yet
//! shown. The lines are the shrine's to show (`shrine::Sanctum`), under the piece picked.

use crate::world::World;

/// The lines on one thing, by its name.
pub struct Lore { pub name: &'static str, pub text: &'static str }

impl Lore {
    const fn new(name: &'static str, text: &'static str) -> Lore { Lore { name, text } }

    pub const ALL: &'static [Lore] = &[
        // What the hero woke with.
        Self::new("Wayfarer's Sword", "A plain sword, nicked along both edges. You woke with it in your hand and do not remember buying it. The grip is worn to someone else's fingers."),
        Self::new("Pilgrim's Buckler", "A small round shield, of the kind once sold to pilgrims bound for the Spire. Hardly anyone makes that walk any more. Hardly anyone ever came back from it."),
        Self::new("Wayfarer's Hood", "A traveller's hood, stiff with old rain. Inside the brim a name was stitched once, and unpicked since."),
        Self::new("Wayfarer's Cloak", "A traveller's cloak, patched more often than worn. It smells of a road you have no memory of walking."),
        Self::new("Leather Wraps", "Strips of leather wound about the hands. Under them are faint marks like old burns, in shapes too regular to be accidents."),
        Self::new("Road Boots", "Boots for the long road, resoled more times than they were made for. The road wears out boots and walkers at much the same pace."),
        // Off the dead.
        Self::new("Husk Cleaver", "A butcher's cleaver, gripped by a husk long after the hand forgot why. The hollow keep what they held. They no longer know what it was for."),
        Self::new("Lancer's Shortspear", "A short spear of the Warrens watch, who once held the east gate. The watch is mostly hollow now. They hold the gate still, against everyone."),
        Self::new("Kite Shield", "A tall shield once painted with a sun behind a veil. The paint was scraped off in a hurry, but the shape of it stays in the wood."),
        Self::new("Warden's Bulwark", "A heavy shield of Hollowmere's bell wardens, who kept the bells and the gate. When the bells fell silent, the wardens kept the gate against their own."),
        Self::new("Duelist's Parma", "A light parrying shield from Stonehollow, a city where every quarrel ends at the point of a blade. The city is no quieter for it."),
        Self::new("Iron Helm", "A dented iron helm with the visor torn away. Tusk sells a dozen like it off his cart in the Warrens, and never says where they come from."),
        Self::new("Chain Hauberk", "A shirt of riveted rings, rusted at the hem. Its last owner walked into the mist wearing it, and walked out of it still wearing it."),
        Self::new("Shieldbearer's Gauntlets", "Gauntlets of the shieldbearers, who still turn their shields toward a raised voice. They are waiting for an order no one is left to give."),
        Self::new("Lancer's Greaves", "Greaves of the gate watch. The lancers swore to stand their ground, and the hollow ones stand it yet, wherever it was they fell."),
        Self::new("Ember Ring", "A ring set with a coal that will not go out. It is warm against the skin, the way a shrine fire is warm."),
        Self::new("Ring of the Quiet Step", "A thief's ring from the Warrens, worn smooth. It is said a fox-folk archer once ran those rooftops and left nothing behind him, not even sound."),
        Self::new("Vigil Ring", "Worn by those who sat the long watch by the shrine fires. Something must stay awake, they said, while the dead walk."),
        Self::new("Hoarder's Ring", "Taken from a hand that would not open in death. Echoes cling to whatever was greedy in life."),
        Self::new("Pilgrim's Token", "A clay token stamped with the Spire, given to pilgrims as they set out. Scratched on the back, later and by another hand: do not drink from the wells."),
        Self::new("Keeper's Sigil", "A sigil of the shrine keepers, cut with marks like those on the old stones. The keepers will not say why their fires burn where the stones once stood."),
        // From the bosses and the chests behind them.
        Self::new("Warden's Bell", "The hand-bell the Bell Warden rang to call Hollowmere home at dusk. It still rings faintly, on its own, whenever something stirs in the well."),
        Self::new("Vren's Loupe", "A jeweller's glass in a brass ring, the lens scratched by a hundred appraisals. Through it every relic looks smaller and dearer at once. Vren never said who he took it from."),
        Self::new("Order Agent's Beaked Mask", "A leather beak packed with herbs and charcoal, brass-riveted, round glass at the eyes. Brannoc made them for an Order that paid well and said little: beaks, to keep the gas out."),
Self::new("The Collector's Tally", "The Debt Collector's tally-stick, notched for every debt he settled. Twenty winters of notches, and the last of them cut by a hand that was no longer his."),
        Self::new("Duelist's Rapier", "A thrusting sword from Stonehollow, where a duel is a trade like any other. Its last owner won every bout but one."),
        Self::new("Rimed Sabre", "A sabre that frosts at the edge whatever the weather. Its grip is wound in grey cloth, the kind once worn by an order of rune-wardens no one in the Warrens remembers."),
        Self::new("Executioner's Greatsword", "The blade of Stonehollow's headsman, who served the Magistrate's court. When the court fell silent he kept working, and the queue never grew shorter."),
        // Forged from what the region bosses leave.
        Self::new("Bell-Hammer", "Forged from the clapper of Hollowmere's great bell. It rang all one night to warn the village of the mist, and by morning there were few left to warn."),
        Self::new("Drowned Wail", "Forged from what was left of the voice in Hollowmere's well. Struck, the blade whispers, always in the voice of whoever last died near it."),
        Self::new("Chieftain's Cleaver", "The Blackridge chief's own axe, a notch in its head for every caravan. He took the ridge with it, and held it against everything but the mist."),
        Self::new("Gavel of Sentence", "Forged from the Hollow Magistrate's seal. He sat in judgement long after he died, over a court of the dead, and every verdict he passed was the same."),
        Self::new("Reaper's Glaive", "Forged from the hollow crown of the Wraith Lord, who ruled what the Shrine's dark gave him. A wraith takes only what it is owed, or so the wraiths say."),
        Self::new("Lance of Light", "Forged from a shard of the Angel Priest's halo. He swore to guard what lay beneath his cathedral, always. He kept his word longer than his flesh did."),
        Self::new("Stoneheart Greatsword", "Forged from the heart of Eryndor's angel statue, cut free knot by knot. Stone keeps an oath better than flesh. It did not keep it better than the Taint."),
        Self::new("Veil-Piercer", "Forged around the red lens of a plague-doctor's mask. Its point finds the gap in anything, the way its owner found the gap in everyone."),
        Self::new("Exodus's Red Lens", "The red glass from Exodus Lightfoot's beaked mask. Through it the world looks the way the Order sees it: one colour, and all of it theirs."),
        Self::new("Feather of the Fallen Light", "A feather of the Archangel of the Spire, white once. Held up to the light it shows, faintly, the shape of a star that is not in our sky."),
        Self::new("Gravewarden's Slab", "A slab of a sword no one living could lift, kept by the Rune Sentinel in the sealed vault. What it was made to fight is not written anywhere the living can read."),
        Self::new("Keening Antler", "A spear of the Keening Cage's own antler, barbed with a black tendril-tip, forged for someone who went out the east gate before anyone sent them. Swung, it keens faintly. Near the rune, both flare."),
        Self::new("Caravan Signet", "A heavy signet from the Blackridge crew's take, cut off a caravan master with the finger still in it. Its seal is a sun behind a veil. The caravans the crew hit were not carrying grain."),
        Self::new("Richald's Signet", "Richald Steel's ring, from the hand he had before the armour. Its seal is a closed eye. It is cold to wear, and he says he missed the cold."),
        Self::new("Bounty's Longbow", "A fox-folk longbow of yew gone dark in Eryndor's damp, a fox's head burned into the grip. Its note said whoever finds it should take it. It draws long, and flies flat."),
        Self::new("Starglass Core", "The heart of the construct on the Starlit Wastes, starglass round a cold white light. On its face, worn almost smooth, a sun behind a veil. It let the masked ones by."),
        Self::new("The Drake's Warped Fang", "A fang off the drake that nests where the sky broke, slate gone violet along every crack. Whatever came through up there did not kill it. It changed it."),
        Self::new("Starglass Maul", "The construct's core, set by the smith in a maul's head. Struck, it rings like a bell under water, and for a moment the light in it remembers a road kept for pilgrims."),
        Self::new("Warped Fang", "The drake's fang on a hunter's shaft. It still remembers the broken sky, and drinks from whatever it opens."),
        Self::new("Yellow-Glass Bead", "A bead of cheap yellow glass on a frayed cord, the kind Warrens children trade. It was found where the wood is darkest, far from any child."),
        Self::new("Maker's Chisel", "A carver's chisel left by an empty plinth, its handle worn to one old hand's grip. A smith's mark is stamped in the steel. The Order kept the carving, not the carver."),
        Self::new("Gravedigger's Ring", "An iron band off a gravedigger's finger, left in a chest among the graves. The chest was bait. So, likely, was he."),
        Self::new("Skif's Rusted Knife", "A youngling's knife, more rust than edge, its grip worn smooth by two hands clutching it through every night of a long watch at the east gate. No touch, no take, he was told. He gave it anyway."),
        Self::new("Fin-S's Carved Hunter", "A goblin no taller than a thumb, cut from pale wood: big ears, a bow, a scowl. Fin-S carved it while you slept. It is warm, the way the rune is warm."),
        Self::new("Fin-S's Carved Listener", "A small carved goblin with one ear gone, head tipped as if listening for something far off. Fin-S says you hum his name in your sleep."),
        Self::new("Varnholt Straw Charm", "Straw from a barn that did not burn, plaited by hands that shook. It is all they had, and they gave it to whoever stood in the road."),
        Self::new("Krix's Notched Axe", "A wood-axe, its haft dark with handling, its edge notched once for every hunter its owner outlived. Jek. Mara. Four more from the black water. He stopped cutting them when he gave it away."),
        Self::new("Lurz's Spear", "A goblin hunter's spear, the shaft spliced where a tendril once splintered it. It always found the glowing crack in a thing. Its owner swore he would not run twice, and then that he would not walk back in."),
        Self::new("Ripper's Tendril", "A razor tendril of the thing in the fog, forged stiff around a hunter's grip. Krix had a word for them. It still twitches toward whatever breathes nearest."),
        Self::new("The Horror's Eye-Crack", "A sliver of the fog hollow's horror where one of its eyes split open. It stays violet, and it stays open. The smith can make an edge of what it was."),
        Self::new("Rune-Marked Blade", "An old blade of pitted bronze-iron out of the Drowned Fingers, its length cut with the same jagged marks as the rune. Not a ward, not a blessing, a scholar would say. A call. It has been answering something ever since."),
        Self::new("Vren's Chipped Ring", "A ring of dull green stone, chipped at the bezel, given by a scholar who does not trade straight. It draws echoes to its wearer, and the blows that come with them."),
        Self::new("Mara's Whistle", "A hunter's whistle of bone on a frayed cord, found among the old stones where the black water keeps its dead. Blown twice, it meant the trail was found. Someone at the guild still hears it at night."),
        Self::new("Drowner's Hook", "A hook-blade forged from one of the Mother's grey fingers, a hunter's rope still knotted at the knuckle. What it catches it draws back, as the black water did."),
        // In the other hand.
        Self::new("Hunter's Shortbow", "A hunter's bow, short enough to draw on a rooftop. Burned into the grip is a fox's head: the mark a Warrens archer named Bounty left on everything he gave away."),
        Self::new("Bloodrunner's Bow", "The bow of the bloodrunners, who learned to pour their own life into the arrow. It takes its price in blood, and sometimes gives back more than it took."),
        Self::new("Frostweaver's Focus", "A focus of rimed glass. The frostweavers' order kept the living safe with rune and frost, until the Taint got into them. Their order is ended. The frostweavers do not say who ended it."),
        Self::new("Blood Crossbow", "A heavy crossbow off the Blackridge ledge, its stock stained dark. The bloodrunners took to it when their arrows ran out and their blood did not."),
        Self::new("Frost Bolt", "A frostweaver's first working: cold that holds a wound shut and a foe still. It was taught for mercy, long before it was taught for war."),
        // Remnants, for the smith.
        Self::new("Bell Warden's Clapper", "The clapper of Hollowmere's great bell, wrenched free when its Warden fell. Bell-metal remembers every note it rang. The smith can make something of it."),
        Self::new("Screamer's Drowned Voice", "What is left of the thing that came up Hollowmere's well: a knot of wet flesh that still murmurs, in more voices than one. The smith can forge it, if he can bear to touch it."),
        Self::new("Chief's Notched Axe-Head", "The head of the Blackridge chief's axe, notched to the socket. Ten silver a head, twenty for the chief. The smith can forge it."),
        Self::new("Magistrate's Seal", "The seal of Stonehollow's court. The Magistrate stamped every judgement with it, the last of them in a hand that had stopped bleeding. The smith can forge it."),
        Self::new("Wraith Lord's Hollow Crown", "A crown with nothing inside it, worn by what ruled the Hollowed Shrine's dark. The smith can forge it."),
        Self::new("Shard of the Priest's Halo", "A shard of the Varnholt priest's halo, still warm. The priest's light outlasted the priest. The smith can forge it."),
        Self::new("The Statue's Stone Heart", "The heart of Eryndor's angel statue, its last knot cut away. It still beats, slowly, once in an hour. The smith can forge it."),
        Self::new("A Feather of the Archangel", "A feather of the Archangel of the Spire. The smith can forge it into something lighter than any blade should be."),
        Self::new("Violet-Veined Bone", "A shard of the Keening Cage's antler, white and hard, veined with violet like its ribs. Held near the rune, both flare, as if they talk to each other. The smith can make something of it."),
        Self::new("The Mother's Hooked Finger", "One of the long grey fingers of the thing in the black water, hooked like a gaff. Old rope is wound around it, cut from a hunter's net. The smith can make it pull the other way."),
        // What opens roads.
        Self::new("Hollowmere Rune", "A jagged stone pried from Hollowmere's well, cut with old runes. The old woman says such stones sang once, before the mists came. Now they only watch. This one hums, and only for you."),
        Self::new("Climbing Hook", "The Blackridge chief's climbing hook. His crew came and went by the cliffs, and Stonehollow's upper city never once saw them coming."),
        Self::new("Pale Lantern", "A lantern-seller's last lamp, left burning on the stall. Its light is pale and steady, and the dark under the city gives way to it."),
        Self::new("Veil Sigil", "The Wraith Lord's sigil. Varnholt's violet ward knows it and lets it pass, the way a dog knows its master's hand, whoever's hand it is in."),
        Self::new("Heart's Due", "What the Hollowed Shrine's carved heart took to open: blood, warm, freely given. It stays open after, the way a wound remembers the blade."),
        Self::new("First Shard of the Eye", "A piece of the Eye that Sees Beyond, cold as death. Stars turn inside it that are not ours. It pulls, faintly, toward the others."),
        Self::new("Second Shard of the Eye", "A piece of the Eye that Sees Beyond, taken from a guardian who kept it too long. Near the first piece, both grow colder."),
        Self::new("Third Shard of the Eye", "The last piece of the Eye that Sees Beyond. With the others near, it shows the Spire in its depths, and something at the top looking back."),
        Self::new(crate::vials::SHRINE_GLASS, "Clouded glass, the kind the shrine-keepers blow their flasks from. Rest at a shrine and it is melted into one more flask: the road is longer than it was, and so is the drink."),
        Self::new("Ash Shard", "Grey ore broken off the armour of the hollow dead. The smith says the mist leaves something in the iron, and the iron is better for it. He does not say better at what."),
        Self::new("Ember Shard", "Ore that stays warm long after the dead it came off are cold. It comes only off those who walked deep into the Taint."),
        Self::new("Star Shard", "Ore flecked with a light like starlight, found only on the dead nearest the Spire. It hums, very faintly, the way the old stones hum."),
    ];

    pub fn of(name: &str) -> Option<&'static str> {
        // Every piece of shrine glass reads alike.
        let name = if crate::vials::Glass::named(name).is_some() { crate::vials::SHRINE_GLASS } else { name };
        Self::ALL.iter().find(|l| l.name == name).map(|l| l.text)
    }

    /// What the hero carries that is not worn, for the shrine's keepsakes: each as listed (with
    /// how many, for shards) and the name its lines are under.
    pub fn keepsakes(w: &World) -> Vec<(String, &'static str)> {
        let keys = w.keys.iter().filter(|k| !matches!(k, crate::atlas::Key::Glass(_))).map(|k| (k.name().to_string(), k.name()));
        // The shrine glass, counted as one keepsake.
        let glass = Some(w.glass()).filter(|&n| n > 0).map(|n| (format!("{}\tx{n}", crate::vials::SHRINE_GLASS), crate::vials::SHRINE_GLASS));
        let remnants = w.remnants.iter().map(|&r| (r.to_string(), r));
        let shards = crate::gear::SHARDS.iter().zip(w.shards).filter(|&(_, n)| n > 0).map(|(&s, n)| (format!("{s}\tx{n}"), s));
        keys.chain(glass).chain(remnants).chain(shards).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_piece_remnant_key_and_shard_has_its_lines() {
        let named = crate::gear::ITEMS.iter().map(|i| i.name)
            .chain(crate::gear::RECIPES.iter().map(|r| r.remnant))
            .chain(crate::atlas::Key::ALL.iter().map(|k| k.name()))
            .chain(crate::gear::SHARDS);
        for name in named { assert!(Lore::of(name).is_some(), "no lines for {name}"); }
        for l in Lore::ALL {
            assert!(l.text.len() < 240, "{} runs long: {}", l.name, l.text.len());
            assert_eq!(Lore::ALL.iter().filter(|o| o.name == l.name).count(), 1, "{} twice", l.name);
        }
    }

    #[test]
    fn keepsakes_list_keys_remnants_and_the_shards_carried() {
        let mut w = World::new(0.0);
        assert!(Lore::keepsakes(&w).is_empty());
        w.keys.push(crate::atlas::Key::Hook);
        w.remnants.push("Magistrate's Seal");
        w.shards = [3, 0, 1];
        let k = Lore::keepsakes(&w);
        assert_eq!(k.iter().map(|e| e.1).collect::<Vec<_>>(), vec!["Climbing Hook", "Magistrate's Seal", "Ash Shard", "Star Shard"]);
        assert_eq!(k[2].0, "Ash Shard\tx3");
    }
}
