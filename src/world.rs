//! The world the road walks through: the journey's places (`assets/scenes/hollowmere.journey.json`),
//! the enemies each holds, when the walk goes on to the next, which way a fork goes, and the
//! shrines (the bonfires): rest to be made whole, rise in level and have the place's enemies back;
//! die to wake at the last one rested at, with the souls carried left where the hero fell.
//!
//! Everything here is in road metres (`walked`, never coming round), so it is tested without
//! PathForge; the game reads the walk's state and sends what `World` asks for.

use crate::fight::{Archetype, Enemy, Fight, HERO_D};
use quartz_path_forge::Branch;

/// One place on the journey.
pub struct Place {
    /// The journey's stop.
    pub id: &'static str,
    pub title: &'static str,
    /// (kind, metres past where the hero arrives, lane).
    pub foes: &'static [(Archetype, f32, i32)],
    /// Metres walked from arriving before the road goes on (once every enemy is down).
    pub length: f32,
    /// A shrine this many metres in, if the place has one.
    pub shrine: Option<f32>,
    /// The wandering smith waits by the road this many metres in, until met.
    pub smith: Option<f32>,
    /// A boss bars the way this many metres in, once the place's other enemies are down: the
    /// camera swings round and it is fought side-on (`side`). The road goes on once it falls.
    pub boss: Option<f32>,
    /// The stop its side paths lead to (the scene's `path.fork`), once per pilgrimage: the hero
    /// turns down one and walks to the end of it, and comes back to where they left the road.
    pub explore: Option<&'static str>,
    /// Something left at the end of the place this many metres in: a piece of gear the hero
    /// lacks, or souls.
    pub cache: Option<f32>,
    /// Ambushes waiting along it (`Ambush`).
    pub ambushes: &'static [Ambush],
}

