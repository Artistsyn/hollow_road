//! The world map: eight regions of places (PathForge scenes) joined by roads, walked either way.
//!
//! A road between two places is an [`Edge`]; most are open both ways, some need a key item or
//! ability (`Gate::Keys`), and each region has a shortcut that opens only from its far side
//! (`Gate::Shortcut`), the soulslike loop back to a shrine already found. A place has at most three
//! roads, so walking in by one leaves at most two to choose between: the road goes on, or forks.
//!
//! PathForge walks a journey of stops, and a stop walks one way, so every place is a stop for each
//! road it can be walked in by: `<place>@<from>` (`stop`). Its `next` is what lies ahead walking
//! that way: the other road (`Go`), the other two (`Fork`, left and right as the map lies), or,
//! at a dead end's end, nothing (`End`: the hero turns round, into `<place>@^`). The journey is compiled from here
//! (`journey`, in a test that writes `assets/scenes/world.journey.json`); the game's rules about
//! which roads are open live in `World`.

use crate::fight::Archetype;
use Archetype::{Archer, Bloater, Brute, Gravecaller, Hexer, Hound, Husk, Lancer, Shieldbearer, Wretch};

/// A region: its name, and how far into the world it lies (its enemies grow with it, and a
/// hero who arrives far under it is warned).
pub struct Region { pub name: &'static str, pub tier: u32 }

pub const REGIONS: [Region; 9] = [
    Region { name: "The Ashen Warrens", tier: 0 },
    Region { name: "Wretched Vale and Hollowmere", tier: 1 },
    Region { name: "Blackridge", tier: 2 },
    Region { name: "Stonehollow", tier: 3 },
    Region { name: "The Hollowed Shrine", tier: 4 },
    Region { name: "Varnholt", tier: 5 },
    Region { name: "Eryndor and the Blackwood", tier: 6 },
    Region { name: "The Celestial Spire", tier: 7 },
    Region { name: "The Order's Stronghold", tier: 8 },
];

/// What opens a locked road: things found or won that stay with the hero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// The rune pried from Hollowmere's well, the Screamer's: nothing is barred to it, but it hums
    /// at a fork toward the way the story goes on (`World::rune_hums`).
    Rune,
    /// The Bandit Chief's climbing hook: up the cliff path into Stonehollow's upper city.
    Hook,
    /// The pale lantern from the Lantern Market: light for the Hollowed Shrine's dark.
    Lantern,
    /// The Wraith Lord's sigil: passage through Varnholt's violet ward.
    Sigil,
    /// The three shards of the Eye, from their guardians: the Angel Priest, the Statue Archangel
    /// and the Archangel. The first two open the Spire, as in the book.
    ShardI,
    ShardII,
    ShardIII,
    /// The Hollowed Shrine's heart mural, paid in blood (`hollowed`): the way to the sanctum.
    Heart,
    /// A piece of shrine glass (`vials::GLASS`, by index): one more flask at the next rest.
    Glass(u8),
}

impl Key {
    pub const ALL: [Key; 20] = [Key::Rune, Key::Hook, Key::Lantern, Key::Sigil, Key::ShardI, Key::ShardII, Key::ShardIII, Key::Heart,
        Key::Glass(0), Key::Glass(1), Key::Glass(2), Key::Glass(3), Key::Glass(4), Key::Glass(5), Key::Glass(6), Key::Glass(7),
        Key::Glass(8), Key::Glass(9), Key::Glass(10), Key::Glass(11)];
    pub fn name(self) -> &'static str {
        match self {
            Key::Rune => "Hollowmere Rune", Key::Hook => "Climbing Hook", Key::Lantern => "Pale Lantern", Key::Sigil => "Veil Sigil",
            Key::ShardI => "First Shard of the Eye", Key::ShardII => "Second Shard of the Eye", Key::ShardIII => "Third Shard of the Eye",
            Key::Heart => "Heart's Due",
            Key::Glass(n) => crate::vials::GLASS[n as usize].name,
        }
    }
}

/// Who bars a place until beaten: a region's boss, or a mini-boss guarding a way or a prize.
pub struct Boss {
    pub breed: &'static crate::side::Breed,
    pub souls: u32,
    /// What it leaves that opens roads.
    pub key: Option<Key>,
    /// A mini-boss (a named, stronger road enemy) rather than a region's boss.
    pub mini: bool,
    /// The remnant it leaves, which the smith forges into a weapon of its own (`gear::RECIPES`).
    pub remnant: Option<&'static str>,
    /// Whether the rune hums toward it: the story's hunts, not a beast no one is sent after.
    pub sought: bool,
}

impl Boss {
    pub fn name(&self) -> &'static str { self.breed.name }
}

use crate::order::{CRIMSON_CAPTAIN, EXODUS_LIGHTFOOT};
use crate::side::{ALPHA, AGENT, ARCHANGEL, BUTCHER, CHIEF, COLLECTOR, CONSTRUCT, DRAKE, KEENER, MAGISTRATE, MOTHER, PRIEST, HORROR, SCREAMER, SENTINEL, STALKER, STATUE, WARDEN, WRAITH};

pub static BELL_WARDEN: Boss = Boss { breed: &WARDEN, souls: 1500, key: None, mini: false, remnant: Some("Bell Warden's Clapper"), sought: true };
pub static HOLLOWMERE_SCREAMER: Boss = Boss { breed: &SCREAMER, souls: 2200, key: Some(Key::Rune), mini: false, remnant: Some("Screamer's Drowned Voice"), sought: true };
pub static BANDIT_CHIEF: Boss = Boss { breed: &CHIEF, souls: 3000, key: Some(Key::Hook), mini: false, remnant: Some("Chief's Notched Axe-Head"), sought: true };
pub static HOLLOW_MAGISTRATE: Boss = Boss { breed: &MAGISTRATE, souls: 4200, key: None, mini: false, remnant: Some("Magistrate's Seal"), sought: true };
pub static WRAITH_LORD: Boss = Boss { breed: &WRAITH, souls: 5200, key: Some(Key::Sigil), mini: false, remnant: Some("Wraith Lord's Hollow Crown"), sought: true };
pub static ANGEL_PRIEST: Boss = Boss { breed: &PRIEST, souls: 6800, key: Some(Key::ShardI), mini: false, remnant: Some("Shard of the Priest's Halo"), sought: true };
pub static STATUE_ARCHANGEL: Boss = Boss { breed: &STATUE, souls: 8500, key: Some(Key::ShardII), mini: false, remnant: Some("The Statue's Stone Heart"), sought: true };
pub static THE_ARCHANGEL: Boss = Boss { breed: &ARCHANGEL, souls: 12000, key: Some(Key::ShardIII), mini: false, remnant: Some("A Feather of the Archangel"), sought: true };
/// Out past the Warrens' east gate from the start, and the story's only once the Hollowmere bounty is
/// paid: by then, unless the hero came first, Olekeen has killed it (`World::story_moves`).
pub static KEENING_CAGE: Boss = Boss { breed: &KEENER, souls: 1800, key: None, mini: false, remnant: Some("Violet-Veined Bone"), sought: true };
/// Past the east gate, in the black water: Krix's beast, which the book never kills and no one is
/// sent after. The hero who finds it finds what it kept.
pub static BLACK_WATER_MOTHER: Boss = Boss { breed: &MOTHER, souls: 2400, key: None, mini: false, remnant: Some("The Mother's Hooked Finger"), sought: false };
pub static FOG_HORROR: Boss = Boss { breed: &HORROR, souls: 2800, key: None, mini: false, remnant: Some("The Horror's Eye-Crack"), sought: false };
pub static DEBT_COLLECTOR: Boss = Boss { breed: &COLLECTOR, souls: 400, key: None, mini: true, remnant: None, sought: true };
pub static PACK_ALPHA: Boss = Boss { breed: &ALPHA, souls: 900, key: None, mini: true, remnant: None, sought: true };
pub static ORDER_AGENT: Boss = Boss { breed: &AGENT, souls: 1400, key: None, mini: true, remnant: None, sought: true };
pub static RUNE_SENTINEL: Boss = Boss { breed: &SENTINEL, souls: 2000, key: None, mini: true, remnant: None, sought: true };
pub static THE_BUTCHER: Boss = Boss { breed: &BUTCHER, souls: 2600, key: None, mini: true, remnant: None, sought: true };
pub static VOID_STALKER: Boss = Boss { breed: &STALKER, souls: 3200, key: None, mini: true, remnant: None, sought: true };
pub static RADIANT_CONSTRUCT: Boss = Boss { breed: &CONSTRUCT, souls: 9500, key: None, mini: false, remnant: Some("Starglass Core"), sought: true };
pub static WARPED_DRAKE: Boss = Boss { breed: &DRAKE, souls: 10500, key: None, mini: false, remnant: Some("The Drake's Warped Fang"), sought: true };

