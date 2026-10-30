//! The world the road walks through: the atlas's places (`atlas`), the enemies each holds and
//! which of them are down, when the walk goes on and which road it takes, the gates and shortcuts
//! on the way, and the shrines (the bonfires): rest to be made whole, rise in level and have the
//! world's enemies back; die to wake at the last one rested at, with the souls carried left where
//! the hero fell.
//!
//! The walk is in a stop (`atlas::stop`: a place walked in by one road). Everything here is in
//! road metres (`walked`, never coming round), so it is tested without PathForge; the game reads
//! the walk's state and sends what `World` asks for.

use crate::atlas::{self, Barred, Key, Place};
use crate::fight::{Ai, Archetype, Enemy, Fight, HERO_D};
use quartz_path_forge::Branch;

pub use crate::atlas::START;

/// What the fallen leave, which buys levels and forging (the player's word for it; the code says
/// `souls`), and the same with a capital.
pub const CURRENCY: &str = "echoes";
pub const CURRENCY_TITLE: &str = "Echoes";
/// The most enemies any place holds: the game makes this many enemy objects.
pub const MOST_FOES: usize = 7;
/// How near the shrine the hero must stand to rest, metres.
pub const SHRINE_REACH: f32 = 3.0;
/// How far past where a dead end holds the hero its wall stands, and Gruk behind his counter
/// (within `SHRINE_REACH`), metres.
pub const END_PAST: f32 = 5.0;
pub const GRUK_PAST: f32 = 2.6;
/// Metres before a door from which its walls close the side lanes and draw the hero to its middle.
const DOOR_FUNNEL: f32 = 4.0;
/// Metres past a door the hero's body is still in it.
const DOOR_PAST: f32 = 1.0;
/// Metres before a door at which a hero outside its opening meets the wall.
const DOOR_FACE: f32 = 0.5;
/// Half the hero's width, metres: what must fit inside a door's opening.
const HERO_HALF: f32 = 0.35;
/// Metres within which a living enemy, ahead or behind, keeps what wants a quiet moment (a cache
/// taken, a boss stepping out, a side path left) from happening.
const QUIET: f32 = 8.0;
/// The veil hangs this far (metres, as the place is laid out) past where its boss waits;
/// E reaches it from this far short of that.
pub const VEIL_AHEAD: f32 = 0.7;
const VEIL_REACH: f32 = 0.6;
/// The journey's stop where the hero sits at rest: a clearing ringed by mist, the keeper on its
/// left, the shrine in its middle, the smith (once met) on its right. Not a place on the road.
pub const SANCTUM: &str = "sanctum";
/// Shards (what the smith forges with) each kind leaves when felled.
pub fn shards(k: Archetype) -> u32 { match k { Archetype::Lancer => 1, Archetype::Shieldbearer => 2, _ => 0 } }
/// Metres before the last moment to choose at which the lane held becomes the branch taken:
/// leaning into a lane says which way, and the choice is made late so the camera turns onto the
/// branch right at the junction (and the player can change their mind until then).
pub const COMMIT_M: f32 = 0.8;
/// Metres before the last moment at which the walk waits for a branch not yet taken.
pub const HOLD_M: f32 = COMMIT_M + 0.6;
/// The camera turns onto a branch over this many metres (the journey's `steer_m`).
pub const STEER_M: f32 = 6.0;
/// Where the signpost stands past the junction: on the island where the branches part
/// (the paths' width over the angle between them, for the road's 2.4 m half width at 35 degrees).
pub const SIGNPOST_PAST: f32 = 4.4;
/// Sideways metres per metre of each branch (the plan PathForge makes for this canvas: 35 degrees).
pub const FORK_SLOPE: f32 = 0.689;
/// How near a side path's mouth the hero must stand to turn down it, metres.
pub const SIDE_REACH: f32 = 1.6;
/// Levels under a region's (its tier times this) at which arriving there warns the hero.
pub const LEVELS_PER_TIER: u32 = 6;

/// What the game should tell the road to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Go {
    /// Walk on to where the journey leads.
    Next,
    /// Take this branch of the fork ahead.
    Choose(Branch),
    /// Cut to this stop (waking at a shrine; a pilgrimage done, the road again).
    Warp(String),
    /// Cut to this stop, this many metres in (turned round partway along a place).
    WarpAt(String, f32),
    /// Cut to this stop, this many metres short of where its walk on hands over (turned round past
    /// a boundary or a fork's junction: the way back runs up to it).
    TurnInto(String, f32),
    /// Cut to this stop with its walk on this many metres under way (turned round short of where
    /// a place was walked into: the way back out is that far along).
    WalkOn(String, f32),
}

/// The road's walk on from the stop as the frame shown has it (`WalkState::travel`, `handover`,
/// `length`): metres into the crossing or fork under way, where it hands over to the next place,
/// and where it arrives there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Underway { pub travel: Option<f32>, pub handover: Option<f32>, pub length: Option<f32> }