/// Foes that spring out on the hero: the camera swings round and they are fought side-on
/// (`side::SideFight::ambush`) until every one is down. Sprung `at` metres into the place, or, for
/// a `chest`, on reaching the place's cache, which then opens only once they are beaten. Each
/// stands `s` metres along the road from the hero (both sides: it is an ambush). Once beaten it
/// stays beaten, like the boss.
#[derive(Debug)]
pub struct Ambush { pub at: f32, pub foes: &'static [(Archetype, f32)], pub chest: bool }

use Archetype::{Husk, Lancer, Shieldbearer, Wretch};

/// The first stretch teaches one kind at a time; each branch then leans on what it taught (the
/// marsh: fast wretches in packs; the chapel: shields and lances together); the gate mixes them.
pub const PLACES: [Place; 7] = [
    Place { id: "road", title: "The Road to Hollowmere", length: 64.0, shrine: Some(1.5), smith: Some(59.0), boss: None, explore: None, cache: None, ambushes: &[],
        foes: &[(Husk, 12.0, 0), (Lancer, 26.0, 1), (Wretch, 38.0, -1), (Shieldbearer, 52.0, 0)] },
    // Each branch begins under the road's own sky, its ground and trees turning toward where it
    // leads; the change of light comes after, through a crossing (a blend into the marsh, a ruined
    // arch into the graveyard), which handles it far better than a fork can.
    Place { id: "vale_edge", title: "The Marsh Edge", length: 26.0, shrine: None, smith: None, boss: None, explore: None, cache: None, ambushes: &[],
        foes: &[(Wretch, 14.0, 0)] },
    Place { id: "chapel_edge", title: "The Chapel Edge", length: 26.0, shrine: None, smith: None, boss: None, explore: None, cache: None, ambushes: &[],
        foes: &[(Lancer, 15.0, 0)] },
    // The marsh's wretches come as a pack out of the reeds; the graves hold a lancer and a husk.
    Place { id: "vale", title: "The Wretched Vale", length: 56.0, shrine: None, smith: None, boss: None, explore: None, cache: None,
        ambushes: &[Ambush { at: 30.0, foes: &[(Wretch, -3.4), (Wretch, 3.0), (Husk, 4.6)], chest: false }],
        foes: &[(Wretch, 12.0, -1), (Wretch, 15.0, 1), (Husk, 26.0, 0), (Wretch, 36.0, 0), (Lancer, 46.0, -1)] },
    Place { id: "chapel", title: "Chapel Road", length: 58.0, shrine: None, smith: None, boss: None, explore: Some("hollow"), cache: None,
        ambushes: &[Ambush { at: 38.0, foes: &[(Lancer, -3.6), (Husk, 3.2)], chest: false }],
        foes: &[(Shieldbearer, 12.0, 0), (Lancer, 22.0, 1), (Husk, 32.0, -1), (Shieldbearer, 44.0, 0), (Lancer, 47.0, -1)] },
    Place { id: "gate", title: "Hollowmere Gate", length: 46.0, shrine: Some(2.0), smith: None, boss: Some(40.0), explore: None, cache: None, ambushes: &[],
        foes: &[(Lancer, 14.0, 1), (Shieldbearer, 18.0, -1), (Wretch, 30.0, 0), (Husk, 36.0, 1)] },
    // Down a side path off Chapel Road: a dead end among the graves, with something left there,
    // and the chest is a trap.
    Place { id: "hollow", title: "A Hollow Among the Graves", length: 24.0, shrine: None, smith: None, boss: None, explore: None, cache: Some(20.0),
        ambushes: &[Ambush { at: 19.0, foes: &[(Husk, 3.0), (Wretch, -3.2)], chest: true }],
        foes: &[(Wretch, 11.0, 0), (Husk, 15.0, 1)] },
];

/// The most enemies any place holds: the game makes this many enemy objects.
pub const MOST_FOES: usize = 5;
/// Where the walk begins, and the first shrine.
pub const START: &str = "road";
/// How near the shrine the hero must stand to rest, metres.
pub const SHRINE_REACH: f32 = 3.0;
/// The journey's stop where the hero sits at rest: a clearing ringed by mist, the keeper on its
/// left, the shrine in its middle, the smith (once met) on its right. Not a place on the road.
pub const SANCTUM: &str = "sanctum";
/// Shards (what the smith forges with) each kind leaves when felled.
pub fn shards(k: Archetype) -> u32 { match k { Lancer => 1, Shieldbearer => 2, _ => 0 } }
/// Metres before the last moment to choose at which the lane held becomes the branch taken:
/// leaning into a lane says which way, and the choice is made late so the camera turns onto the
/// branch right at the junction (and the player can change their mind until then).
pub const COMMIT_M: f32 = 0.8;
/// The camera turns onto a branch over this many metres (the journey's `steer_m`).
pub const STEER_M: f32 = 6.0;
/// Where the signpost stands past the junction: on the island where the branches part
/// (the paths' width over the angle between them, for the road's 2.4 m half width at 35 degrees).
pub const SIGNPOST_PAST: f32 = 4.4;
/// Sideways metres per metre of each branch (the plan PathForge makes for this canvas: 35 degrees).
pub const FORK_SLOPE: f32 = 0.689;

pub fn place(id: &str) -> Option<&'static Place> { PLACES.iter().find(|p| p.id == id) }

/// What the game should tell the road to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Go {
    /// Walk on to where the journey leads.
    Next,
    /// Take this branch of the fork ahead.
    Choose(Branch),
    /// Cut to this stop (waking at a shrine; a pilgrimage done, the road again).
    Warp(String),
}

/// Something worth a line on the screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Arrived(&'static str),
    ShrineFound,
    Rested,
    Rose(u32),
    Fork,
    /// Leaning toward a branch at a fork.
    Lean(Branch),
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
    /// Foes sprang out (true: from a chest).
    Ambushed(bool),
    /// The journey's end reached: the road again, every place full.
    Pilgrimage(u32),
}

/// Souls left where the hero fell: the stop, metres past arriving there, how many.
#[derive(Clone, Debug, PartialEq)]
pub struct Stain { pub stop: String, pub at: f32, pub souls: u32 }