/// Foes that spring out on the hero: the camera swings round and they are fought side-on
/// (`side::SideFight::ambush`) until every one is down. Sprung `at` metres into the place, or, for
/// a `chest`, on reaching the place's cache, which then opens only once they are beaten. Each
/// stands `s` metres along the road from the hero (both sides: it is an ambush). Once beaten it
/// stays beaten, like the boss.
#[derive(Debug)]
pub struct Ambush { pub at: f32, pub foes: &'static [(Archetype, f32)], pub chest: bool }

/// One place: a stop of the journey for each road into it.
pub struct Place {
    pub id: &'static str,
    pub title: &'static str,
    /// Its region (`REGIONS`), and where it lies on the map (0..100 across, 0..60 down).
    pub region: usize,
    pub map: (f32, f32),
    /// The road whose walk in the place is laid out for: walked in from `from`, everything stands
    /// where it says; walked the other way, mirrored (`World::foes`). Empty for a side path.
    pub from: &'static str,
    /// Its scene, in `assets/scenes`.
    pub scene: &'static str,
    /// (kind, metres past where the hero arrives, lane).
    pub foes: &'static [(Archetype, f32, i32)],
    /// Metres walked from arriving before the road goes on (once every enemy is down).
    pub length: f32,
    /// A shrine this many metres in, if the place has one.
    pub shrine: Option<f32>,
    /// The wandering smith waits by the road this many metres in, until met.
    pub smith: Option<f32>,
    /// A boss (or mini-boss) bars the way this many metres in, once the place's other enemies are
    /// down: the camera swings round and it is fought side-on (`side`). The road goes on once it
    /// falls.
    pub boss: Option<(f32, &'static Boss)>,
    /// The stop its side paths lead to (the scene's `path.fork`), once per pilgrimage: the hero
    /// turns down one and walks to the end of it, and comes back to where they left the road.
    pub explore: Option<&'static str>,
    /// Something left at the end of the place this many metres in: a key item if `relic` names
    /// one, else the piece `loot` names if the hero lacks it, else a piece of gear the hero lacks,
    /// or souls.
    pub cache: Option<f32>,
    pub relic: Option<Key>,
    pub loot: Option<&'static str>,
    /// Ambushes waiting along it (`Ambush`).
    pub ambushes: &'static [Ambush],
    /// What must be beaten before its road goes on, walked the way it is laid out (walked back,
    /// the way is never held). Most places let the hero run by.
    pub hold: Hold,
}

impl Place {
    /// How many roads it is from `to`, first walked into from `from`, to the nearest place `want`
    /// picks (0 for `to` itself), never back through `from`; None if none can be reached.
    pub fn steps(from: &str, to: &'static str, want: impl Fn(&Place) -> bool) -> Option<usize> {
        let mut seen: Vec<&str> = vec![from, to];
        let mut ring = vec![to];
        for n in 0..PLACES.len() {
            if ring.iter().any(|id| place_of(id).is_some_and(&want)) { return Some(n); }
            ring = ring.iter().flat_map(|id| neighbours(id)).filter(|id| !seen.contains(id)).collect();
            ring.dedup();
            seen.extend(&ring);
            if ring.is_empty() { return None; }
        }
        None
    }
}

/// What holds a place's way on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hold {
    /// Nothing: an enemy left standing can be run past.
    Free,
    /// These of its `foes` (by index) must be down: a keeper of the way.
    Keepers(&'static [usize]),
    /// Every one of its foes must be down: a gauntlet.
    Clear,
}

const fn place(id: &'static str, title: &'static str, region: usize, map: (f32, f32), from: &'static str, scene: &'static str,
               length: f32, foes: &'static [(Archetype, f32, i32)]) -> Place {
    Place { id, title, region, map, from, scene, foes, length, shrine: None, smith: None, boss: None, explore: None, cache: None, relic: None, loot: None, ambushes: &[], hold: Hold::Free }
}

/// How the road between two places is walked across: straight on, a blend over some metres,
/// through a doorway-like set piece (PathForge's `threshold: Gate` with that marker), or through a
/// door in the first room's end wall (half its width and its height, metres).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Via { Walk, Blend(f32), Arch(&'static str), Door(f32, f32) }

/// What a road needs before it can be walked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gate {
    Open,
    /// Every one of these keys, either way.
    Keys(&'static [Key]),
    /// Walked from `a` to `b` it opens, and stays open both ways; from `b` it is shut until then
    /// (a door barred on the near side, a ladder to kick down).
    Shortcut,
}

/// A road between places `a` and `b`.
pub struct Edge { pub a: &'static str, pub b: &'static str, pub gate: Gate, pub via: Via }

const fn road(a: &'static str, b: &'static str) -> Edge { Edge { a, b, gate: Gate::Open, via: Via::Walk } }

/// The places. The first stretch teaches one kind of enemy at a time; each branch then leans on
/// what it taught (the marsh: fast wretches in packs; the chapel: shields and lances together); the
/// gate mixes them; the regions past it ask for everything at once, harder the farther they lie.
pub const PLACES: &[Place] = &[
    // ── The Ashen Warrens: the slum the pilgrimage leaves from, behind the road ─────────────
    Place { shrine: Some(3.0), explore: Some("broken_spire"), ..place("warrens", "The Ashen Warrens", 0, (8.0, 46.0), "warrens_lane", "warrens.json", 40.0, &[(Husk, 22.0, 0)]) },
    // The Collector's chest stands behind him: his Tally, once he is down.
    Place { boss: Some((22.0, &DEBT_COLLECTOR)), cache: Some(27.0), loot: Some("The Collector's Tally"), ..place("guild_hall", "The Guild Hall", 0, (3.0, 55.0), "warrens", "guild_hall.json", 30.0, &[(Husk, 12.0, -1)]) },
    place("warrens_lane", "Stilt Lane", 0, (14.0, 40.0), "road", "warrens_lane.json", 36.0, &[(Wretch, 14.0, 1), (Husk, 26.0, -1)]),
    // ── Wretched Vale and Hollowmere ────────────────────────────────────────────────────────
    // Its shieldbearer keeps the fork.
    Place { shrine: Some(1.5), smith: Some(59.0), hold: Hold::Keepers(&[3]), ..place("road", "The Road to Hollowmere", 1, (20.0, 34.0), "warrens_lane", "road_to_hollowmere.json", 64.0,
        &[(Husk, 12.0, 0), (Lancer, 26.0, 1), (Wretch, 38.0, -1), (Shieldbearer, 52.0, 0)]) },
    // Each branch begins under the road's own sky, its ground and trees turning toward where it
    // leads; the change of light comes after, through a crossing (a blend into the marsh, a ruined
    // arch into the graveyard), which handles it far better than a fork can.
    place("vale_edge", "The Marsh Edge", 1, (24.0, 26.0), "road", "vale_edge.json", 26.0, &[(Wretch, 14.0, 0)]),
    place("chapel_edge", "The Chapel Edge", 1, (28.0, 38.0), "road", "chapel_edge.json", 26.0, &[(Lancer, 15.0, 0)]),
    // The marsh's wretches come as a pack out of the reeds; the graves hold a lancer and a husk.
    Place { ambushes: &[Ambush { at: 30.0, foes: &[(Wretch, -3.4), (Wretch, 3.0), (Husk, 4.6)], chest: false }],
        ..place("vale", "The Wretched Vale", 1, (30.0, 21.0), "vale_edge", "wretched_vale.json", 56.0,
        &[(Wretch, 12.0, -1), (Wretch, 15.0, 1), (Husk, 26.0, 0), (Wretch, 36.0, 0), (Lancer, 46.0, -1)]) },
    Place { explore: Some("hollow"), ambushes: &[Ambush { at: 38.0, foes: &[(Lancer, -3.6), (Husk, 3.2)], chest: false }],
        ..place("chapel", "Chapel Road", 1, (34.0, 40.0), "chapel_edge", "chapel_road.json", 58.0,
        &[(Shieldbearer, 12.0, 0), (Lancer, 22.0, 1), (Husk, 32.0, -1), (Shieldbearer, 44.0, 0), (Lancer, 47.0, -1)]) },
    Place { shrine: Some(2.0), boss: Some((40.0, &BELL_WARDEN)), ..place("gate", "Hollowmere Gate", 1, (39.0, 30.0), "vale", "hollowmere_gate.json", 46.0,
        &[(Lancer, 14.0, 1), (Shieldbearer, 18.0, -1), (Wretch, 30.0, 0), (Husk, 36.0, 1)]) },
    // Down a side path off Chapel Road: a dead end among the graves, with something left there,
    // and the chest is a trap.
    Place { cache: Some(20.0), loot: Some("Gravedigger's Ring"), ambushes: &[Ambush { at: 19.0, foes: &[(Husk, 3.0), (Wretch, -3.2)], chest: true }],
        ..place("hollow", "A Hollow Among the Graves", 1, (38.0, 46.0), "", "side_hollow.json", 24.0, &[(Wretch, 11.0, 0), (Husk, 15.0, 1)]) },
    Place { boss: Some((44.0, &HOLLOWMERE_SCREAMER)), ..place("hollowmere", "Hollowmere", 1, (47.0, 25.0), "gate", "hollowmere.json", 52.0,
        &[(Wretch, 10.0, 0), (Husk, 18.0, -1), (Wretch, 22.0, 1), (Shieldbearer, 30.0, 0)]) },
    // Its wheel stopped in the flood: past it, a ladder down to the marsh kicked loose from above.
    Place { cache: Some(30.0), ..place("drowned_mill", "The Drowned Mill", 1, (38.0, 15.0), "hollowmere", "drowned_mill.json", 36.0,
        &[(Wretch, 12.0, -1), (Wretch, 14.0, 1), (Husk, 24.0, 0)]) },
    // ── Blackridge: pine roads and bandit hollows north of the marsh ────────────────────────
    // The bandits' bowmen: the first stands alone down the road, to learn its arrows on.
    place("pine_road", "The Pine Road", 2, (52.0, 16.0), "hollowmere", "pine_road.json", 56.0,
        &[(Lancer, 12.0, 0), (Wretch, 24.0, -1), (Wretch, 26.0, 1), (Hound, 33.0, -1), (Hound, 34.5, 1), (Archer, 42.0, 0)]),
    Place { shrine: Some(3.0), ..place("ridge_camp", "Ridge Camp", 2, (58.0, 9.0), "pine_road", "ridge_camp.json", 34.0, &[]) },
    Place { boss: Some((40.0, &PACK_ALPHA)), explore: Some("bandit_cave"), ambushes: &[Ambush { at: 24.0, foes: &[(Hound, -3.0), (Hound, 3.2), (Archer, 4.8)], chest: false }],
        ..place("bandit_hollows", "The Bandit Hollows", 2, (64.0, 16.0), "ridge_camp", "bandit_hollows.json", 48.0,
        &[(Wretch, 10.0, 0), (Lancer, 18.0, 1), (Husk, 30.0, -1)]) },
    Place { boss: Some((40.0, &BANDIT_CHIEF)), ..place("chief_hold", "The Chief's Hold", 2, (60.0, 2.0), "ridge_camp", "chief_hold.json", 46.0,
        &[(Shieldbearer, 12.0, 0), (Lancer, 20.0, -1), (Lancer, 22.0, 1), (Archer, 32.0, 0)]) },
    // Partway along, a rope kicked down the cliff to the Pine Road: from above only.
    place("cliff_path", "The Cliff Path", 2, (70.0, 12.0), "chief_hold", "cliff_path.json", 40.0, &[(Wretch, 16.0, 0), (Archer, 28.0, 1)]),
    // ── Stonehollow: the walled city ────────────────────────────────────────────────────────
    place("stone_gate", "Stonehollow Gate", 3, (66.0, 24.0), "bandit_hollows", "stone_gate.json", 44.0,
        &[(Shieldbearer, 12.0, 0), (Lancer, 20.0, 1), (Shieldbearer, 32.0, -1), (Brute, 40.0, 0)]),
    place("lower_stairs", "The Lower Stairs", 3, (61.0, 31.0), "stone_gate", "lower_stairs.json", 40.0, &[(Husk, 12.0, 0), (Lancer, 22.0, -1), (Wretch, 30.0, 1)]),
    // A lantern-seller's last lamp, left on the stall: the Hollowed Shrine needs its light.
    Place { shrine: Some(3.0), cache: Some(30.0), relic: Some(Key::Lantern), ..place("stone_market", "The Lantern Market", 3, (68.0, 35.0), "lower_stairs", "stone_market.json", 36.0, &[(Husk, 20.0, 1)]) },
    // Under the city to the Warrens: a grate that lifts only from below.
    // The last husk keeps the passage out.
    Place { hold: Hold::Keepers(&[3]), ..place("sewers", "The Old Sewers", 3, (50.0, 40.0), "lower_stairs", "sewers.json", 60.0, &[(Wretch, 14.0, 0), (Husk, 26.0, -1), (Wretch, 40.0, 1), (Husk, 48.0, 0), (Brute, 55.0, 0)]) },
    // The Agent's chest stands behind it: its beaked mask, once it is down.
    Place { boss: Some((36.0, &ORDER_AGENT)), cache: Some(40.0), loot: Some("Order Agent's Beaked Mask"), ..place("bridge_ward", "The Bridge Ward", 3, (66.0, 43.0), "stone_market", "bridge_ward.json", 44.0,
        &[(Lancer, 12.0, -1), (Shieldbearer, 20.0, 1)]) },
    // The shieldbearer at its gate keeps it.
    Place { hold: Hold::Keepers(&[0]), ..place("stone_upper", "The Upper City", 3, (74.0, 27.0), "stone_market", "stone_upper.json", 44.0, &[(Shieldbearer, 14.0, 0), (Lancer, 24.0, 1), (Hexer, 36.0, -1)]) },
    Place { boss: Some((38.0, &HOLLOW_MAGISTRATE)), ..place("magistrate", "The Magistrate's Court", 3, (80.0, 32.0), "stone_upper", "magistrate.json", 44.0,
        &[(Shieldbearer, 12.0, -1), (Shieldbearer, 14.0, 1), (Lancer, 24.0, 0)]) },
    // ── The Hollowed Shrine: sunk under the city, dark without the lantern ──────────────────
    // The plague-dead below the city: a bloater in the dark of the stair, and in its yard another
    // with a gravecaller behind it.
    Place { ambushes: &[Ambush { at: 27.0, foes: &[(Bloater, -3.0), (Gravecaller, 4.4)], chest: false }],
        ..place("sunken_steps", "The Sunken Steps", 4, (60.0, 49.0), "bridge_ward", "sunken_steps.json", 42.0, &[(Husk, 12.0, 0), (Bloater, 17.0, -1), (Wretch, 22.0, 1), (Hexer, 34.0, -1)]) },
    Place { shrine: Some(3.0), ..place("vine_halls", "The Vine Halls", 4, (52.0, 52.0), "sunken_steps", "vine_halls.json", 40.0, &[(Wretch, 16.0, -1), (Lancer, 26.0, 1), (Gravecaller, 34.0, 0)]) },
    Place { boss: Some((30.0, &WRAITH_LORD)), ..place("wraith_sanctum", "The Wraith Sanctum", 4, (58.0, 58.0), "vine_halls", "wraith_sanctum.json", 36.0, &[(Husk, 12.0, 0)]) },
    // Out of the vault, a catacomb stair up to Chapel Road, its door barred from this side. The
    // Sentinel kept watch over a slab of a sword no one living could lift.
    Place { boss: Some((30.0, &RUNE_SENTINEL)), cache: Some(36.0), loot: Some("Gravewarden's Slab"), ..place("rune_vault", "The Rune Vault", 4, (44.0, 50.0), "vine_halls", "rune_vault.json", 40.0,
        &[(Husk, 12.0, 0), (Bloater, 16.0, 1), (Wretch, 20.0, -1)]) },
    // ── Varnholt: the violet town past the ward ─────────────────────────────────────────────
    // Husks shamble in front while a hexer works from behind them; a brute and more wait in the yard.
    Place { ambushes: &[Ambush { at: 30.0, foes: &[(Brute, -3.2), (Husk, 3.0), (Hexer, 4.6)], chest: false }], hold: Hold::Clear,
        ..place("violet_road", "The Violet Road", 5, (86.0, 37.0), "magistrate", "violet_road.json", 50.0, &[(Husk, 12.0, 0), (Husk, 14.0, 1), (Hexer, 22.0, 0), (Wretch, 28.0, -1), (Husk, 38.0, 0), (Brute, 46.0, 0)]) },
    Place { shrine: Some(3.0), ..place("varnholt", "Varnholt", 5, (90.0, 45.0), "violet_road", "varnholt.json", 40.0, &[(Husk, 20.0, -1), (Husk, 24.0, 1)]) },
    // The pens back onto the Bridge Ward: a gate chained on the town side.
    Place { boss: Some((34.0, &THE_BUTCHER)), ..place("husk_pens", "The Husk Pens", 5, (78.0, 48.0), "varnholt", "husk_pens.json", 44.0,
        &[(Husk, 10.0, 0), (Husk, 12.0, -1), (Husk, 14.0, 1), (Bloater, 19.0, 0), (Shieldbearer, 26.0, 0)]) },
    Place { boss: Some((40.0, &ANGEL_PRIEST)), ..place("cathedral", "The Tainted Cathedral", 5, (96.0, 39.0), "varnholt", "cathedral.json", 46.0,
        &[(Shieldbearer, 12.0, 0), (Husk, 20.0, -1), (Husk, 22.0, 1), (Gravecaller, 31.0, 0)]) },
    // ── Eryndor and the Blackwood: the ruined angel city in the void-touched forest ─────────
    place("blackwood", "The Blackwood Edge", 6, (92.0, 28.0), "cathedral", "blackwood.json", 52.0,
        &[(Wretch, 12.0, 0), (Wretch, 14.0, -1), (Lancer, 26.0, 1), (Hound, 33.0, -1), (Hound, 34.5, 0), (Hound, 36.0, 1), (Archer, 44.0, 0)]),
    Place { shrine: Some(3.0), ..place("angel_ruins", "The Ruins of Eryndor", 6, (86.0, 19.0), "blackwood", "angel_ruins.json", 40.0, &[(Lancer, 20.0, 0), (Hound, 30.0, -1), (Hound, 31.5, 1)]) },
    // Where Eryndor's void-wraiths went back to: the Blackwood gone wrong, past its edge (`bounty`).
    Place { cache: Some(44.0), loot: Some("Yellow-Glass Bead"),
        ..place("void_wood", "The Void-Twisted Forest", 6, (99.0, 25.0), "blackwood", "void_wood.json", 48.0, &[(Hound, 14.0, 0), (Hexer, 22.0, 1), (Wretch, 28.0, -1), (Wretch, 29.5, 1)]) },
    // Behind the altars, a stair down into the Magistrate's Court, sealed from the ruins' side.
    Place { boss: Some((34.0, &VOID_STALKER)), ..place("whisper_altars", "The Whispering Altars", 6, (79.0, 22.0), "angel_ruins", "whisper_altars.json", 44.0,
        &[(Wretch, 10.0, 1), (Wretch, 12.0, -1), (Husk, 22.0, 0), (Hexer, 32.0, 0)]) },
    Place { boss: Some((40.0, &STATUE_ARCHANGEL)), ..place("statue_court", "The Court of the Statue", 6, (91.0, 9.0), "angel_ruins", "statue_court.json", 46.0,
        &[(Shieldbearer, 12.0, 0), (Lancer, 20.0, -1), (Lancer, 22.0, 1)]) },
    // ── The Celestial Spire: north, "where starlight meets stone" (`spire_road`) ─────────────
    // The pilgrims' road across a glassy plain under stars too close, kept by the angels' construct.
    Place { shrine: Some(3.0), boss: Some((46.0, &RADIANT_CONSTRUCT)),
        ..place("starlit_wastes", "The Starlit Wastes", 7, (96.0, 15.0), "statue_court", "starlit_wastes.json", 54.0,
        &[(Lancer, 12.0, 0), (Archer, 22.0, 1), (Hexer, 30.0, -1), (Shieldbearer, 38.0, 0)]) },
    // A pass under the broken sky, where a drake nests in the tear; the Order took Olekeen here.
    Place { boss: Some((48.0, &WARPED_DRAKE)),
        ..place("sky_peaks", "The Shattered Sky Peaks", 7, (101.0, 11.0), "starlit_wastes", "sky_peaks.json", 56.0,
        &[(Hound, 12.0, -1), (Hound, 13.5, 1), (Brute, 24.0, 0), (Hexer, 34.0, 1), (Archer, 40.0, -1)]) },
    place("spire_stair", "The Spire Stair", 7, (101.0, 4.0), "sky_peaks", "spire_stair.json", 56.0,
        &[(Shieldbearer, 12.0, 0), (Lancer, 22.0, -1), (Archer, 32.0, 1), (Brute, 40.0, 0), (Hexer, 50.0, 0)]),
    Place { shrine: Some(3.0), ..place("sealed_slab", "The Sealed Slab", 7, (96.0, 6.0), "spire_stair", "sealed_slab.json", 30.0, &[]) },
    Place { boss: Some((30.0, &THE_ARCHANGEL)), ..place("spire_summit", "The Summit", 7, (98.0, 1.0), "sealed_slab", "spire_summit.json", 36.0, &[]) },
    // ── The Order's stronghold: down the Spire's far side, where the Eye was carried (`finale`) ──
    place("crimson_stair", "The Crimson Stair", 8, (91.0, 2.0), "spire_summit", "crimson_stair.json", 52.0,
        &[(Lancer, 12.0, 0), (Archer, 20.0, 1), (Lancer, 28.0, -1), (Hexer, 36.0, 0), (Shieldbearer, 44.0, 0)]),
    // Its gatehouse lets a rope bridge down across to the Sealed Slab, from the gate's side only.
    Place { shrine: Some(3.0), boss: Some((36.0, &CRIMSON_CAPTAIN)), ..place("radiant_gate", "The Radiant Gate", 8, (85.0, 4.0), "crimson_stair", "radiant_gate.json", 44.0,
        &[(Shieldbearer, 14.0, 0), (Lancer, 22.0, -1), (Archer, 24.0, 1)]) },
    // The cells hold Olekeen until every gaoler is down; past them, the foundry one way, the Eye the other.
    Place { hold: Hold::Clear, ..place("penitent_cells", "The Penitents' Cells", 8, (79.0, 3.0), "radiant_gate", "penitent_cells.json", 46.0,
        &[(Lancer, 10.0, 0), (Hexer, 18.0, 1), (Brute, 26.0, 0), (Lancer, 34.0, -1)]) },
    // On a carver's bench by the empty plinth, the chisel Unit 0 was cut with.
    Place { cache: Some(28.0), loot: Some("Maker's Chisel"),
        ..place("unit_foundry", "The Unit Foundry", 8, (76.0, 8.0), "penitent_cells", "unit_foundry.json", 30.0, &[(Husk, 10.0, 0), (Husk, 11.5, 1), (Brute, 24.0, 0)]) },
    Place { boss: Some((30.0, &EXODUS_LIGHTFOOT)), ..place("eye_chamber", "The Chamber of the Eye", 8, (73.0, 1.0), "penitent_cells", "eye_chamber.json", 40.0, &[]) },
    // ── The Warrens guild's back room, through the door at the guild hall's end: Gruk's counter.
    place("guild_office", "Gruk's Counting Room", 0, (1.5, 58.0), "guild_hall", "guild_office.json", 0.5, &[]),
    // Off Stilt Lane, out through the Warrens' east gate, a lashed arch, to where the mire begins:
    // the book's bone thing and its hounds. Open from the start; the story sends the hero only
    // after Hollowmere.
    Place { boss: Some((30.0, &KEENING_CAGE)), ..place("east_gate", "The East Gate", 0, (18.0, 50.0), "warrens_lane", "east_gate.json", 36.0,
        &[(Hound, 12.0, -1), (Hound, 13.5, 1)]) },
    // Down a side path off the Bandit Hollows, into the ridge: the hole the Blackridge crew keep
    // their take in. Its stash is there for whoever comes before their chief falls; after, his
    // crew run off with it (`World::story_moves`), and the chest's guards with them.
    Place { cache: Some(30.0), loot: Some("Caravan Signet"), ambushes: &[Ambush { at: 29.0, foes: &[(Lancer, -3.2), (Archer, 4.4)], chest: true }],
        ..place("bandit_cave", "The Bandits' Hole", 2, (68.0, 20.0), "", "bandit_cave.json", 34.0, &[(Lancer, 10.0, 0), (Archer, 20.0, 1)]) },
    // On past the east gate to where the mire goes black, among old stones: Krix's beast, and at
    // the far end, among what it kept, Mara's whistle.
    Place { boss: Some((34.0, &BLACK_WATER_MOTHER)), cache: Some(41.0), loot: Some("Mara's Whistle"), ..place("black_water", "The Black Water", 0, (26.0, 56.0), "east_gate", "black_water.json", 44.0,
        &[(Wretch, 12.0, -1), (Wretch, 13.5, 1), (Husk, 22.0, 0)]) },
    // Down a side path at the Warrens' fringe, a crooked tower half-sunk in the mire: Vren's.
    place("broken_spire", "Suspicious Alley", 0, (4.0, 40.0), "", "broken_spire.json", 26.0, &[(Wretch, 12.0, 0)]),
    // North off the Marsh Edge, a ruin of old stones in the mud, where the rune-marked blade lay
    // until a shaken hunter found it once the Hollowmere bounty was paid (`World::story_moves`).
    // Its far end meets a ladder up from the Drowned Mill, kicked down from below only.
    Place { cache: Some(44.0), loot: Some("Rune-Marked Blade"), ..place("drowned_fingers", "The Drowned Fingers", 1, (28.0, 12.0), "vale_edge", "drowned_fingers.json", 48.0,
        &[(Wretch, 12.0, -1), (Wretch, 14.0, 1), (Husk, 24.0, 0), (Lancer, 34.0, 1)]) },
    // On from the Bandit Hollows into a fog-filled hollow, the book's chapter 8: its hounds, and
    // in the clearing past them the horror that waits for Krix and Lurz (`hunters`).
    Place { boss: Some((30.0, &FOG_HORROR)), ..place("fog_hollow", "The Fog Hollow", 2, (75.0, 19.0), "bandit_hollows", "fog_hollow.json", 38.0,
        &[(Hound, 11.0, -1), (Hound, 12.5, 1), (Hound, 20.0, 0)]) },
];

/// The roads.
pub const EDGES: &[Edge] = &[
    road("warrens", "guild_hall"),
    Edge { via: Via::Door(1.0, 2.4), ..road("guild_hall", "guild_office") },
    road("warrens", "warrens_lane"),
    Edge { via: Via::Arch("Gate"), ..road("warrens_lane", "east_gate") },
    Edge { via: Via::Blend(5.0), ..road("east_gate", "black_water") },
    road("warrens_lane", "road"),
    road("road", "vale_edge"),
    road("road", "chapel_edge"),
    Edge { via: Via::Blend(5.0), ..road("vale_edge", "vale") },
    Edge { via: Via::Arch("RuinedArch"), ..road("chapel_edge", "chapel") },
    road("vale", "gate"),
    road("chapel", "gate"),
    road("gate", "hollowmere"),
    road("hollowmere", "drowned_mill"),
    Edge { gate: Gate::Shortcut, ..road("drowned_mill", "vale") },
    Edge { via: Via::Blend(5.0), ..road("vale_edge", "drowned_fingers") },
    Edge { gate: Gate::Shortcut, ..road("drowned_mill", "drowned_fingers") },
    road("hollowmere", "pine_road"),
    road("pine_road", "ridge_camp"),
    road("ridge_camp", "bandit_hollows"),
    road("ridge_camp", "chief_hold"),
    Edge { gate: Gate::Keys(&[Key::Hook]), ..road("chief_hold", "cliff_path") },
    road("cliff_path", "stone_upper"),
    Edge { gate: Gate::Shortcut, ..road("cliff_path", "pine_road") },
    road("bandit_hollows", "stone_gate"),
    Edge { via: Via::Blend(5.0), ..road("bandit_hollows", "fog_hollow") },
    road("stone_gate", "lower_stairs"),
    road("lower_stairs", "stone_market"),
    road("lower_stairs", "sewers"),
    Edge { gate: Gate::Shortcut, ..road("sewers", "warrens") },
    road("stone_market", "bridge_ward"),
    road("stone_market", "stone_upper"),
    road("stone_upper", "magistrate"),
    Edge { gate: Gate::Keys(&[Key::Lantern]), ..road("bridge_ward", "sunken_steps") },
    road("sunken_steps", "vine_halls"),
    Edge { gate: Gate::Keys(&[Key::Heart]), ..road("vine_halls", "wraith_sanctum") },
    road("vine_halls", "rune_vault"),
    Edge { gate: Gate::Shortcut, ..road("rune_vault", "chapel") },
    Edge { gate: Gate::Keys(&[Key::Sigil]), ..road("magistrate", "violet_road") },
    road("violet_road", "varnholt"),
    road("varnholt", "husk_pens"),
    Edge { gate: Gate::Shortcut, ..road("husk_pens", "bridge_ward") },
    Edge { via: Via::Door(2.6, 5.5), ..road("varnholt", "cathedral") },
    Edge { via: Via::Door(2.6, 5.5), ..road("cathedral", "blackwood") },
    road("blackwood", "angel_ruins"),
    road("blackwood", "void_wood"),
    road("angel_ruins", "whisper_altars"),
    Edge { gate: Gate::Shortcut, ..road("whisper_altars", "magistrate") },
    road("angel_ruins", "statue_court"),
    road("statue_court", "starlit_wastes"),
    road("starlit_wastes", "sky_peaks"),
    road("sky_peaks", "spire_stair"),
    // A ledge from the stair down to the Wastes' shrine, let down from the stair's side.
    Edge { gate: Gate::Shortcut, ..road("spire_stair", "starlit_wastes") },
    // The Sealed Slab opens when the two shards come near, as in the book.
    Edge { gate: Gate::Keys(&[Key::ShardI, Key::ShardII]), ..road("spire_stair", "sealed_slab") },
    road("sealed_slab", "spire_summit"),
    road("spire_summit", "crimson_stair"),
    road("crimson_stair", "radiant_gate"),
    Edge { gate: Gate::Shortcut, ..road("radiant_gate", "sealed_slab") },
    road("radiant_gate", "penitent_cells"),
    road("penitent_cells", "unit_foundry"),
    road("penitent_cells", "eye_chamber"),
];

/// Where the pilgrimage begins: the Ashen Warrens, at their shrine, walked in toward the guild
/// hall, where Gruk's board holds the bounty that starts it (`docs/story.md` §2.0).
pub const START: &str = "warrens@warrens_lane";
/// Where the Eye is made whole, and taken (`finale`).
pub const SUMMIT: &str = "spire_summit";
/// The last place: its boss beaten and the Eye chosen for, the pilgrimage is done.
pub const FINALE: &str = "eye_chamber";

pub fn place_of(id: &str) -> Option<&'static Place> { PLACES.iter().find(|p| p.id == id) }

/// The place a stop is in (`<place>@<from>`, or a stop that is a place alone).
pub fn node(stop: &str) -> &str { stop.split('@').next().unwrap_or(stop) }
/// The road a stop was walked in by.
pub fn came_from(stop: &str) -> Option<&str> { stop.split_once('@').map(|(_, f)| f) }
pub fn stop(place: &str, from: &str) -> String { format!("{place}@{from}") }
/// The lane a fork's branch runs on before it reaches `place` from `from`: `from`'s ground still,
/// crossing into `place`'s.
pub fn lane(place: &str, from: &str) -> String { format!("~{place}@{from}") }
/// Whether a journey stop is such a lane.
pub fn is_lane(stop: &str) -> bool { stop.starts_with('~') }
/// Metres a fork's lane takes to cross into the place it leads to.
pub const LANE_BLEND_M: f32 = 6.0;

/// The road between `a` and `b`, if there is one.
pub fn edge(a: &str, b: &str) -> Option<&'static Edge> { EDGES.iter().find(|e| (e.a == a && e.b == b) || (e.a == b && e.b == a)) }

/// The places one road from `id`.
pub fn neighbours(id: &str) -> Vec<&'static str> {
    EDGES.iter().filter_map(|e| if e.a == id { Some(e.b) } else if e.b == id { Some(e.a) } else { None }).collect()
}

/// The roads ahead walking through `id` in from `from`, left first as the map lies (seen walking
/// from `from`'s spot to `id`'s).
pub fn exits(id: &str, from: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = neighbours(id).into_iter().filter(|n| *n != from).collect();
    let (Some(here), Some(back)) = (place_of(id), place_of(from)) else { return out };
    let heading = (here.map.0 - back.map.0, here.map.1 - back.map.1);
    // Screen y runs down, so a positive cross product turns clockwise: to the right.
    let turn = |n: &str| place_of(n).map_or(0.0, |p| { let d = (p.map.0 - here.map.0, p.map.1 - here.map.1); heading.0 * d.1 - heading.1 * d.0 });
    out.sort_by(|a, b| turn(a).total_cmp(&turn(b)));
    out
}

/// Whether the place is laid out for walking it in from `from` (else everything is mirrored).
pub fn forward(id: &str, from: &str) -> bool { place_of(id).is_none_or(|p| p.from.is_empty() || p.from == from) }

/// A three-road place is a Y: its scene is the trunk, walked in from `Place::from`, and the
/// junction where its two branches part is at the scene's end. `<place>@^` walks the trunk back
/// down, from the junction to `from`'s end and on into `from`.
pub const BACK: &str = "^";
/// Whether `id` is such a Y.
pub fn is_y(id: &str) -> bool { neighbours(id).len() == 3 }
/// Whether `id` has one road only: walked in, the way ends there; `<place>@^` walks it back out,
/// from its end to that road.
pub fn is_dead_end(id: &str) -> bool { neighbours(id).len() == 1 }
/// The roads a place is walked in by as journey stops: each road into it, and `BACK` for a Y's
/// trunk walked back down or a dead end walked back out.
pub fn walked_from(id: &str) -> impl Iterator<Item = &'static str> {
    neighbours(id).into_iter().chain((is_y(id) || is_dead_end(id)).then_some(BACK))
}
/// A place's trunk: the road it is laid out to be walked in by.
pub fn trunk(id: &str) -> &'static str { place_of(id).map_or("", |p| p.from) }

