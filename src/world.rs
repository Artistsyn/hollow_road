//! The world the road walks through: the atlas's places (`atlas`), the enemies each holds and
//! which of them are down, when the walk goes on and which road it takes, the gates and shortcuts
//! on the way, and the shrines (the bonfires): rest to be made whole, rise in level and have the
//! world's enemies back; die to wake at the last one rested at, with the souls carried left where
//! the hero fell.
//!
//! The walk is in a stop (`atlas::stop`: a place walked in by one road). Everything here is in
//! road metres (`walked`, never coming round), so it is tested without PathForge; the game reads
//! the walk's state and sends what `World` asks for.

use crate::atlas::{self, Barred, Key, Place, FINALE};
use crate::fight::{Ai, Archetype, Enemy, Fight, HERO_D};
use quartz_path_forge::Branch;

pub use crate::atlas::START;

/// What the fallen leave, which buys levels and forging (the player's word for it; the code says
/// `souls`), and the same with a capital.
pub const CURRENCY: &str = "echoes";
pub const CURRENCY_TITLE: &str = "Echoes";
/// The most enemies any place holds: the game makes this many enemy objects.
pub const MOST_FOES: usize = 5;
/// How near the shrine the hero must stand to rest, metres.
pub const SHRINE_REACH: f32 = 3.0;
/// Metres within which a living enemy, ahead or behind, keeps what wants a quiet moment (a cache
/// taken, a boss stepping out, a side path left) from happening.
const QUIET: f32 = 8.0;
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
}

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
    /// The smith raised the weapon or the armour to this.
    Forged(&'static str, u32),
    /// Met someone on the road.
    Met(&'static str),
    /// A piece of gear off the dead, and one put on (or a place left empty: "nothing").
    Found(&'static str),
    Wore(&'static str),
    /// A boss felled, and the souls it gave.
    Vanquished(&'static str, u32),
    /// The smith forged this from a boss's remnant.
    Crafted(&'static str),
    /// Foes sprang out (true: from a chest).
    Ambushed(bool),
    /// The last boss beaten: the pilgrimage begins again, harder (its count).
    Pilgrimage(u32),
}

/// Souls left where the hero fell: the place, metres into it as it is laid out, how many.
#[derive(Clone, Debug, PartialEq)]
pub struct Stain { pub stop: String, pub at: f32, pub souls: u32 }

#[derive(Clone, Debug)]
pub struct World {
    /// The stop the hero is in, and the walk's metres when they arrived.
    pub at: String,
    pub base: f64,
    /// `Next` sent from this stop (wait for the arrival).
    pub sent: bool,
    /// A warp sent and not yet seen in the walk's state, and how far into that stop it lands.
    pub warping: Option<String>,
    warp_into: f32,
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
    /// Shards carried, for the smith.
    pub shards: u32,
    /// Whether the keeper has spoken her first words, and the smith been met on the road (he waits
    /// at the shrines after).
    pub met_keeper: bool,
    pub met_smith: bool,
    /// Times each has been talked to (the keeper, the smith): the next thing they say.
    pub talks: [u32; 2],
    /// What the hero owns and wears.
    pub gear: crate::gear::Loadout,
    /// How many of each kind of enemy have fallen (for what they leave), and a count that stands
    /// in for chance.
    pub fallen: [u32; Archetype::ALL.len()],
    pub luck: u32,
    /// Places whose boss has fallen, this pilgrimage.
    pub beaten: Vec<&'static str>,
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
            at: START.into(), base: walked, sent: false, warping: None, warp_into: 0.0, checkpoint: START.into(), rested: Vec::new(), souls: 0, stain: None,
            pilgrimages: 0, signpost: None, lean: None, chose: false, chose_at: None, took: None, shards: 0, met_keeper: false, met_smith: false, talks: [0, 0],
            gear: crate::gear::Loadout::starting(), fallen: [0; Archetype::ALL.len()], luck: 0, beaten: Vec::new(), keys: Vec::new(), remnants: Vec::new(), opened: Vec::new(),
            seen: vec![atlas::node(START)], slain: Vec::new(), spawned: Vec::new(), detour: None, explored: Vec::new(), taken: Vec::new(),
            cleared: Vec::new(), told: false, lane: None, keep_hero: false, held_told: false, carry: None,
        }
    }

    /// The place the hero is in.
    pub fn here(&self) -> &'static Place { atlas::place_of(atlas::node(&self.at)).unwrap_or(&atlas::PLACES[0]) }
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
    fn road_at(&self, m: f32) -> f64 { self.base + (HERO_D + self.along(m)) as f64 }

    /// How strong the place's enemies are against the road's first: (health, blows). Deeper
    /// regions and each pilgrimage again make them stronger.
    pub fn scale(&self) -> (f32, f32) {
        let tier = atlas::REGIONS[self.here().region].tier as f32 - 1.0;
        let again = 1.0 + 0.5 * self.pilgrimages as f32;
        ((1.0 + 0.3 * tier).max(0.8) * again, (1.0 + 0.22 * tier).max(0.85) * again)
    }

    /// The enemies of the stop the hero is in that are not down, placed from where they arrived
    /// (mirrored walking the place the other way), as strong as the region makes them.
    pub fn foes(&mut self) -> Vec<Enemy> {
        let p = self.here();
        let scale = self.scale();
        let flip = if self.forward() { 1 } else { -1 };
        self.spawned = (0..p.foes.len()).filter(|&i| !self.slain.contains(&(p.id, i))).collect();
        self.spawned.iter().map(|&i| { let (k, ahead, lane) = p.foes[i]; Enemy::new(k, self.road_at(ahead), lane * flip).scaled(scale).unaware() }).collect()
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
        let was = self.here();
        let first = self.seen.is_empty();
        self.at = stop.to_owned();
        self.base = walked - into as f64;
        self.sent = false;
        self.told = false;
        self.held_told = false;
        self.signpost = None;
        self.lean = None;
        self.chose = false;
        self.chose_at = None;
        self.took = None;
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

    /// Whether the hero can turn round here: on the road (not down a side path, nor mid-fork,
    /// nor waiting on a warp), no enemy close.
    pub fn can_turn(&self, fight: &Fight, walked: f64) -> bool {
        self.detour.is_none() && self.warping.is_none() && !self.chose && self.from().is_some() && !fight.hero.dead
            && fight.enemies.iter().all(|e| !e.alive() || e.gap(walked).abs() > 8.0)
    }

    /// Turn round: the walk goes back the way it came, from where the hero stands (from a dead
    /// end, back up the road out of it).
    pub fn turn(&mut self, fight: &Fight, walked: f64) -> Option<Go> {
        let from = self.from()?.to_owned();
        self.mark_slain(fight);
        let p = self.here();
        let into = ((walked - self.base) as f32).clamp(0.0, p.length);
        let to = atlas::turned(p.id, &from);
        let in_place = atlas::node(&to) == p.id;
        let back_into = if in_place { p.length - into } else { 0.0 };
        // Turned about where they stand: what was behind is ahead, as it was.
        self.carry = in_place.then(|| fight.enemies.iter().zip(&self.spawned).filter(|(e, _)| e.alive()).map(|(e, &i)| (i, e.clone(), e.gap(walked))).collect());
        self.keep_hero = true;
        self.warping = Some(to.clone());
        self.warp_into = back_into;
        Some(Go::WarpAt(to, back_into))
    }

    /// What is left at a place's end: its key item, else the piece laid there, else a piece of
    /// gear the hero lacks (not a boss's, nor one the smith forges, nor one laid somewhere), or
    /// souls.
    fn open_cache(&mut self) -> Vec<Note> {
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
    pub fn boss_beaten(&self) -> bool { self.beaten.contains(&self.here().id) }

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

    /// At the place's end with the way on held: the walk goes no further.
    pub fn held_at_end(&self, fight: &Fight, walked: f64) -> bool {
        walked - self.base >= self.here().length as f64 && self.held(fight).is_some()
    }

    /// No living enemy within `QUIET` metres, ahead or behind.
    fn quiet(fight: &Fight, walked: f64) -> bool {
        fight.enemies.iter().all(|e| !e.alive() || e.gap(walked).abs() > QUIET)
    }

    /// The boss here, if one steps out now: this place has one not yet beaten, no enemy is on the
    /// hero (one run past may still be coming), and the hero has walked as far as it waits.
    pub fn boss_due(&self, fight: &Fight, walked: f64) -> Option<&'static atlas::Boss> {
        let (at, boss) = self.here().boss?;
        (!self.boss_beaten() && Self::quiet(fight, walked) && walked - self.base >= self.along(at) as f64).then_some(boss)
    }

    /// The boss fell: its souls, what it leaves, and the road open again.
    pub fn boss_felled(&mut self) -> Vec<Note> {
        let Some((_, boss)) = self.here().boss else { return Vec::new() };
        if !self.boss_beaten() { self.beaten.push(self.here().id); }
        let n = (boss.souls as f32 * self.gear.kit().souls * (1.0 + 0.5 * self.pilgrimages as f32)).round() as u32;
        self.souls += n;
        let mut notes = vec![Note::Vanquished(boss.name(), n)];
        if let Some(k) = boss.key.filter(|k| !self.keys.contains(k)) {
            self.keys.push(k);
            notes.push(Note::Found(k.name()));
        }
        if let Some(r) = boss.remnant {
            self.remnants.push(r);
            notes.push(Note::Found(r));
        }
        if std::ptr::eq(boss, &atlas::BELL_WARDEN) {
            let bell = crate::gear::find("Warden's Bell");
            if self.gear.gain(bell) { notes.push(Note::Found(crate::gear::ITEMS[bell].name)); }
        }
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
            .find(|&(k, a)| !self.beaten_ambush(k) && into >= self.along(a.at) as f64 && (!a.chest || (!self.cache_taken() && fight.enemies.iter().all(|e| !e.alive()))))
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
    fn standing(walked: f64) -> f64 { walked + HERO_D as f64 }

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
        fight.hero = crate::fight::Hero::with(fight.hero.stats);
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
            "pilgrimages": self.pilgrimages, "shards": self.shards, "met_keeper": self.met_keeper, "met_smith": self.met_smith, "talks": self.talks,
            "gear": {
                "owned": self.gear.owned.iter().map(|&i| item(i)).collect::<Vec<_>>(),
                "worn": self.gear.worn.iter().map(|w| w.map(item)).collect::<Vec<_>>(),
            },
            "fallen": fallen, "luck": self.luck,
            "beaten": self.beaten, "keys": self.keys.iter().map(|k| k.name()).collect::<Vec<_>>(), "remnants": self.remnants,
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
        w.shards = num("shards");
        w.met_keeper = v["met_keeper"].as_bool().unwrap_or(false);
        w.met_smith = v["met_smith"].as_bool().unwrap_or(false);
        for (n, t) in w.talks.iter_mut().enumerate() { *t = v["talks"][n].as_u64().unwrap_or(0) as u32; }
        let piece = |name: &str| crate::gear::ITEMS.iter().position(|i| i.name == name);
        if let Some(owned) = v["gear"]["owned"].as_array() {
            w.gear.owned = owned.iter().filter_map(|n| piece(n.as_str()?)).collect();
            for (n, slot) in w.gear.worn.iter_mut().enumerate() {
                *slot = v["gear"]["worn"][n].as_str().and_then(piece);
            }
            let owned = w.gear.owned.clone();
            for slot in w.gear.worn.iter_mut() { if slot.is_some_and(|i| !owned.contains(&i)) { *slot = None; } }
        }
        for k in Archetype::ALL { w.fallen[k as usize] = v["fallen"][format!("{k:?}")].as_u64().unwrap_or(0) as u32; }
        w.luck = num("luck");
        w.beaten = names("beaten");
        w.keys = strs(&v["keys"]).iter().filter_map(|n| Key::ALL.into_iter().find(|k| k.name() == n)).collect();
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
    fn wake(&mut self) -> Go {
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
        fight.hero = crate::fight::Hero::with(*st);
        Some(Note::Rose(lvl))
    }

    /// What the smith could forge from the remnants carried: each recipe whose remnant is held and
    /// whose piece is not yet owned, and whether it can be paid for.
    pub fn craftable(&self) -> Vec<(&'static crate::gear::Recipe, bool)> {
        crate::gear::RECIPES.iter()
            .filter(|r| self.remnants.contains(&r.remnant) && !self.gear.owns(crate::gear::find(r.item)))
            .map(|r| (r, self.souls >= r.echoes && self.shards >= r.shards)).collect()
    }

    /// The smith forges recipe `r`'s piece: the remnant, the echoes and the shards spent.
    pub fn craft(&mut self, r: &'static crate::gear::Recipe) -> Option<Note> {
        let at = self.remnants.iter().position(|x| *x == r.remnant)?;
        let item = crate::gear::find(r.item);
        if self.gear.owns(item) || self.souls < r.echoes || self.shards < r.shards { return None; }
        self.remnants.remove(at);
        self.souls -= r.echoes;
        self.shards -= r.shards;
        self.gear.gain(item);
        Some(Note::Crafted(r.item))
    }

    /// The souls and shards the smith asks to take gear from +`lvl` to the next; None at the top.
    pub fn forge_cost(lvl: u32) -> Option<(u32, u32)> {
        (lvl < crate::fight::GEAR_MAX).then(|| (80 * (lvl + 1), lvl + 1))
    }

    /// The smith forges the weapon (0) or the armour (1) up a step, if it can be paid for.
    pub fn forge(&mut self, fight: &mut Fight, which: usize) -> Option<Note> {
        let st = &mut fight.hero.stats;
        let lvl = if which == 0 { st.weapon } else { st.armour };
        let (souls, shards) = Self::forge_cost(lvl)?;
        if self.souls < souls || self.shards < shards { return None; }
        self.souls -= souls;
        self.shards -= shards;
        if which == 0 { st.weapon += 1; } else { st.armour += 1; }
        Some(Note::Forged(if which == 0 { "Weapon" } else { "Armour" }, lvl + 1))
    }

    /// The hero fell: the souls stay where they fell (one chance to fetch them; an older stain is
    /// lost), the world's enemies stand again, and the walk wakes at the last shrine.
    pub fn died(&mut self, walked: f64) -> Go {
        let into = (Self::standing(walked) - self.base - HERO_D as f64).max(0.0) as f32;
        let at = self.along(into);
        self.stain = (self.souls > 0).then(|| Stain { stop: self.here().id.into(), at, souls: self.souls });
        self.souls = 0;
        self.slain.clear();
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
            let keyed = l.through.into_iter().chain([l.road]).find_map(|(a, c)| atlas::gated(a, c))?;
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
    fn ahead(&self) -> Vec<atlas::Lead> { self.from().map_or(Vec::new(), |f| atlas::leads(self.here().id, f)) }

    /// Branch `b` of the fork ahead.
    fn branch(&self, b: Branch) -> Option<atlas::Lead> { self.ahead().into_iter().nth(match b { Branch::Left => 0, Branch::Right => 1 }) }

    /// Why a way on cannot be walked now, if it cannot, and the place whose road is barred: the
    /// road into a junction just ahead, or the road on from it.
    fn barred_to(&self, l: &atlas::Lead) -> Option<(&'static str, Barred)> {
        l.through.into_iter().chain([l.road]).find_map(|(a, b)| atlas::barred(a, b, &self.keys, &self.opened).map(|why| (b, why)))
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
        for (a, b) in l.through.into_iter().chain([l.road]) {
            let Some(e) = atlas::edge(a, b).filter(|e| e.gate == atlas::Gate::Shortcut && e.a == a) else { continue };
            if self.opened.contains(&e.a) { continue; }
            self.opened.push(e.a);
            notes.push(Note::Opened(atlas::place_of(e.b).map_or(e.b, |p| p.title)));
        }
        notes
    }

    pub fn felled(&mut self, kinds: impl IntoIterator<Item = Archetype>) -> Vec<Note> {
        let mut notes = Vec::new();
        let tier = atlas::REGIONS[self.here().region].tier.saturating_sub(1) as f32;
        let more = self.gear.kit().souls * (1.0 + 0.6 * tier) * (1.0 + 0.5 * self.pilgrimages as f32);
        for k in kinds {
            self.souls += (k.souls() as f32 * more).round() as u32;
            self.shards += shards(k);
            // The first of a kind leaves the first piece of its kind the hero lacks; after that,
            // one in eight leaves one of them.
            let n = &mut self.fallen[k as usize];
            *n += 1;
            let first = *n == 1;
            self.luck = self.luck.wrapping_add(1);
            let roll = self.luck.wrapping_mul(0x9E37_79B9).rotate_left(13).wrapping_mul(0x85EB_CA6B);
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
        self.keys.clear();
        self.opened.clear();
        self.slain.clear();
        self.cleared.clear();
        self.explored.clear();
        self.taken.clear();
        self.rested.clear();
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
            let into = std::mem::take(&mut self.warp_into);
            let (foes, n) = self.arrive(&w, walked, into, level);
            fight.enemies = foes;
            if !std::mem::take(&mut self.keep_hero) { fight.hero = crate::fight::Hero::with(fight.hero.stats); }
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
                self.signpost = Some(walked + (left + STEER_M + SIGNPOST_PAST) as f64);
                let title = |b| self.branch(b).map_or("", |l| Self::shown(&l));
                notes.push(Note::Fork(title(Branch::Left), title(Branch::Right)));
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
        let barred = self.here().boss.is_some() && !self.boss_beaten();
        // Held: the walk stops at the end (`held_at_end`) and the hero is told what holds it,
        // while whatever they ran past catches up.
        if let Some(kind) = self.held(fight).filter(|_| far && !self.sent) {
            if !std::mem::replace(&mut self.held_told, true) { notes.push(Note::Held(kind, self.here().hold == atlas::Hold::Clear)); }
            return (None, notes);
        }
        if self.sent || !far || barred { return (None, notes); }
        // The last boss beaten: the pilgrimage is done.
        if self.here().id == FINALE {
            let (go, note) = self.conclude();
            notes.push(note);
            return (Some(go), notes);
        }
        match self.ahead().as_slice() {
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
    use crate::fight::Ai;
    use Archetype::{Husk, Lancer, Wretch};

    fn fell(f: &mut Fight) { for e in f.enemies.iter_mut() { e.ai = Ai::Dead; } }

    #[test]
    fn every_place_fits_the_enemy_objects() {
        for p in atlas::PLACES { assert!(p.foes.len() <= MOST_FOES && p.foes.iter().all(|f| f.1 < p.length), "{}", p.id); }
    }

    /// Turned back at the very start, the road's enemies all lie behind: the trunk is walked
    /// back into the lane before it, not walked forever waiting on them.
    #[test]
    fn turning_back_at_the_start_leaves_the_roads_enemies_behind_and_walks_on() {
        let mut w = World::new(0.0);
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
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        assert_eq!(f.enemies[3].kind, Archetype::Shieldbearer);
        for e in &mut f.enemies[..3] { e.ai = Ai::Dead; }
        let (go, notes) = w.tick(&mut f, 64.0, Some(START), None);
        assert_eq!((go, notes), (None, vec![Note::Held(Archetype::Shieldbearer, false)]));
        assert!(w.held_at_end(&f, 64.0) && !w.held_at_end(&f, 60.0));
        assert_eq!(w.tick(&mut f, 64.5, Some(START), None), (None, vec![]), "told once");
        // Its keeper down and the others standing: on.
        let mut v = World::new(0.0);
        let mut g = Fight::new(v.foes());
        g.enemies[3].ai = Ai::Dead;
        assert_eq!(v.tick(&mut g, 66.0, Some(START), None).0, Some(Go::Next));
        // Walked back the trunk, the same keeper holds nothing.
        let mut b = World::new(0.0);
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
        let mut w = World::new(0.0);
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
    fn a_cleared_place_walks_on_and_a_fork_goes_the_way_the_hero_stands() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        assert_eq!(f.enemies.len(), 4);
        // Not while an enemy stands ahead (no walk passes one), nor before the place is walked.
        let last = f.enemies[3].s;
        f.enemies[3].s = 75.0 + HERO_D as f64;
        assert_eq!(w.tick(&mut f, 70.0, Some(START), None).0, None);
        f.enemies[3].s = last;
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 30.0, Some(START), None).0, None);
        assert_eq!(w.tick(&mut f, 64.0, Some(START), None).0, Some(Go::Next));
        assert_eq!(w.tick(&mut f, 65.0, Some(START), None).0, None, "sent once");
        // The fork: nothing chosen in the middle lane, and at the last moment the walk waits.
        let (go, notes) = w.tick(&mut f, 66.0, Some(START), Some(12.0));
        assert_eq!((go, notes), (None, vec![Note::Fork("The Marsh Edge", "The Chapel Edge")]));
        assert!(w.signpost.is_some_and(|s| s > 66.0 + 12.0 + STEER_M as f64));
        assert!(!w.holds_at_fork(Some(12.0), 0) && w.holds_at_fork(Some(1.0), 0));
        // Leaning right is not yet a choice, and leaning left again changes it.
        f.hero.lane = 1;
        assert_eq!(w.tick(&mut f, 67.0, Some(START), Some(11.0)), (None, vec![Note::Lean(Branch::Right, "The Chapel Edge")]));
        f.hero.lane = -1;
        assert_eq!(w.tick(&mut f, 70.0, Some(START), Some(8.0)).1, vec![Note::Lean(Branch::Left, "The Marsh Edge")]);
        f.hero.lane = 1;
        w.tick(&mut f, 74.0, Some(START), Some(4.0));
        assert!(!w.holds_at_fork(Some(0.5), 1), "a side is held: walk on");
        // At the last moment the lane held is the branch, and the hero steps back to the middle.
        assert_eq!(w.tick(&mut f, 77.5, Some(START), Some(0.5)).0, Some(Go::Choose(Branch::Right)));
        assert_eq!((f.hero.lane, w.tick(&mut f, 77.6, Some(START), Some(0.4)).0), (0, None), "once");
        // Past the junction, the branch's lane walks on into where it leads, once.
        assert_eq!(w.tick(&mut f, 80.0, Some("~chapel_edge@road"), None).0, Some(Go::Next));
        assert_eq!(w.tick(&mut f, 81.0, Some("~chapel_edge@road"), None), (None, vec![]));
        assert_eq!(w.at, START, "a lane is not a place");
        // Arriving: the chapel edge's enemies, placed from there.
        let (_, notes) = w.tick(&mut f, 90.0, Some("chapel_edge@road"), None);
        assert_eq!(notes, vec![Note::Arrived("The Chapel Edge")]);
        assert_eq!((f.enemies.len(), f.enemies[0].kind), (1, Archetype::Lancer));
        assert_eq!(f.enemies[0].s, 90.0 + (HERO_D + 15.0) as f64);
        assert!(w.lean.is_none() && !w.chose);
    }

    #[test]
    fn death_wakes_at_the_last_shrine_and_leaves_the_souls_where_the_hero_fell() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        let found = w.felled([Husk, Lancer]);
        assert_eq!(found, vec![Note::Found("Husk Cleaver"), Note::Found("Lancer's Shortspear")], "the first of each kind leaves its first piece");
        assert_eq!(w.souls, 70);
        // Over to the vale; fall 20 m in.
        w.tick(&mut f, 100.0, Some("vale@vale_edge"), None);
        f.hero.hp = 0.0;
        f.hero.dead = true;
        assert_eq!(w.died(120.0), Go::Warp(START.into()));
        assert_eq!((w.souls, w.stain.clone()), (0, Some(Stain { stop: "vale".into(), at: 20.0, souls: 70 })));
        // Nothing happens until the walk shows the shrine's stop; then the road is full again.
        assert_eq!(w.tick(&mut f, 121.0, Some("vale@vale_edge"), None), (None, vec![]));
        let (_, notes) = w.tick(&mut f, 122.0, Some(START), None);
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
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        // Walked in its own way, the shrine is where the walk comes in.
        w.rest(&mut f);
        assert_eq!(w.leave_rest(), Go::Warp(START.into()));
        w.tick(&mut f, 10.0, Some(START), None);
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
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        assert!(w.can_rest(&f, 0.0), "the road starts at a shrine");
        assert!(!w.can_rest(&f, 10.0), "too far past it");
        fell(&mut f);
        f.hero.hp = 10.0;
        f.hero.flasks = 0;
        w.rest(&mut f);
        assert_eq!((f.hero.hp, f.hero.flasks, f.enemies.iter().filter(|e| e.alive()).count()), (100.0, crate::fight::FLASKS, 4));
        assert_eq!(w.travel_list(), vec![("The Road to Hollowmere", START.to_owned())]);
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
        let mut w = World::new(0.0);
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
        // Out of the sewers up into the Warrens: at the sewers' end their junction, and either way
        // on lifts the grate from below; it stays open.
        assert!(!w.opened.contains(&"sewers"));
        w.tick(&mut f, 300.0, Some("sewers@lower_stairs"), None);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 361.0, Some("sewers@lower_stairs"), None).0, Some(Go::Next));
        f.hero.lane = -1;
        let (go, notes) = w.tick(&mut f, 362.0, Some("sewers@lower_stairs"), Some(0.5));
        assert_eq!(go, Some(Go::Choose(Branch::Left)));
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
        w.tick(&mut f, 200.0, Some("spire_summit@sealed_slab"), None);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 236.0, Some("spire_summit@sealed_slab"), None), (None, vec![]), "the Archangel bars the way");
        assert!(w.boss_due(&f, 236.0).is_some());
        w.boss_felled();
        let (go, notes) = w.tick(&mut f, 236.0, Some("spire_summit@sealed_slab"), None);
        assert_eq!((go, notes), (Some(Go::Warp(START.into())), vec![Note::Pilgrimage(1)]));
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
        (w.souls, w.shards, w.pilgrimages, w.luck, w.talks) = (812, 4, 1, 9, [2, 1]);
        (w.met_keeper, w.met_smith) = (true, true);
        w.stain = Some(Stain { stop: shrine.clone(), at: 12.5, souls: 300 });
        let kite = crate::gear::find("Kite Shield");
        w.gear.gain(kite);
        assert!(w.gear.wear(1, Some(kite)));
        w.fallen[Archetype::Lancer as usize] = 7;
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
        assert_eq!((back.checkpoint.as_str(), back.rested.len(), back.souls, back.shards, back.pilgrimages, back.luck, back.talks), (shrine.as_str(), 2, 812, 4, 1, 9, [2, 1]));
        assert!(back.met_keeper && back.met_smith);
        assert_eq!(back.stain.as_ref().map(|s| (s.at, s.souls)), Some((12.5, 300)));
        assert_eq!((back.gear.owned.clone(), back.gear.worn), (w.gear.owned.clone(), w.gear.worn));
        assert_eq!(back.gear.kit().off.map(|o| o.art), Some("kite_shield"));
        assert_eq!(back.fallen, w.fallen);
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
        let odd = World::restored(0.0, &v).unwrap();
        assert_eq!(odd.checkpoint, START);
        assert_eq!(odd.gear.owned, vec![kite]);
        assert!(World::restored(0.0, &serde_json::json!("not a world")).is_none());
    }
}