#[derive(Clone, Debug)]
pub struct World {
    /// The stop the hero is in, and the walk's metres when they arrived.
    pub at: String,
    pub base: f64,
    /// `Next` sent from this stop (wait for the arrival).
    pub sent: bool,
    /// A warp sent and not yet seen in the walk's state.
    pub warping: Option<String>,
    /// The shrine last rested at (where death wakes the hero).
    pub checkpoint: String,
    pub souls: u32,
    pub stain: Option<Stain>,
    /// Journeys walked to the end.
    pub pilgrimages: u32,
    /// The fork's signpost, road metres, once the fork is in view.
    pub signpost: Option<f64>,
    /// The branch the hero leans toward at a fork (the lane they hold), and whether it is taken.
    pub lean: Option<Branch>,
    pub chose: bool,
    /// Where the walk was when the branch was taken (the camera turns onto it from there).
    pub chose_at: Option<f64>,
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
    pub fallen: [u32; 4],
    pub luck: u32,
    /// The boss of the place that has one has fallen, this pilgrimage.
    pub boss_beaten: bool,
    /// Down a side path: where on the road the hero left it, to come back to.
    pub detour: Option<Detour>,
    /// Side paths walked this pilgrimage (by the stop they lead to), and whether this one's cache
    /// is taken.
    pub explored: Vec<&'static str>,
    pub cache_taken: bool,
    /// Ambushes beaten: (place, which).
    pub cleared: Vec<(&'static str, usize)>,
    /// The next warp keeps the hero as they are (a detour), rather than whole again (a respawn).
    keep_hero: bool,
}

/// Where the hero left the road for a side path: the stop, metres in, its enemies as they were
/// (positions from where the place began), and the side it opened on.
#[derive(Clone, Debug, PartialEq)]
pub struct Detour { pub from: String, pub at: f64, pub foes: Vec<Enemy>, pub side: f32 }

/// How near a side path's mouth the hero must stand to turn down it, metres.
pub const SIDE_REACH: f32 = 1.6;

impl World {
    pub fn new(walked: f64) -> World {
        World { at: START.into(), base: walked, sent: false, warping: None, checkpoint: START.into(), souls: 0, stain: None, pilgrimages: 0, signpost: None, lean: None, chose: false, chose_at: None, shards: 0, met_keeper: false, met_smith: false, talks: [0, 0], gear: crate::gear::Loadout::starting(), fallen: [0; 4], luck: 0, boss_beaten: false, detour: None, explored: Vec::new(), cache_taken: false, cleared: Vec::new(), keep_hero: false }
    }

    pub fn here(&self) -> &'static Place { place(&self.at).unwrap_or(&PLACES[0]) }

    /// The enemies of the stop the hero is in, placed from where they arrived.
    pub fn foes(&self) -> Vec<Enemy> {
        self.here().foes.iter().map(|&(k, ahead, lane)| Enemy::new(k, self.base + (HERO_D + ahead) as f64, lane)).collect()
    }

    /// The shrine's place on the road, metres, if this stop has one.
    pub fn shrine(&self) -> Option<f64> { self.here().shrine.map(|m| self.base + (HERO_D + m) as f64) }

    /// The souls left on the road here, metres, if the hero fell in this stop.
    pub fn stain_at(&self) -> Option<f64> {
        self.stain.as_ref().filter(|s| s.stop == self.at).map(|s| self.base + (HERO_D + s.at) as f64)
    }

    /// Arrived at `stop` (`walked` metres in): its enemies, and what to say.
    pub fn arrive(&mut self, stop: &str, walked: f64) -> (Vec<Enemy>, Vec<Note>) {
        self.at = stop.to_owned();
        self.base = walked;
        self.sent = false;
        self.signpost = None;
        self.lean = None;
        self.chose = false;
        self.chose_at = None;
        let mut notes = vec![Note::Arrived(self.here().title)];
        if self.here().shrine.is_some() { notes.push(Note::ShrineFound); }
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
        self.cache_taken = false;
        self.keep_hero = true;
        self.warping = Some(to.into());
        Some(Go::Warp(to.into()))
    }