/// One way on at the end of a stop: the road it takes (from place, to place), the stop it goes
/// into, and the place it is offered as.
#[derive(Clone, Debug, PartialEq)]
pub struct Lead { pub road: (&'static str, &'static str), pub stop: String, pub shown: &'static str }

/// The ways on at the end of walking `id` in from `from` (or `BACK` down a Y's trunk), left first:
/// one, two at a fork, or none at a dead end's end. A road up one of a Y's branches crosses into
/// the Y's ground at its junction (`is_junction`), whose fork leads down the Y's trunk
/// (`<y>@^`) or up its other branch.
pub fn leads(id: &'static str, from: &str) -> Vec<Lead> {
    let ways: Vec<&'static str> = if from == BACK { vec![trunk(id)] } else { exits(id, from) };
    let down = is_junction(&stop(id, from));
    ways.into_iter().map(|to| {
        let stop = if down && to == trunk(id) { stop(id, BACK) } else { stop(to, id) };
        Lead { road: (id, to), stop, shown: to }
    }).collect()
}

/// Whether `stop` is a Y walked in up one of its branches: its ground just inside the junction,
/// nothing in it, and the fork straight ahead (`junction`).
pub fn is_junction(stop: &str) -> bool {
    let id = node(stop);
    came_from(stop).is_some_and(|from| is_y(id) && from != BACK && from != trunk(id))
}

/// Metres walked into a Y's junction (`is_junction`) before its fork: none, the fork's own walk
/// is the way up to it.
pub const JUNCTION_M: f32 = 0.0;
/// How far ahead a Y's junction shows, walked in up a branch: the stretch of its ground between
/// the boundary and the fork, shorter than a place's.
pub const JUNCTION_APPROACH_M: f32 = 20.0;

/// Metres walked in `stop` before its way on: its place's length, or a junction's.
pub fn length_of(stop: &str) -> f32 {
    let id = node(stop);
    if is_junction(stop) { JUNCTION_M } else { place_of(id).map_or(0.0, |p| p.length) }
}

/// A Y as the junction a branch walks into: its ground and name, nothing in it, `JUNCTION_M` long.
pub fn junction(id: &str) -> Option<&'static Place> {
    static JUNCTIONS: std::sync::LazyLock<Vec<Place>> = std::sync::LazyLock::new(|| {
        PLACES.iter().filter(|p| is_y(p.id)).map(|p| place(p.id, p.title, p.region, p.map, p.from, p.scene, JUNCTION_M, &[])).collect()
    });
    JUNCTIONS.iter().find(|p| p.id == id)
}