/// Something worth a line on the screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Arrived(&'static str),
    /// Into a new region (its name), and whether it lies far beyond the hero.
    Region(&'static str, bool),
    ShrineFound,
    Rested,
    Rose(u32),
    /// A fork: the places left and right.
    Fork(&'static str, &'static str),
    /// Leaning toward a branch at a fork, and the place it leads to.
    Lean(Branch, &'static str),
    /// At a fork, the rune hums toward this branch (`World::rune_hums`).
    Hums(Branch),
    /// The way ahead (to this place) cannot be walked: the key it needs, or None for a shortcut
    /// shut from this side.
    Barred(&'static str, Option<Key>),
    /// A shortcut opened, back to this place.
    Opened(&'static str),
    /// Turned round on the road.
    Turned,
    /// The way on is held until this kind of enemy (the place's keeper) is down; `true` for a
    /// gauntlet, held until every one is.
    Held(Archetype, bool),
    Recovered(u32),
    /// The smith raised a piece to this; gave one at its top this capstone.
    Forged(&'static str, u32),
    Capped(&'static str, &'static str),
    /// Met someone on the road.
    Met(&'static str),
    /// A piece of gear off the dead, and one put on (or a place left empty: "nothing").
    Found(&'static str),
    Wore(&'static str),
    /// The weapon worn, made for both hands, now held in one (true) or both.
    Grip(&'static str, bool),
    /// Put on, though the hero is short of what it needs (what it wants, said).
    WoreWeak(&'static str, String),
    /// A boss felled, and the souls it gave.
    Vanquished(&'static str, u32),
    /// The smith forged this from a boss's remnant.
    Crafted(&'static str),
    /// The smith fletched a bundle: how many, of what (arrows or bolts).
    Fletched(u32, &'static str),
    /// Foes sprang out (true: from a chest).
    Ambushed(bool),
    /// The last boss beaten: the pilgrimage begins again, harder (its count).
    Pilgrimage(u32),
    /// A bounty taken from a board (its notice), and one paid for: who paid, how many echoes.
    Bounty(&'static str),
    Paid(&'static str, u32),
    /// The walk has reached a dead end: the way on is back the way the hero came.
    DeadEnd,
    /// The Collector's Tally paid this many more souls for a kill after a parry.
    Tallied(u32),
    /// Someone answered a call at the veil.
    Called(crate::allies::Summoner),
    /// A thing that only tested the hero melted into the dark.
    Fled(&'static str),
    /// Blood given to the Hollowed Shrine's heart.
    Bled,
    /// A line of the story, said as it is.
    Told(&'static str),
}

/// Souls left where the hero fell: the place, metres into it as it is laid out, how many.
#[derive(Clone, Debug, PartialEq)]
pub struct Stain { pub stop: String, pub at: f32, pub souls: u32 }

#[derive(Clone, Debug)]
pub struct World {
    /// The stop the hero is in, and the walk's metres when they arrived.
    pub at: String,
    pub base: f64,
    /// Metres into the stop, as walked, where the hero came in: a boss or an ambush waits only
    /// past this (turned round short of one, it is behind them, not just passed).
    entered: f32,
    /// `Next` sent from this stop (wait for the arrival).
    pub sent: bool,
    /// A warp sent and not yet seen in the walk's state, and how far into that stop it lands.
    pub warping: Option<String>,
    warp_into: f32,
    /// A `TurnInto` sent: metres short of its stop's hand-over the hero stands.
    turn_short: Option<f32>,
    /// A `WalkOn` sent: its stop's walk on is under way when it lands.
    turn_on: bool,
    /// The walk on from this stop as last seen, and the road metres at which it passed its
    /// boundary or junction (turned round beyond it, the way back runs up to it).
    underway: Underway,
    pub(crate) handed: Option<f64>,
    /// Road metres walked in the frame shown, as `follow` last had them.
    shown: f64,
    /// The shrine last rested at (where death wakes the hero), as the stop rested in.
    pub checkpoint: String,
    /// Every shrine rested at, to travel between from the sanctum.
    pub rested: Vec<String>,
    pub souls: u32,
    pub stain: Option<Stain>,
    /// Times the last boss has been beaten.
    pub pilgrimages: u32,
    /// The fork's signpost, road metres, once the fork is in view.
    pub signpost: Option<f64>,
    /// The branch the hero leans toward at a fork (the lane they hold), and whether it is taken.
    pub lean: Option<Branch>,
    pub chose: bool,
    /// Where the walk was when the branch was taken (the camera turns onto it from there).
    pub chose_at: Option<f64>,
    /// The branch taken: the one leaned into, or the only one open whatever the lean.
    pub took: Option<Branch>,
    /// Shards carried for the smith, by grade (`gear::SHARDS`).
    pub shards: [u32; 3],
    /// Whether the keeper has spoken her first words, and the smith been met on the road (he waits
    /// at the shrines after).
    pub met_keeper: bool,
    pub met_smith: bool,
    /// Times each has been talked to (the keeper, the smith): the next thing they say.
    pub talks: [u32; 2],
    /// The news each has told (the keeper, the smith), a bit a beat (`voices::Speaker::talk`).
    pub heard: [u32; 2],
    /// Bosses someone else killed while the hero was elsewhere (`World::story_moves`): gone, and
    /// nothing of theirs for the hero.
    pub claimed: Vec<&'static str>,
    /// The meetings on the road already had (`visitors::VISITS`).
    pub meetings: Vec<&'static str>,
    /// The meeting just had, whose visitor stays until the hero leaves the place.
    pub lingering: Option<&'static str>,
    /// The bounties Gruk has posted to the hero and paid for.
    pub guild: crate::guild::Guild,
    /// What the horror in the fog hollow has taken of Krix and Lurz.
    pub hunters: crate::hunters::Hunters,
    /// What the hero said back when asked (`summons::ASKS`), and who answered a call at the veil
    /// here (gone once the boss falls, the hero does, or the place is left).
    pub answers: Vec<&'static str>,
    pub called: Option<crate::allies::Summoner>,
    /// What the hero owns and wears.
    pub gear: crate::gear::Loadout,
    /// The arrows and bolts carried as last kept (`fight::Hero::quiver` is the one shot from).
    pub ammo: crate::fight::Quiver,
    /// How many of each kind of enemy have fallen (for what they leave), and a count that stands
    /// in for chance.
    pub fallen: [u32; Archetype::ALL.len()],
    pub luck: u32,
    /// A parry made since the last kill (the Collector's Tally pays on the next one). Not kept.
    pub parried: bool,
    /// Places whose boss has fallen, this pilgrimage.
    pub beaten: Vec<&'static str>,
    /// Places whose boss has been faced at least once (its entrance plays only the first time).
    pub faced: Vec<&'static str>,
    /// Key items held, and shortcuts opened (by the place they open from).
    pub keys: Vec<Key>,
    /// Bosses' remnants carried, for the smith to forge (kept from one pilgrimage to the next).
    pub remnants: Vec<&'static str>,
    pub opened: Vec<&'static str>,
    /// Places walked into, for the map.
    pub seen: Vec<&'static str>,
    /// Enemies down, (place, which), until the hero rests or dies; and which of the place's the
    /// fight's enemies are.
    pub slain: Vec<(&'static str, usize)>,
    spawned: Vec<usize>,
    /// Down a side path: where on the road the hero left it, to come back to.
    pub detour: Option<Detour>,
    /// Side paths walked this pilgrimage (by the stop they lead to), and caches taken.
    pub explored: Vec<&'static str>,
    pub taken: Vec<&'static str>,
    /// Ambushes beaten: (place, which).
    pub cleared: Vec<(&'static str, usize)>,
    /// A barred way already told of in this stop.
    told: bool,
    /// The fork's lane walked on last (`Next` sent from it).
    lane: Option<String>,
    /// The next warp keeps the hero as they are (a detour, a turn), rather than whole again.
    keep_hero: bool,
    /// The way held already told of in this stop.
    held_told: bool,
    /// Turned round partway along a place: its living enemies as they stood (by which of the
    /// place's they are) and how far ahead of the hero, metres, to stand mirrored after the turn.
    carry: Option<Vec<(usize, Enemy, f32)>>,
}

/// Where the hero left the road for a side path: the stop, metres in, its enemies as they were
/// (positions from where the place began), and the side it opened on.
#[derive(Clone, Debug, PartialEq)]
pub struct Detour { pub from: String, pub at: f64, pub foes: Vec<Enemy>, pub side: f32 }

impl World {
    pub fn new(walked: f64) -> World {
        World {
            at: START.into(), base: walked, entered: 0.0, sent: false, warping: None, warp_into: 0.0, turn_short: None, turn_on: false, underway: Underway::default(), shown: walked, handed: None, checkpoint: START.into(), rested: Vec::new(), souls: 0, stain: None,
            pilgrimages: 0, signpost: None, lean: None, chose: false, chose_at: None, took: None, shards: [0; 3], met_keeper: false, met_smith: false, talks: [0, 0], heard: [0, 0], claimed: Vec::new(), meetings: Vec::new(), lingering: None, guild: crate::guild::Guild::default(), hunters: crate::hunters::Hunters::default(), answers: Vec::new(), called: None,
            gear: crate::gear::Loadout::starting(), ammo: crate::fight::Quiver::SETTING_OUT, fallen: [0; Archetype::ALL.len()], luck: 0, parried: false, beaten: Vec::new(), faced: Vec::new(), keys: Vec::new(), remnants: Vec::new(), opened: Vec::new(),
            seen: vec![atlas::node(START)], slain: Vec::new(), spawned: Vec::new(), detour: None, explored: Vec::new(), taken: Vec::new(),
            cleared: Vec::new(), told: false, lane: None, keep_hero: false, held_told: false, carry: None,
        }
    }

    /// The place the hero is in.
    pub fn here(&self) -> &'static Place {
        let id = atlas::node(&self.at);
        let place = if atlas::is_junction(&self.at) { atlas::junction(id) } else { atlas::place_of(id) };
        place.unwrap_or(&atlas::PLACES[0])
    }
    /// The road the hero walked in by (None in a side path).
    fn from(&self) -> Option<&str> { atlas::came_from(&self.at) }
    /// Whether the place is walked the way it is laid out.
    fn forward(&self) -> bool { self.from().is_none_or(|f| atlas::forward(self.here().id, f)) }
    /// Which way across the road the place's verges lie: 1 as laid out, -1 walked backwards (the
    /// shrine on the right verge is on the left walking the other way).
    pub fn side(&self) -> f32 { if self.forward() { 1.0 } else { -1.0 } }
    /// Metres into the place, as walked now, of something `m` metres in as laid out.
    fn along(&self, m: f32) -> f32 { if self.forward() { m } else { (self.here().length - m).max(0.0) } }
    /// The road metres of something `m` metres into the place as laid out.
    pub(crate) fn road_at(&self, m: f32) -> f64 { self.base + (HERO_D + self.along(m)) as f64 }

    /// How strong the place's enemies are against the road's first: (health, blows). Deeper
    /// regions and each pilgrimage again make them stronger.
    pub fn scale(&self) -> (f32, f32) { Self::scale_at(atlas::REGIONS[self.here().region].tier, self.pilgrimages) }

    /// What the place's region makes of the road's kinds of foe (`fight::Variant`).
    pub fn variant(&self) -> crate::fight::Variant { crate::fight::Variant::of_tier(atlas::REGIONS[self.here().region].tier) }

    /// The enemies' (health, blows) in a region of `tier`, `pilgrimages` in: deeper, harder, and
    /// harder again by what the smith's levels a hero is expected to carry there (`forged_at`)
    /// add past those the curve was first set against (one weapon and one armour level, 8% and
    /// 5%, every second tier). Blows answer only half the armour: a forged hero is felt to be
    /// tougher, and a late boss's blow takes about a third of an expected hero's health, not half.
    pub fn scale_at(tier: u32, pilgrimages: u32) -> (f32, f32) {
        let t = tier as f32 - 1.0;
        let again = 1.0 + 0.5 * pilgrimages as f32;
        let ((main, armour), first) = (Self::forged_at(tier), (tier / 2) as f32);
        let health = (1.0 + crate::gear::MAIN_STEP * main as f32) / (1.0 + 0.08 * first);
        let blows = (1.0 - 0.05 * first) / (1.0 - 0.5 * crate::gear::ARMOUR_STEP * armour as f32);
        ((1.0 + 0.3 * t).max(0.8) * again * health, (1.0 + 0.16 * t).max(0.85) * again * blows)
    }

    /// The smith's levels a hero is expected to have by a region of `tier`, from the shards the
    /// regions before it give: the main hand's, and the armour's together.
    pub fn forged_at(tier: u32) -> (u32, u32) {
        const MAIN: [u32; 8] = [0, 1, 3, 4, 5, 6, 7, 8];
        const ARMOUR: [u32; 8] = [0, 2, 6, 8, 10, 12, 14, 16];
        let t = (tier as usize).min(7);
        (MAIN[t], ARMOUR[t])
    }

    /// The shards a `k` felled in a region of `tier` leaves, by `roll`: a lancer's and a
    /// shieldbearer's always, any other's one time in six; the common grade in the first regions,
    /// rarer ones deeper (`shard_grade`). A rare one found is told.
    fn shards_from(&mut self, k: Archetype, tier: u32, roll: u32) -> Option<Note> {
        let n = match shards(k) { 0 if (roll >> 16) % 6 == 0 => 1, n => n };
        if n == 0 { return None; }
        let grade = Self::shard_grade(tier, roll >> 20);
        self.shards[grade] += n;
        (grade > 0).then(|| Note::Found(crate::gear::SHARDS[grade]))
    }

    /// The grade of shard a region of `tier` gives, by `roll`: ash to Blackridge; from
    /// Stonehollow ember half the time; from Eryndor star a third of the time, else ember.
    pub fn shard_grade(tier: u32, roll: u32) -> usize {
        match tier {
            0..=2 => 0,
            3..=5 => if roll % 2 == 0 { 1 } else { 0 },
            _ => if roll % 3 == 0 { 2 } else { 1 },
        }
    }

    /// The enemies of the stop the hero is in that are not down, placed from where they arrived
    /// (mirrored walking the place the other way), as strong as the region makes them.
    pub fn foes(&mut self) -> Vec<Enemy> {
        let p = self.here();
        let (scale, variant) = (self.scale(), self.variant());
        let flip = if self.forward() { 1 } else { -1 };
        self.spawned = (0..p.foes.len()).filter(|&i| !self.slain.contains(&(p.id, i))).collect();
        self.spawned.iter().map(|&i| { let (k, ahead, lane) = p.foes[i]; Enemy::new(k, self.road_at(ahead), lane * flip).scaled(scale).of(variant).unaware() }).collect()
    }

    /// Remember the fight's enemies that are down as down.
    fn mark_slain(&mut self, fight: &Fight) {
        let id = self.here().id;
        for (e, &i) in fight.enemies.iter().zip(&self.spawned) {
            if !e.alive() && !self.slain.contains(&(id, i)) { self.slain.push((id, i)); }
        }
    }

    /// The shrine's place on the road, metres, if this stop has one.
    pub fn shrine(&self) -> Option<f64> { self.here().shrine.map(|m| self.road_at(m)) }

    /// The cache's place on the road, metres, if this stop has one.
    pub fn cache_at(&self) -> Option<f64> { self.here().cache.map(|m| self.road_at(m)) }
    /// Whether this place's cache has been taken.
    pub fn cache_taken(&self) -> bool { self.taken.contains(&self.here().id) }

    /// The souls left on the road here, metres, if the hero fell in this place.
    pub fn stain_at(&self) -> Option<f64> {
        self.stain.as_ref().filter(|s| s.stop == self.here().id).map(|s| self.road_at(s.at))
    }

    /// Arrived at `stop` (`walked` metres in, or `into` metres into it after a turn): its enemies,
    /// and what to say. `level` is the hero's, to warn them of a region far beyond it.
    pub fn arrive(&mut self, stop: &str, walked: f64, into: f32, level: u32) -> (Vec<Enemy>, Vec<Note>) {
        // Whoever was met here has moved on once the hero is elsewhere.
        if atlas::node(stop) != self.here().id { (self.lingering, self.called) = (None, None); }
        let was = self.here();
        let first = self.seen.is_empty();
        self.at = stop.to_owned();
        self.base = walked - into as f64;
        self.entered = into;
        self.sent = false;
        self.told = false;
        self.held_told = false;
        self.signpost = None;
        self.lean = None;
        self.chose = false;
        self.chose_at = None;
        self.took = None;
        self.handed = None;
        // Off any fork's lane: the next time a branch is taken, even the same one (after dying,
        // or turning back), it leads on again rather than running on as an endless lane.
        self.lane = None;
        let p = self.here();
        if !self.seen.contains(&p.id) { self.seen.push(p.id); }
        let mut notes = vec![Note::Arrived(p.title)];
        if p.region != was.region || first {
            let r = &atlas::REGIONS[p.region];
            notes.push(Note::Region(r.name, level + LEVELS_PER_TIER < r.tier * LEVELS_PER_TIER));
        }
        if p.shrine.is_some() { notes.push(Note::ShrineFound); }
        (self.foes(), notes)
    }

    /// The side of the side path whose mouth the hero stands at, if this place's lead somewhere
    /// not yet walked and no enemy is close. `side_paths` is the walk's (metres ahead, side).
    pub fn side_path_at(&self, fight: &Fight, walked: f64, side_paths: &[(f32, f32)]) -> Option<f32> {
        let to = self.here().explore?;
        if self.explored.contains(&to) || self.detour.is_some() || fight.hero.dead { return None; }
        if fight.enemies.iter().any(|e| e.alive() && e.gap(walked).abs() < 8.0) { return None; }
        side_paths.iter().find(|(d, _)| (d - HERO_D).abs() < SIDE_REACH).map(|&(_, s)| s)
    }

    /// Turn down the side path on `side`: the road as it is now is kept to come back to.
    pub fn explore(&mut self, fight: &Fight, walked: f64, side: f32) -> Option<Go> {
        let to = self.here().explore?;
        let foes = fight.enemies.iter().cloned().map(|mut e| { e.s -= self.base; e }).collect();
        self.detour = Some(Detour { from: self.at.clone(), at: walked - self.base, foes, side });
        self.explored.push(to);
        self.keep_hero = true;
        self.warping = Some(to.into());
        self.warp_into = 0.0;
        Some(Go::Warp(to.into()))
    }

    /// Whether the hero can turn back from the side path now: at its end, every enemy down.
    pub fn can_come_back(&self, fight: &Fight, walked: f64) -> bool {
        self.detour.is_some() && Self::quiet(fight, walked) && walked - self.base >= self.here().length as f64 - 2.0
    }

    /// Back up the side path to where the hero left the road.
    pub fn come_back(&mut self) -> Option<Go> {
        let from = self.detour.as_ref()?.from.clone();
        self.keep_hero = true;
        self.warping = Some(from.clone());
        self.warp_into = 0.0;
        Some(Go::Warp(from))
    }

    /// Whether the hero can turn round here: on the road (not down a side path, nor waiting on a
    /// warp), no enemy close. Past a fork's junction too, the way back running up to it; not
    /// between a Y's junction and the branch's boundary behind it, which no stop walks back.
    pub fn can_turn(&self, fight: &Fight, walked: f64) -> bool {
        self.detour.is_none() && self.warping.is_none() && self.from().is_some() && !fight.hero.dead
            && (self.handed.is_none() || self.heading().is_some())
            && !(atlas::is_junction(&self.at) && self.handed.is_none())
            && fight.enemies.iter().all(|e| !e.alive() || e.gap(walked).abs() > 8.0)
    }

    /// Turn round: the walk goes back the way it came, from where the hero stands (from a dead
    /// end, back up the road out of it). Past the place's end, short of the next boundary or
    /// junction, the place walked the other way starts that much behind the hero; past it, the
    /// way back runs up to it.
    pub fn turn(&mut self, fight: &Fight, walked: f64) -> Option<Go> {
        let from = self.from()?.to_owned();
        self.mark_slain(fight);
        self.keep_hero = true;
        if let Some(at) = self.handed { return self.turn_past((walked - at).max(0.0) as f32); }
        let place = self.here();
        let into = (walked - self.base) as f32;
        let to = atlas::turned(place.id, &from);
        let in_place = atlas::node(&to) == place.id;
        let back_into = if in_place { place.length - into } else { 0.0 };
        // Turned about where they stand: what was behind is ahead, as it was.
        self.carry = in_place.then(|| fight.enemies.iter().zip(&self.spawned).filter(|(e, _)| e.alive()).map(|(e, &i)| (i, e.clone(), e.gap(walked))).collect());
        self.warping = Some(to.clone());
        self.warp_into = back_into;
        // Short of where this place was walked into: the way back out of it is under way.
        if back_into > place.length {
            self.turn_on = true;
            return Some(Go::WalkOn(to, back_into - place.length));
        }
        Some(Go::WarpAt(to, back_into))
    }

    /// Turned round `behind` metres past the boundary or junction the walk on from here went
    /// through: the place it leads into, walked the other way, that far short of it.
    fn turn_past(&mut self, behind: f32) -> Option<Go> {
        let lead = self.heading()?;
        let back = match self.took {
            Some(_) => self.back_to_junction(&lead),
            None => atlas::turned(atlas::node(&lead.stop), atlas::came_from(&lead.stop)?),
        };
        self.carry = None;
        self.warping = Some(back.clone());
        if self.took.is_none() && atlas::node(&back) != atlas::node(&lead.stop) {
            // Over the boundary into a Y's junction: back into the branch, short of its start.
            self.warp_into = -behind;
            return Some(Go::WarpAt(back, -behind));
        }
        self.turn_short = Some(behind);
        Some(Go::TurnInto(back, behind))
    }

    /// The way the walk on from here takes: the branch taken at a fork, else the way on.
    fn heading(&self) -> Option<atlas::Lead> {
        match self.took { Some(b) => self.branch(b), None => self.ahead().into_iter().next() }
    }

    /// The stop whose walk on is the junction `lead` went through, walked from `lead`'s side: the
    /// Y up its trunk when the branch taken was its trunk (down it, or into the place it comes
    /// from), else the Y's junction walked in from that branch.
    fn back_to_junction(&self, lead: &atlas::Lead) -> String {
        let y = self.here().id;
        let into = atlas::node(&lead.stop);
        if into == y || into == atlas::trunk(y) { return atlas::stop(y, atlas::trunk(y)); }
        atlas::stop(y, into)
    }

    /// The road's walk on from here as the frame shown has it (`shown`: its walked metres): kept
    /// for a turn, and where it passed its boundary or junction.
    pub fn follow(&mut self, shown: f64, underway: Underway) {
        (self.underway, self.shown) = (underway, shown);
        if self.handed.is_some() || self.warping.is_some() { return; }
        if let (Some(travel), Some(handover)) = (underway.travel, underway.handover) {
            if travel >= handover { self.handed = Some(shown - f64::from(travel - handover)); }
        }
    }

    /// What is left at a place's end: its key item, else the piece laid there, else a piece of
    /// gear the hero lacks (not a boss's, nor one the smith forges, nor one laid somewhere), or
    /// souls.
    pub(crate) fn open_cache(&mut self) -> Vec<Note> {
        let mut notes = self.cache_piece();
        notes.extend(self.glass_won(crate::vials::By::Chest));
        notes
    }

    /// The piece (or souls) a place's cache holds, taken.
    fn cache_piece(&mut self) -> Vec<Note> {
        self.taken.push(self.here().id);
        if let Some(k) = self.here().relic {
            if !self.keys.contains(&k) { self.keys.push(k); return vec![Note::Found(k.name())]; }
        }
        if let Some(i) = self.here().loot.map(crate::gear::find).filter(|&i| !self.gear.owns(i)) {
            self.gear.gain(i);
            return vec![Note::Found(crate::gear::ITEMS[i].name)];
        }
        let bell = crate::gear::find("Warden's Bell");
        let kept = |i: usize| {
            let name = crate::gear::ITEMS[i].name;
            i == bell || crate::gear::RECIPES.iter().any(|r| r.item == name) || crate::atlas::PLACES.iter().any(|p| p.loot == Some(name))
        };
        let lacking: Vec<usize> = (6..crate::gear::ITEMS.len()).filter(|&i| !kept(i) && !self.gear.owns(i)).collect();
        match (!lacking.is_empty()).then(|| lacking[self.luck as usize % lacking.len()]) {
            Some(i) => { self.gear.gain(i); vec![Note::Found(crate::gear::ITEMS[i].name)] }
            None => { self.souls += 300; vec![Note::Recovered(300)] }
        }
    }

    /// Whether the place's boss has fallen this pilgrimage.
    pub fn boss_beaten(&self) -> bool { self.beaten.contains(&self.here().id) || self.claimed.contains(&self.here().id) }

    /// The pilgrimage done (the last boss beaten): the world begun again, stronger, and the walk
    /// sent back to the first shrine.
    pub fn conclude(&mut self) -> (Go, Note) {
        self.sent = true;
        self.begin_again();
        self.warping = Some(START.into());
        (Go::Warp(START.into()), Note::Pilgrimage(self.pilgrimages))
    }

    /// What holds the way on, if anything still does: the kind of the first keeper standing (for a
    /// gauntlet, of any foe standing). Only walked as laid out.
    pub fn held(&self, fight: &Fight) -> Option<Archetype> {
        if !self.forward() || self.detour.is_some() { return None; }
        let standing = fight.enemies.iter().zip(&self.spawned).filter(|(e, _)| e.alive());
        match self.here().hold {
            atlas::Hold::Free => None,
            atlas::Hold::Clear => standing.map(|(e, _)| e.kind).next(),
            atlas::Hold::Keepers(k) => standing.filter(|(_, i)| k.contains(i)).map(|(e, _)| e.kind).next(),
        }
    }

    /// At the place's end with the way on held, or none (a dead end): the walk goes no further.
    pub fn held_at_end(&self, fight: &Fight, walked: f64) -> bool {
        walked - self.base >= self.here().length as f64 && (self.held(fight).is_some() || self.at_dead_end())
    }

    /// No living enemy within `QUIET` metres, ahead or behind.
    pub(crate) fn quiet(fight: &Fight, walked: f64) -> bool {
        fight.enemies.iter().all(|e| !e.alive() || e.gap(walked).abs() > QUIET)
    }

    /// Where the veil hangs across the road (road metres), while this place's boss stands:
    /// `VEIL_AHEAD` past the metre where the boss waits, so a hero held at that metre faces it.
    /// Turned round past it, it hangs behind them, still there.
    pub fn veil(&self) -> Option<f64> {
        let (at, _) = self.boss_here()?;
        (!self.boss_beaten() && !self.strikes()).then(|| self.road_at(at + VEIL_AHEAD))
    }

    /// Whether the walk is held at the veil: the boss here stands, the hero came in short of
    /// it, and has walked up to it. Back is still free.
    pub fn held_at_veil(&self, walked: f64) -> bool {
        let Some((at, _)) = self.boss_here() else { return false };
        let mark = self.along(at);
        !self.boss_beaten() && !self.strikes() && self.entered < mark && walked - self.base >= mark as f64
    }

    /// Whether E steps through the veil: held at it (or a step short of it), nothing on the hero.
    pub fn at_veil(&self, fight: &Fight, walked: f64) -> bool {
        let Some((at, _)) = self.boss_here() else { return false };
        let mark = self.along(at);
        !self.boss_beaten() && !self.strikes() && !fight.hero.dead && self.warping.is_none() && Self::quiet(fight, walked)
            && self.entered < mark && walked - self.base >= (mark - VEIL_REACH) as f64
    }

    /// Whether this place's boss has been faced before; facing it now marks it so.
    pub fn face_boss(&mut self) -> bool {
        let id = self.here().id;
        let before = self.faced.contains(&id);
        if !before { self.faced.push(id); }
        before
    }

    /// The boss here, if one steps out now: this place has one not yet beaten, no enemy is on the
    /// hero (one run past may still be coming), and the hero has walked as far as it waits.
    pub fn boss_due(&self, fight: &Fight, walked: f64) -> Option<&'static atlas::Boss> {
        let (at, boss) = self.boss_here()?;
        let mark = self.along(at);
        (!self.boss_beaten() && Self::quiet(fight, walked) && self.entered < mark && walked - self.base >= mark as f64).then_some(boss)
    }

    /// The boss fell: its souls, what it leaves, and the road open again.
    pub fn boss_felled(&mut self) -> Vec<Note> {
        let Some((_, boss)) = self.boss_here() else { return Vec::new() };
        if std::ptr::eq(boss, &crate::alley::HOODED_THING) { return self.alley_fled(); }
        if !self.boss_beaten() { self.beaten.push(self.here().id); }
        // Whoever stood aside for the fight has their say about how it went; whoever was called
        // to it goes home.
        (self.lingering, self.called) = (None, None);
        let n = (boss.souls as f32 * self.kit().souls * (1.0 + 0.5 * self.pilgrimages as f32)).round() as u32;
        self.souls += n;
        let mut notes = vec![Note::Vanquished(boss.name(), n)];
        if let Some(k) = boss.key.filter(|k| !self.keys.contains(k)) {
            self.keys.push(k);
            notes.push(Note::Found(k.name()));
        }
        notes.extend(self.glass_won(crate::vials::By::Boss));
        if let Some(r) = boss.remnant {
            self.remnants.push(r);
            notes.push(Note::Found(r));
        }
        if std::ptr::eq(boss, &atlas::BELL_WARDEN) {
            let bell = crate::gear::find("Warden's Bell");
            if self.gear.gain(bell) { notes.push(Note::Found(crate::gear::ITEMS[bell].name)); }
        }
        notes.extend(self.echo_leaves(boss));
        notes.extend(self.story_moves());
        notes
    }

    /// Whether ambush `k` of the place the hero is in has been beaten.
    fn beaten_ambush(&self, k: usize) -> bool { self.cleared.contains(&(self.here().id, k)) }

    /// Whether this place's cache is a trap not yet sprung and beaten.
    fn trapped(&self) -> bool { self.here().ambushes.iter().enumerate().any(|(k, a)| a.chest && !self.beaten_ambush(k)) }

    /// The ambush to spring where the hero stands, if any: past its metre, not beaten, none of the
    /// road's own enemies close (the ambush is its own fight), the hero on their feet.
    pub fn ambush_due(&self, fight: &Fight, walked: f64) -> Option<usize> {
        if fight.hero.dead || self.warping.is_some() { return None; }
        if fight.enemies.iter().any(|e| e.alive() && e.gap(walked).abs() < 8.0) { return None; }
        let into = walked - self.base;
        self.here().ambushes.iter().enumerate()
            .find(|&(k, a)| !self.beaten_ambush(k) && self.entered < self.along(a.at) && into >= self.along(a.at) as f64 && (!a.chest || (!self.cache_taken() && fight.enemies.iter().all(|e| !e.alive()))))
            .map(|(k, _)| k)
    }

    /// Ambush `k`'s foes, where they stand along the road from the hero (mirrored walking the
    /// place the other way).
    pub fn ambush_foes(&self, k: usize) -> Vec<(Archetype, f32)> {
        let flip = if self.forward() { 1.0 } else { -1.0 };
        self.here().ambushes[k].foes.iter().map(|&(kind, s)| (kind, s * flip)).collect()
    }

    /// Ambush `k` beaten: remembered, its dead's souls and leavings, and a trapped chest opened.
    pub fn ambush_cleared(&mut self, k: usize, fallen: Vec<Archetype>) -> Vec<Note> {
        let at = (self.here().id, k);
        if !self.cleared.contains(&at) { self.cleared.push(at); }
        let mut notes = self.felled(fallen);
        if self.here().ambushes[k].chest && !self.cache_taken() { notes.extend(self.open_cache()); }
        notes
    }

    /// Where the hero stands on the road: the walk's metres plus the hero's own distance.
    pub(crate) fn standing(walked: f64) -> f64 { walked + HERO_D as f64 }

    /// Whether the hero can rest now: by the shrine with no enemy close.
    pub fn can_rest(&self, fight: &Fight, walked: f64) -> bool {
        let Some(s) = self.shrine() else { return false };
        let near = (s - Self::standing(walked)).abs() <= SHRINE_REACH as f64;
        near && !fight.hero.dead && Self::quiet(fight, walked)
    }

    /// Rest at the shrine: whole again, the world's enemies back, death wakes here, and the
    /// shrine one to travel to.
    pub fn rest(&mut self, fight: &mut Fight) -> Vec<Note> {
        self.checkpoint = self.at.clone();
        if !self.rested.contains(&self.at) { self.rested.push(self.at.clone()); }
        self.slain.clear();
        // Worn again as the hero now is: shrine glass found since melts into its flasks here.
        fight.hero.stats.kit = self.kit();
        fight.hero = fight.hero.renewed();
        fight.enemies = self.foes();
        vec![Note::Rested]
    }

    /// What a save keeps of the world (`profile::record`): the progress the road cannot give
    /// back, by name (stops, places, pieces, keys, kinds of enemy) so that a save outlives the
    /// lists being reordered. Where on the road the hero stood, what lies slain until the next
    /// rest, and a walk, fork or detour under way are not kept: a pilgrimage carried on wakes at
    /// the shrine last rested at (`resume`).
    pub fn saved(&self) -> serde_json::Value {
        use serde_json::json;
        let item = |i: usize| crate::gear::ITEMS[i].name;
        let fallen: serde_json::Map<String, serde_json::Value> = Archetype::ALL.iter().map(|k| (format!("{k:?}"), json!(self.fallen[*k as usize]))).collect();
        json!({
            "checkpoint": self.checkpoint, "rested": self.rested, "souls": self.souls,
            "stain": self.stain.as_ref().map(|s| json!({ "stop": s.stop, "at": s.at, "souls": s.souls })),
            "pilgrimages": self.pilgrimages, "shards": self.shards, "met_keeper": self.met_keeper, "met_smith": self.met_smith, "talks": self.talks, "heard": self.heard, "claimed": self.claimed, "meetings": self.meetings, "guild": self.guild.saved(), "hunters": self.hunters.saved(), "answers": self.answers,
            "gear": {
                "owned": self.gear.owned.iter().map(|&i| item(i)).collect::<Vec<_>>(),
                "worn": self.gear.worn.iter().map(|w| w.map(item)).collect::<Vec<_>>(),
                "one_hand": self.gear.one_hand,
                "levels": self.gear.levels.iter().map(|(&i, n)| (item(i).to_owned(), json!(n))).collect::<serde_json::Map<_, _>>(),
                "capstones": self.gear.capstones.iter().map(|(&i, c)| (item(i).to_owned(), json!(c.name()))).collect::<serde_json::Map<_, _>>(),
            },
            "ammo": { "arrows": self.ammo.arrows, "bolts": self.ammo.bolts },
            "fallen": fallen, "luck": self.luck,
            "beaten": self.beaten, "faced": self.faced, "keys": self.keys.iter().map(|k| k.name()).collect::<Vec<_>>(), "remnants": self.remnants,
            "opened": self.opened, "seen": self.seen, "explored": self.explored, "taken": self.taken,
            "cleared": self.cleared.iter().map(|(p, i)| json!([p, i])).collect::<Vec<_>>(),
        })
    }

    /// A world set out into from a save (`saved`), with the walk at `walked`; `resume` then takes
    /// the hero to the shrine it last rested at. None when `v` is not a world's save. Names this
    /// game no longer knows (a piece or a place since renamed) are passed over; a shrine it no
    /// longer knows wakes the hero at the start.
    pub fn restored(walked: f64, v: &serde_json::Value) -> Option<World> {
        v.as_object()?;
        let mut w = World::new(walked);
        let num = |k: &str| v[k].as_u64().unwrap_or(0) as u32;
        let strs = |x: &serde_json::Value| -> Vec<String> { x.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect()).unwrap_or_default() };
        let names = |k: &str| -> Vec<&'static str> { strs(&v[k]).iter().map(|s| intern(s)).collect() };
        let known = |stop: &String| atlas::place_of(atlas::node(stop)).is_some();
        w.checkpoint = v["checkpoint"].as_str().map(str::to_owned).filter(known).unwrap_or_else(|| START.into());
        w.rested = strs(&v["rested"]).into_iter().filter(known).collect();
        w.souls = num("souls");
        w.stain = v["stain"].as_object().and_then(|s| Some(Stain { stop: s.get("stop")?.as_str()?.to_owned(), at: s.get("at")?.as_f64()? as f32, souls: s.get("souls")?.as_u64()? as u32 }))
            .filter(|s| known(&s.stop));
        w.pilgrimages = num("pilgrimages");
        // A save from before shards had grades: what it carried, the common grade.
        w.shards = match v["shards"].as_array() {
            Some(a) => std::array::from_fn(|g| a.get(g).and_then(|n| n.as_u64()).unwrap_or(0) as u32),
            None => [num("shards"), 0, 0],
        };
        w.met_keeper = v["met_keeper"].as_bool().unwrap_or(false);
        w.met_smith = v["met_smith"].as_bool().unwrap_or(false);
        for (n, t) in w.talks.iter_mut().enumerate() { *t = v["talks"][n].as_u64().unwrap_or(0) as u32; }
        w.claimed = strs(&v["claimed"]).iter().filter_map(|id| atlas::PLACES.iter().find(|p| p.id == id).map(|p| p.id)).collect();
        w.meetings = strs(&v["meetings"]).iter().filter_map(|id| crate::visitors::Visit::every().find(|m| m.id == id).map(|m| m.id)).collect();
        w.answers = strs(&v["answers"]).iter().filter_map(|id| World::answer_named(id)).collect();
        for (n, h) in w.heard.iter_mut().enumerate() { *h = u32::try_from(v["heard"][n].as_u64().unwrap_or(0)).unwrap_or(0); }
        w.guild = crate::guild::Guild::restored(&v["guild"]);
        w.hunters = crate::hunters::Hunters::restored(&v["hunters"]);
        let piece = |name: &str| crate::gear::ITEMS.iter().position(|i| i.name == name);
        if let Some(owned) = v["gear"]["owned"].as_array() {
            w.gear.owned = owned.iter().filter_map(|n| piece(n.as_str()?)).collect();
            for (n, slot) in w.gear.worn.iter_mut().enumerate() {
                *slot = v["gear"]["worn"][n].as_str().and_then(piece);
            }
            let owned = w.gear.owned.clone();
            for slot in w.gear.worn.iter_mut() { if slot.is_some_and(|i| !owned.contains(&i)) { *slot = None; } }
            w.gear.one_hand = v["gear"]["one_hand"].as_bool().unwrap_or(false);
            if let Some(levels) = v["gear"]["levels"].as_object() {
                w.gear.levels = levels.iter().filter_map(|(n, l)| Some((piece(n)?, l.as_u64()? as u32))).filter(|&(i, l)| l > 0 && w.gear.owned.contains(&i)).collect();
            }
            if let Some(caps) = v["gear"]["capstones"].as_object() {
                w.gear.capstones = caps.iter().filter_map(|(n, c)| Some((piece(n)?, crate::capstone::Capstone::named(c.as_str()?)?)))
                    .filter(|&(i, c)| w.gear.owned.contains(&i) && crate::capstone::Capstone::choices(&crate::gear::ITEMS[i]).contains(&c)).collect();
            }
        }
        // A save from before arrows were carried sets out with what a new pilgrimage does.
        if v["ammo"].is_object() { w.ammo = crate::fight::Quiver { arrows: v["ammo"]["arrows"].as_u64().unwrap_or(0) as u32, bolts: v["ammo"]["bolts"].as_u64().unwrap_or(0) as u32, imbued: 0 }; }
        for k in Archetype::ALL { w.fallen[k as usize] = v["fallen"][format!("{k:?}")].as_u64().unwrap_or(0) as u32; }
        w.luck = num("luck");
        w.beaten = names("beaten");
        w.faced = names("faced");
        w.keys = strs(&v["keys"]).iter().filter_map(|n| Key::ALL.into_iter().find(|k| k.name() == n)).collect();
        // A pilgrimage kept from before the Screamer left the rune has it all the same.
        if w.beaten.contains(&"hollowmere") && !w.keys.contains(&Key::Rune) { w.keys.push(Key::Rune); }
        w.remnants = names("remnants");
        w.opened = names("opened");
        w.seen = names("seen");
        if !w.seen.contains(&atlas::node(START)) { w.seen.push(atlas::node(START)); }
        w.explored = names("explored");
        w.taken = names("taken");
        w.cleared = v["cleared"].as_array().map(|a| a.iter().filter_map(|c| Some((intern(c[0].as_str()?), c[1].as_u64()? as usize))).collect()).unwrap_or_default();
        Some(w)
    }

    /// The title of the place the hero will wake in.
    pub fn checkpoint_title(&self) -> &'static str { atlas::place_of(atlas::node(&self.checkpoint)).map_or(atlas::PLACES[0].title, |p| p.title) }

    /// Back on the road from a save: to the shrine last rested at, as after a death, with nothing
    /// lost on the way.
    pub fn resume(&mut self) -> Go { self.wake() }

    /// Off to the shrine last rested at, the hero standing where they stood to rest: as far short
    /// of it as a place walked its own way starts (a shrine rested at walking the place backwards
    /// is near its far end, not where the walk comes in).
    pub(crate) fn wake(&mut self) -> Go {
        let cp = self.checkpoint.clone();
        let into = atlas::place_of(atlas::node(&cp)).map_or(0.0, |p| {
            let m = p.shrine.unwrap_or(0.0);
            let fwd = atlas::came_from(&cp).is_none_or(|f| atlas::forward(p.id, f));
            if fwd { 0.0 } else { (p.length - 2.0 * m).max(0.0) }
        });
        self.warping = Some(cp.clone());
        self.warp_into = into;
        if into > 0.0 { Go::WarpAt(cp, into) } else { Go::Warp(cp) }
    }

    /// The shrines to travel to, by place title (one a place, whichever way it was rested in).
    pub fn travel_list(&self) -> Vec<(&'static str, String)> {
        let mut out: Vec<(&'static str, String)> = Vec::new();
        for s in &self.rested {
            let p = atlas::place_of(atlas::node(s)).unwrap_or(&atlas::PLACES[0]);
            if !out.iter().any(|(t, _)| *t == p.title) { out.push((p.title, s.clone())); }
        }
        out
    }

    /// Wake at another shrine rested at, the next time the hero rises.
    pub fn travel(&mut self, stop: &str) { if self.rested.iter().any(|s| s == stop) { self.checkpoint = stop.to_owned(); } }

    /// Where the smith waits on the road here, metres, until he is met.
    pub fn smith_at(&self) -> Option<f64> {
        if self.met_smith { return None; }
        self.here().smith.map(|m| self.road_at(m))
    }

    /// Whether the hero stands by the smith with no enemy close.
    pub fn can_meet_smith(&self, fight: &Fight, walked: f64) -> bool {
        let Some(s) = self.smith_at() else { return false };
        (s - Self::standing(walked)).abs() <= SHRINE_REACH as f64 && !fight.hero.dead && Self::quiet(fight, walked)
    }

    /// The dead end the hero stands in or is walking into, and the road metres of `m` (given the
    /// place) metres into it. One walked into begins where the walk arrives there, entered forward.
    pub(crate) fn dead_end_at(&self, m: impl Fn(&Place) -> f32) -> Option<(&'static Place, f64)> {
        if self.at_dead_end() { return Some((self.here(), self.road_at(m(self.here())))); }
        let Underway { travel: Some(travel), length: Some(length), .. } = self.underway else { return None };
        let lead = self.heading()?;
        let next = atlas::place_of(lead.road.1)?;
        atlas::leads(next.id, lead.road.0).is_empty().then(|| (next, self.shown + f64::from(length - travel + HERO_D + m(next))))
    }

    /// Where the road ends ahead, walked into a dead end or on the way into one: road metres of
    /// the wall across it.
    pub fn end_wall(&self) -> Option<f64> { self.dead_end_at(|p| p.length + END_PAST).map(|(_, at)| at) }

    /// The door the walk on goes through, while a crossing through one is under way: metres from
    /// the hero to it (negative once passed) and its half width.
    fn door_ahead(&self) -> Option<(f32, f32)> {
        let Underway { travel: Some(travel), handover: Some(handover), .. } = self.underway else { return None };
        let lead = self.heading()?;
        let atlas::Via::Door(half, _) = atlas::edge(lead.road.0, lead.road.1)?.via else { return None };
        Some((handover - travel - HERO_D, half))
    }

    /// Whether the road narrows to its middle lane: a door close ahead, or the hero in it.
    pub fn narrows(&self) -> bool { self.door_ahead().is_some_and(|(gap, _)| gap > -DOOR_PAST && gap < DOOR_FUNNEL) }

    /// Whether the wall beside a door stops a hero standing `x` metres out: at its face, not
    /// inside the opening.
    pub fn held_at_door(&self, x: f32) -> bool {
        self.door_ahead().is_some_and(|(gap, half)| (0.0..DOOR_FACE).contains(&gap) && x.abs() + HERO_HALF > half)
    }

    /// Where the hero is drawn across the road, `x` metres out: drawn into the middle of a door
    /// as they pass through it, so they go through the opening and not the wall beside it.
    pub fn through_door(&self, x: f32) -> f32 {
        let Some((gap, _)) = self.door_ahead() else { return x };
        let k = ((DOOR_FUNNEL - gap.abs()) / (DOOR_FUNNEL - 1.0)).clamp(0.0, 1.0);
        x * (1.0 - k * k * (3.0 - 2.0 * k))
    }

    /// Where Gruk stands behind his counter at the guild hall's end, road metres: within reach
    /// of the hero held there, and already there as the room comes into view through its door.
    pub fn gruk_at(&self) -> Option<f64> {
        self.dead_end_at(|p| p.length + GRUK_PAST).filter(|(p, _)| p.id == crate::guild::HALL).map(|(_, at)| at)
    }

    /// Whether the hero stands at Gruk's counter with no enemy close.
    pub fn can_meet_gruk(&self, fight: &Fight, walked: f64) -> bool {
        let Some(at) = self.gruk_at().filter(|_| self.at_dead_end()) else { return false };
        (at - Self::standing(walked)).abs() <= SHRINE_REACH as f64 && !fight.hero.dead && Self::quiet(fight, walked)
    }

    /// Up from rest: back to the shrine's place (its enemies there again, as resting brings them).
    pub fn leave_rest(&mut self) -> Go { self.wake() }

    /// Spend souls on one level of `which` (0 vigor, 1 endurance, 2 might, 3 dexterity, 4 focus);
    /// the new level.
    pub fn rise(&mut self, fight: &mut Fight, which: usize) -> Option<Note> {
        let st = &mut fight.hero.stats;
        let cost = st.level_cost();
        if self.souls < cost { return None; }
        self.souls -= cost;
        match which { 0 => st.vigor += 1, 1 => st.endurance += 1, 2 => st.might += 1, 3 => st.dexterity += 1, _ => st.focus += 1 }
        let lvl = st.level();
        fight.hero = crate::fight::Hero { stats: *st, ..fight.hero.renewed() };
        Some(Note::Rose(lvl))
    }

    /// What the smith could forge from the remnants carried: each recipe whose remnant is held and
    /// whose piece is not yet owned, and whether it can be paid for.
    pub fn craftable(&self) -> Vec<(&'static crate::gear::Recipe, bool)> {
        crate::gear::RECIPES.iter()
            .filter(|r| self.remnants.contains(&r.remnant) && !self.gear.owns(crate::gear::find(r.item)))
            .map(|r| (r, self.souls >= r.echoes && self.shards[r.grade] >= r.shards)).collect()
    }

    /// The smith forges recipe `r`'s piece: the remnant, the echoes and the shards spent.
    pub fn craft(&mut self, r: &'static crate::gear::Recipe) -> Option<Note> {
        let at = self.remnants.iter().position(|x| *x == r.remnant)?;
        let item = crate::gear::find(r.item);
        if self.gear.owns(item) || self.souls < r.echoes || self.shards[r.grade] < r.shards { return None; }
        self.remnants.remove(at);
        self.souls -= r.echoes;
        self.shards[r.grade] -= r.shards;
        self.gear.gain(item);
        Some(Note::Crafted(r.item))
    }

    /// What the smith fletches for souls: a bundle of arrows, and one of bolts (how many, the price).
    pub const FLETCH: [(&'static str, u32, u32); 2] = [("arrows", 10, 40), ("bolts", 6, 60)];

    /// Buy bundle `which` of `FLETCH` for the hero's quiver; None when it cannot be paid for, or
    /// there is no room for it.
    pub fn fletch(&mut self, fight: &mut Fight, which: usize) -> Option<Note> {
        let (what, n, price) = *Self::FLETCH.get(which)?;
        let q = &mut fight.hero.quiver;
        let full = if which == 0 { q.arrows >= crate::fight::Quiver::ARROWS } else { q.bolts >= crate::fight::Quiver::BOLTS };
        if self.souls < price || full { return None; }
        self.souls -= price;
        if which == 0 { q.add(n, 0) } else { q.add(0, n) }
        self.ammo = crate::fight::Quiver { imbued: 0, ..*q };
        Some(Note::Fletched(n, what))
    }

    /// The places (`gear::SLOTS`) whose worn piece the smith can reinforce: the weapon, the
    /// offhand and the armour, as far as something is worn there.
    pub fn forgeable(&self) -> Vec<usize> {
        (0..crate::gear::SLOTS.len()).filter(|&p| self.gear.worn[p].is_some() && crate::gear::SLOTS[p].0.most() > 0).collect()
    }

    /// What the smith asks to take the piece worn at `place` one level further: (grade, shards,
    /// echoes). None when nothing is worn there or it is as far as it goes.
    pub fn reinforce_cost(&self, place: usize) -> Option<(usize, u32, u32)> {
        let i = self.gear.worn.get(place).copied().flatten()?;
        let (slot, n) = (crate::gear::ITEMS[i].slot, self.gear.level(i));
        (n < slot.most()).then(|| slot.reinforcing(n + 1))
    }

    /// The shards carried, as the HUD shows them: ash / ember / star.
    pub fn shards_line(&self) -> String { format!("{}/{}/{}", self.shards[0], self.shards[1], self.shards[2]) }

    /// The shards carried, as the smith counts them: "Ash Shards 3, Ember 1, Star 0".
    pub fn shards_text(&self) -> String {
        format!("Ash Shards {}, Ember {}, Star {}", self.shards[0], self.shards[1], self.shards[2])
    }

    /// Shards of `grade` named for a price: "1 Ash Shard", "3 Ember Shards".
    pub fn shards_named(grade: usize, n: u32) -> String {
        format!("{n} {}{}", crate::gear::SHARDS[grade], if n == 1 { "" } else { "s" })
    }

    /// Whether the piece worn at `place` is at its top and waits for its capstone.
    pub fn cappable(&self, place: usize) -> bool {
        self.gear.worn.get(place).copied().flatten().is_some_and(|i| self.reinforce_cost(place).is_none() && !self.gear.capstones.contains_key(&i) && !crate::capstone::Capstone::choices(&crate::gear::ITEMS[i]).is_empty())
    }

    /// The smith gives the piece worn at `place`, at its top, capstone `cap`; the hero's kit is
    /// worn again with it.
    pub fn cap(&mut self, fight: &mut Fight, place: usize, cap: crate::capstone::Capstone) -> Option<Note> {
        let i = self.gear.worn.get(place).copied().flatten()?;
        if !self.gear.cap(i, cap) { return None; }
        fight.hero.stats.kit = self.kit();
        Some(Note::Capped(crate::gear::ITEMS[i].name, cap.name()))
    }

    /// The smith reinforces the piece worn at `place` one level, if it can be paid for; the
    /// hero's kit is worn again with it.
    pub fn reinforce(&mut self, fight: &mut Fight, place: usize) -> Option<Note> {
        let (grade, shards, souls) = self.reinforce_cost(place)?;
        if self.souls < souls || self.shards[grade] < shards { return None; }
        let i = self.gear.worn[place]?;
        self.gear.reinforce(i);
        self.souls -= souls;
        self.shards[grade] -= shards;
        fight.hero.stats.kit = self.kit();
        Some(Note::Forged(crate::gear::ITEMS[i].name, self.gear.level(i)))
    }

    /// The hero fell: the souls stay where they fell (one chance to fetch them; an older stain is
    /// lost), the world's enemies stand again, and the walk wakes at the last shrine. Fallen down a
    /// side path, it is open to walk again, for the souls and whatever it still holds.
    pub fn died(&mut self, walked: f64) -> Go {
        let into = (Self::standing(walked) - self.base - HERO_D as f64).max(0.0) as f32;
        let at = self.along(into);
        self.stain = (self.souls > 0).then(|| Stain { stop: self.here().id.into(), at, souls: self.souls });
        self.souls = 0;
        self.slain.clear();
        (self.lingering, self.called) = (None, None);
        self.hunters_fall_with_the_hero();
        if self.detour.take().is_some() {
            let side = self.here().id;
            self.explored.retain(|&e| e != side);
        }
        self.wake()
    }

    /// Where the signpost stands across the road, metres: in the middle until a branch is taken;
    /// as the camera turns onto it, everything past the junction swings the other way, the island
    /// and its signpost with it.
    pub fn signpost_x(&self, walked: f64) -> f32 { self.past_junction_x(walked, SIGNPOST_PAST) }

    /// How far across the road a thing `past` metres beyond the junction has swung, metres: the
    /// way the branch TAKEN turns (the only open one is taken whichever side the hero leans to).
    pub fn past_junction_x(&self, walked: f64, past: f32) -> f32 {
        let (Some(at), Some(b)) = (self.chose_at, self.took) else { return 0.0 };
        let t = ((walked - at) as f32 / STEER_M).clamp(0.0, 1.0);
        let steer = t * t * (3.0 - 2.0 * t);
        let side = match b { Branch::Left => -1.0, Branch::Right => 1.0 };
        -side * steer * FORK_SLOPE * past
    }

    /// The fork ahead's barred ways, once it is in view: which branch, and whether a key opens it
    /// (else it opens only from its far side). For the barrier stood across its mouth.
    pub fn barred_branches(&self) -> Vec<(Branch, bool)> {
        self.gates_ahead().into_iter().filter(|g| !g.2).map(|(b, keyed, _)| (b, keyed)).collect()
    }

    /// Every gate on the fork ahead's ways, once it is in view, barred or opened: which branch,
    /// whether it is a key's gate (else a shortcut's door), and whether it stands open. An opened
    /// one still stands, its door swung back, so the way reads the same before and after.
    pub fn gates_ahead(&self) -> Vec<(Branch, bool, bool)> {
        if self.signpost.is_none() { return Vec::new(); }
        [Branch::Left, Branch::Right].into_iter().filter_map(|b| {
            let l = self.branch(b)?;
            let keyed = atlas::gated(l.road.0, l.road.1)?;
            Some((b, keyed, self.barred_to(&l).is_none()))
        }).collect()
    }

    /// Whether the walk waits at the fork for the hero to pick a side (still in the middle lane at
    /// the last moment, or leaning toward a way that is barred): the junction never chooses for
    /// them.
    pub fn holds_at_fork(&self, choose_within: Option<f32>, lane: i32) -> bool {
        let barred = match lane { l if l < 0 => self.branch_barred(Branch::Left), l if l > 0 => self.branch_barred(Branch::Right), _ => true };
        !self.chose && barred && choose_within.is_some_and(|l| l <= HOLD_M)
    }

    /// The one way open at the fork ahead, when the other is barred: the walk takes it whichever
    /// lane the hero holds (a fork with a locked door down one side is no choice).
    fn only_open(&self) -> Option<Branch> {
        match (self.branch_barred(Branch::Left), self.branch_barred(Branch::Right)) {
            (true, false) => Some(Branch::Right),
            (false, true) => Some(Branch::Left),
            _ => None,
        }
    }

    /// The ways on at this stop's end, left first (`atlas::leads`: the same ways the journey was
    /// compiled with, so what the hero is offered is what the walk does).
    /// Walked into a dead end: no way on from its end.
    fn at_dead_end(&self) -> bool { self.from().is_some() && self.ahead().is_empty() }

    pub(crate) fn ahead(&self) -> Vec<atlas::Lead> { self.from().map_or(Vec::new(), |f| atlas::leads(self.here().id, f)) }

    /// Branch `b` of the fork ahead.
    fn branch(&self, b: Branch) -> Option<atlas::Lead> { self.ahead().into_iter().nth(match b { Branch::Left => 0, Branch::Right => 1 }) }

    /// Why a way on cannot be walked now, if it cannot, and the place whose road is barred.
    fn barred_to(&self, l: &atlas::Lead) -> Option<(&'static str, Barred)> {
        let (from, to) = l.road;
        atlas::barred(from, to, &self.keys, &self.opened).map(|why| (to, why))
    }
    /// With the rune carried, the branch of the fork ahead that leads sooner to the next region
    /// boss still standing (the nearest region first): where the story goes on. None when both
    /// are as near, or without the rune.
    pub fn rune_hums(&self) -> Option<Branch> {
        if !self.keys.contains(&Key::Rune) { return None; }
        let standing = |p: &atlas::Place| p.boss.is_some_and(|(_, b)| !b.mini && b.sought) && !self.beaten.contains(&p.id) && !self.claimed.contains(&p.id);
        let next = atlas::PLACES.iter().filter(|p| standing(p)).map(|p| p.region).min()?;
        let steps = |b| self.branch(b).and_then(|l| atlas::Place::steps(l.road.0, l.road.1, |p| standing(p) && p.region == next));
        match (steps(Branch::Left), steps(Branch::Right)) {
            (Some(l), Some(r)) if l != r => Some(if l < r { Branch::Left } else { Branch::Right }),
            (Some(_), None) => Some(Branch::Left),
            (None, Some(_)) => Some(Branch::Right),
            _ => None,
        }
    }
    /// Whether branch `b` of the fork ahead is barred by the heart alone (`hollowed`).
    pub(crate) fn wants_only_blood(&self, b: Branch) -> bool {
        self.branch(b).and_then(|l| self.barred_to(&l)).is_some_and(|(_, why)| why == Barred::Keys(vec![Key::Heart]))
    }
    fn branch_barred(&self, b: Branch) -> bool { self.branch(b).is_none_or(|l| self.barred_to(&l).is_some()) }
    fn shown(l: &atlas::Lead) -> &'static str { atlas::place_of(l.shown).map_or("", |p| p.title) }

    /// A barred way, as a note.
    fn barred_note(to: &'static str, why: Barred) -> Note {
        let title = atlas::place_of(to).map_or(to, |p| p.title);
        Note::Barred(title, match why { Barred::Keys(k) => k.first().copied(), Barred::Shut => None })
    }

    /// Walking a way on: a shortcut it crosses that opens from this side opens.
    fn pass(&mut self, l: &atlas::Lead) -> Vec<Note> {
        let mut notes = Vec::new();
        let (from, to) = l.road;
        let Some(e) = atlas::edge(from, to).filter(|e| e.gate == atlas::Gate::Shortcut && e.a == from) else { return notes };
        if !self.opened.contains(&e.a) {
            self.opened.push(e.a);
            notes.push(Note::Opened(atlas::place_of(e.b).map_or(e.b, |p| p.title)));
        }
        notes
    }

    pub fn felled(&mut self, kinds: impl IntoIterator<Item = Archetype>) -> Vec<Note> {
        let mut notes = Vec::new();
        let tier = atlas::REGIONS[self.here().region].tier.saturating_sub(1) as f32;
        let more = self.kit().souls * (1.0 + 0.6 * tier) * (1.0 + 0.5 * self.pilgrimages as f32);
        let region = atlas::REGIONS[self.here().region].tier;
        let tally = self.kit().tally;
        for k in kinds {
            self.souls += (k.souls() as f32 * more).round() as u32;
            // The debt a parry ran up, collected on the next to fall.
            if std::mem::take(&mut self.parried) && tally > 0.0 {
                let due = (k.souls() as f32 * more * tally).round() as u32;
                self.souls += due;
                notes.push(Note::Tallied(due));
            }
            // The first of a kind leaves the first piece of its kind the hero lacks; after that,
            // one in eight leaves one of them.
            let n = &mut self.fallen[k as usize];
            *n += 1;
            let first = *n == 1;
            self.luck = self.luck.wrapping_add(1);
            let roll = self.luck.wrapping_mul(0x9E37_79B9).rotate_left(13).wrapping_mul(0x85EB_CA6B);
            notes.extend(self.shards_from(k, region, roll));
            let lacking: Vec<usize> = crate::gear::drops(k).iter().map(|n| crate::gear::find(n)).filter(|&i| !self.gear.owns(i)).collect();
            if lacking.is_empty() || !(first || roll % 8 == 0) { continue; }
            let i = if first { lacking[0] } else { lacking[(roll >> 8) as usize % lacking.len()] };
            self.gear.gain(i);
            notes.push(Note::Found(crate::gear::ITEMS[i].name));
        }
        notes
    }

    /// The pilgrimage done: everything the world holds is back, the bosses and the roads' locks
    /// with it, and the hero sets out again from the start, the world harder.
    fn begin_again(&mut self) {
        self.pilgrimages += 1;
        self.beaten.clear();
        // The roads' keys are left behind; the flasks' glass goes on with the hero.
        self.keys.retain(|k| matches!(k, Key::Glass(_)));
        self.opened.clear();
        self.slain.clear();
        self.cleared.clear();
        self.explored.clear();
        self.taken.clear();
        self.rested.clear();
        // The heart is paid again, so what Helena drew of it can be drawn again.
        self.answers.retain(|a| !["helena_draws_heart", "helena_heart_kept"].contains(a));
        self.meetings.retain(|m| *m != "helena_sees_the_heart");
        self.forget_the_finale();
        self.checkpoint = START.into();
        self.warping = Some(START.into());
        self.warp_into = 0.0;
    }

    /// Each tick: what the road should do, and what to say. `stop` is the walk's stop now,
    /// `choose_within` the metres left to choose a branch (None: no fork waiting).
    pub fn tick(&mut self, fight: &mut Fight, walked: f64, stop: Option<&str>, choose_within: Option<f32>) -> (Option<Go>, Vec<Note>) {
        let mut notes = Vec::new();
        let level = fight.hero.stats.level();
        // A warp waits until the walk shows it; then the place starts over.
        if let Some(w) = &self.warping {
            if stop != Some(w.as_str()) { return (None, notes); }
            let w = w.clone();
            self.warping = None;
            let mut into = std::mem::take(&mut self.warp_into);
            let short = self.turn_short.take();
            if let Some(short) = short {
                // Turned past a boundary: the place's end lies `short` less its hand-over behind
                // the hero, or ahead of them; past it, its walk on is already under way.
                let length = atlas::length_of(&w);
                into = length + self.underway.handover.map_or(0.0, |h| h - short);
            }
            let (foes, n) = self.arrive(&w, walked, into, level);
            if short.is_some() || std::mem::take(&mut self.turn_on) { self.sent = self.underway.travel.is_some(); }
            fight.enemies = foes;
            if !std::mem::take(&mut self.keep_hero) { fight.hero = fight.hero.renewed(); }
            if let Some(carry) = self.carry.take() {
                // Seen the other way: left is right, behind is ahead.
                fight.hero.lane = -fight.hero.lane;
                fight.hero.x = -fight.hero.x;
                for (k, &i) in self.spawned.iter().enumerate() {
                    let Some((_, e, gap)) = carry.iter().find(|c| c.0 == i) else { continue };
                    let mut e = e.clone();
                    e.s = walked + (HERO_D - gap) as f64;
                    (e.lane, e.drift) = (-e.lane, -e.drift);
                    if let Some(idle) = e.idle.as_mut() { (idle.to, idle.facing) = (-idle.to, -idle.facing); }
                    if matches!(e.ai, Ai::Attack { .. }) { e.ai = Ai::Wait(20); }
                    fight.enemies[k] = e;
                }
            }
            // Back from a side path: where the hero left the road, its dead still dead.
            if let Some(d) = self.detour.take_if(|d| d.from == w) {
                self.base = walked - d.at;
                self.entered = d.at as f32;
                fight.enemies = d.foes.into_iter().map(|mut e| { e.s += self.base; e }).collect();
                return (None, vec![Note::Arrived(self.here().title)]);
            }
            return (None, n);
        }
        self.mark_slain(fight);
        // Past a fork, on the branch's lane (still this place's ground): on into where it leads.
        if let Some(s) = stop.filter(|s| atlas::is_lane(s)) {
            if self.lane.as_deref() == Some(s) { return (None, notes); }
            self.lane = Some(s.to_owned());
            return (Some(Go::Next), notes);
        }
        if let Some(s) = stop.filter(|s| *s != self.at && *s != SANCTUM) {
            let (foes, n) = self.arrive(s, walked, 0.0, level);
            fight.enemies = foes;
            return (None, n);
        }
        // The souls left here, fetched by walking over them (not by waking past them: those
        // behind the shrine are fetched by turning back for them).
        if let Some(s) = self.stain_at() {
            if (0.0..=1.0).contains(&(Self::standing(walked) - s)) {
                let n = self.stain.take().map_or(0, |s| s.souls);
                self.souls += n;
                notes.push(Note::Recovered(n));
            }
        }
        // A fork in view: the lane the hero holds says which way, taken at the last moment (a
        // barred way is not taken: the hero is told, and the walk waits for the other).
        if let Some(left) = choose_within {
            if self.signpost.is_none() {
                // From the junction itself: a walk begun short of it chooses where it begins, not
                // `STEER_M` before the junction.
                let junction = self.underway.travel.zip(self.underway.handover).map_or(left + STEER_M, |(travel, at)| at - travel);
                self.signpost = Some(walked + f64::from(junction + SIGNPOST_PAST));
                let title = |b| self.branch(b).map_or("", |l| Self::shown(&l));
                notes.push(Note::Fork(title(Branch::Left), title(Branch::Right)));
                notes.extend(self.rune_hums().map(Note::Hums));
                // A barred way is told as soon as it is seen, not only once leaned into: with
                // one way shut the walk takes the other, and the hero should know why.
                for b in [Branch::Left, Branch::Right] {
                    if let Some((to, why)) = self.branch(b).and_then(|l| self.barred_to(&l)) { notes.push(Self::barred_note(to, why)); }
                }
            }
            if self.chose { return (None, notes); }
            let lean = match fight.hero.lane { l if l < 0 => Some(Branch::Left), l if l > 0 => Some(Branch::Right), _ => None };
            if lean != self.lean {
                if let Some((b, l)) = lean.and_then(|b| Some((b, self.branch(b)?))) {
                    match self.barred_to(&l) {
                        Some((to, why)) => notes.push(Self::barred_note(to, why)),
                        None => notes.push(Note::Lean(b, Self::shown(&l))),
                    }
                }
            }
            self.lean = lean;
            // The way leaned into at the last moment; with one way shut, the open one as soon as the
            // walk waits for it (never left to run on past the junction unchosen).
            let pick = lean.filter(|&b| left <= COMMIT_M && !self.branch_barred(b)).or(self.only_open().filter(|_| left <= HOLD_M));
            if let Some(b) = pick {
                self.chose = true;
                self.chose_at = Some(walked);
                self.took = Some(b);
                // Onto the middle of the branch while the camera turns onto it.
                fight.hero.lane = 0;
                if let Some(l) = self.branch(b) { notes.extend(self.pass(&l)); }
                return (Some(Go::Choose(b)), notes);
            }
            return (None, notes);
        }
        // Clear once nothing alive stands ahead: walking on, no enemy is ever passed, but one
        // left behind by turning back never comes after the hero, and waiting on it made the
        // road endless.
        // What was left at the place's end, taken by walking up to it with no enemy on the hero.
        if let (Some(c), false, false) = (self.cache_at(), self.cache_taken(), self.trapped()) {
            if Self::quiet(fight, walked) && Self::standing(walked) >= c - 1.0 {
                notes.extend(self.open_cache());
            }
        }
        // A side path's end goes nowhere: the hero turns back (`come_back`).
        if self.detour.is_some() { return (None, notes); }
        let far = walked - self.base >= self.here().length as f64;
        // A standing boss bars the way on past it; turned round short of it, the way back is open.
        let barred = self.boss_here().is_some_and(|(at, _)| self.entered < self.along(at)) && !self.boss_beaten();
        // Held: the walk stops at the end (`held_at_end`) and the hero is told what holds it,
        // while whatever they ran past catches up.
        if let Some(kind) = self.held(fight).filter(|_| far && !self.sent) {
            if !std::mem::replace(&mut self.held_told, true) { notes.push(Note::Held(kind, self.here().hold == atlas::Hold::Clear)); }
            return (None, notes);
        }
        if self.sent || !far || barred { return (None, notes); }
        match self.ahead().as_slice() {
            // A dead end: nothing on; the hero turns round.
            [] if self.at_dead_end() => {
                if !std::mem::replace(&mut self.held_told, true) { notes.push(Note::DeadEnd); }
                return (None, notes);
            }
            // One road on: walked, unless it is barred; then the hero is told and turns back.
            [l] => {
                if let Some((to, why)) = self.barred_to(l) {
                    self.sent = true;
                    notes.push(Self::barred_note(to, why));
                    notes.push(Note::Turned);
                    let back = atlas::turned(self.here().id, self.from().unwrap_or_default());
                    self.keep_hero = true;
                    self.warping = Some(back.clone());
                    self.warp_into = 0.0;
                    return (Some(Go::Warp(back)), notes);
                }
                notes.extend(self.pass(l));
            }
            // Both ways ahead barred: the walk waits at the fork; the hero turns round.
            [a, b] if self.barred_to(a).is_some() && self.barred_to(b).is_some() => return (None, notes),
            _ => {}
        }
        self.sent = true;
        (Some(Go::Next), notes)
    }
}

/// A name read back from a save as the `&'static str` the world keeps it as: each distinct one
/// made once and kept for the run (a handful, read when a pilgrimage is carried on).
fn intern(name: &str) -> &'static str {
    static NAMES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<&'static str>>> = std::sync::OnceLock::new();
    let mut names = NAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(n) = names.get(name) { return n; }
    let n: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.insert(n);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Road to Hollowmere, walked out of Stilt Lane: the first place these tests walk.
    const ROAD: &str = "road@warrens_lane";

    /// A world begun on the Road to Hollowmere, the start before the Warrens were walked first.
    fn on_road() -> World {
        let mut w = World::new(0.0);
        (w.at, w.checkpoint) = (ROAD.into(), ROAD.into());
        w.seen = vec![atlas::node(ROAD)];
        w
    }
    use crate::fight::Ai;
    use Archetype::{Husk, Lancer, Wretch};

    #[test]
    fn the_smiths_levels_stay_with_their_piece_and_an_old_save_moves_its_levels_onto_what_is_worn() {
        use crate::gear::{Loadout, ITEMS, SLOTS};
        let mut g = Loadout::starting();
        let (weapon, helm) = (g.worn[0].unwrap(), g.worn[2].unwrap());
        for _ in 0..9 { g.reinforce(weapon); }
        for _ in 0..9 { g.reinforce(helm); }
        assert_eq!((g.level(weapon), g.level(helm)), (8, 4), "a weapon goes to +8, armour to +4");
        let k = g.kit();
        assert!(k.main == 8 && (k.forged_armour - 4.0 * crate::gear::ARMOUR_STEP).abs() < 1e-6);
        // Put away, a piece keeps its level and adds nothing; worn again, it is as it was.
        g.worn[0] = None;
        assert_eq!((g.kit().main, g.level(weapon)), (0, 8));
        g.worn[0] = Some(weapon);
        assert_eq!(g.kit().main, 8);
        // An old save's hero-wide levels: the weapon's onto the weapon, the armour's spread so
        // about the same share of a blow is turned.
        let mut old = Loadout::starting();
        old.take_old_levels(3, 2);
        let armour: Vec<u32> = (2..6).map(|p| old.level(old.worn[p].unwrap())).collect();
        assert_eq!((old.level(old.worn[0].unwrap()), armour), (3, vec![2, 2, 2, 1]));
        assert!((old.kit().forged_armour - 0.105).abs() < 1e-6, "old +2 turned 10%");
        let mut most = Loadout::starting();
        most.take_old_levels(5, 5);
        assert!((2..6).all(|p| most.level(most.worn[p].unwrap()) == 4));
        // Not twice: a save that has levels of its own keeps them.
        most.take_old_levels(1, 0);
        assert_eq!(most.level(most.worn[0].unwrap()), 5);
        assert!(SLOTS.iter().all(|(s, _)| s.reinforcing(1).1 >= 1) && ITEMS[weapon].slot.reinforcing(8) == (2, 2, 640));
    }

    #[test]
    fn a_piece_at_its_top_takes_one_capstone_of_its_kind_for_good_and_a_save_keeps_it() {
        use crate::capstone::Capstone;
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        let (weapon, hood) = (w.gear.worn[0].unwrap(), w.gear.worn[2].unwrap());
        assert!(!w.cappable(0) && w.cap(&mut f, 0, Capstone::Ember).is_none(), "not at its top yet");
        w.gear.levels.insert(weapon, 8);
        w.gear.levels.insert(hood, 4);
        assert!(w.cappable(0) && w.cappable(2) && !w.cappable(3));
        assert_eq!(w.cap(&mut f, 0, Capstone::Vigil), None, "not a weapon's");
        assert_eq!(w.cap(&mut f, 0, Capstone::Ember), Some(Note::Capped(crate::gear::ITEMS[weapon].name, "Ember Edge")));
        assert_eq!(w.cap(&mut f, 0, Capstone::Rime), None, "for good");
        assert!(!w.cappable(0) && f.hero.stats.kit.capped.burn > 0.0 && f.hero.stats.kit.capped.worn[0] == Some(Capstone::Ember));
        assert!(w.cap(&mut f, 2, Capstone::Vigil).is_some());
        let hp = f.hero.stats.max_hp();
        let back = World::restored(0.0, &w.saved()).unwrap();
        assert_eq!((back.gear.capstone(weapon), back.gear.capstone(hood)), (Some(Capstone::Ember), Some(Capstone::Vigil)));
        assert!((back.gear.kit().hp - w.gear.kit().hp).abs() < 1e-6 && hp > crate::fight::HP);
    }

    #[test]
    fn deeper_regions_give_rarer_shards_and_a_rare_one_is_told() {
        let grades = |tier: u32| -> [usize; 3] { let mut n = [0; 3]; for r in 0..600 { n[World::shard_grade(tier, r)] += 1; } n };
        assert_eq!(grades(0), [600, 0, 0]);
        assert_eq!(grades(2), [600, 0, 0]);
        assert_eq!(grades(4), [300, 300, 0]);
        assert_eq!(grades(6), [0, 400, 200]);
        let mut w = World::new(0.0);
        assert_eq!(w.shards_from(Lancer, 0, 0), None);
        assert_eq!(w.shards_from(Lancer, 6, 0), Some(Note::Found("Star Shard")));
        assert_eq!(w.shards, [1, 0, 1]);
        // Any other kind leaves one now and then.
        let some = (0..60u32).filter(|&r| { let before = w.shards; w.shards_from(Husk, 0, r << 16); w.shards != before }).count();
        assert_eq!(some, 10);
        // The expected hero's levels only climb, and reach the top in the last region.
        assert!((1..8).all(|t| World::forged_at(t).0 >= World::forged_at(t - 1).0) && World::forged_at(7) == (8, 16));
    }

    fn fell(f: &mut Fight) { for e in f.enemies.iter_mut() { e.ai = Ai::Dead; } }

    #[test]
    fn every_place_fits_the_enemy_objects() {
        for p in atlas::PLACES { assert!(p.foes.len() <= MOST_FOES && p.foes.iter().all(|f| f.1 < p.length), "{}", p.id); }
    }

    /// Turned back at the very start, the road's enemies all lie behind: the trunk is walked
    /// back into the lane before it, not walked forever waiting on them.
    #[test]
    fn turning_back_at_the_start_leaves_the_roads_enemies_behind_and_walks_on() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        let Some(Go::WarpAt(to, into)) = w.turn(&f, 2.0) else { panic!("no turn") };
        assert_eq!((to.as_str(), into), ("road@^", 62.0));
        w.tick(&mut f, 2.0, Some("road@^"), None);
        assert_eq!(f.enemies.len(), 4);
        assert!(f.enemies.iter().all(|e| e.alive() && e.gap(2.0) < -2.0), "all behind");
        assert_eq!(w.tick(&mut f, 3.0, Some("road@^"), None).0, None, "not before its end");
        assert_eq!(w.tick(&mut f, 4.5, Some("road@^"), None).0, Some(Go::Next));
        let (_, notes) = w.tick(&mut f, 10.0, Some("warrens_lane@road"), None);
        assert_eq!(notes[0], Note::Arrived("Stilt Lane"));
        assert_eq!(f.enemies.iter().map(|e| e.kind).collect::<Vec<_>>(), vec![Archetype::Wretch, Archetype::Husk]);
        assert!(f.enemies.iter().all(|e| e.gap(10.0) > 5.0), "ahead");
    }

    /// The road's shieldbearer keeps the fork: the rest can be run past, but the walk stops at the
    /// end, told why, until it falls. Walked back, nothing is held.
    #[test]
    fn a_kept_way_holds_until_its_keeper_falls_and_the_rest_can_be_run_past() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        assert_eq!(f.enemies[3].kind, Archetype::Shieldbearer);
        for e in &mut f.enemies[..3] { e.ai = Ai::Dead; }
        let (go, notes) = w.tick(&mut f, 64.0, Some(ROAD), None);
        assert_eq!((go, notes), (None, vec![Note::Held(Archetype::Shieldbearer, false)]));
        assert!(w.held_at_end(&f, 64.0) && !w.held_at_end(&f, 60.0));
        assert_eq!(w.tick(&mut f, 64.5, Some(ROAD), None), (None, vec![]), "told once");
        // Its keeper down and the others standing: on.
        let mut v = on_road();
        let mut g = Fight::new(v.foes());
        g.enemies[3].ai = Ai::Dead;
        assert_eq!(v.tick(&mut g, 66.0, Some(ROAD), None).0, Some(Go::Next));
        // Walked back the trunk, the same keeper holds nothing.
        let mut b = on_road();
        let f = Fight::new(b.foes());
        b.turn(&f, 2.0);
        let mut f = f;
        b.tick(&mut f, 2.0, Some("road@^"), None);
        assert!(b.held(&f).is_none() && f.enemies[3].alive());
    }

    /// Turned round partway along, the place's enemies stay where they were and as they were:
    /// the one run past is ahead now, still hurt and still after the hero.
    #[test]
    fn turning_round_keeps_the_enemies_where_they_stood() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        let husk = &mut f.enemies[0];
        (husk.hp, husk.idle, husk.lane) = (10.0, None, 1);
        let was = f.enemies[0].gap(20.0);
        assert!(was < -5.0, "run past: {was}");
        f.hero.lane = -1;
        let Some(Go::WarpAt(to, into)) = w.turn(&f, 20.0) else { panic!("no turn") };
        assert_eq!((to.as_str(), into), ("road@^", 44.0));
        w.tick(&mut f, 20.5, Some("road@^"), None);
        let husk = &f.enemies[0];
        assert!((husk.gap(20.5) + was).abs() < 1e-4, "as far ahead as it was behind: {}", husk.gap(20.5));
        assert_eq!((husk.hp, husk.idle, husk.lane, f.hero.lane), (10.0, None, -1, 1), "as it was, seen the other way");
        // The ones never reached are behind the hero now.
        assert!(f.enemies[1..].iter().all(|e| e.gap(20.5) < 0.0));
    }

    #[test]
    fn turned_round_past_a_fork_or_short_of_it_the_way_back_runs_to_that_junction() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        fell(&mut f);
        // Past the road's end, short of its fork's junction: the road walked the other way, the
        // hero that far short of its start.
        assert_eq!(w.tick(&mut f, 64.0, Some(ROAD), None).0, Some(Go::Next));
        w.follow(70.0, Underway { travel: Some(6.0), handover: Some(40.0), length: None });
        assert_eq!(w.turn(&f, 70.0), Some(Go::WarpAt("road@^".into(), -6.0)));
        w.tick(&mut f, 70.0, Some("road@^"), None);
        assert!((70.0 - w.base + 6.0).abs() < 1e-9, "6 m short of the road's start: {}", w.base);
        // Turned again at once: the fork's walk is 6 m along again.
        assert_eq!(w.turn(&f, 70.0), Some(Go::WalkOn(ROAD.into(), 6.0)));
        w.follow(70.0, Underway { travel: Some(6.0), handover: Some(40.0), length: None });
        w.tick(&mut f, 70.0, Some(ROAD), Some(20.0));
        assert!(w.sent && (70.0 - w.base - 70.0).abs() < 1e-9, "6 m past its end, the fork under way: {}", 70.0 - w.base);
        // Leaning left and taken, then 5 m past the junction: the road's junction walked in from
        // the marsh edge, the junction 5 m ahead.
        f.hero.lane = -1;
        assert_eq!(w.tick(&mut f, 92.0, Some(ROAD), Some(0.5)).0, Some(Go::Choose(Branch::Left)));
        w.follow(103.0, Underway { travel: Some(45.0), handover: Some(40.0), length: None });
        assert!(w.can_turn(&f, 103.0), "TURN is offered past the junction");
        let back = atlas::stop("road", "vale_edge");
        assert_eq!(w.turn(&f, 103.0), Some(Go::TurnInto(back.clone(), 5.0)));
        // Landing with that fork under way (its junction 40 m in): 5 m short of it.
        w.follow(103.0, Underway { travel: Some(35.0), handover: Some(40.0), length: None });
        w.tick(&mut f, 103.0, Some(&back), Some(20.0));
        let length = atlas::JUNCTION_M;
        assert!(w.sent && w.at == back && (103.0 - w.base - f64::from(length + 35.0)).abs() < 1e-4, "{} m in", 103.0 - w.base);
    }

    #[test]
    fn turned_past_a_ys_junction_the_way_back_is_the_one_whose_walk_on_is_that_junction() {
        let mut w = World::new(0.0);
        let f = Fight::new(Vec::new());
        // Each way out of the Warrens' junction, turned round 4 m past it: (walked in by, way taken)
        // and the stop walked back toward that junction.
        let cases = [
            // Out of the guild hall, down the trunk toward Stilt Lane: the Warrens up its trunk.
            ("warrens@guild_hall", "warrens_lane", "warrens@warrens_lane"),
            // Up the trunk into the guild hall, or the sewers: the junction walked in from it.
            ("warrens@warrens_lane", "guild_hall", "warrens@guild_hall"),
            ("warrens@warrens_lane", "sewers", "warrens@sewers"),
            // Out of the sewers up into the guild hall: the junction walked in from the hall.
            ("warrens@sewers", "guild_hall", "warrens@guild_hall"),
        ];
        for (at, way, back) in cases {
            w.arrive(at, 0.0, 0.0, 1);
            let ways = w.ahead();
            let side = ways.iter().position(|l| l.road.1 == way).expect("a way there");
            w.took = Some(if side == 0 { Branch::Left } else { Branch::Right });
            w.handed = Some(50.0);
            assert_eq!(w.turn(&f, 54.0), Some(Go::TurnInto(back.into(), 4.0)), "from {at} toward {way}");
            w.warping = None;
        }
    }

    #[test]
    fn walking_through_a_door_into_a_dead_end_its_wall_stands_from_the_door_on_and_the_hero_goes_through_the_middle() {
        let mut w = World::new(0.0);
        w.lingering = None;
        w.arrive(&atlas::stop("guild_hall", "warrens"), 100.0, 0.0, 0);
        let (handover, length) = (36.0, 41.0);
        w.follow(120.0, Underway { travel: Some(20.0), handover: Some(handover), length: Some(length) });
        let seen = w.end_wall().expect("the office's wall stands while the door is ahead");
        assert!((w.through_door(1.0) - 1.0).abs() < 1e-6, "far from the door the hero walks their lane");
        assert!(w.gruk_at().is_some_and(|at| at < seen), "Gruk behind his counter is in the room from the door on");
        assert!(!w.narrows() && !w.held_at_door(crate::fight::LANE_W), "far from the door every lane is open");
        w.follow(136.0 - f64::from(HERO_D) - 0.2, Underway { travel: Some(handover - HERO_D - 0.2), handover: Some(handover), length: Some(length) });
        assert!(w.narrows() && w.held_at_door(crate::fight::LANE_W) && !w.held_at_door(0.0), "at the door's face a side lane meets the wall, the middle goes through");
        w.follow(136.0 - f64::from(HERO_D), Underway { travel: Some(handover - HERO_D), handover: Some(handover), length: Some(length) });
        assert!(w.through_door(1.0).abs() < 1e-6, "at the door the hero is in its middle");
        // Arrived, the wall is where it stood all along.
        w.arrive(&atlas::stop("guild_office", "guild_hall"), 120.0 + f64::from(length - 20.0), 0.0, 0);
        w.follow(141.0, Underway::default());
        assert!(w.end_wall().is_some_and(|at| (at - seen).abs() < 1e-3), "{seen} then {:?}", w.end_wall());
        assert!(!w.narrows(), "through the door, the room's lanes are open");
    }

    #[test]
    fn turned_round_in_a_dead_end_the_hero_walks_back_out_from_where_they_stood() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        // The guild hall, its boss beaten, turned round 22 m in: the hall walked back out, 8 m in.
        w.arrive("guild_hall@warrens", 100.0, 0.0, 1);
        w.beaten.push("guild_hall");
        assert!(w.can_turn(&f, 122.0));
        assert_eq!(w.turn(&f, 122.0), Some(Go::WarpAt("guild_hall@guild_office".into(), 8.0)));
        w.tick(&mut f, 122.0, Some("guild_hall@guild_office"), None);
        assert!(w.at == "guild_hall@guild_office" && (122.0 - w.base - 8.0).abs() < 1e-9, "{} m in", 122.0 - w.base);
        assert!(w.boss_due(&f, 122.0).is_none() && w.veil().is_none());
        // Out of it, into the Warrens at their junction.
        assert_eq!(w.ahead().into_iter().map(|l| l.stop).collect::<Vec<_>>(), vec!["warrens@guild_hall"]);
        // Turned again: back in, where they stood.
        assert_eq!(w.turn(&f, 122.0), Some(Go::WarpAt("guild_hall@warrens".into(), 22.0)));
        // Turned 3 m past the Warrens' junction toward the hall: the junction walked in from the
        // hall, 3 m ahead.
        w.warping = None;
        w.arrive("warrens@warrens_lane", 200.0, 0.0, 1);
        w.took = Some(if w.ahead()[0].stop == "guild_hall@warrens" { Branch::Left } else { Branch::Right });
        w.handed = Some(200.0);
        assert_eq!(w.turn(&f, 203.0), Some(Go::TurnInto("warrens@guild_hall".into(), 3.0)));
        // In that junction, short of its fork, no way back is walked: TURN waits for the fork.
        w.warping = None;
        w.arrive("warrens@guild_hall", 250.0, 0.0, 1);
        assert!(!w.can_turn(&f, 251.0) && w.here().foes.is_empty() && w.here().shrine.is_none());
        // Through the door at the hall's end into Gruk's room, a dead end: the walk is held at its
        // end with Gruk in reach, the hero is told once, and TURN is offered.
        w.warping = None;
        w.arrive("guild_office@guild_hall", 300.0, 0.0, 1);
        let end = 300.0 + f64::from(w.here().length);
        assert!(w.held_at_end(&f, end) && !w.held_at_end(&f, end - 0.4) && w.can_turn(&f, end));
        assert!(w.can_meet_gruk(&f, end) && w.end_wall().is_some_and(|at| at > w.gruk_at().unwrap_or(f64::MAX)));
        assert_eq!(w.tick(&mut f, end, Some("guild_office@guild_hall"), None), (None, vec![Note::DeadEnd]));
        assert_eq!(w.tick(&mut f, end + 0.5, Some("guild_office@guild_hall"), None), (None, vec![]));
        assert_eq!(w.turn(&f, end), Some(Go::WarpAt("guild_office@^".into(), 0.0)));
    }

    #[test]
    fn the_collectors_chest_holds_his_tally_and_a_parry_doubles_the_next_kills_souls() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        // Behind the Debt Collector, his chest: taken walking up to it once he is down.
        w.arrive("guild_hall@warrens", 100.0, 0.0, 1);
        w.beaten.push("guild_hall");
        let chest = w.cache_at().unwrap_or(0.0);
        let (_, notes) = w.tick(&mut f, chest - HERO_D as f64, Some("guild_hall@warrens"), None);
        assert!(notes.contains(&Note::Found("The Collector's Tally")), "{notes:?}");
        // Unworn, a parry is owed nothing, and the next kill clears it.
        w.parried = true;
        let plain = { let before = w.souls; w.felled([Archetype::Husk]); w.souls - before };
        assert!(!w.parried);
        let tally = crate::gear::find("The Collector's Tally");
        w.gear.worn[crate::gear::SLOTS.iter().position(|(s, _)| *s == crate::gear::Slot::Talisman).unwrap_or(0)] = Some(tally);
        w.parried = true;
        let before = w.souls;
        let notes = w.felled([Archetype::Husk, Archetype::Husk]);
        assert_eq!(w.souls - before, 3 * plain, "the first of the two doubled: {notes:?}");
        assert_eq!(notes.iter().filter(|n| matches!(n, Note::Tallied(_))).count(), 1);
    }

    #[test]
    fn the_rune_hums_toward_the_next_region_boss_and_only_when_one_way_is_nearer() {
        let mut w = on_road();
        w.at = "hollowmere@gate".into();
        w.beaten = vec!["gate", "hollowmere", "east_gate"];
        assert_eq!(w.rune_hums(), None, "no rune, no hum");
        w.keys.push(Key::Rune);
        let hums = w.rune_hums().expect("one way is nearer the Blackridge chief");
        assert_eq!(w.branch(hums).map(|l| l.road.1), Some("pine_road"));
        // At the first fork both ways reach the Hollowmere gate as soon.
        (w.at, w.beaten) = (ROAD.into(), vec!["east_gate"]);
        assert_eq!(w.rune_hums(), None);
        assert_eq!(atlas::Place::steps("road", "vale_edge", |p| p.id == "gate"), Some(2));
    }

    #[test]
    fn a_cleared_place_walks_on_and_a_fork_goes_the_way_the_hero_stands() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        assert_eq!(f.enemies.len(), 4);
        // Not while an enemy stands ahead (no walk passes one), nor before the place is walked.
        let last = f.enemies[3].s;
        f.enemies[3].s = 75.0 + HERO_D as f64;
        assert_eq!(w.tick(&mut f, 70.0, Some(ROAD), None).0, None);
        f.enemies[3].s = last;
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 30.0, Some(ROAD), None).0, None);
        assert_eq!(w.tick(&mut f, 64.0, Some(ROAD), None).0, Some(Go::Next));
        assert_eq!(w.tick(&mut f, 65.0, Some(ROAD), None).0, None, "sent once");
        // The fork: nothing chosen in the middle lane, and at the last moment the walk waits.
        let (go, notes) = w.tick(&mut f, 66.0, Some(ROAD), Some(12.0));
        assert_eq!((go, notes), (None, vec![Note::Fork("The Marsh Edge", "The Chapel Edge")]));
        assert!(w.signpost.is_some_and(|s| s > 66.0 + 12.0 + STEER_M as f64));
        assert!(!w.holds_at_fork(Some(12.0), 0) && w.holds_at_fork(Some(1.0), 0));
        // Leaning right is not yet a choice, and leaning left again changes it.
        f.hero.lane = 1;
        assert_eq!(w.tick(&mut f, 67.0, Some(ROAD), Some(11.0)), (None, vec![Note::Lean(Branch::Right, "The Chapel Edge")]));
        f.hero.lane = -1;
        assert_eq!(w.tick(&mut f, 70.0, Some(ROAD), Some(8.0)).1, vec![Note::Lean(Branch::Left, "The Marsh Edge")]);
        f.hero.lane = 1;
        w.tick(&mut f, 74.0, Some(ROAD), Some(4.0));
        assert!(!w.holds_at_fork(Some(0.5), 1), "a side is held: walk on");
        // At the last moment the lane held is the branch, and the hero steps back to the middle.
        assert_eq!(w.tick(&mut f, 77.5, Some(ROAD), Some(0.5)).0, Some(Go::Choose(Branch::Right)));
        assert_eq!((f.hero.lane, w.tick(&mut f, 77.6, Some(ROAD), Some(0.4)).0), (0, None), "once");
        // Past the junction, the branch's lane walks on into where it leads, once.
        assert_eq!(w.tick(&mut f, 80.0, Some("~chapel_edge@road"), None).0, Some(Go::Next));
        assert_eq!(w.tick(&mut f, 81.0, Some("~chapel_edge@road"), None), (None, vec![]));
        assert_eq!(w.at, ROAD, "a lane is not a place");
        // Arriving: the chapel edge's enemies, placed from there.
        let (_, notes) = w.tick(&mut f, 90.0, Some("chapel_edge@road"), None);
        assert_eq!(notes, vec![Note::Arrived("The Chapel Edge")]);
        assert_eq!((f.enemies.len(), f.enemies[0].kind), (1, Archetype::Lancer));
        assert_eq!(f.enemies[0].s, 90.0 + (HERO_D + 15.0) as f64);
        assert!(w.lean.is_none() && !w.chose);
    }

    #[test]
    fn a_branch_taken_again_after_dying_still_leads_on() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        let lane = atlas::lane("vale_edge", START);
        assert_eq!(w.tick(&mut f, 80.0, Some(&lane), None).0, Some(Go::Next));
        w.tick(&mut f, 90.0, Some("vale_edge@road"), None);
        w.tick(&mut f, 120.0, Some("vale@vale_edge"), None);
        f.hero.dead = true;
        w.died(130.0);
        w.tick(&mut f, 131.0, Some(START), None);
        // The same branch again, from the shrine: on into the marsh, not an endless lane.
        assert_eq!(w.tick(&mut f, 211.0, Some(&lane), None).0, Some(Go::Next), "the lane leads on");
        // Turned back from the marsh's edge and down the same branch again, too.
        w.tick(&mut f, 221.0, Some("vale_edge@road"), None);
        w.tick(&mut f, 240.0, Some(START), None);
        assert_eq!(w.tick(&mut f, 260.0, Some(&lane), None).0, Some(Go::Next), "and again");
    }

    #[test]
    fn death_wakes_at_the_last_shrine_and_leaves_the_souls_where_the_hero_fell() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        let found = w.felled([Husk, Lancer]);
        assert_eq!(found, vec![Note::Found("Husk Cleaver"), Note::Found("Lancer's Shortspear")], "the first of each kind leaves its first piece");
        assert_eq!(w.souls, 70);
        // Over to the vale; fall 20 m in.
        w.tick(&mut f, 100.0, Some("vale@vale_edge"), None);
        f.hero.hp = 0.0;
        f.hero.dead = true;
        assert_eq!(w.died(120.0), Go::Warp(ROAD.into()));
        assert_eq!((w.souls, w.stain.clone()), (0, Some(Stain { stop: "vale".into(), at: 20.0, souls: 70 })));
        // Nothing happens until the walk shows the shrine's stop; then the road is full again.
        assert_eq!(w.tick(&mut f, 121.0, Some("vale@vale_edge"), None), (None, vec![]));
        let (_, notes) = w.tick(&mut f, 122.0, Some(ROAD), None);
        assert_eq!(notes, vec![Note::Arrived("The Road to Hollowmere"), Note::ShrineFound]);
        assert!(!f.hero.dead && f.hero.hp == f.hero.stats.max_hp() && f.enemies.len() == 4 && f.enemies.iter().all(|e| e.alive()));
        // Back in the vale, walking over the stain fetches them.
        w.tick(&mut f, 300.0, Some("vale@vale_edge"), None);
        assert_eq!(w.tick(&mut f, 319.0, Some("vale@vale_edge"), None).1, vec![]);
        assert_eq!(w.tick(&mut f, 320.0, Some("vale@vale_edge"), None).1, vec![Note::Recovered(70)]);
        assert_eq!((w.souls, w.stain.is_none()), (70, true));
    }

    #[test]
    fn waking_stands_the_hero_by_the_shrine_rested_at_whichever_way_it_was_walked() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        // Walked in its own way, the shrine is where the walk comes in.
        w.rest(&mut f);
        assert_eq!(w.leave_rest(), Go::Warp(ROAD.into()));
        w.tick(&mut f, 10.0, Some(ROAD), None);
        assert!(w.can_rest(&f, 10.0));
        // Turned round and rested walking back: the shrine is near the walk's far end, and the
        // hero wakes as far short of it, rising and after a fall alike.
        let Some(Go::WarpAt(to, _)) = w.turn(&f, 12.0) else { panic!("no turn") };
        w.tick(&mut f, 13.0, Some(&to), None);
        assert!(w.can_rest(&f, 13.0));
        w.rest(&mut f);
        let len = w.here().length;
        assert_eq!(w.leave_rest(), Go::WarpAt(to.clone(), len - 3.0));
        w.tick(&mut f, 100.0, Some(&to), None);
        assert!(w.can_rest(&f, 100.0), "woke {} m from it", w.shrine().unwrap() - World::standing(100.0));
        assert_eq!(w.died(100.0), Go::WarpAt(to.clone(), len - 3.0));
        // Souls left behind the waking place stay there until the hero turns back for them.
        w.stain = Some(Stain { stop: "road".into(), at: len - 1.0, souls: 40 });
        w.tick(&mut f, 200.0, Some(&to), None);
        assert!(w.can_rest(&f, 200.0));
        assert!(!w.tick(&mut f, 200.5, Some(&to), None).1.contains(&Note::Recovered(40)));
    }

    #[test]
    fn resting_makes_the_hero_whole_brings_the_enemies_back_and_souls_buy_levels() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        assert!(w.can_rest(&f, 0.0), "the road starts at a shrine");
        assert!(!w.can_rest(&f, 10.0), "too far past it");
        fell(&mut f);
        f.hero.hp = 10.0;
        f.hero.flasks = 0;
        w.rest(&mut f);
        assert_eq!((f.hero.hp, f.hero.flasks, f.enemies.iter().filter(|e| e.alive()).count()), (100.0, crate::fight::FLASKS, 4));
        assert_eq!(w.travel_list(), vec![("The Road to Hollowmere", ROAD.to_owned())]);
        assert_eq!(w.rise(&mut f, 0), None, "no souls");
        w.souls = 145;
        assert_eq!(w.rise(&mut f, 0), Some(Note::Rose(1)));
        assert_eq!((w.souls, f.hero.hp, f.hero.stats.level_cost()), (85, 108.0, 80));
        assert_eq!(w.rise(&mut f, 2), Some(Note::Rose(2)));
        // The starting sword takes might at C.
        assert!(w.souls == 5 && (f.hero.stats.damage() - 1.045).abs() < 1e-6, "{}", f.hero.stats.damage());
    }

    #[test]
    fn the_dead_stay_dead_walking_back_until_a_rest() {
        let mut w = on_road();
        let mut f = Fight::new(w.foes());
        f.enemies[0].ai = Ai::Dead;
        // Turn round 20 m in: the walk goes back out the way it came, from where the hero stood.
        assert!(w.can_turn(&f, 20.0 - HERO_D as f64));
        let go = w.turn(&f, 20.0);
        assert_eq!(go, Some(Go::WarpAt("road@^".into(), 44.0)));
        let (_, notes) = w.tick(&mut f, 21.0, Some("road@^"), None);
        assert!(notes.contains(&Note::Arrived("The Road to Hollowmere")));
        // The husk felled stays down; the rest stand, mirrored: the lancer 26 m in is 38 m in
        // walking back, in the other lane.
        assert_eq!(f.enemies.len(), 3);
        assert!(!f.enemies.iter().any(|e| e.kind == Husk));
        let lancer = f.enemies.iter().find(|e| e.kind == Lancer).unwrap();
        assert_eq!((lancer.s, lancer.lane), (21.0 - 44.0 + (HERO_D + 38.0) as f64, -1));
        w.rest(&mut f);
        assert_eq!(f.enemies.len(), 4, "a rest brings them all back");
    }

    #[test]
    fn a_locked_road_turns_the_hero_back_and_a_shortcut_opens_from_its_far_side() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        // The chief's hold, the climbing hook not yet won: the cliff path is barred.
        w.tick(&mut f, 100.0, Some("chief_hold@ridge_camp"), None);
        fell(&mut f);
        w.beaten.push("chief_hold");
        w.keys.retain(|k| *k != Key::Hook);
        let (go, notes) = w.tick(&mut f, 150.0, Some("chief_hold@ridge_camp"), None);
        assert_eq!(go, Some(Go::Warp("chief_hold@cliff_path".into())));
        assert!(notes.contains(&Note::Barred("The Cliff Path", Some(Key::Hook))) && notes.contains(&Note::Turned), "{notes:?}");
        // With it, the way is open.
        let mut w = World::new(0.0);
        w.beaten.push("chief_hold");
        w.keys.push(Key::Hook);
        w.tick(&mut f, 200.0, Some("chief_hold@ridge_camp"), None);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 250.0, Some("chief_hold@ridge_camp"), None).0, Some(Go::Next));
        // Out of the sewers up into the Warrens' junction: walking on lifts the grate from below;
        // it stays open.
        assert!(!w.opened.contains(&"sewers"));
        w.tick(&mut f, 300.0, Some("sewers@lower_stairs"), None);
        fell(&mut f);
        let (go, notes) = w.tick(&mut f, 361.0, Some("sewers@lower_stairs"), None);
        assert_eq!(go, Some(Go::Next));
        assert!(notes.contains(&Note::Opened("The Ashen Warrens")) && w.opened.contains(&"sewers"), "{notes:?}");
    }

    #[test]
    fn a_fork_with_one_way_shut_takes_the_other() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        // The vale: on to the gate, or the shortcut to the mill, shut from this side.
        w.tick(&mut f, 100.0, Some("vale@vale_edge"), None);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 157.0, Some("vale@vale_edge"), None).0, Some(Go::Next));
        let ahead = w.ahead();
        let mill = if ahead[0].shown == "drowned_mill" { Branch::Left } else { Branch::Right };
        f.hero.lane = if mill == Branch::Left { -1 } else { 1 };
        let (_, notes) = w.tick(&mut f, 158.0, Some("vale@vale_edge"), Some(8.0));
        assert!(notes.contains(&Note::Barred("The Drowned Mill", None)), "{notes:?}");
        assert_eq!(w.barred_branches(), vec![(mill, false)], "a barrier stands across the shut way's mouth");
        assert_eq!(w.gates_ahead(), vec![(mill, false, false)]);
        // Opened from the mill's side, its door still stands there, open, and the way is free.
        let mut o = w.clone();
        o.opened.push("drowned_mill");
        assert_eq!((o.gates_ahead(), o.barred_branches()), (vec![(mill, false, true)], vec![]));
        // The walk waits at the last moment, and the open way is taken there.
        assert!(w.holds_at_fork(Some(1.2), f.hero.lane));
        let gate = if mill == Branch::Left { Branch::Right } else { Branch::Left };
        assert_eq!(w.tick(&mut f, 164.8, Some("vale@vale_edge"), Some(1.2)).0, Some(Go::Choose(gate)));
        assert!(!w.holds_at_fork(Some(1.2), f.hero.lane), "taken: walk on");
        // Leaning the shut way, the open one is taken; the island swings the way the walk turns,
        // not the way the hero leaned (it stood in the middle of the road taken).
        assert_eq!(w.took, Some(gate));
        let x = w.signpost_x(164.8 + STEER_M as f64);
        assert!(if gate == Branch::Right { x < -1.0 } else { x > 1.0 }, "{x}");
    }

    #[test]
    fn turned_round_short_of_a_boss_it_stays_behind_until_walked_back_to() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 100.0, Some("chief_hold@ridge_camp"), None);
        fell(&mut f);
        let mark = (0..400).map(|k| 100.0 + k as f64 * 0.25).find(|&m| w.boss_due(&f, m).is_some()).expect("the chief steps out somewhere");
        // Turned two metres short of him: walking away, he does not step out, near or far.
        let at = mark - 2.0;
        let Some(Go::WarpAt(to, into)) = w.turn(&f, at) else { panic!("could not turn") };
        w.tick(&mut f, at, Some(&to), None);
        assert!(w.warping.is_none() && (w.base + into as f64 - at).abs() < 1e-3, "arrived turned at {into}");
        fell(&mut f);
        for k in 0..120 { assert!(w.boss_due(&f, at + k as f64 * 0.25).is_none(), "stepped out {} m on, walking away", k as f64 * 0.25); }
        // Turned again, back toward him: he steps out where he waits.
        let back = at + 10.0;
        let Some(Go::WarpAt(to, _)) = w.turn(&f, back) else { panic!("could not turn back") };
        w.tick(&mut f, back, Some(&to), None);
        fell(&mut f);
        assert!(w.boss_due(&f, back + 1.0).is_none());
        assert!(w.boss_due(&f, back + 12.5).is_some(), "walked back to him");
    }

    #[test]
    fn a_boss_leaves_its_key_and_the_world_grows_harder_with_its_regions() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 100.0, Some("chief_hold@ridge_camp"), None);
        fell(&mut f);
        let boss = w.boss_due(&f, 141.0).expect("the chief steps out");
        assert_eq!(boss.name(), "The Bandit Chief");
        let notes = w.boss_felled();
        assert!(notes.contains(&Note::Found("Climbing Hook")) && w.keys.contains(&Key::Hook));
        assert!(w.boss_due(&f, 141.0).is_none(), "beaten stays beaten");
        let deep = w.scale();
        w.tick(&mut f, 300.0, Some("warrens@warrens_lane"), None);
        let home = w.scale();
        assert!(deep.0 > 1.2 && deep.1 > 1.1 && home.0 < 1.0, "{deep:?} {home:?}");
        // Arriving far under a region's level warns the hero.
        let (_, notes) = w.arrive("cathedral@varnholt", 400.0, 0.0, 0);
        assert!(notes.contains(&Note::Region("Varnholt", true)), "{notes:?}");
    }

    #[test]
    fn a_hero_fallen_down_a_side_path_can_walk_it_again_and_turn_round() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 100.0, Some("chapel@chapel_edge"), None);
        for e in &mut f.enemies { e.ai = Ai::Dead; }
        w.explore(&f, 108.0, -1.0);
        w.tick(&mut f, 130.0, Some("hollow"), None);
        w.souls = 50;
        let wake = w.died(140.0);
        assert!(w.detour.is_none() && !w.explored.contains(&"hollow"), "the hollow is open again");
        assert_eq!(w.stain.as_ref().map(|s| s.stop.as_str()), Some("hollow"), "the souls lie down it");
        let (Go::Warp(cp) | Go::WarpAt(cp, _)) = wake else { panic!("no waking: {wake:?}") };
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 300.0, Some(&cp), None);
        assert!(w.can_turn(&f, 300.0), "the walk is the road's again: {}", w.at);
    }

    #[test]
    fn a_side_path_is_walked_to_its_end_and_the_road_is_as_it_was_left() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 100.0, Some("chapel@chapel_edge"), None);
        let foes = f.enemies.len();
        assert!(foes >= 2, "the chapel has its dead to fight");
        f.enemies[0].ai = Ai::Dead;
        let mouth = [(crate::fight::HERO_D + 0.4, -1.0)];
        // Not with an enemy close; then, once they are past, at the mouth.
        for e in f.enemies.iter_mut().skip(1) { e.s = 100.0 + 60.0; }
        assert_eq!(w.side_path_at(&f, 100.0, &[(30.0, -1.0)]), None, "no mouth here");
        assert_eq!(w.side_path_at(&f, 100.0, &mouth), Some(-1.0));
        let go = w.explore(&f, 108.0, -1.0);
        assert_eq!(go, Some(Go::Warp("hollow".into())));
        // The hollow: its own dead, the road's kept; the hero as they were.
        f.hero.hp = 40.0;
        w.tick(&mut f, 130.0, Some("hollow"), None);
        assert_eq!((w.at.as_str(), f.hero.hp), ("hollow", 40.0));
        assert!(f.enemies.iter().all(|e| e.alive()) && w.side_path_at(&f, 130.0, &mouth).is_none());
        // Not with one of its dead on the hero; one run past and left far behind does not stop it.
        let kept = f.enemies[1].s;
        f.enemies[1].s = 154.0 + (crate::fight::HERO_D - 3.0) as f64;
        assert!(!w.can_come_back(&f, 154.0), "not with the hollow's dead on the hero");
        f.enemies[1].s = kept;
        assert!(w.can_come_back(&f, 154.0), "run past");
        fell(&mut f);
        let (go, notes) = w.tick(&mut f, 152.0, Some("hollow"), None);
        assert_eq!(go, None, "a side path leads nowhere");
        // Its cache is a trapped chest: beaten, it opens.
        assert!(notes.is_empty() && w.ambush_due(&f, 152.0) == Some(0), "{notes:?}");
        let notes = w.ambush_cleared(0, Vec::new());
        assert!(matches!(notes.as_slice(), [Note::Found(_)]), "{notes:?}");
        assert!(w.can_come_back(&f, 153.0));
        assert_eq!(w.come_back(), Some(Go::Warp("chapel@chapel_edge".into())));
        let (_, notes) = w.tick(&mut f, 170.0, Some("chapel@chapel_edge"), None);
        assert_eq!(notes, vec![Note::Arrived(w.here().title)]);
        // Back where the road was left: the same metre, the felled still down, the rest ahead.
        assert_eq!((f.enemies.len(), f.hero.hp), (foes, 40.0));
        assert!(!f.enemies[0].alive() && f.enemies[1].alive());
        assert!((f.enemies[1].s - (170.0 - 8.0 + 60.0)).abs() < 1e-9, "{}", f.enemies[1].s);
        assert!(w.side_path_at(&f, 170.0, &mouth).is_none(), "walked already");
    }

    #[test]
    fn an_ambush_springs_once_and_a_trapped_chest_opens_only_after_it() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 100.0, Some("vale@vale_edge"), None);
        fell(&mut f);
        assert_eq!(w.ambush_due(&f, 100.0 + 29.0), None, "not yet");
        assert_eq!(w.ambush_due(&f, 100.0 + 30.5), Some(0));
        let souls = w.souls;
        let notes = w.ambush_cleared(0, vec![Wretch, Wretch, Husk]);
        assert!(w.souls > souls && !notes.is_empty());
        assert_eq!(w.ambush_due(&f, 100.0 + 31.0), None, "beaten stays beaten");
        // The hollow's chest: walking up to it springs the trap instead of opening it.
        w.tick(&mut f, 200.0, Some("hollow"), None);
        fell(&mut f);
        let (_, notes) = w.tick(&mut f, 200.0 + 21.0, Some("hollow"), None);
        assert!(!w.cache_taken() && notes.is_empty(), "{notes:?}");
        assert_eq!(w.ambush_due(&f, 200.0 + 21.0), Some(0));
        let notes = w.ambush_cleared(0, vec![Husk, Wretch]);
        assert!(w.cache_taken() && notes.iter().any(|n| matches!(n, Note::Found(_) | Note::Recovered(300))), "{notes:?}");
    }

    #[test]
    fn the_last_boss_beaten_begins_the_pilgrimage_again_harder() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.keys = vec![Key::ShardI, Key::ShardII, Key::ShardIII];
        let easy = w.scale();
        let chamber = atlas::stop(atlas::FINALE, "penitent_cells");
        w.tick(&mut f, 200.0, Some(&chamber), None);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 236.0, Some(&chamber), None), (None, vec![]), "Exodus bars the way");
        assert!(w.boss_due(&f, 236.0).is_some());
        w.boss_felled();
        let (go, notes) = w.tick(&mut f, 241.0, Some(&chamber), None);
        assert!(go.is_none() && !notes.contains(&Note::Pilgrimage(1)), "walked to its end, only the Eye's choice concludes it");
        assert_eq!(w.conclude(), (Go::Warp(START.into()), Note::Pilgrimage(1)));
        assert_eq!((w.checkpoint.as_str(), w.keys.len(), w.beaten.len()), (START, 0, 0));
        w.tick(&mut f, 300.0, Some(START), None);
        assert!(w.scale().0 > easy.0, "the world again, harder");
    }

    #[test]
    fn every_kind_of_enemy_can_fall_and_leave_something() {
        let mut w = World::new(0.0);
        for &k in Archetype::ALL.iter() {
            let found = w.felled([k]);
            assert!(!found.is_empty(), "the first {k:?} leaves a piece");
        }
        assert!(w.fallen.iter().all(|&n| n == 1));
    }

    #[test]
    fn a_world_saved_and_restored_keeps_its_progress_and_wakes_at_its_shrine() {
        let mut w = World::new(0.0);
        let place = atlas::PLACES[3].id;
        let shrine = format!("{place}@{}", atlas::PLACES[2].id);
        w.checkpoint = shrine.clone();
        w.rested = vec![START.into(), shrine.clone()];
        (w.souls, w.shards, w.pilgrimages, w.luck, w.talks, w.heard) = (812, [4, 2, 1], 1, 9, [2, 1], [5, 2]);
        (w.met_keeper, w.met_smith) = (true, true);
        w.stain = Some(Stain { stop: shrine.clone(), at: 12.5, souls: 300 });
        let kite = crate::gear::find("Kite Shield");
        w.gear.gain(kite);
        assert!(w.gear.wear(1, Some(kite)));
        w.fallen[Archetype::Lancer as usize] = 7;
        w.ammo = crate::fight::Quiver { arrows: 13, bolts: 4, imbued: 3 };
        w.beaten = vec![place];
        w.keys = vec![Key::Hook, Key::ShardII];
        w.remnants = vec!["Bell Warden's Clapper"];
        w.seen.push(place);
        w.taken = vec![place];
        w.cleared = vec![(place, 1)];
        // In the middle of things too: slain foes and a walk under way are not kept.
        w.slain = vec![(place, 0)];
        let text = serde_json::to_string(&w.saved()).unwrap();
        let back = World::restored(50.0, &serde_json::from_str(&text).unwrap()).expect("a world's save");
        assert_eq!((back.checkpoint.as_str(), back.rested.len(), back.souls, back.shards, back.pilgrimages, back.luck, back.talks), (shrine.as_str(), 2, 812, [4, 2, 1], 1, 9, [2, 1]));
        assert!(back.met_keeper && back.met_smith && back.heard == [5, 2]);
        assert_eq!(back.stain.as_ref().map(|s| (s.at, s.souls)), Some((12.5, 300)));
        assert_eq!((back.gear.owned.clone(), back.gear.worn), (w.gear.owned.clone(), w.gear.worn));
        assert_eq!(back.gear.kit().off.map(|o| o.art), Some("kite_shield"));
        assert_eq!(back.fallen, w.fallen);
        // The arrows and bolts carried; an imbue under way is not kept.
        assert_eq!(back.ammo, crate::fight::Quiver { arrows: 13, bolts: 4, imbued: 0 });
        assert_eq!((back.beaten.clone(), back.keys.clone(), back.remnants.clone(), back.seen.clone(), back.taken.clone(), back.cleared.clone()), (w.beaten.clone(), w.keys.clone(), w.remnants.clone(), w.seen.clone(), w.taken.clone(), w.cleared.clone()));
        assert!(back.slain.is_empty() && back.base == 50.0);
        assert_eq!(back.checkpoint_title(), atlas::PLACES[3].title);
        // Carried on: off to that shrine.
        let mut back = back;
        assert_eq!(back.resume(), Go::Warp(shrine.clone()));
        assert_eq!(back.warping, Some(shrine));
        // Names it no longer knows are passed over; a shrine it no longer knows is the start.
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["checkpoint"] = serde_json::json!("nowhere@never");
        v["gear"]["owned"] = serde_json::json!(["No Such Thing", "Kite Shield"]);
        v.as_object_mut().unwrap().remove("ammo");
        let odd = World::restored(0.0, &v).unwrap();
        assert_eq!(odd.checkpoint, START);
        assert_eq!(odd.ammo, crate::fight::Quiver::SETTING_OUT, "a save from before arrows were carried");
        assert_eq!(odd.gear.owned, vec![kite]);
        assert!(World::restored(0.0, &serde_json::json!("not a world")).is_none());
    }
}