    /// Whether the hero can turn back from the side path now: at its end, every enemy down.
    pub fn can_come_back(&self, fight: &Fight, walked: f64) -> bool {
        self.detour.is_some() && fight.enemies.iter().all(|e| !e.alive()) && walked - self.base >= self.here().length as f64 - 2.0
    }

    /// Back up the side path to where the hero left the road.
    pub fn come_back(&mut self) -> Option<Go> {
        let from = self.detour.as_ref()?.from.clone();
        self.keep_hero = true;
        self.warping = Some(from.clone());
        Some(Go::Warp(from))
    }

    /// What is left at a side path's end: the first piece of gear the hero lacks (not the
    /// boss's), or souls.
    fn open_cache(&mut self) -> Vec<Note> {
        let bell = crate::gear::find("Warden's Bell");
        let lacking: Vec<usize> = (6..crate::gear::ITEMS.len()).filter(|&i| i != bell && !self.gear.owns(i)).collect();
        match (!lacking.is_empty()).then(|| lacking[self.luck as usize % lacking.len()]) {
            Some(i) => { self.gear.gain(i); vec![Note::Found(crate::gear::ITEMS[i].name)] }
            None => { self.souls += 300; vec![Note::Recovered(300)] }
        }
    }

    /// Whether the boss steps out now: this place has one not yet beaten, every other enemy here
    /// is down, and the hero has walked as far as it waits.
    pub fn boss_due(&self, fight: &Fight, walked: f64) -> bool {
        let Some(at) = self.here().boss else { return false };
        !self.boss_beaten && fight.enemies.iter().all(|e| !e.alive()) && walked - self.base >= at as f64
    }

    /// The boss fell: its souls, its talisman, and the road open again.
    pub fn boss_felled(&mut self, name: &'static str, souls: u32) -> Vec<Note> {
        self.boss_beaten = true;
        let n = (souls as f32 * self.gear.kit().souls).round() as u32;
        self.souls += n;
        let mut notes = vec![Note::Vanquished(name, n)];
        let bell = crate::gear::find("Warden's Bell");
        if self.gear.gain(bell) { notes.push(Note::Found(crate::gear::ITEMS[bell].name)); }
        notes
    }

    /// Whether ambush `k` of the place the hero is in has been beaten.
    fn beaten(&self, k: usize) -> bool { self.cleared.contains(&(self.here().id, k)) }

    /// Whether this place's cache is a trap not yet sprung and beaten.
    fn trapped(&self) -> bool { self.here().ambushes.iter().enumerate().any(|(k, a)| a.chest && !self.beaten(k)) }

    /// The ambush to spring where the hero stands, if any: past its metre, not beaten, none of the
    /// road's own enemies close (the ambush is its own fight), the hero on their feet.
    pub fn ambush_due(&self, fight: &Fight, walked: f64) -> Option<usize> {
        if fight.hero.dead || self.warping.is_some() { return None; }
        if fight.enemies.iter().any(|e| e.alive() && e.gap(walked).abs() < 8.0) { return None; }
        let into = walked - self.base;
        self.here().ambushes.iter().enumerate()
            .find(|&(k, a)| !self.beaten(k) && into >= a.at as f64 && (!a.chest || (!self.cache_taken && fight.enemies.iter().all(|e| !e.alive()))))
            .map(|(k, _)| k)
    }

    /// Ambush `k` beaten: remembered, its dead's souls and leavings, and a trapped chest opened.
    pub fn ambush_cleared(&mut self, k: usize, fallen: Vec<Archetype>) -> Vec<Note> {
        let at = (self.here().id, k);
        if !self.cleared.contains(&at) { self.cleared.push(at); }
        let mut notes = self.felled(fallen);
        if self.here().ambushes[k].chest && !self.cache_taken {
            self.cache_taken = true;
            notes.extend(self.open_cache());
        }
        notes
    }

    /// Where the hero stands on the road: the walk's metres plus the hero's own distance.
    fn standing(walked: f64) -> f64 { walked + HERO_D as f64 }