/// The stop that walks `id` the other way (turning round partway, from the mirrored metres): a Y
/// up its trunk toward the junction, or back down it; at its junction, back into the branch walked
/// up; a road with two ends, in by the other end; a dead end, walked back out of it.
pub fn turned(id: &str, from: &str) -> String {
    if is_y(id) {
        if from == trunk(id) { return stop(id, BACK); }
        return if from == BACK { stop(id, trunk(id)) } else { stop(from, id) };
    }
    match neighbours(id).into_iter().find(|n| *n != from) {
        Some(other) => stop(id, other),
        None => stop(id, BACK),
    }
}

/// Why a road cannot be walked from `from` to `to` now: the keys it needs, or a shortcut not yet
/// opened from its far side. None when it is open.
#[derive(Clone, Debug, PartialEq)]
pub enum Barred { Keys(Vec<Key>), Shut }

/// Whether the road from `from` to `to` has a gate on it, open or not: one a key opens (true),
/// or a shortcut's door (false). None for an open road.
pub fn gated(from: &str, to: &str) -> Option<bool> {
    match edge(from, to)?.gate { Gate::Open => None, Gate::Keys(_) => Some(true), Gate::Shortcut => Some(false) }
}

pub fn barred(from: &str, to: &str, keys: &[Key], opened: &[&str]) -> Option<Barred> {
    let e = edge(from, to)?;
    match e.gate {
        Gate::Open => None,
        Gate::Keys(need) => {
            let lack: Vec<Key> = need.iter().copied().filter(|k| !keys.contains(k)).collect();
            (!lack.is_empty()).then_some(Barred::Keys(lack))
        }
        Gate::Shortcut => (from == e.b && !opened.contains(&e.a)).then_some(Barred::Shut),
    }
}