    /// Whether the hero can rest now: by the shrine with no enemy close.
    pub fn can_rest(&self, fight: &Fight, walked: f64) -> bool {
        let Some(s) = self.shrine() else { return false };
        let near = (s - Self::standing(walked)).abs() <= SHRINE_REACH as f64;
        near && !fight.hero.dead && fight.enemies.iter().all(|e| !e.alive() || e.gap(walked) > 8.0)
    }

    /// Rest at the shrine: whole again, the place's enemies back, death wakes here.
    pub fn rest(&mut self, fight: &mut Fight) -> Vec<Note> {
        self.checkpoint = self.at.clone();
        fight.hero = crate::fight::Hero::with(fight.hero.stats);
        fight.enemies = self.foes();
        vec![Note::Rested]
    }

    /// Where the smith waits on the road here, metres, until he is met.
    pub fn smith_at(&self) -> Option<f64> {
        if self.met_smith { return None; }
        self.here().smith.map(|m| self.base + (HERO_D + m) as f64)
    }

    /// Whether the hero stands by the smith with no enemy close.
    pub fn can_meet_smith(&self, fight: &Fight, walked: f64) -> bool {
        let Some(s) = self.smith_at() else { return false };
        (s - Self::standing(walked)).abs() <= SHRINE_REACH as f64 && !fight.hero.dead
            && fight.enemies.iter().all(|e| !e.alive() || e.gap(walked) > 8.0)
    }

    /// Up from rest: back to the shrine's place (its enemies there again, as resting brings them).
    pub fn leave_rest(&mut self) -> Go {
        self.warping = Some(self.checkpoint.clone());
        Go::Warp(self.checkpoint.clone())
    }

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
    /// lost), and the walk wakes at the last shrine.
    pub fn died(&mut self, walked: f64) -> Go {
        let at = (Self::standing(walked) - self.base - HERO_D as f64).max(0.0) as f32;
        self.stain = (self.souls > 0).then(|| Stain { stop: self.at.clone(), at, souls: self.souls });
        self.souls = 0;
        self.warping = Some(self.checkpoint.clone());
        Go::Warp(self.checkpoint.clone())
    }

    /// The souls of every enemy felled this tick.
    /// Where the signpost stands across the road, metres: in the middle until a branch is taken;
    /// as the camera turns onto it, everything past the junction swings the other way, the island
    /// and its signpost with it.
    pub fn signpost_x(&self, walked: f64) -> f32 {
        let (Some(at), Some(b)) = (self.chose_at, self.lean) else { return 0.0 };
        let t = ((walked - at) as f32 / STEER_M).clamp(0.0, 1.0);
        let steer = t * t * (3.0 - 2.0 * t);
        let side = match b { Branch::Left => -1.0, Branch::Right => 1.0 };
        -side * steer * FORK_SLOPE * SIGNPOST_PAST
    }

    /// Whether the walk waits at the fork for the hero to pick a side (still in the middle lane at
    /// the last moment): the junction never chooses for them.
    pub fn holds_at_fork(&self, choose_within: Option<f32>, lane: i32) -> bool {
        !self.chose && lane == 0 && choose_within.is_some_and(|l| l <= COMMIT_M + 0.6)
    }

    pub fn felled(&mut self, kinds: impl IntoIterator<Item = Archetype>) -> Vec<Note> {
        let mut notes = Vec::new();
        let more = self.gear.kit().souls;
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

    /// Each tick: what the road should do, and what to say. `stop` is the walk's stop now,
    /// `choose_within` the metres left to choose a branch (None: no fork waiting), `end` whether
    /// the stop leads nowhere.
    pub fn tick(&mut self, fight: &mut Fight, walked: f64, stop: Option<&str>, choose_within: Option<f32>, end: bool) -> (Option<Go>, Vec<Note>) {
        let mut notes = Vec::new();
        // A warp waits until the walk shows it; then the place starts over.
        if let Some(w) = &self.warping {
            if stop != Some(w.as_str()) { return (None, notes); }
            let w = w.clone();
            self.warping = None;
            let (foes, n) = self.arrive(&w, walked);
            fight.enemies = foes;
            if !std::mem::take(&mut self.keep_hero) { fight.hero = crate::fight::Hero::with(fight.hero.stats); }
            // Back from a side path: where the hero left the road, its dead still dead.
            if let Some(d) = self.detour.take_if(|d| d.from == w) {
                self.base = walked - d.at;
                fight.enemies = d.foes.into_iter().map(|mut e| { e.s += self.base; e }).collect();
                return (None, vec![Note::Arrived(self.here().title)]);
            }
            return (None, n);
        }
        if let Some(s) = stop.filter(|s| *s != self.at) {
            let (foes, n) = self.arrive(s, walked);
            fight.enemies = foes;
            return (None, n);
        }
        // The souls left here, fetched by walking over them.
        if let Some(s) = self.stain_at() {
            if Self::standing(walked) >= s {
                let n = self.stain.take().map_or(0, |s| s.souls);
                self.souls += n;
                notes.push(Note::Recovered(n));
            }
        }
        // A fork in view: the lane the hero holds says which way, taken at the last moment.
        if let Some(left) = choose_within {
            if self.signpost.is_none() {
                self.signpost = Some(walked + (left + STEER_M + SIGNPOST_PAST) as f64);
                notes.push(Note::Fork);
            }
            if self.chose { return (None, notes); }
            let lean = match fight.hero.lane { l if l < 0 => Some(Branch::Left), l if l > 0 => Some(Branch::Right), _ => None };
            if lean != self.lean && lean.is_some() { notes.push(Note::Lean(lean.unwrap())); }
            self.lean = lean;
            if let (true, Some(b)) = (left <= COMMIT_M, lean) {
                self.chose = true;
                self.chose_at = Some(walked);
                // Onto the middle of the branch while the camera turns onto it.
                fight.hero.lane = 0;
                return (Some(Go::Choose(b)), notes);
            }
            return (None, notes);
        }
        let clear = fight.enemies.iter().all(|e| !e.alive());
        // What was left at the end of a side path, taken by walking up to it.
        if let (Some(c), false, false) = (self.here().cache, self.cache_taken, self.trapped()) {
            if clear && walked - self.base >= c as f64 - 1.0 {
                self.cache_taken = true;
                notes.extend(self.open_cache());
            }
        }
        // A side path's end goes nowhere: the hero turns back (`come_back`).
        if self.detour.is_some() { return (None, notes); }
        let far = walked - self.base >= self.here().length as f64;
        let barred = self.here().boss.is_some() && !self.boss_beaten;
        if self.sent || !clear || !far || barred { return (None, notes); }
        self.sent = true;
        if end {
            // The end of the journey: back to its start, every place full again.
            self.pilgrimages += 1;
            self.checkpoint = START.into();
            self.warping = Some(START.into());
            notes.push(Note::Pilgrimage(self.pilgrimages));
            return (Some(Go::Warp(START.into())), notes);
        }
        (Some(Go::Next), notes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::Ai;

    fn fell(f: &mut Fight) { for e in f.enemies.iter_mut() { e.ai = Ai::Dead; } }

    #[test]
    fn every_place_is_a_stop_of_the_journey_and_fits_the_enemy_objects() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/scenes/hollowmere.journey.json");
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let stops = j["stops"].as_object().unwrap();
        assert_eq!(j["start"], START);
        for p in &PLACES {
            assert!(stops.contains_key(p.id), "{} is not a stop", p.id);
            assert!(p.foes.len() <= MOST_FOES && p.foes.iter().all(|f| f.1 < p.length));
        }
        assert!(stops.contains_key(SANCTUM));
        assert_eq!(stops.len(), PLACES.len() + 1, "every stop is a place, and the sanctum");
    }

    #[test]
    fn a_cleared_place_walks_on_and_a_fork_goes_the_way_the_hero_stands() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        assert_eq!(f.enemies.len(), 4);
        // Not while enemies stand, nor before the place is walked.
        assert_eq!(w.tick(&mut f, 70.0, Some("road"), None, false).0, None);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 30.0, Some("road"), None, false).0, None);
        assert_eq!(w.tick(&mut f, 64.0, Some("road"), None, false).0, Some(Go::Next));
        assert_eq!(w.tick(&mut f, 65.0, Some("road"), None, false).0, None, "sent once");
        // The fork: nothing chosen in the middle lane, and at the last moment the walk waits.
        let (go, notes) = w.tick(&mut f, 66.0, Some("road"), Some(12.0), false);
        assert_eq!((go, notes), (None, vec![Note::Fork]));
        assert!(w.signpost.is_some_and(|s| s > 66.0 + 12.0 + STEER_M as f64));
        assert!(!w.holds_at_fork(Some(12.0), 0) && w.holds_at_fork(Some(1.0), 0));
        // Leaning right is not yet a choice, and leaning left again changes it.
        f.hero.lane = 1;
        assert_eq!(w.tick(&mut f, 67.0, Some("road"), Some(11.0), false), (None, vec![Note::Lean(Branch::Right)]));
        f.hero.lane = -1;
        assert_eq!(w.tick(&mut f, 70.0, Some("road"), Some(8.0), false).1, vec![Note::Lean(Branch::Left)]);
        f.hero.lane = 1;
        w.tick(&mut f, 74.0, Some("road"), Some(4.0), false);
        assert!(!w.holds_at_fork(Some(0.5), 1), "a side is held: walk on");
        // At the last moment the lane held is the branch, and the hero steps back to the middle.
        assert_eq!(w.tick(&mut f, 77.5, Some("road"), Some(0.5), false).0, Some(Go::Choose(Branch::Right)));
        assert_eq!((f.hero.lane, w.tick(&mut f, 77.6, Some("road"), Some(0.4), false).0), (0, None), "once");
        // Arriving: the chapel edge's enemies, placed from there.
        let (_, notes) = w.tick(&mut f, 90.0, Some("chapel_edge"), None, false);
        assert_eq!(notes, vec![Note::Arrived("The Chapel Edge")]);
        assert_eq!((f.enemies.len(), f.enemies[0].kind), (1, Archetype::Lancer));
        assert_eq!(f.enemies[0].s, 90.0 + (HERO_D + 15.0) as f64);
        assert!(w.lean.is_none() && !w.chose);
    }