/// The world as a PathForge journey: every place once for each road into it, its next what lies
/// ahead that way. The start is `START`; the sanctum (where the hero sits at rest) is a stop of its
/// own, and so is each side path.
#[cfg(test)]
pub fn journey() -> serde_json::Value {
    use serde_json::json;
    let via = |a: &str, b: &str| match edge(a, b).map(|e| e.via) {
        Some(Via::Blend(m)) => json!({ "blend_m": m }),
        Some(Via::Arch(marker)) => json!({ "threshold": "Gate", "marker": marker }),
        Some(Via::Door(half, high)) => {
            let cm = |m: f32| (f64::from(m) * 100.0).round() / 100.0;
            json!({ "threshold": "Doorway", "door": [cm(half), cm(high)] })
        }
        _ => json!({}),
    };
    let mut stops = serde_json::Map::new();
    for p in PLACES.iter().filter(|p| !p.from.is_empty()) {
        for from in walked_from(p.id) {
            let ways = leads(p.id, from);
            let next = match ways.as_slice() {
                [] => json!("End"),
                [l] => json!({ "Go": { "to": l.stop, "transition": via(l.road.0, l.road.1) } }),
                // Left alone, a fork takes its ungated way (a locked door or a shut shortcut down
                // one side is no way at all until it opens; the game leans into it then).
                // Each branch first runs on as a lane of this place (its own ground and sky), then
                // crosses into where it leads: a fork between two other worlds shows them both at
                // the junction, and two unlike scenes meet there badly.
                [left, right] => {
                    let gated = |l: &Lead| edge(l.road.0, l.road.1).is_some_and(|e| e.gate != Gate::Open);
                    let default = if gated(left) && !gated(right) { "Right" } else { "Left" };
                    let mut names = Vec::new();
                    for l in [left, right] {
                        // Each branch runs on as a lane of the Y; down its own trunk from the
                        // junction, the lane is the trunk's own ground, blended into it.
                        let name = lane(l.road.1, p.id);
                        let t = match edge(l.road.0, l.road.1).map(|e| e.via) {
                            _ if node(&l.stop) == p.id => json!({ "blend_m": LANE_BLEND_M }),
                            Some(Via::Blend(_) | Via::Arch(_) | Via::Door(..)) => via(l.road.0, l.road.1),
                            _ => json!({ "blend_m": LANE_BLEND_M }),
                        };
                        stops.insert(name.clone(), json!({ "scene": p.scene, "next": { "Go": { "to": l.stop, "transition": t } } }));
                        names.push(name);
                    }
                    let approach = if is_junction(&stop(p.id, from)) { JUNCTION_APPROACH_M } else { 40.0 };
                    json!({ "Fork": { "left": names[0], "right": names[1], "fork": { "steer_m": crate::world::STEER_M, "approach_m": approach, "wedge_m": -1.0, "default_branch": default } } })
                }
                _ => panic!("{} has more than three roads", p.id),
            };
            stops.insert(stop(p.id, from), json!({ "scene": p.scene, "next": next }));
        }
    }
    for p in PLACES.iter().filter(|p| p.from.is_empty()) { stops.insert(p.id.into(), json!({ "scene": p.scene, "next": "End" })); }
    stops.insert(crate::world::SANCTUM.into(), json!({ "scene": "sanctum.json", "next": "End" }));
    json!({ "version": 1, "name": "The Shattered Realms", "start": START, "stops": stops })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn journey_path() -> std::path::PathBuf { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scenes/world.journey.json") }

    #[test]
    fn every_road_joins_two_places_and_no_place_has_more_than_three() {
        let ids: HashSet<&str> = PLACES.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), PLACES.len(), "place ids are unique");
        for e in EDGES {
            assert!(ids.contains(e.a) && ids.contains(e.b), "{} - {}", e.a, e.b);
            // By its ends, not its address: a `const` array's items have no one address to compare.
            assert!(EDGES.iter().filter(|f| (f.a, f.b) == (e.a, e.b) || (f.a, f.b) == (e.b, e.a)).count() == 1, "one road {} - {}", e.a, e.b);
            assert!(edge(e.a, e.b).is_some_and(|g| (g.a, g.b) == (e.a, e.b)), "found by its ends {} - {}", e.a, e.b);
        }
        for p in PLACES.iter().filter(|p| !p.from.is_empty()) {
            let n = neighbours(p.id);
            assert!(!n.is_empty() && n.len() <= 3, "{} has {} roads", p.id, n.len());
            assert!(n.contains(&p.from), "{} is laid out from {}, which is not a road into it", p.id, p.from);
            assert!(p.region < REGIONS.len());
            assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scenes").join(p.scene).exists(), "{} has no scene {}", p.id, p.scene);
        }
    }

    /// Every region is joined to the world by more than one road (a way in and another way back),
    /// and has a shortcut that opens from its far side.
    #[test]
    fn regions_link_by_more_than_one_route_and_each_has_a_shortcut() {
        let region = |id: &str| place_of(id).map(|p| p.region).unwrap();
        for r in 0..REGIONS.len() {
            let out = EDGES.iter().filter(|e| (region(e.a) == r) != (region(e.b) == r)).count();
            let last = r == REGIONS.len() - 1;
            assert!(out >= 2 || last, "{} has {out} road(s) out", REGIONS[r].name);
            let shortcut = EDGES.iter().any(|e| e.gate == Gate::Shortcut && (region(e.a) == r || region(e.b) == r));
            assert!(shortcut || last, "{} has no shortcut", REGIONS[r].name);
        }
    }

    /// Walked from the start, taking every road that is open and winning everything met: every
    /// place is reached, the finale last of all needing every shard, and nothing reached can
    /// strand the hero (from anywhere, some open road leads back to the start).
    #[test]
    fn the_whole_world_can_be_walked_and_nothing_strands_the_hero() {
        let mut keys: Vec<Key> = Vec::new();
        let mut opened: Vec<&str> = Vec::new();
        let mut seen: HashSet<&str> = [node(START)].into();
        // The round of the walk each place was first reached in: how far into the game it lies.
        let mut first: std::collections::HashMap<&str, usize> = [(node(START), 0)].into();
        for round in 1.. {
            let before = (seen.len(), keys.len(), opened.len());
            let here: Vec<&Place> = PLACES.iter().filter(|p| seen.contains(p.id)).collect();
            for p in here {
                if let Some(k) = p.boss.and_then(|(_, b)| b.key).into_iter().chain(p.relic).next() { if !keys.contains(&k) { keys.push(k); } }
                // The heart is paid for at its door.
                if p.id == "vine_halls" && !keys.contains(&Key::Heart) { keys.push(Key::Heart); }
                for n in neighbours(p.id) {
                    if barred(p.id, n, &keys, &opened).is_none() {
                        if let Some(e) = edge(p.id, n).filter(|e| e.gate == Gate::Shortcut && e.a == p.id) { if !opened.contains(&e.a) { opened.push(e.a); } }
                        if seen.insert(n) { first.insert(n, round); }
                    }
                }
            }
            if (seen.len(), keys.len(), opened.len()) == before { break; }
        }
        // A shortcut opens from the side reached later, once the hero has come far enough: back
        // to somewhere found before.
        for e in EDGES.iter().filter(|e| e.gate == Gate::Shortcut) {
            assert!(first[e.a] > first[e.b], "the shortcut {} - {} opens from the side reached first", e.a, e.b);
        }
        for p in PLACES.iter().filter(|p| !p.from.is_empty()) { assert!(seen.contains(p.id), "{} is never reached", p.id); }
        // Without the second shard the Spire is shut; the rune opens nothing.
        assert!(barred("spire_stair", "sealed_slab", &[Key::Rune, Key::ShardI], &[]).is_some());
        assert!(barred("spire_stair", "sealed_slab", &[Key::ShardI, Key::ShardII], &[]).is_none());
        // From every place, back to the start along roads open now.
        for &from in &seen {
            let mut reach: HashSet<&str> = [from].into();
            let mut todo = vec![from];
            while let Some(a) = todo.pop() {
                for b in neighbours(a) { if barred(a, b, &keys, &opened).is_none() && reach.insert(b) { todo.push(b); } }
            }
            assert!(reach.contains(node(START)), "stranded at {from}");
        }
        // Before any shortcut is opened, its far side is shut.
        assert_eq!(barred("warrens", "sewers", &[], &[]), Some(Barred::Shut));
        assert_eq!(barred("sewers", "warrens", &[], &[]), None);
        assert_eq!(barred("warrens", "sewers", &[], &["sewers"]), None);
    }

    #[test]
    fn a_fork_is_left_and_right_as_the_map_lies() {
        // Out of the Warrens toward Hollowmere, the marsh lies to the left (north) and the chapel
        // to the right.
        assert_eq!(exits("road", "warrens_lane"), vec!["vale_edge", "chapel_edge"]);
        assert_eq!(turned("guild_office", "guild_hall"), "guild_office@^");
        assert!(forward("road", "warrens_lane") && !forward("road", "vale_edge"));
    }

    /// Turning round anywhere, the way back links up with every fork passed: a Y walked up its
    /// trunk turns back down it (no fork), and coming back up one of its branches the road crosses
    /// into the Y's junction, whose fork leads down the trunk or up the other branch.
    #[test]
    fn turning_back_meets_the_forks_it_passed() {
        assert_eq!(turned("road", "warrens_lane"), "road@^");
        assert_eq!(turned("road", BACK), "road@warrens_lane");
        assert_eq!(turned("vale", "vale_edge"), "vale@^");
        let stops = |id, from| leads(id, from).into_iter().map(|l| l.stop).collect::<Vec<_>>();
        assert_eq!(stops("road", BACK), vec!["warrens_lane@road"]);
        // A dead end ends; turned round in, it is walked back out into the junction it hangs off,
        // on that junction's own ground, and the junction is turned back from into the branch.
        assert_eq!(turned("guild_office", BACK), "guild_office@guild_hall");
        assert!(stops("guild_office", "guild_hall").is_empty());
        assert_eq!(stops("guild_office", BACK), vec!["guild_hall@guild_office"]);
        assert_eq!(stops("guild_hall", "guild_office"), vec!["warrens@guild_hall"]);
        assert!(is_junction("warrens@guild_hall") && !is_junction("warrens@warrens_lane") && !is_junction("warrens@^"));
        assert_eq!(turned("warrens", "guild_hall"), "guild_hall@warrens");
        let mut out = stops("warrens", "guild_hall");
        out.sort();
        assert_eq!(out, vec!["sewers@warrens", "warrens@^"]);
        // Back up a branch the same: into the junction, then down the trunk or up the other branch.
        assert_eq!(stops("chapel_edge", "chapel"), vec!["road@chapel_edge"]);
        let mut back_up = stops("road", "vale_edge");
        back_up.sort();
        assert_eq!(back_up, vec!["chapel_edge@road", "road@^"]);
        // Every stop the journey has leads somewhere it has too, and every way on is a road.
        let j = journey();
        let all = j["stops"].as_object().unwrap();
        for p in PLACES.iter().filter(|p| !p.from.is_empty()) {
            for from in walked_from(p.id) {
                for l in leads(p.id, from) {
                    assert!(all.contains_key(&l.stop), "{}@{from} leads to {} which is no stop", p.id, l.stop);
                    assert!(edge(l.road.0, l.road.1).is_some(), "{l:?}");
                }
            }
        }
    }

    /// The journey file the game opens is compiled from the atlas (`ATLAS_WRITE=1` writes it).
    #[test]
    fn the_journey_file_is_the_atlas() {
        let want = serde_json::to_string_pretty(&journey()).unwrap() + "\n";
        if std::env::var("ATLAS_WRITE").is_ok() { std::fs::write(journey_path(), &want).unwrap(); }
        let have = std::fs::read_to_string(journey_path()).unwrap_or_default();
        assert!(have == want, "assets/scenes/world.journey.json is stale: run ATLAS_WRITE=1 cargo test --release the_journey_file_is_the_atlas");
    }
}