    #[test]
    fn death_wakes_at_the_last_shrine_and_leaves_the_souls_where_the_hero_fell() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(w.foes());
        let found = w.felled([Archetype::Husk, Archetype::Lancer]);
        assert_eq!(found, vec![Note::Found("Husk Cleaver"), Note::Found("Lancer's Shortspear")], "the first of each kind leaves its first piece");
        assert_eq!(w.souls, 70);
        // Over to the vale; fall 20 m in.
        w.tick(&mut f, 100.0, Some("vale"), None, false);
        f.hero.hp = 0.0;
        f.hero.dead = true;
        assert_eq!(w.died(120.0), Go::Warp("road".into()));
        assert_eq!((w.souls, w.stain.clone()), (0, Some(Stain { stop: "vale".into(), at: 20.0, souls: 70 })));
        // Nothing happens until the walk shows the shrine's stop; then the road is full again.
        assert_eq!(w.tick(&mut f, 121.0, Some("vale"), None, false), (None, vec![]));
        let (_, notes) = w.tick(&mut f, 122.0, Some("road"), None, false);
        assert_eq!(notes, vec![Note::Arrived("The Road to Hollowmere"), Note::ShrineFound]);
        assert!(!f.hero.dead && f.hero.hp == f.hero.stats.max_hp() && f.enemies.len() == 4 && f.enemies.iter().all(|e| e.alive()));
        // Back in the vale, walking over the stain fetches them.
        w.tick(&mut f, 300.0, Some("vale"), None, false);
        assert_eq!(w.tick(&mut f, 319.0, Some("vale"), None, false).1, vec![]);
        assert_eq!(w.tick(&mut f, 320.0, Some("vale"), None, false).1, vec![Note::Recovered(70)]);
        assert_eq!((w.souls, w.stain.is_none()), (70, true));
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
        assert_eq!(w.rise(&mut f, 0), None, "no souls");
        w.souls = 145;
        assert_eq!(w.rise(&mut f, 0), Some(Note::Rose(1)));
        assert_eq!((w.souls, f.hero.hp, f.hero.stats.level_cost()), (85, 108.0, 80));
        assert_eq!(w.rise(&mut f, 2), Some(Note::Rose(2)));
        // The starting sword takes might at C.
        assert!(w.souls == 5 && (f.hero.stats.damage() - 1.045).abs() < 1e-6, "{}", f.hero.stats.damage());
    }

    #[test]
    fn a_side_path_is_walked_to_its_end_and_the_road_is_as_it_was_left() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 100.0, Some("chapel"), None, false);
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
        w.tick(&mut f, 130.0, Some("hollow"), None, false);
        assert_eq!((w.at.as_str(), f.hero.hp), ("hollow", 40.0));
        assert!(f.enemies.iter().all(|e| e.alive()) && w.side_path_at(&f, 130.0, &mouth).is_none());
        assert!(!w.can_come_back(&f, 154.0), "not with the hollow's dead standing");
        fell(&mut f);
        let (go, notes) = w.tick(&mut f, 152.0, Some("hollow"), None, true);
        assert_eq!(go, None, "a side path leads nowhere, even at the journey's end");
        // Its cache is a trapped chest: beaten, it opens.
        assert!(notes.is_empty() && w.ambush_due(&f, 152.0) == Some(0), "{notes:?}");
        let notes = w.ambush_cleared(0, Vec::new());
        assert!(matches!(notes.as_slice(), [Note::Found(_)]), "{notes:?}");
        assert!(w.can_come_back(&f, 153.0));
        assert_eq!(w.come_back(), Some(Go::Warp("chapel".into())));
        let (_, notes) = w.tick(&mut f, 170.0, Some("chapel"), None, false);
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
        w.tick(&mut f, 100.0, Some("vale"), None, false);
        fell(&mut f);
        assert_eq!(w.ambush_due(&f, 100.0 + 29.0), None, "not yet");
        assert_eq!(w.ambush_due(&f, 100.0 + 30.5), Some(0));
        let souls = w.souls;
        let notes = w.ambush_cleared(0, vec![Wretch, Wretch, Husk]);
        assert!(w.souls > souls && !notes.is_empty());
        assert_eq!(w.ambush_due(&f, 100.0 + 31.0), None, "beaten stays beaten");
        // The hollow's chest: walking up to it springs the trap instead of opening it.
        w.tick(&mut f, 200.0, Some("hollow"), None, false);
        fell(&mut f);
        let (_, notes) = w.tick(&mut f, 200.0 + 21.0, Some("hollow"), None, true);
        assert!(!w.cache_taken && notes.is_empty(), "{notes:?}");
        assert_eq!(w.ambush_due(&f, 200.0 + 21.0), Some(0));
        let notes = w.ambush_cleared(0, vec![Husk, Wretch]);
        assert!(w.cache_taken && notes.iter().any(|n| matches!(n, Note::Found(_) | Note::Recovered(300))), "{notes:?}");
    }

    #[test]
    fn the_journeys_end_walks_the_road_again() {
        let mut w = World::new(0.0);
        let mut f = Fight::new(Vec::new());
        w.tick(&mut f, 200.0, Some("gate"), None, true);
        fell(&mut f);
        assert_eq!(w.tick(&mut f, 246.0, Some("gate"), None, true), (None, vec![]), "the Bell Warden bars the way");
        w.boss_beaten = true;
        let (go, notes) = w.tick(&mut f, 246.0, Some("gate"), None, true);
        assert_eq!((go, notes), (Some(Go::Warp("road".into())), vec![Note::Pilgrimage(1)]));
        assert_eq!(w.checkpoint, "road");
    }
}
