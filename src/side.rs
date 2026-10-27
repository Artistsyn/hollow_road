//! The side-on fight: a boss at the gate, or an ambush on the road. The camera swings round to
//! look across the road and the fight runs along it: the hero walks left and right, rolls with
//! Space and a direction (through an enemy, too, inside the roll's i-frames), jumps, strikes,
//! guards and parries as on the road, against foes whose blows come high, low along the ground,
//! as a ring rolling out across the stones, or as a charge. What jumping is for: a low sweep or
//! the ring passes under a hero in the air.
//!
//! Each foe is of a [`Breed`]: its health, stance, size, blows and how it picks them. In an
//! ambush several stand at once, but only one winds up at a time and the rest keep their
//! distance, so a crowd presses without its blows landing all together unreadably.
//!
//! Positions are metres along the road from the arena's middle (`s`), and up (`y`). Events are
//! the road fight's own (`fight::Event`, each foe by its index), so hitstop, callouts, bursts and
//! sound all follow it unchanged.

use crate::fight::{Archetype, Bulwark, Event, Guarded, Kind, Move, Reel, Spec, Stats, DOWN_FALL, PERFECT};
use crate::gear::{Class, Kit};

/// Half the arena's length: the walls of the gate's yard, metres.
pub const HALF: f32 = 6.0;
/// Ticks per second.
const TPS: f32 = 60.0;
const WALK: f32 = 2.6 / TPS;
/// A run, against a walk; it costs the road's run stamina.
const RUN_K: f32 = 1.8;
/// A jump: up at this speed (m/tick), pulled down by this (m/tick²), and over the top of it (while
/// slower than `HANG_V`) by only `HANG` of that: about a metre and a half high and 0.85 s in the
/// air, a quarter of it floating at the top, time enough to clear a sweep or a ring.
const JUMP_V: f32 = 7.8 / TPS;
const GRAVITY: f32 = 21.0 / (TPS * TPS);
const HANG_V: f32 = 1.6 / TPS;
const HANG: f32 = 0.55;
const JUMP_COST: f32 = 10.0;
/// A roll: ticks, metres, and its i-frames (from, to).
pub const ROLL_T: u32 = 28;
const ROLL_M: f32 = 3.0;
const ROLL_IFRAMES: (u32, u32) = (2, 21);
/// Half the hero's width, metres.
const HERO_HALF: f32 = 0.3;
/// What a strike on a foe left open (by a parry or a broken stance) does, against its usual.
const OPEN_DAMAGE: f32 = 2.5;
/// The gap foes keep between their bodies, metres.
const SPACING: f32 = 0.35;
/// Hits a boss or mini-boss takes squared up before it answers them (`Breed::reply`): it does
/// not stand and take a beating, so its openings are its blows' ends, its staggers and parries.
const TEMPER: u32 = 3;

/// What the player pressed this tick, and what is held.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SideInput {
    /// Held: walk left or right; at a run (a double tap of the way, held).
    pub left: bool,
    pub right: bool,
    pub run: bool,
    /// Pressed: a roll (Shift with A or D; Space with a direction held, or the way faced), a jump,
    /// and the rest.
    pub roll: bool,
    pub jump: bool,
    pub light: bool,
    pub heavy: bool,
    pub parry: bool,
    pub flask: bool,
    /// A ranged attack: an arrow or a bolt from what the other hand holds.
    pub loose: bool,
    /// A spell through the catalyst the other hand holds.
    pub cast: bool,
    /// The weapon's art (`arts`).
    pub art: bool,
    /// Held.
    pub guard: bool,
}

/// The hero's arrow or frost bolt in flight across the yard: where it is, which way it goes,
/// how high, what loosed it, what it deals, metres left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideBolt { pub s: f32, pub dir: f32, pub y: f32, pub ranged: crate::gear::Ranged, pub damage: f32, pub stance: f32, pub left: f32, pub age: u32, pub imbued: bool, pub pierced: u64, pub keen: bool }

impl SideBolt {
    /// As the road's (`fight::Bolt::pierces`, `fight::Bolt::drinks`).
    pub fn pierces(&self) -> bool { self.imbued && self.ranged == crate::gear::Ranged::BloodCrossbow }
    pub fn drinks(&self) -> bool { self.imbued && self.ranged == crate::gear::Ranged::BloodBow }
}

/// The thrown Hatchet across the yard (`arts::WeaponArt::Hatchet`): where it is, which way it
/// flew out, metres left before it turns, coming back, and the foes it has bitten each way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideHatchet { pub s: f32, pub dir: f32, pub left: f32, pub back: bool, pub bit: (u64, u64), pub age: u32 }

/// What the hero is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act { Light, Heavy, Roll(i8), Parry, Flask, Loose, Cast, Art }

impl Act {
    /// A strike as a weapon of `class` throws it here: the road's (`Move::spec_for`) with the
    /// windup two ticks quicker (a heavy's recovery too) and 0.9 m less reach, the yard having
    /// no lanes to cross.
    pub fn strike_spec(self, class: Class) -> Spec {
        let mut s = self.as_move().spec_for(class, None);
        s.frames.0 -= 2;
        if self == Act::Heavy { s.frames.2 -= 2; }
        s.reach -= 0.9;
        s
    }

    /// `strike_spec` as `kit` swings it: slower held one-handed (`Spec::one_handed`).
    pub fn struck(self, kit: &Kit) -> Spec {
        let s = self.strike_spec(kit.arms.class);
        if kit.one_hand { s.one_handed() } else { s }
    }

    /// Windup, live and recovery ticks, with what `kit` holds (its weapon's class; the parry, its
    /// offhand).
    pub fn frames(self, kit: &Kit) -> (u32, u32, u32) {
        match self {
            Act::Light | Act::Heavy => self.struck(kit).frames,
            Act::Roll(_) => (0, ROLL_T, 0),
            Act::Parry | Act::Flask => self.as_move().spec_for(kit.arms.class, kit.off.map(|o| o.guard)).frames,
            Act::Cast if kit.imbues() => Move::imbue_spec().frames,
            Act::Loose | Act::Cast => Move::loose_spec(self.shot(kit).unwrap_or(crate::gear::Ranged::Bow)).frames,
            Act::Art => Act::art_spec(kit).frames,
        }
    }
    /// The weapon's art as the yard has it: the road's with 0.9 m less reach (as a strike's).
    pub fn art_spec(kit: &Kit) -> Spec {
        let mut s = kit.art.unwrap_or(crate::arts::WeaponArt::Toll).spec();
        s.reach -= 0.9;
        s
    }
    fn total(self, kit: &Kit) -> u32 { let f = self.frames(kit); f.0 + f.1 + f.2 }
    /// As the road fight costs and names it.
    fn as_move(self) -> Move {
        match self { Act::Light => Move::Light, Act::Heavy => Move::Heavy, Act::Roll(d) => Move::Step(d as i32), Act::Parry => Move::Parry, Act::Flask => Move::Flask, Act::Loose => Move::Loose, Act::Cast => Move::Cast, Act::Art => Move::Art }
    }
    /// What a ranged attack or a cast sends, as `kit` holds it.
    pub fn shot(self, kit: &Kit) -> Option<crate::gear::Ranged> { match self { Act::Loose => kit.shot(false), Act::Cast => kit.shot(true), _ => None } }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SideHero {
    pub stats: Stats,
    pub hp: f32,
    pub stamina: f32,
    rested: u32,
    pub flasks: u32,
    pub s: f32,
    pub y: f32,
    vy: f32,
    /// 1 facing right (along the road, away from the start), -1 left.
    pub facing: f32,
    pub act: Option<(Act, u32)>,
    pub guarding: bool,
    pub reel: Reel,
    pub reeling: u32,
    pub poise: f32,
    poise_wait: u32,
    /// Where a roll began (for its travel) and whether a strike has landed.
    from: f32,
    landed: bool,
    pub dead: bool,
    /// Walking this tick (for the drawing).
    pub walking: bool,
    /// The last press, and how many ticks it has waited for the hero to be free.
    buffer: Option<(SideInput, u32)>,
    /// Arrows and bolts carried (`fight::Quiver`) and frost charges, brought in from the road and
    /// taken back to it.
    pub quiver: crate::fight::Quiver,
    pub charges: f32,
    /// A Warden's ward, brought in from the road and taken back to it.
    pub bulwark: Bulwark,
    /// The art meter, brought in from the road and taken back to it; Stone Skin's ticks left;
    /// the foe Sentence marked and ticks until its blow; the foes the art under way struck.
    pub art: f32,
    pub stone: u32,
    pub mark: Option<(usize, u32)>,
    pierced: u64,
}

impl SideHero {
    /// As the road's (`fight::Hero::mend`).
    pub fn mend(&mut self) { self.hp = (self.hp + self.stats.kit.capped.mend * self.stats.max_hp()).min(self.stats.max_hp()); }
    pub fn airborne(&self) -> bool { self.y > 0.0 }
    pub fn rising(&self) -> bool { self.vy > 0.0 }
    fn free(&self) -> bool { self.act.is_none() && self.reeling == 0 && !self.dead }
    fn invulnerable(&self) -> bool {
        matches!(self.act, Some((Act::Roll(_), t)) if (ROLL_IFRAMES.0..ROLL_IFRAMES.1).contains(&t))
            || matches!(self.act, Some((Act::Art, t)) if Act::art_spec(&self.stats.kit).iframes.is_some_and(|(a, b)| (a..b).contains(&t)))
            || (self.reel == Reel::Down && self.reeling < Reel::Down.ticks() - 8)
    }
    fn spend(&mut self, n: f32) { if n > 0.0 { self.stamina = (self.stamina - n).max(0.0); self.rested = 0; } }
    fn reel(&mut self, r: Reel) { self.reel = r; self.reeling = r.ticks(); self.act = None; }
    /// Whether the weapon has an art and the meter holds its cost.
    pub fn can_art(&self) -> bool { self.stats.kit.art.is_some_and(|a| self.art >= a.cost()) }
    /// Whether a ranged attack or a cast has a shot to give (as `fight::Hero::can_shoot`).
    fn can_shoot(&self, a: Act) -> bool {
        if a == Act::Cast && self.stats.kit.imbues() { return self.can_imbue(); }
        if a == Act::Cast && !self.stats.spell_ready() { return false; }
        match a.shot(&self.stats.kit) {
            Some(crate::gear::Ranged::Frost) => self.charges >= 1.0,
            Some(r) => self.quiver.has(r),
            None => false,
        }
    }
    /// Whether Blood Imbue can be called (as `fight::Hero::can_imbue`).
    pub fn can_imbue(&self) -> bool {
        self.stats.kit.imbues() && self.quiver.imbued < crate::fight::IMBUE_SHOTS && self.hp > self.stats.max_hp() * crate::fight::IMBUE_COST + 1.0
    }
}

/// How a blow travels once it is live.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    /// Where it was thrown, over its `reach`.
    Strike,
    /// A ring rolling out both ways along the ground from where it struck, low.
    Ring,
    /// The foe itself running at this speed (m/tick) through its live frames, its body the blow.
    Charge(f32),
    /// Up and onto where the hero stood when it committed (it follows them until its tell), then
    /// down over its `reach` around where it lands.
    Leap,
    /// Something thrown or cast, flying from the foe's hand at this speed (m/tick) until it meets
    /// the hero or the yard's end.
    Shot(f32),
    /// Cast onto the ground where the hero stood at its tell (followed until then), opening over
    /// its `reach` around that spot; the foe does not move.
    Rune,
    /// The dead called up out of the ground beside the foe (`RISEN`, `SideFight::raise_risen`):
    /// it strikes nothing itself.
    Raise,
}

/// What may follow a blow at once, as the next of a string: blow `to`, started `at` ticks into
/// it, `odds` times in ten (four more enraged). A string that only sometimes goes on is what makes
/// a player wait for the end of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Follow { pub to: usize, pub at: u32, pub odds: u32 }

/// One of a foe's blows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub name: &'static str,
    pub frames: (u32, u32, u32),
    /// Metres in front of the foe's middle it covers (from, to), and heights (from, to).
    pub reach: (f32, f32),
    pub height: (f32, f32),
    pub damage: f32,
    pub kind: Kind,
    pub parry: bool,
    /// Stamina a guard pays to hold it.
    pub guard: f32,
    pub motion: Motion,
    /// The clip it is drawn with (its own name, or one it borrows until it has art).
    pub art: &'static str,
    /// How far through its windup it shows what is coming (the warning flash, and where a leap or
    /// a shot stops following the hero). A delayed blow tells early and holds: rolling at the tell
    /// rolls too soon.
    pub tell: f32,
    pub follow: Option<Follow>,
    /// Ticks more it is stuck in its recovery when it meets nothing: the opening a dodge earns.
    pub stuck: u32,
}

#[allow(clippy::too_many_arguments)]
const fn blow(name: &'static str, frames: (u32, u32, u32), reach: (f32, f32), height: (f32, f32), damage: f32, kind: Kind, parry: bool, guard: f32, motion: Motion) -> Blow {
    Blow { name, frames, reach, height, damage, kind, parry, guard, motion, art: name, tell: 0.7, follow: None, stuck: 0 }
}

impl Blow {
    /// Drawn with another blow's clip.
    const fn art(self, art: &'static str) -> Blow { Blow { art, ..self } }
    /// Tells this far through its windup (0.7 by default; a delayed blow, earlier).
    const fn tell(self, tell: f32) -> Blow { Blow { tell, ..self } }
    /// Followed by blow `to`, `at` ticks in, `odds` in ten.
    const fn then(self, to: usize, at: u32, odds: u32) -> Blow { Blow { follow: Some(Follow { to, at, odds }), ..self } }
    /// Stuck this many ticks more after a miss.
    const fn stuck(self, stuck: u32) -> Blow { Blow { stuck, ..self } }
}

/// A kind of foe.
#[derive(Debug, PartialEq)]
pub struct Breed {
    pub name: &'static str,
    /// The road enemy it is drawn as and pays souls as (None: the Bell Warden's own).
    pub kind: Option<Archetype>,
    pub hp: f32,
    pub stance: f32,
    /// Half its width and its height, metres.
    pub half: f32,
    pub height: f32,
    pub blows: &'static [Blow],
    /// Which blow next: given whether the hero is within `near` metres, a roll of 0..10, and
    /// whether it is in its second phase (past half its health, if it enrages).
    pub pick: fn(bool, u32, bool) -> usize,
    pub near: f32,
    /// How fast it closes in while it waits, m/s, and from how far off it stops.
    pub walk: f32,
    pub keep: f32,
    /// Ticks it squares up between blows (plus up to 20 by chance), and how long it stays open.
    pub rest: u32,
    pub open: u32,
    /// Whether past half its health it is quicker and its strings go on more often.
    pub enrages: bool,
    /// What its blows do, against what they are drawn as (`Blow::damage`): the bosses and
    /// mini-bosses deeper in the world hit harder with the same blows.
    pub power: f32,
    /// The blow it opens its second phase with, at once, the moment it enrages.
    pub wake: Option<usize>,
    /// What it answers with when it is hit `TEMPER` times while squared up (None: whatever it
    /// would throw next, at once). Only a breed that enrages answers.
    pub reply: Option<usize>,
    /// Its blow for a hero who hides behind a guard within reach (thrown half the time it reads
    /// one).
    pub grab: Option<usize>,
    /// Its own art (`assets/pixellab/<folder>`, clips by role: idle, walk, toll, sweep, knell,
    /// charge, leap, throw, grab, stagger, death) and how tall it is drawn, metres. None: drawn
    /// as its road kind, a head taller.
    pub sprite: Option<(&'static str, f32)>,
}

/// How fast a ring rolls out, m/tick, and how wide its crest is.
const RING_V: f32 = 7.0 / TPS;
const RING_HALF: f32 = 0.35;
/// Half a shot's length, and how far in front of the foe it leaves the hand.
const SHOT_HALF: f32 = 0.3;
const SHOT_FROM: f32 = 0.9;

use Kind::{GuardBreak, Physical, Unblockable};

/// The Bell Warden's blows.
pub const BLOWS: [Blow; 7] = [
    // The bell-hammer brought down overhead: parry it. Often the sweep comes straight after: a
    // parry, then a jump.
    blow("toll", (40, 6, 42), (0.3, 3.3), (0.0, 3.6), 34.0, Physical, true, 40.0, Motion::Strike).then(1, 12, 4),
    // Swung low across the ground: jump it (or block, paying for it). Sometimes the hammer is rung
    // on the stones as it comes round: jump again.
    blow("sweep", (34, 8, 38), (-0.7, 3.7), (0.0, 0.75), 24.0, GuardBreak, false, 45.0, Motion::Strike).then(2, 20, 3),
    // The hammer rung on the stones: a ring rolls out both ways, low. Jump it or roll through it.
    blow("knell", (62, 66, 40), (0.0, 0.0), (0.0, 0.55), 28.0, Unblockable, false, 0.0, Motion::Ring),
    // A charge across the yard, shoulder first: roll through it. Half the time it turns at the end
    // and brings the hammer down on whoever rolled behind it.
    blow("charge", (30, 42, 38), (-0.95, 0.95), (0.0, 2.6), 30.0, GuardBreak, false, 50.0, Motion::Charge(8.5 / TPS)).then(0, 14, 5),
    // Second phase: the hammer held high far longer than the toll, then down. It tells early: a
    // parry at the tell is a parry too soon.
    blow("late toll", (84, 6, 44), (0.3, 3.3), (0.0, 3.6), 38.0, Physical, true, 44.0, Motion::Strike).art("toll").tell(0.4),
    // Its waking: the bell rung twice, both ways.
    blow("double knell", (40, 66, 20), (0.0, 0.0), (0.0, 0.55), 26.0, Unblockable, false, 0.0, Motion::Ring).art("knell").then(2, 30, 10),
    // Its answer to a beating: the haft driven out short and quick. Roll it.
    blow("haft", (16, 5, 28), (0.2, 2.4), (0.0, 3.0), 22.0, GuardBreak, false, 34.0, Motion::Strike).art("toll"),
];

/// The boss at the gate.
pub static WARDEN: Breed = Breed {
    name: "The Bell Warden", kind: None, hp: 520.0, stance: 110.0, half: 0.95, height: 3.2, blows: &BLOWS,
    pick: |near, r, wild| match (near, r, wild) {
        (true, 0..=2, true) => 4,
        (true, 0..=4, _) => 0, (true, 5..=8, _) => 1, (true, _, _) => 2,
        (false, 0..=5, _) => 3, (false, _, _) => 2,
    },
    near: 3.4, walk: 1.3, keep: 3.0, rest: 36, open: 110, enrages: true, power: 1.0, wake: Some(5), reply: Some(6), grab: None, sprite: Some(("bell_warden", 3.4)),
};

/// A region boss on the Warden's frame and art (until each has its own): its own blows, how it
/// picks them, and its own pace.
#[allow(clippy::too_many_arguments)]
const fn boss(name: &'static str, hp: f32, stance: f32, rest: u32, power: f32, blows: &'static [Blow], pick: fn(bool, u32, bool) -> usize, near: f32, wake: Option<usize>, sprite: (&'static str, f32)) -> Breed {
    Breed { name, hp, stance, rest, power, blows, pick, near, wake, sprite: Some(sprite), ..WARDEN_LIKE }
}
const WARDEN_LIKE: Breed = Breed {
    name: "", kind: None, hp: 520.0, stance: 110.0, half: 0.95, height: 3.2, blows: &BLOWS,
    pick: |_, _, _| 0, near: 3.4, walk: 1.3, keep: 3.0, rest: 46, open: 150, enrages: true, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};

/// The Hollowmere Screamer: a drowned thing of the marsh. Its scream rolls out at chest height, so
/// no jump clears it: roll through. Its rakes come in strings of two or three (wait for the
/// third), and it leaps from afar onto where you stood: dodge late, and it is stuck in the mud.
const SCREAMER_BLOWS: [Blow; 6] = [
    blow("rake", (26, 6, 18), (0.2, 2.6), (0.0, 2.4), 18.0, Physical, true, 26.0, Motion::Strike).art("toll").then(1, 10, 7),
    blow("rake again", (16, 6, 20), (0.2, 2.6), (0.0, 2.4), 18.0, Physical, true, 26.0, Motion::Strike).art("toll").then(2, 8, 5),
    blow("rending rake", (22, 8, 40), (0.2, 3.0), (0.0, 2.4), 26.0, GuardBreak, false, 40.0, Motion::Strike).art("sweep"),
    blow("scream", (50, 70, 40), (0.0, 0.0), (0.0, 2.2), 24.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    blow("drowning leap", (60, 8, 30), (-1.6, 1.6), (0.0, 2.8), 32.0, GuardBreak, false, 50.0, Motion::Leap).art("leap").tell(0.55).stuck(70),
    blow("bog wail", (40, 90, 30), (0.0, 0.0), (0.4, 2.0), 20.0, Physical, false, 30.0, Motion::Shot(6.0 / TPS)).art("throw").then(3, 20, 10),
];
pub static SCREAMER: Breed = boss("The Hollowmere Screamer", 640.0, 120.0, 44, 1.15, &SCREAMER_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=5, _) => 0, (true, 6..=7, _) => 3, (true, _, _) => 2, (false, 0..=4, _) => 4, (false, 5..=7, _) => 5, (false, _, true) => 5, (false, _, _) => 3 },
    3.0, Some(5), ("hollowmere_screamer", 3.6));

/// The Bandit Chief: an axe and a temper. Two chops then a long-held overhead (parry the last, not
/// the first two); a hatchet thrown at range (block it, or roll); a grab for a hero who hides
/// behind a shield; a shoulder charge.
const CHIEF_BLOWS: [Blow; 7] = [
    blow("chop", (28, 5, 16), (0.3, 3.0), (0.0, 2.8), 22.0, Physical, true, 30.0, Motion::Strike).art("toll").then(1, 6, 8),
    blow("chop again", (20, 5, 18), (0.3, 3.0), (0.0, 2.8), 22.0, Physical, true, 30.0, Motion::Strike).art("toll").then(2, 0, 6),
    blow("headsman's drop", (70, 6, 46), (0.3, 3.2), (0.0, 3.4), 40.0, Physical, true, 46.0, Motion::Strike).art("toll").tell(0.35),
    // The hatchet thrown, and as often as not the rush behind it: roll the one, then the other.
    blow("hatchet", (32, 80, 30), (0.0, 0.0), (0.6, 1.9), 18.0, Physical, false, 24.0, Motion::Shot(9.0 / TPS)).art("throw").then(5, 0, 5),
    blow("throttle", (34, 6, 44), (0.2, 1.7), (0.0, 2.2), 36.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30),
    blow("bull rush", (30, 40, 40), (-0.95, 0.95), (0.0, 2.6), 30.0, GuardBreak, false, 50.0, Motion::Charge(9.0 / TPS)).art("charge"),
    // Its answer to a beating: the axe's butt in the face, quick.
    blow("butt", (15, 4, 26), (0.2, 2.2), (0.0, 2.8), 20.0, GuardBreak, false, 30.0, Motion::Strike).art("toll"),
];
pub static CHIEF: Breed = Breed { grab: Some(4), reply: Some(6), open: 120, ..boss("The Bandit Chief", 760.0, 130.0, 36, 1.3, &CHIEF_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=6, _) => 4, (true, 7, _) => 2, (true, _, true) => 2, (true, _, _) => 0, (false, 0..=5, _) => 3, (false, _, _) => 5 },
    3.2, Some(5), ("bandit_chief", 3.3)) };

/// The Hollow Magistrate: judgement handed down. The gavel held high (it tells early: parry late);
/// an edict that bursts all round it (no shield holds it: back off or roll out); chains thrown low
/// along the ground (jump them); the sentence rung out as a ring.
const MAGISTRATE_BLOWS: [Blow; 5] = [
    blow("gavel", (66, 6, 40), (0.3, 3.2), (0.0, 3.6), 36.0, Physical, true, 42.0, Motion::Strike).art("toll").tell(0.4),
    blow("edict", (52, 10, 36), (-2.8, 2.8), (0.0, 3.0), 30.0, Unblockable, false, 0.0, Motion::Strike).art("knell"),
    blow("chains", (30, 90, 26), (0.0, 0.0), (0.0, 0.6), 20.0, GuardBreak, false, 30.0, Motion::Shot(7.0 / TPS)).art("throw"),
    blow("sentence", (56, 66, 36), (0.0, 0.0), (0.0, 0.55), 28.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    // Waking: the edict, then the gavel at once.
    blow("contempt", (40, 10, 10), (-2.8, 2.8), (0.0, 3.0), 30.0, Unblockable, false, 0.0, Motion::Strike).art("knell").then(0, 20, 10),
];
pub static MAGISTRATE: Breed = boss("The Hollow Magistrate", 900.0, 140.0, 40, 1.45, &MAGISTRATE_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=7, _) => 1, (true, _, true) => 4, (true, _, _) => 3, (false, 0..=6, _) => 2, (false, _, _) => 3 },
    3.0, Some(4), ("hollow_magistrate", 3.6));

/// The Wraith Lord: a scythe that reaps both ways at once, soul bolts in volleys of up to three,
/// a drain that seizes (roll away from it), and a reaping leap.
const WRAITH_BLOWS: [Blow; 6] = [
    blow("reap", (40, 10, 34), (-2.4, 3.4), (0.0, 1.2), 28.0, GuardBreak, false, 40.0, Motion::Strike).art("sweep"),
    blow("soul bolt", (26, 90, 12), (0.0, 0.0), (0.8, 2.2), 18.0, Physical, false, 22.0, Motion::Shot(8.0 / TPS)).art("throw").then(2, 6, 7),
    blow("soul bolt again", (18, 90, 12), (0.0, 0.0), (0.8, 2.2), 18.0, Physical, false, 22.0, Motion::Shot(8.0 / TPS)).art("throw").then(3, 6, 5),
    blow("low bolt", (18, 90, 30), (0.0, 0.0), (0.0, 0.6), 18.0, Physical, false, 22.0, Motion::Shot(8.0 / TPS)).art("throw"),
    blow("drain", (36, 8, 46), (0.2, 2.0), (0.0, 2.6), 40.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(40),
    blow("reaping leap", (56, 8, 30), (-1.8, 1.8), (0.0, 3.0), 34.0, GuardBreak, false, 50.0, Motion::Leap).art("leap").tell(0.5).stuck(60),
];
pub static WRAITH: Breed = Breed { grab: Some(4), ..boss("The Wraith Lord", 980.0, 140.0, 38, 1.6, &WRAITH_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=7, _) => 4, (true, _, _) => 5, (false, 0..=5, _) => 1, (false, _, true) => 5, (false, _, _) => 1 },
    3.0, Some(1), ("wraith_lord", 3.8)) };

/// The Angel Priest: a mace swung in threes (the last a guard-breaker), a lance of light thrown
/// high (roll or block), radiance rung out round it, and a smite from above.
const PRIEST_BLOWS: [Blow; 6] = [
    blow("mace", (26, 6, 16), (0.3, 2.9), (0.0, 3.0), 26.0, Physical, true, 32.0, Motion::Strike).art("toll").then(1, 8, 8),
    blow("mace again", (18, 6, 18), (0.3, 2.9), (0.0, 3.0), 26.0, Physical, true, 32.0, Motion::Strike).art("toll").then(2, 4, 6),
    blow("judgement", (40, 8, 44), (-0.7, 3.4), (0.0, 0.8), 32.0, GuardBreak, false, 46.0, Motion::Strike).art("sweep"),
    blow("lance of light", (36, 80, 28), (0.0, 0.0), (0.7, 2.4), 26.0, Physical, false, 30.0, Motion::Shot(11.0 / TPS)).art("throw"),
    blow("radiance", (60, 66, 36), (0.0, 0.0), (0.0, 1.6), 28.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    blow("smite", (64, 8, 30), (-1.8, 1.8), (0.0, 3.2), 40.0, GuardBreak, false, 54.0, Motion::Leap).art("leap").tell(0.5).stuck(70),
];
pub static PRIEST: Breed = boss("The Angel Priest", 1100.0, 150.0, 36, 1.8, &PRIEST_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=5, _) => 0, (true, 6..=7, _) => 4, (true, _, _) => 2, (false, 0..=4, _) => 3, (false, 5..=7, _) => 5, (false, _, _) => 4 },
    3.0, Some(4), ("angel_priest", 3.6));

/// The Statue Archangel: stone, slow and enormous. Each swing held long (patience, then parry);
/// a stomp that rings out; a leap that cracks the yard; a march that runs you down.
const STATUE_BLOWS: [Blow; 6] = [
    blow("stone blade", (80, 8, 50), (0.3, 3.8), (0.0, 3.8), 44.0, Physical, true, 50.0, Motion::Strike).art("toll").tell(0.45).then(1, 10, 4),
    // Swept low, and sometimes the stomp's ring after it, rolling out as the hero lands.
    blow("stone sweep", (50, 10, 50), (-0.9, 4.0), (0.0, 0.8), 34.0, GuardBreak, false, 56.0, Motion::Strike).art("sweep").then(2, 16, 5),
    blow("stomp", (54, 70, 40), (0.0, 0.0), (0.0, 0.55), 30.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    // Down from on high, and the yard rings with it: roll the landing, then jump the ring.
    blow("falling statue", (70, 10, 40), (-2.0, 2.0), (0.0, 3.4), 46.0, GuardBreak, false, 60.0, Motion::Leap).art("leap").tell(0.5).stuck(90).then(2, 24, 6),
    // A march that ends, half the time, in the stone blade.
    blow("march", (40, 50, 50), (-1.1, 1.1), (0.0, 3.0), 36.0, GuardBreak, false, 56.0, Motion::Charge(6.5 / TPS)).art("charge").then(0, 30, 5),
    // Its answer to a beating: a tremor about its feet, quick and low. Jump it.
    blow("tremor", (18, 8, 30), (-2.4, 2.4), (0.0, 0.6), 26.0, Unblockable, false, 0.0, Motion::Strike).art("knell"),
];
pub static STATUE: Breed = Breed { reply: Some(5), open: 110, ..boss("The Statue Archangel", 1250.0, 170.0, 32, 2.0, &STATUE_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=7, _) => 1, (true, _, _) => 2, (false, 0..=4, _) => 3, (false, _, true) => 3, (false, _, _) => 4 },
    3.8, Some(2), ("statue_archangel", 4.4)) };

/// The Archangel: everything that came before, quicker. A sword string whose third cut is held
/// (parry it); spears of light thrown in pairs; its wings beat out all round it (unblockable);
/// a dive from on high; a grip that does not let go.
const ARCHANGEL_BLOWS: [Blow; 8] = [
    blow("cut", (22, 5, 14), (0.3, 3.4), (0.0, 3.2), 26.0, Physical, true, 30.0, Motion::Strike).art("toll").then(1, 6, 9),
    blow("cut again", (16, 5, 16), (0.3, 3.4), (0.0, 3.2), 26.0, Physical, true, 30.0, Motion::Strike).art("toll").then(2, 0, 7),
    blow("held cut", (58, 6, 44), (0.3, 3.6), (0.0, 3.4), 40.0, Physical, true, 46.0, Motion::Strike).art("toll").tell(0.38),
    blow("spear of light", (28, 80, 10), (0.0, 0.0), (0.7, 2.4), 26.0, Physical, false, 30.0, Motion::Shot(12.0 / TPS)).art("throw").then(4, 6, 8),
    blow("low spear", (16, 80, 26), (0.0, 0.0), (0.0, 0.6), 24.0, Physical, false, 30.0, Motion::Shot(12.0 / TPS)).art("throw"),
    blow("wingbeat", (44, 10, 34), (-3.0, 3.0), (0.0, 3.6), 32.0, Unblockable, false, 0.0, Motion::Strike).art("knell"),
    blow("dive", (54, 8, 30), (-2.0, 2.0), (0.0, 3.4), 44.0, GuardBreak, false, 56.0, Motion::Leap).art("leap").tell(0.5).stuck(60),
    blow("grip", (30, 6, 46), (0.2, 2.0), (0.0, 2.8), 46.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30),
];
pub static ARCHANGEL: Breed = Breed { grab: Some(7), ..boss("The Archangel", 1500.0, 190.0, 32, 2.3, &ARCHANGEL_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=6, _) => 7, (true, 7, _) => 5, (true, _, true) => 5, (true, _, _) => 0, (false, 0..=4, _) => 3, (false, _, _) => 6 },
    3.4, Some(5), ("the_archangel", 4.0)) };

/// A mini-boss: a named road enemy, stronger, enraging, with blows and art of its own.
const fn elite(base: &Breed, name: &'static str, hp: f32, power: f32, blows: &'static [Blow], pick: fn(bool, u32, bool) -> usize, sprite: (&'static str, f32)) -> Breed {
    Breed { name, hp, stance: base.stance * 2.2, half: base.half * 1.2, height: base.height * 1.2, rest: base.rest * 3 / 4, enrages: true, power, blows, pick, sprite: Some(sprite), ..*base }
}
/// The husks' chop and sweep, as their elites keep them (indices 0 and 1 of theirs).
const HUSK_CHOP: Blow = blow("chop", (34, 5, 30), (0.2, 1.6), (0.0, 2.2), 16.0, Physical, true, 22.0, Motion::Strike);
const HUSK_SWEEP: Blow = blow("sweep", (30, 6, 28), (-0.3, 1.8), (0.0, 0.7), 14.0, GuardBreak, false, 26.0, Motion::Strike);
/// The Debt Collector, the warrens' first lesson: parry the cudgel when it falls, but not too
/// soon (it sometimes holds it up); jump the sweep that follows a chop; a shield held up close
/// gets you taken by the collar, a flask drunk in reach is answered by a lunge across the yard,
/// and a beating taken squared up by a rap of the cudgel. Past half its health, it calls the
/// debt in: chop and sweep at once, quicker.
const COLLECTOR_BLOWS: [Blow; 7] = [
    HUSK_CHOP.art("toll").then(1, 8, 4),
    blow("sweep", (28, 6, 30), (-0.3, 2.0), (0.0, 0.75), 18.0, GuardBreak, false, 30.0, Motion::Strike),
    blow("collar", (26, 6, 40), (0.1, 1.5), (0.0, 2.2), 26.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(36),
    // The cudgel held up, and held: the delay. A parry at the tell is a parry too soon.
    blow("late chop", (54, 5, 32), (0.2, 1.8), (0.0, 2.4), 22.0, Physical, true, 26.0, Motion::Strike).art("toll").tell(0.38).then(1, 6, 3),
    // From across the yard, a hand reaching for the collar: roll through it, and it stumbles on.
    blow("dun's lunge", (30, 24, 30), (-0.6, 0.6), (0.0, 2.2), 22.0, GuardBreak, false, 34.0, Motion::Charge(8.5 / TPS)).art("grab").stuck(40),
    // Its answer to a beating: a quick rap, out of nowhere. Roll it, or do not stand mashing.
    blow("rap", (13, 4, 24), (0.1, 1.8), (0.0, 2.4), 14.0, GuardBreak, false, 22.0, Motion::Strike).art("toll"),
    // Its waking: the debt called in, a chop and the sweep straight after.
    blow("foreclosure", (22, 5, 14), (0.2, 1.8), (0.0, 2.4), 20.0, Physical, true, 26.0, Motion::Strike).art("toll").then(1, 5, 10),
];
pub static COLLECTOR: Breed = Breed {
    rest: 40, walk: 1.5, near: 2.0, open: 96, reply: Some(5), grab: Some(2), wake: Some(6),
    ..elite(&HUSK_BASE, "The Debt Collector", 400.0, 1.4, &COLLECTOR_BLOWS,
        |near, r, wild| match (near, r, wild) {
            (true, 0..=3, _) => 0, (true, 4..=5, _) => 3, (true, 6..=7, _) => 1, (true, 8, _) => 2, (true, _, true) => 6, (true, _, _) => 0,
            (false, 0..=6, _) => 4, (false, _, _) => 3,
        }, ("debt_collector", 2.5))
};
/// The Pack Alpha: rakes in threes and a leap from afar.
pub static ALPHA: Breed = elite(&WRETCH_BASE, "The Pack Alpha", 240.0, 1.8,
    &[blow("rake", (18, 6, 14), (0.1, 1.4), (0.0, 1.8), 10.0, Physical, false, 16.0, Motion::Strike).art("sweep").then(2, 4, 8),
      blow("pounce", (26, 18, 28), (-0.4, 0.4), (0.0, 1.6), 14.0, GuardBreak, false, 24.0, Motion::Charge(8.0 / TPS)).art("charge"),
      blow("rake again", (12, 6, 16), (0.1, 1.4), (0.0, 1.8), 10.0, Physical, false, 16.0, Motion::Strike).art("sweep").then(0, 4, 4),
      blow("alpha's leap", (40, 8, 24), (-1.0, 1.0), (0.0, 1.8), 18.0, GuardBreak, false, 30.0, Motion::Leap).art("leap").tell(0.5).stuck(50)],
    |near, r, _| match (near, r) { (true, 0..=6) => 0, (true, _) => 2, (false, 0..=5) => 3, (false, _) => 1 }, ("pack_alpha", 2.3));
/// The Order Agent: a lance in a flurry of three, the last held; a lunge from afar.
pub static AGENT: Breed = elite(&LANCER_BASE, "The Order Agent", 300.0, 2.0,
    &[blow("thrust", (24, 4, 12), (0.3, 2.6), (0.6, 1.8), 15.0, Physical, true, 20.0, Motion::Strike).then(2, 4, 8),
      blow("lunge", (32, 22, 30), (-0.4, 0.4), (0.0, 1.9), 18.0, GuardBreak, false, 30.0, Motion::Charge(8.0 / TPS)).art("charge"),
      blow("thrust again", (14, 4, 12), (0.3, 2.6), (0.6, 1.8), 15.0, Physical, true, 20.0, Motion::Strike).art("thrust").then(3, 0, 7),
      blow("piercing thrust", (46, 4, 34), (0.3, 3.0), (0.6, 1.8), 22.0, Physical, true, 26.0, Motion::Strike).art("thrust").tell(0.4)],
    |near, r, _| if near && r < 7 { 0 } else { 1 }, ("order_agent", 2.5));
/// The Rune Sentinel: slow chops, and a rune rung into the ground.
pub static SENTINEL: Breed = elite(&HUSK_BASE, "The Rune Sentinel", 380.0, 2.4,
    &[HUSK_CHOP.art("toll").tell(0.45), HUSK_SWEEP, blow("rune", (50, 66, 36), (0.0, 0.0), (0.0, 0.55), 18.0, Unblockable, false, 0.0, Motion::Ring).art("knell")],
    |near, r, _| match (near, r) { (true, 0..=5) => 0, (true, 6..=7) => 1, (true, _) => 2, (false, _) => 2 }, ("rune_sentinel", 2.9));
/// The Butcher: a cleaver held up too long, a hook for the turtle and thrown on its chain at
/// whoever keeps away, and past half its health the chop and the held cleave together.
pub static BUTCHER: Breed = Breed { grab: Some(3), reply: Some(5), wake: Some(6), rest: 40, walk: 1.4, near: 2.2, ..elite(&HUSK_BASE, "The Butcher", 420.0, 2.7,
    &[HUSK_CHOP.art("toll").then(2, 0, 5), HUSK_SWEEP.then(0, 6, 4),
      blow("cleave", (60, 6, 40), (0.2, 1.9), (0.0, 2.4), 24.0, Physical, true, 30.0, Motion::Strike).art("toll").tell(0.38),
      blow("meathook", (30, 6, 40), (0.1, 1.6), (0.0, 2.2), 26.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30),
      // The hook thrown on its chain at chest height: roll under it, or block it.
      blow("hook and chain", (34, 70, 30), (0.0, 0.0), (0.7, 1.9), 18.0, Physical, false, 26.0, Motion::Shot(9.0 / TPS)).art("grab"),
      // Its answer to a beating: the cleaver's flat, quick.
      blow("flat of the cleaver", (14, 4, 24), (0.1, 1.9), (0.0, 2.4), 16.0, GuardBreak, false, 24.0, Motion::Strike).art("toll"),
      // Its waking: the chop, and the held cleave straight after (parry the second late).
      blow("butchery", (22, 5, 10), (0.2, 1.9), (0.0, 2.4), 20.0, Physical, true, 26.0, Motion::Strike).art("toll").then(2, 0, 10)],
    |near, r, wild| match (near, r, wild) {
        (true, 0..=3, _) => 0, (true, 4..=5, _) => 2, (true, 6..=7, _) => 1, (true, 8, _) => 3, (true, _, true) => 6, (true, _, _) => 0,
        (false, 0..=6, _) => 4, (false, _, _) => 0,
    }, ("the_butcher", 2.9)) };
/// The Void Stalker: leaps, and bolts of the void from afar.
pub static STALKER: Breed = elite(&WRETCH_BASE, "The Void Stalker", 420.0, 3.0,
    &[blow("rake", (18, 6, 18), (0.1, 1.3), (0.0, 1.6), 10.0, Physical, false, 16.0, Motion::Strike).art("sweep").then(2, 6, 5),
      blow("void bolt", (30, 90, 26), (0.0, 0.0), (0.4, 1.6), 14.0, Physical, false, 20.0, Motion::Shot(8.0 / TPS)).art("throw"),
      blow("void leap", (40, 8, 24), (-1.0, 1.0), (0.0, 1.8), 18.0, GuardBreak, false, 30.0, Motion::Leap).art("leap").tell(0.5).stuck(50)],
    |near, r, _| match (near, r) { (true, 0..=6) => 0, (true, _) => 2, (false, 0..=5) => 1, (false, _) => 2 }, ("void_stalker", 2.6));

/// The breed a road enemy fights as when it springs an ambush (shieldbearers do not).
pub fn breed_of(kind: Archetype) -> &'static Breed {
    match kind { Archetype::Lancer => &LANCER, Archetype::Wretch => &WRETCH, Archetype::Archer => &ARCHER, Archetype::Hexer => &HEXER, Archetype::Brute => &BRUTE, Archetype::Hound => &HOUND,
        Archetype::Bloater => &BLOATER, Archetype::Gravecaller => &GRAVECALLER, _ => &HUSK }
}

/// The road's dead, as they spring an ambush: weaker than the Warden and quicker to fall, each
/// with one blow to parry or block, one to jump or roll, and one of their own to learn.
pub static HUSK: Breed = HUSK_BASE;
const HUSK_BASE: Breed = Breed {
    name: "Husk", kind: Some(Archetype::Husk), hp: 70.0, stance: 34.0, half: 0.4, height: 1.9,
    blows: &[
        HUSK_CHOP,
        HUSK_SWEEP,
        // The cleaver held up, and held, and down: the husk's delay. Parry it late.
        blow("late chop", (58, 5, 32), (0.2, 1.6), (0.0, 2.2), 18.0, Physical, true, 24.0, Motion::Strike).art("chop").tell(0.38),
    ],
    pick: |_, r, _| match r { 0..=4 => 0, 5..=7 => 1, _ => 2 }, near: 1.8, walk: 0.9, keep: 1.3, rest: 70, open: 110, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
pub static LANCER: Breed = LANCER_BASE;
const LANCER_BASE: Breed = Breed {
    name: "Lancer", kind: Some(Archetype::Lancer), hp: 62.0, stance: 30.0, half: 0.35, height: 1.9,
    blows: &[
        // Sometimes the thrust comes twice.
        blow("thrust", (28, 4, 26), (0.3, 2.6), (0.6, 1.8), 15.0, Physical, true, 20.0, Motion::Strike).then(2, 8, 3),
        blow("lunge", (32, 22, 30), (-0.4, 0.4), (0.0, 1.9), 18.0, GuardBreak, false, 30.0, Motion::Charge(7.5 / TPS)),
        blow("thrust again", (16, 4, 28), (0.3, 2.6), (0.6, 1.8), 15.0, Physical, true, 20.0, Motion::Strike).art("thrust"),
    ],
    pick: |near, r, _| if near && r < 7 { 0 } else { 1 }, near: 2.6, walk: 1.1, keep: 2.2, rest: 64, open: 100, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
pub static WRETCH: Breed = WRETCH_BASE;
const WRETCH_BASE: Breed = Breed {
    name: "Wretch", kind: Some(Archetype::Wretch), hp: 44.0, stance: 22.0, half: 0.35, height: 1.5,
    blows: &[
        blow("rake", (18, 6, 22), (0.1, 1.3), (0.0, 1.6), 10.0, Physical, false, 16.0, Motion::Strike),
        blow("pounce", (26, 18, 28), (-0.4, 0.4), (0.0, 1.4), 14.0, GuardBreak, false, 24.0, Motion::Charge(8.0 / TPS)),
        // From afar, up and onto you: step aside late and it sprawls.
        blow("leap", (36, 6, 22), (-0.8, 0.8), (0.0, 1.6), 14.0, GuardBreak, false, 24.0, Motion::Leap).art("pounce").tell(0.5).stuck(40),
    ],
    pick: |near, r, _| match (near, r) { (true, 0..=5) => 0, (true, _) => 1, (false, 0..=5) => 1, (false, _) => 2 }, near: 1.4, walk: 1.8, keep: 1.0, rest: 48, open: 90, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};

/// Hangs back down the yard and shoots: an arrow at the chest (duck under it with a roll, or
/// block), one at the shins (jump it), and a kick for whoever closes in.
pub static ARCHER: Breed = Breed {
    name: "Archer", kind: Some(Archetype::Archer), hp: 46.0, stance: 20.0, half: 0.35, height: 1.85,
    blows: &[
        blow("loose", (44, 80, 24), (0.0, 0.0), (1.0, 1.7), 14.0, Physical, false, 14.0, Motion::Shot(12.0 / TPS)).tell(0.6),
        blow("low arrow", (40, 80, 24), (0.0, 0.0), (0.0, 0.6), 12.0, Physical, false, 14.0, Motion::Shot(12.0 / TPS)).art("loose").tell(0.6),
        blow("kick", (20, 5, 26), (0.2, 1.2), (0.0, 1.4), 10.0, Physical, true, 14.0, Motion::Strike),
    ],
    pick: |near, r, _| match (near, r) { (true, 0..=5) => 2, (_, 0..=5) => 0, _ => 1 }, near: 1.6, walk: 1.0, keep: 6.0, rest: 60, open: 90, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
/// Hangs back and casts: a slow hex (a shield pays for it), a rune that opens under the hero
/// (move off the mark), and a ring thrown out round itself for whoever closes (jump it).
pub static HEXER: Breed = Breed {
    name: "Hexer", kind: Some(Archetype::Hexer), hp: 52.0, stance: 22.0, half: 0.35, height: 1.9,
    blows: &[
        blow("hex", (50, 140, 30), (0.0, 0.0), (0.6, 1.8), 16.0, GuardBreak, false, 26.0, Motion::Shot(5.0 / TPS)).tell(0.55),
        blow("rune", (70, 10, 40), (-0.9, 0.9), (0.0, 2.0), 22.0, Unblockable, false, 0.0, Motion::Rune).tell(0.6),
        blow("repel", (28, 40, 36), (0.0, 0.0), (0.0, 0.55), 14.0, GuardBreak, false, 24.0, Motion::Ring),
    ],
    pick: |near, r, _| match (near, r) { (true, 0..=6) => 2, (_, 0..=5) => 0, _ => 1 }, near: 1.8, walk: 0.8, keep: 5.5, rest: 80, open: 100, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
/// Slow, huge and hard to stop: a slam no guard holds (roll it), a sweep at the shins (jump it, or
/// parry), and the slam held aloft before it falls. Its recoveries are long: punish them.
pub static BRUTE: Breed = Breed {
    name: "Brute", kind: Some(Archetype::Brute), hp: 140.0, stance: 60.0, half: 0.55, height: 2.5,
    blows: &[
        blow("slam", (50, 6, 50), (0.2, 2.4), (0.0, 2.6), 26.0, GuardBreak, false, 50.0, Motion::Strike).stuck(30),
        blow("sweep", (42, 6, 40), (0.2, 2.6), (0.0, 0.8), 18.0, Physical, true, 30.0, Motion::Strike),
        blow("late slam", (78, 6, 50), (0.2, 2.4), (0.0, 2.6), 28.0, GuardBreak, false, 50.0, Motion::Strike).art("slam").tell(0.35).stuck(30),
    ],
    pick: |_, r, _| match r { 0..=4 => 0, 5..=7 => 1, _ => 2 }, near: 2.2, walk: 0.6, keep: 1.8, rest: 90, open: 130, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
/// Low and quick: a bite (and sometimes a second), and from afar a leap onto where the hero
/// stood (step aside late and it sprawls).
pub static HOUND: Breed = Breed {
    name: "Hound", kind: Some(Archetype::Hound), hp: 34.0, stance: 14.0, half: 0.45, height: 1.25,
    blows: &[
        blow("bite", (16, 5, 20), (0.1, 1.3), (0.0, 1.1), 9.0, Physical, true, 12.0, Motion::Strike).then(1, 10, 4),
        blow("snap", (12, 5, 22), (0.1, 1.3), (0.0, 1.1), 9.0, Physical, true, 12.0, Motion::Strike).art("bite"),
        blow("pounce", (34, 6, 22), (-0.7, 0.7), (0.0, 1.3), 13.0, GuardBreak, false, 22.0, Motion::Leap).tell(0.5).stuck(40),
    ],
    pick: |near, r, _| match (near, r) { (true, 0..=6) => 0, (true, _) | (false, _) => 2 }, near: 1.4, walk: 2.0, keep: 1.0, rest: 40, open: 80, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};

/// Swollen and slow: bile at the chest (roll it, or pay to block it) and a belly slam to parry.
/// Felled, it swells and bursts (`SIDE_BURST`): get clear, or roll through it.
pub static BLOATER: Breed = Breed {
    name: "Bloater", kind: Some(Archetype::Bloater), hp: 80.0, stance: 34.0, half: 0.5, height: 1.95,
    blows: &[
        blow("belch", (46, 8, 36), (0.2, 2.4), (0.4, 1.9), 16.0, GuardBreak, false, 30.0, Motion::Strike),
        blow("slam", (36, 5, 40), (0.1, 1.6), (0.0, 1.6), 18.0, Physical, true, 24.0, Motion::Strike),
    ],
    pick: |near, r, _| match (near, r) { (true, 0..=5) => 1, _ => 0 }, near: 1.6, walk: 0.6, keep: 1.4, rest: 80, open: 110, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
/// A felled bloater's burst, `SIDE_BURST_AT` ticks after it falls, all round it.
const SIDE_BURST: Blow = blow("burst", (0, 6, 0), (-2.4, 2.4), (0.0, 2.6), 26.0, Unblockable, false, 0.0, Motion::Strike);
const SIDE_BURST_AT: u32 = 48;
/// Hangs back and calls up the dead (`RISEN`, either side of it), opens grave-hands under the
/// hero (move off the mark), and swipes whoever closes.
pub static GRAVECALLER: Breed = Breed {
    name: "Gravecaller", kind: Some(Archetype::Gravecaller), hp: 60.0, stance: 22.0, half: 0.4, height: 2.1,
    blows: &[
        blow("raise", (56, 10, 36), (0.0, 0.0), (0.0, 0.0), 0.0, Physical, false, 0.0, Motion::Raise),
        blow("grasp", (60, 10, 40), (-0.9, 0.9), (0.0, 2.0), 20.0, Unblockable, false, 0.0, Motion::Rune).tell(0.55),
        blow("swipe", (26, 6, 32), (0.2, 1.8), (0.0, 1.9), 14.0, GuardBreak, false, 22.0, Motion::Strike),
    ],
    pick: |near, r, _| match (near, r) { (true, 0..=6) => 2, (_, 0..=2) => 0, _ => 1 }, near: 1.8, walk: 0.8, keep: 5.5, rest: 80, open: 100, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
/// A husk a gravecaller called up: frail, drawn as a husk, and leaving nothing when it falls.
pub static RISEN: Breed = Breed {
    name: "Risen", kind: Some(Archetype::Husk), hp: 40.0, stance: 20.0, half: 0.4, height: 1.9,
    blows: &[HUSK_CHOP, HUSK_SWEEP],
    pick: |_, r, _| if r < 6 { 0 } else { 1 }, near: 1.8, walk: 0.8, keep: 1.3, rest: 80, open: 110, enrages: false, power: 1.0, wake: None, reply: None, grab: None, sprite: None,
};
/// How many of the risen may stand at once, and the beat before one moves.
const MOST_RISEN: usize = 2;
const RISE_WAIT: u32 = 50;

/// What a foe is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    /// Squaring up for this many ticks.
    Wait(u32),
    /// Blow `i`, `t` ticks in; `hit` once it has struck the hero; `from` where it began; `aim`
    /// where on the ground it is meant for (a leap's landing, followed until its tell).
    Blow { i: usize, t: u32, hit: bool, from: f32, aim: f32 },
    /// Open after a parry or a broken stance.
    Open(u32),
    Dead,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Foe {
    pub breed: &'static Breed,
    pub s: f32,
    pub facing: f32,
    pub hp: f32,
    pub stance: f32,
    stance_wait: u32,
    pub ai: Ai,
    /// Ticks of the flash of the last hit taken.
    pub hurt: u32,
    /// Blows thrown, which picks the next.
    count: u32,
    /// Ticks since it fell (for the drawing of its fall).
    pub down: u32,
    /// Its whole health, and what its blows do against what they are drawn as: its breed's, and
    /// more deeper in the world (`scaled`).
    pub top: f32,
    pub power: f32,
    /// Whether it has opened its second phase.
    pub woke: bool,
    /// Ticks left of a frost bolt's chill (`fight::CHILL`), and of a capstone's burn.
    pub chill: u32,
    pub burn: crate::fight::Burn,
    /// What its region has made of a road kind (`fight::Variant`).
    pub variant: crate::fight::Variant,
    /// Hits taken while squared up since its last blow (`TEMPER` of them and it answers).
    pressed: u32,
}

impl Foe {
    /// A foe of `breed` at `s`, facing `facing`, squaring up for `wait` ticks.
    pub fn new(breed: &'static Breed, s: f32, facing: f32, wait: u32) -> Foe {
        Foe { breed, s, facing, hp: breed.hp, stance: breed.stance, stance_wait: 0, ai: Ai::Wait(wait), hurt: 0, count: 0, down: 0, top: breed.hp, power: breed.power, woke: false, chill: 0, burn: crate::fight::Burn::default(), variant: crate::fight::Variant::Plain, pressed: 0 }
    }
    /// As strong as the region it is met in makes it: (health, blows) times its breed's.
    pub fn scaled(mut self, (hp, power): (f32, f32)) -> Foe {
        self.hp *= hp;
        self.top *= hp;
        self.power *= power;
        self
    }
    /// As its region breeds it (`fight::Variant`).
    pub fn of(self, variant: crate::fight::Variant) -> Foe { Foe { variant, ..self } }
    pub fn max_hp(&self) -> f32 { self.top }
    pub fn max_stance(&self) -> f32 { self.breed.stance }
    pub fn alive(&self) -> bool { self.ai != Ai::Dead }
    /// A breed that enrages, past half its health: quicker, and its strings going on more often.
    pub fn enraged(&self) -> bool { self.breed.enrages && self.hp < 0.5 * self.top }
    /// Winding up or live: the moment others hold back for.
    fn pressing(&self) -> bool { self.blow().is_some_and(|(_, ph, _)| ph != crate::fight::Phase::Recover) }
    /// The blow under way: which, its phase, how far through the windup (0..1).
    pub fn blow(&self) -> Option<(Blow, crate::fight::Phase, f32)> {
        let Ai::Blow { i, t, .. } = self.ai else { return None };
        let b = self.timed(i);
        let ph = if t < b.frames.0 { crate::fight::Phase::Windup } else if t < b.frames.0 + b.frames.1 { crate::fight::Phase::Active } else { crate::fight::Phase::Recover };
        Some((b, ph, (t as f32 / b.frames.0 as f32).min(1.0)))
    }
    /// Blow `i` as this foe throws it now (quicker windups when enraged).
    pub fn timed(&self, i: usize) -> Blow {
        let mut b = self.breed.blows[i];
        if self.enraged() { b.frames.0 = (b.frames.0 as f32 * 0.8) as u32; }
        b
    }
    /// A ring blow's ring now: where it struck and how far out it has rolled, while live.
    pub fn ring(&self) -> Option<(f32, f32)> {
        let Ai::Blow { i, t, from, .. } = self.ai else { return None };
        let b = self.timed(i);
        (b.motion == Motion::Ring && t >= b.frames.0 && t < b.frames.0 + b.frames.1).then(|| (from + self.facing * 1.4, (t - b.frames.0) as f32 * RING_V))
    }
    /// A shot in flight: its middle now, while live and within the yard.
    pub fn shot(&self) -> Option<f32> {
        let Ai::Blow { i, t, from, .. } = self.ai else { return None };
        let b = self.timed(i);
        let Motion::Shot(v) = b.motion else { return None };
        let at = from + self.facing * (SHOT_FROM + (t as f32 - b.frames.0 as f32) * v);
        (t >= b.frames.0 && t < b.frames.0 + b.frames.1 && at.abs() < HALF).then_some(at)
    }
    /// How high a leap carries it now, metres (an arc from its tell to its landing).
    pub fn lift(&self) -> f32 {
        let Ai::Blow { i, t, .. } = self.ai else { return 0.0 };
        let b = self.timed(i);
        let tell = Foe::tell_at(&b);
        if b.motion != Motion::Leap || t < tell || t >= b.frames.0 { return 0.0; }
        let k = (t - tell) as f32 / (b.frames.0 - tell).max(1) as f32;
        4.0 * (0.8 + 0.4 * self.breed.half) * k * (1.0 - k)
    }
    /// The tick of blow `b`'s tell.
    fn tell_at(b: &Blow) -> u32 { ((b.frames.0 as f32) * b.tell).ceil() as u32 }
    /// Where on the ground the blow under way will land, (from, to) metres, for its warning.
    pub fn warning(&self) -> Option<(f32, f32, Kind, bool, f32)> {
        let (b, ph, progress) = self.blow()?;
        if ph == crate::fight::Phase::Recover { return None; }
        let span = match b.motion {
            Motion::Ring => { let (at, r) = self.ring().unwrap_or((self.s + self.facing * 1.4, 0.0)); (at - r - RING_HALF, at + r + RING_HALF) }
            Motion::Charge(v) => {
                let run = v * b.frames.1 as f32 + self.breed.half;
                let end = (self.s + self.facing * run).clamp(-HALF, HALF);
                (self.s.min(end), self.s.max(end))
            }
            Motion::Strike => { let (a, z) = (self.s + self.facing * b.reach.0, self.s + self.facing * b.reach.1); (a.min(z), a.max(z)) }
            // Where it will come down (followed until its tell).
            Motion::Leap => {
                let Ai::Blow { aim, .. } = self.ai else { return None };
                let at = if ph == crate::fight::Phase::Active { self.s } else { aim };
                (at + b.reach.0, at + b.reach.1)
            }
            // Where it will open (followed until its tell).
            Motion::Rune => { let Ai::Blow { aim, .. } = self.ai else { return None }; (aim + b.reach.0, aim + b.reach.1) }
            Motion::Raise => return None,
            // Its whole flight while it winds up; itself once loosed.
            Motion::Shot(_) => match self.shot() {
                Some(at) => (at - SHOT_HALF, at + SHOT_HALF),
                None => { let a = self.s + self.facing * SHOT_FROM; let z = if self.facing > 0.0 { HALF } else { -HALF }; (a.min(z), a.max(z)) }
            },
        };
        Some((span.0, span.1, b.kind, b.parry, progress))
    }
}

/// The whole side-on fight.
#[derive(Clone, Debug, PartialEq)]
pub struct SideFight {
    pub hero: SideHero,
    pub foes: Vec<Foe>,
    /// The hero's arrows and bolts in flight.
    pub bolts: Vec<SideBolt>,
    /// The thrown Hatchet, out or on its way back.
    pub hatchets: Vec<SideHatchet>,
    pub ticks: u64,
    /// A count that stands in for chance.
    seed: u32,
}

impl SideFight {
    /// The boss fight: the hero, as the road left them, at the yard's left; the Warden at its
    /// right, facing them.
    pub fn new(stats: Stats, hp: f32, stamina: f32, flasks: u32) -> SideFight {
        Self::boss(stats, hp, stamina, flasks, &WARDEN, (1.0, 1.0))
    }

    /// A boss (or mini-boss) of `breed`, as strong as `scale` makes it, across the yard.
    pub fn boss(stats: Stats, hp: f32, stamina: f32, flasks: u32, breed: &'static Breed, scale: (f32, f32)) -> SideFight {
        Self::with(stats, hp, stamina, flasks, -HALF + 1.5, vec![Foe::new(breed, HALF - 2.0 - (breed.half - 0.95).max(0.0), -1.0, 90).scaled(scale)])
    }

    /// An ambush: the hero in the middle, the foes where `at` puts them (metres along), each
    /// facing the hero and starting a little after the one before.
    pub fn ambush(stats: Stats, hp: f32, stamina: f32, flasks: u32, at: &[(&'static Breed, f32)]) -> SideFight {
        let foes = at.iter().enumerate().map(|(k, &(b, s))| Foe::new(b, s.clamp(-HALF + b.half, HALF - b.half), if s > 0.0 { -1.0 } else { 1.0 }, 70 + 45 * k as u32)).collect();
        Self::with(stats, hp, stamina, flasks, 0.0, foes)
    }

    fn with(stats: Stats, hp: f32, stamina: f32, flasks: u32, s: f32, foes: Vec<Foe>) -> SideFight {
        SideFight {
            hero: SideHero {
                stats, hp, stamina, rested: 30, flasks, s, y: 0.0, vy: 0.0, facing: 1.0, act: None, guarding: false,
                reel: Reel::None, reeling: 0, poise: stats.poise(), poise_wait: 0, from: 0.0, landed: false, dead: false, walking: false, buffer: None,
                quiver: crate::fight::Quiver::default(), charges: stats.max_charges(), bulwark: Bulwark::default(),
                art: 0.0, stone: 0, mark: None, pierced: 0,
            },
            foes,
            bolts: Vec::new(),
            hatchets: Vec::new(),
            ticks: 0,
            seed: 0x51DE,
        }
    }

    /// Every foe down.
    pub fn won(&self) -> bool { self.foes.iter().all(|f| !f.alive()) }

    /// The living foe nearest the hero, by index.
    pub fn nearest(&self) -> Option<usize> {
        let s = self.hero.s;
        (0..self.foes.len()).filter(|&i| self.foes[i].alive()).min_by(|&a, &b| (self.foes[a].s - s).abs().total_cmp(&(self.foes[b].s - s).abs()))
    }

    fn roll(&mut self, n: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.seed >> 16) % n.max(1)
    }

    pub fn tick(&mut self, input: &SideInput) -> Vec<Event> {
        let mut out = Vec::new();
        self.ticks += 1;
        if !self.hero.dead { self.hero_tick(input, &mut out); }
        for i in 0..self.foes.len() {
            if self.foes[i].alive() { self.foe_tick(i, &mut out); continue; }
            self.foes[i].down += 1;
            if self.foes[i].down == SIDE_BURST_AT && self.foes[i].breed.kind == Some(crate::fight::Archetype::Bloater) {
                out.push(Event::Popped { enemy: i, at: self.foes[i].s as f64 });
                self.blow_lands(i, SIDE_BURST, &mut out);
            }
        }
        self.bolts_tick(&mut out);
        self.hatchets_tick(&mut out);
        // Sentence's blow falls when its count runs out.
        if let Some((i, n)) = self.hero.mark {
            self.hero.mark = (n > 1).then_some((i, n - 1));
            if n <= 1 && self.foes.get(i).is_some_and(|f| f.alive()) {
                let spec = Act::art_spec(&self.hero.stats.kit);
                out.push(Event::Sentenced { enemy: i });
                self.art_hit(i, spec.damage, spec.stance, &mut out);
            }
        }
        if self.hero.stats.kit.art.is_some() {
            self.hero.art = (self.hero.art + crate::arts::gained(&out)).min(crate::arts::ART_MAX);
        }
        // Arrows pulled back out of the fallen, and archers' own.
        let felled: Vec<usize> = out.iter().filter_map(|e| if let Event::Felled { enemy } = e { Some(*enemy) } else { None }).collect();
        if !felled.is_empty() {
            let archers = felled.iter().filter(|&&i| self.foes.get(i).is_some_and(|f| f.breed.kind == Some(crate::fight::Archetype::Archer))).count() as u32;
            self.hero.quiver.salvage(self.hero.stats.kit.ranged, felled.len() as u32, archers);
        }
        if self.hero.hp <= 0.0 && !self.hero.dead {
            self.hero.dead = true;
            self.hero.hp = 0.0;
            out.push(Event::Died);
        }
        out
    }

    fn hero_tick(&mut self, input: &SideInput, out: &mut Vec<Event>) {
        let foe_s = self.nearest().map(|i| self.foes[i].s);
        let h = &mut self.hero;
        h.reeling = h.reeling.saturating_sub(1);
        if h.reeling == 0 { h.reel = Reel::None; }
        h.poise_wait = h.poise_wait.saturating_add(1);
        if h.poise_wait > 90 { h.poise = (h.poise + 0.5).min(h.stats.poise()); }
        let dir = (input.right as i8) - (input.left as i8);
        h.guarding = input.guard && h.free() && !h.airborne();
        // A yard is a fight from end to end: the ward never runs down here.
        h.bulwark.tick(h.guarding, false);
        h.walking = false;
        // Presses wait a moment for the hero to be free, as on the road.
        h.stone = h.stone.saturating_sub(1);
        if input.roll || input.jump || input.light || input.heavy || input.parry || input.flask || input.loose || input.cast || input.art { h.buffer = Some((*input, 0)); }
        else if let Some((b, age)) = h.buffer { h.buffer = (age < crate::fight::BUFFER).then_some((b, age + 1)); }
        let p = h.buffer.map(|b| b.0).unwrap_or_default();
        let pdir = (p.right as i8) - (p.left as i8);
        // What was pressed, if the hero is free to do it (in the air: only a strike).
        let want = if p.roll { Some(Act::Roll(if pdir != 0 { pdir } else if dir != 0 { dir } else { h.facing as i8 })) }
            else if p.art { Some(Act::Art) } else if p.flask { Some(Act::Flask) } else if p.parry { Some(Act::Parry) } else if p.loose { Some(Act::Loose) } else if p.cast { Some(Act::Cast) }
            else if p.heavy { Some(Act::Heavy) } else if p.light { Some(Act::Light) } else { None };
        if h.free() {
            if p.jump && !h.airborne() && !h.guarding {
                h.buffer = None;
                if h.stamina <= 0.0 { out.push(Event::Spent); } else { h.spend(h.stats.cost(Move::Backstep, JUMP_COST)); h.vy = JUMP_V; h.y = 1e-4; }
            } else if let Some(a) = want.filter(|a| !h.airborne() || matches!(a, Act::Light | Act::Heavy)) {
                h.buffer = None;
                let cost = h.stats.cost(a.as_move(), match a {
                    Act::Light | Act::Heavy => a.struck(&h.stats.kit).stamina,
                    Act::Roll(_) => 12.0,
                    Act::Cast if h.stats.kit.imbues() => 0.0,
                    Act::Loose | Act::Cast => Move::loose_spec(a.shot(&h.stats.kit).unwrap_or(crate::gear::Ranged::Bow)).stamina,
                    Act::Art => Act::art_spec(&h.stats.kit).stamina,
                    _ => 0.0,
                });
                if cost > 0.0 && h.stamina <= 0.0 {
                    out.push(Event::Spent);
                } else if a == Act::Flask && h.flasks == 0 {
                } else if matches!(a, Act::Loose | Act::Cast) && !h.can_shoot(a) {
                } else if a == Act::Art && !h.can_art() {
                } else {
                    h.spend(cost);
                    if let Act::Roll(d) = a { h.facing = d as f32; }
                    h.act = Some((a, 0));
                    (h.from, h.landed, h.pierced) = (h.s, false, 0);
                    out.push(Event::Began(a.as_move()));
                }
            } else if dir != 0 && !h.guarding {
                // Walking turns the hero the way they go; a run is quicker, while stamina lasts.
                let run = input.run && h.stamina > 0.0;
                h.facing = dir as f32;
                h.s += dir as f32 * WALK * if run { RUN_K } else { 1.0 };
                if run { (h.stamina, h.rested) = ((h.stamina - crate::fight::RUN_COST).max(0.0), 0); }
                h.walking = true;
            } else if let (0, false, Some(fs)) = (dir, h.airborne(), foe_s) {
                // Standing, they face the nearest foe.
                h.facing = if fs >= h.s { 1.0 } else { -1.0 };
            }
        } else if h.airborne() && dir != 0 && h.act.is_none() {
            h.s += dir as f32 * WALK * 0.8;
        }
        // In the air.
        if h.airborne() {
            h.y += h.vy;
            h.vy -= GRAVITY * if h.vy.abs() < HANG_V { HANG } else { 1.0 };
            if h.airborne() && h.act.is_none() && dir != 0 { h.facing = dir as f32; }
            if h.y <= 0.0 { (h.y, h.vy) = (0.0, 0.0); }
        }
        // Stamina back once it has gone unspent a while, and the hero is on their feet.
        h.rested = h.rested.saturating_add(1);
        if h.rested > 30 && !h.airborne() && h.act.is_none_or(|(a, _)| a == Act::Flask) {
            let k = if h.guarding { 1.0 / 3.0 } else { 1.0 };
            h.stamina = (h.stamina + 55.0 / TPS * k * h.stats.regen()).min(h.stats.max_stamina());
        }
        // The move under way.
        if let Some((a, t)) = h.act {
            let kit = h.stats.kit;
            let f = a.frames(&kit);
            match a {
                Act::Roll(d) => {
                    // Fast out of it, easing at the end.
                    let k = (t + 1) as f32 / ROLL_T as f32;
                    h.s = h.from + d as f32 * ROLL_M * (1.0 - (1.0 - k).powi(2));
                }
                Act::Flask if t == f.0 => {
                    h.flasks -= 1;
                    h.hp = (h.hp + 40.0 * h.stats.kit.heal).min(h.stats.max_hp());
                    out.push(Event::Healed);
                }
                // Loosed at the end of the draw, from the chest, the way the hero faces.
                // Blood Imbue: paid for in health as it goes off.
                Act::Cast if t == f.0 && kit.imbues() => {
                    h.hp = (h.hp - h.stats.max_hp() * crate::fight::IMBUE_COST).max(1.0);
                    h.quiver.imbued = crate::fight::IMBUE_SHOTS;
                    out.push(Event::Imbued);
                }
                Act::Loose | Act::Cast if t == f.0 => {
                    if let Some(r) = a.shot(&kit) {
                        let imbued = if r == crate::gear::Ranged::Frost { h.charges = (h.charges - 1.0).max(0.0); false } else { h.quiver.take(r) };
                        let spec = Move::loose_spec(r);
                        let damage = spec.damage * h.stats.ranged_damage(r) * if imbued { crate::fight::IMBUE_DAMAGE } else { 1.0 };
                        let bolt = SideBolt { s: h.s + h.facing * 0.4, dir: h.facing, y: h.y + 1.3, ranged: r, damage, stance: spec.stance, left: crate::fight::BOLT_RANGE, age: 0, imbued, pierced: 0, keen: kit.capped.keen && r != crate::gear::Ranged::Frost };
                        self.bolts.push(bolt);
                        out.push(Event::Shot(r));
                    }
                }
                _ => {}
            }
            h.act = (t + 1 < a.total(&kit)).then_some((a, t + 1));
            if matches!(a, Act::Light | Act::Heavy) && t >= f.0 && t < f.0 + f.1 && !h.landed { self.strike(a, out); }
            if a == Act::Art { if let Some(art) = kit.art { self.art_fires(art, t, out); } }
        }
        // The yard's walls; and the foes' bodies, which only a roll (or a jump over a small
        // one) passes through.
        let h = &mut self.hero;
        h.s = h.s.clamp(-HALF + HERO_HALF, HALF - HERO_HALF);
        // Light Lance's dash passes through bodies as a roll does.
        let dash = h.stats.kit.art == Some(crate::arts::WeaponArt::Lance) && matches!(h.act, Some((Act::Art, t)) if { let f = Act::art_spec(&h.stats.kit).frames; t >= f.0 && t < f.0 + f.1 });
        let rolling = matches!(h.act, Some((Act::Roll(_), _))) || dash;
        for f in self.foes.iter().filter(|f| f.alive()) {
            let gap = h.s - f.s;
            let reach = HERO_HALF + f.breed.half;
            if !rolling && h.y < f.breed.height * 0.8 && gap.abs() < reach {
                let side = if gap == 0.0 { -f.facing } else { gap.signum() };
                let s = f.s + side * reach;
                h.s = if s.abs() > HALF - HERO_HALF { f.s - side * reach } else { s };
            }
        }
    }

    /// The hero's strike `a` in its live frames: the nearest foe in front within reach takes it;
    /// a heavy cleaves through every one there.
    fn strike(&mut self, a: Act, out: &mut Vec<Event>) {
        let reach = a.struck(&self.hero.stats.kit).reach;
        let cleave = a == Act::Heavy;
        let (hs, hf) = (self.hero.s, self.hero.facing);
        let mut struck: Vec<usize> = (0..self.foes.len()).filter(|&i| {
            let f = &self.foes[i];
            let ahead = (f.s - hs) * hf;
            f.alive() && ahead >= -f.breed.half && ahead <= reach + f.breed.half
        }).collect();
        struck.sort_by(|&x, &y| (self.foes[x].s - hs).abs().total_cmp(&(self.foes[y].s - hs).abs()));
        if !cleave { struck.truncate(1); }
        if struck.is_empty() { return; }
        self.hero.landed = true;
        for i in struck { self.hit(i, a, out); }
    }

    /// Foe `i` takes the hero's strike `a`.
    /// A perfect block's ward let go at foe `i`, at `w` of a full ward, as the shield in hand lets
    /// it go (as the road's, `fight::Fight::ward_let`); `blow`, what the blow blocked would have dealt.
    fn ward_let(&mut self, i: usize, w: f32, blow: f32, out: &mut Vec<Event>) {
        use crate::gear::Ward;
        let ward = self.hero.stats.kit.ward.unwrap_or(Ward::Bulwark);
        // A reinforced shield lets it go harder.
        let w = w * self.hero.stats.kit.ward_boost();
        let open = self.foes[i].breed.open;
        let (dealt, down) = match ward {
            Ward::Kite => {
                let h = &mut self.hero;
                h.stamina = (h.stamina + crate::fight::BASH_STAMINA * w).min(h.stats.max_stamina());
                (crate::fight::BASH_DAMAGE * w, open + (open as f32 * crate::fight::BASH_DOWN * w) as u32)
            }
            Ward::Buckler => (blow * (1.0 + crate::fight::TURN_MORE * w), open),
            Ward::Bulwark | Ward::Parma => return self.ward_burst(i, w, out),
        };
        out.push(Event::WardBurst { enemy: i, power: w, ward });
        let b = &mut self.foes[i];
        b.hp -= (dealt * self.hero.stats.damage()).min(b.hp.max(0.0));
        b.hurt = 10;
        if b.hp <= 0.0 {
            b.hp = 0.0;
            b.ai = Ai::Dead;
            out.push(Event::Felled { enemy: i });
        } else {
            b.ai = Ai::Open(down);
            out.push(Event::StanceBroken { enemy: i });
        }
    }

    /// As the road's (`fight::Fight::thorns`): of a blow blocked, its share back into foe `i`.
    fn thorns(&mut self, i: usize, blow: f32, out: &mut Vec<Event>) {
        let back = blow * self.hero.stats.kit.capped.thorns;
        let b = &mut self.foes[i];
        if back <= 0.0 || b.ai == Ai::Dead { return; }
        b.hp -= back.min(b.hp);
        b.hurt = 10;
        if b.hp <= 0.0 {
            (b.hp, b.ai) = (0.0, Ai::Dead);
            out.push(Event::Felled { enemy: i });
        }
    }

    /// A bulwark's ward let go at foe `i`, at `w` of a full ward: it is thrown open, hurt
    /// by its share of `fight::BURST_DAMAGE`; foes near it are hurt by less and thrown back.
    fn ward_burst(&mut self, i: usize, w: f32, out: &mut Vec<Event>) {
        use crate::fight::{BURST_DAMAGE, BURST_REACH, BURST_SHOVE, BURST_SPLASH};
        let dealt = BURST_DAMAGE * w * self.hero.stats.damage();
        let (at, hero) = (self.foes[i].s, self.hero.s);
        out.push(Event::WardBurst { enemy: i, power: w, ward: crate::gear::Ward::Bulwark });
        for (j, b) in self.foes.iter_mut().enumerate() {
            if b.ai == Ai::Dead || (j != i && (b.s - at).abs() >= BURST_REACH) { continue; }
            b.hp -= (if j == i { dealt } else { dealt * BURST_SPLASH }).min(b.hp.max(0.0));
            b.hurt = 10;
            if b.hp <= 0.0 {
                b.hp = 0.0;
                b.ai = Ai::Dead;
                out.push(Event::Felled { enemy: j });
            } else if j == i {
                b.ai = Ai::Open(b.breed.open);
                out.push(Event::StanceBroken { enemy: j });
            } else {
                b.s += BURST_SHOVE * if b.s >= hero { 1.0 } else { -1.0 };
                b.ai = Ai::Wait(40);
            }
        }
    }

    fn hit(&mut self, i: usize, a: Act, out: &mut Vec<Event>) {
        let h = &self.hero;
        let Spec { damage, stance, .. } = a.struck(&h.stats.kit);
        let b = &mut self.foes[i];
        let open = matches!(b.ai, Ai::Open(_));
        // A parma's ward goes into the first blow on an open foe (the yard's riposte).
        let duel = if open && h.stats.kit.ward == Some(crate::gear::Ward::Parma) && h.bulwark.ward >= crate::fight::WARD_BURST { Some(h.bulwark.share()) } else { None };
        let more = duel.map_or(0.0, |w| crate::fight::DUEL_DAMAGE * w * h.stats.damage());
        let dealt = (damage * h.stats.damage() * if open { OPEN_DAMAGE * h.stats.kit.opener } else { 1.0 } + more).min(b.hp.max(0.0));
        b.hp -= dealt;
        // A weapon that drinks gives some of it back.
        let h = &mut self.hero;
        h.hp = (h.hp + dealt * h.stats.kit.leech).min(h.stats.max_hp());
        if let Some(w) = duel {
            h.bulwark.let_go();
            h.hp = (h.hp + more.min(dealt) * crate::fight::DUEL_DRINK).min(h.stats.max_hp());
            out.push(Event::WardBurst { enemy: i, power: w, ward: crate::gear::Ward::Parma });
        }
        // A blow landed up close fills the catalyst.
        if h.stats.kit.spell_cast().is_some() { h.charges = (h.charges + crate::fight::CHARGE_PER_HIT).min(h.stats.max_charges()); }
        let capped = h.stats.kit.capped;
        let b = &mut self.foes[i];
        // The weapon's capstone: the blow burns on, or chills.
        b.burn.light(dealt, capped.burn);
        b.chill = b.chill.max(capped.rime);
        b.hurt = 10;
        b.stance_wait = 0;
        if matches!(b.ai, Ai::Wait(_)) { b.pressed += 1; }
        out.push(Event::Hit { enemy: i, mv: a.as_move() });
        if b.hp <= 0.0 {
            b.hp = 0.0;
            b.ai = Ai::Dead;
            out.push(Event::Felled { enemy: i });
            return;
        }
        if !open {
            b.stance -= stance;
            if b.stance <= 0.0 {
                b.stance = b.breed.stance;
                b.ai = Ai::Open(b.breed.open);
                out.push(Event::StanceBroken { enemy: i });
            }
        }
    }

    /// The hero's arrows and bolts fly on; each strikes the first foe it reaches at its height.
    fn bolts_tick(&mut self, out: &mut Vec<Event>) {
        let mut k = 0;
        while k < self.bolts.len() {
            let b = &mut self.bolts[k];
            let step = b.ranged.speed() / TPS;
            b.age += 1;
            b.s += b.dir * step;
            b.left -= step;
            let (s, y, dir, pierced) = (b.s, b.y, b.dir, b.pierced);
            let reached = |i: usize, f: &Foe| f.alive() && y < f.breed.height + f.lift() && y > f.lift() && ((f.s - s) * dir) <= f.breed.half && ((f.s - s) * dir) > -f.breed.half - step && pierced & (1 << i.min(63)) == 0;
            let hit = (0..self.foes.len()).filter(|&i| reached(i, &self.foes[i])).min_by(|&a, &c| ((self.foes[a].s - s) * dir).total_cmp(&((self.foes[c].s - s) * dir)));
            if let Some(i) = hit {
                // An imbued bolt goes on through, and on into the next.
                if self.bolts[k].pierces() {
                    self.bolts[k].pierced |= 1 << i.min(63);
                    let b = self.bolts[k];
                    self.bolt_hit(i, b, out);
                    k += 1;
                    continue;
                }
                let b = self.bolts.remove(k);
                self.bolt_hit(i, b, out);
                continue;
            }
            if self.bolts[k].left <= 0.0 || self.bolts[k].s.abs() > HALF { self.bolts.remove(k); continue; }
            k += 1;
        }
    }

    /// Foe `i` takes the hero's arrow or bolt `b`: none of a blow's extras (no drinking, no more
    /// against the open); a shieldbearer squared up catches it.
    fn bolt_hit(&mut self, i: usize, b: SideBolt, out: &mut Vec<Event>) {
        let f = &mut self.foes[i];
        if f.breed.kind == Some(crate::fight::Archetype::Shieldbearer) && matches!(f.ai, Ai::Wait(_)) && f.facing * b.dir < 0.0 {
            f.stance -= 1.0;
            out.push(Event::Glanced { enemy: i });
            return;
        }
        let open = matches!(f.ai, Ai::Open(_));
        let dealt = b.damage.min(f.hp.max(0.0));
        f.hp -= dealt;
        f.hurt = 10;
        let capped = self.hero.stats.kit.capped;
        if b.ranged == crate::gear::Ranged::Frost { f.chill = crate::fight::CHILL * capped.winter; } else { f.chill = f.chill.max(capped.shot_chill); }
        if b.ranged != crate::gear::Ranged::Frost { f.burn.light(dealt, capped.shot_burn); }
        let drink = if b.drinks() { crate::fight::IMBUE_DRINK } else if b.ranged == crate::gear::Ranged::Frost { capped.spell_drink } else { 0.0 };
        self.hero.hp = (self.hero.hp + dealt * drink).min(self.hero.stats.max_hp());
        let f = &mut self.foes[i];
        f.stance_wait = 0;
        if matches!(f.ai, Ai::Wait(_)) { f.pressed += 1; }
        out.push(Event::Hit { enemy: i, mv: Move::Loose });
        if f.hp <= 0.0 {
            f.hp = 0.0;
            f.ai = Ai::Dead;
            out.push(Event::Felled { enemy: i });
            return;
        }
        if !open {
            f.stance -= b.stance;
            if f.stance <= 0.0 {
                f.stance = f.breed.stance;
                f.ai = Ai::Open(f.breed.open);
                out.push(Event::StanceBroken { enemy: i });
            }
        }
    }

    fn foe_tick(&mut self, i: usize, out: &mut Vec<Event>) {
        let f = &mut self.foes[i];
        // Burning: it goes on taking a little, and can die of it.
        let burnt = f.burn.tick();
        if burnt > 0.0 && f.ai != Ai::Dead {
            f.hp -= burnt.min(f.hp);
            if f.hp <= 0.0 {
                (f.hp, f.ai) = (0.0, Ai::Dead);
                out.push(Event::Felled { enemy: i });
                return;
            }
        }
        // Chilled by frost: every third tick lost, and no stance back while it lasts.
        if f.chill > 0 {
            (f.chill, f.stance_wait) = (f.chill - 1, 0);
            if f.chill % 3 == 0 { f.hurt = f.hurt.saturating_sub(1); return; }
        }
        let hero_s = self.hero.s;
        // What it reads of the hero: a flask raised, a guard held.
        let (drinking, guarding) = (matches!(self.hero.act, Some((Act::Flask, _))), self.hero.guarding);
        // Another foe winding up or live holds this one back; and it will not walk through one
        // that stands between it and the hero.
        let busy = self.foes.iter().enumerate().any(|(j, f)| j != i && f.alive() && f.pressing());
        let me = self.foes[i].s;
        let blocked = |step: f32| self.foes.iter().enumerate().any(|(j, f)| {
            j != i && f.alive() && (f.s - me).signum() == step.signum() && (f.s - hero_s).abs() < (me - hero_s).abs()
                && (f.s - (me + step)).abs() < f.breed.half + self.foes[i].breed.half + SPACING
        });
        let b = &self.foes[i];
        let step = if hero_s >= b.s { 1.0 } else { -1.0 } * b.breed.walk / TPS;
        let held = blocked(step);
        // A charge stops at another's body, too.
        let charge = match b.ai { Ai::Blow { i: k, .. } => match b.breed.blows[k].motion { Motion::Charge(v) => b.facing * v, _ => 0.0 }, _ => 0.0 };
        // Its second phase opens with its waking blow, the moment it enrages (not mid-blow).
        let wake = (!b.woke && b.enraged() && matches!(b.ai, Ai::Wait(_))).then_some(b.breed.wake).flatten();
        // A body in the way, anywhere ahead along this tick's run (a charge runs on through the
        // hero, so the one it must not run into may stand beyond them).
        let in_way = |run: f32| run != 0.0 && self.foes.iter().enumerate().any(|(j, f)| {
            let room = f.breed.half + self.foes[i].breed.half + SPACING;
            j != i && f.alive() && (f.s - me) * run.signum() > 0.0 && (f.s - me).abs() < run.abs() + room
        });
        let charge_held = in_way(charge);
        // A leap travels from its tell to the end of its windup, landing on its aim; it comes down
        // short of another's body in its way (anywhere along this tick's flight, not only where it
        // lands).
        let (lo, hi) = (-HALF + b.breed.half, HALF - b.breed.half);
        let leap = match b.ai {
            Ai::Blow { i: k, t, aim, .. } => {
                let bl = b.timed(k);
                if bl.motion == Motion::Leap && t >= Foe::tell_at(&bl) && t < bl.frames.0 { (aim.clamp(lo, hi) - b.s) / (bl.frames.0 - t) as f32 } else { 0.0 }
            }
            _ => 0.0,
        };
        let leap_held = in_way(leap);
        let b = &mut self.foes[i];
        b.hurt = b.hurt.saturating_sub(1);
        b.stance_wait += b.variant.recovery();
        if b.stance_wait > 120 { b.stance = (b.stance + 0.4 * b.variant.recovery() as f32).min(b.breed.stance); }
        match b.ai {
            Ai::Dead => {}
            Ai::Open(0) => b.ai = Ai::Wait(30),
            Ai::Open(n) => b.ai = Ai::Open(n - 1),
            Ai::Wait(_) if wake.is_some() || (!b.woke && b.enraged()) => {
                b.woke = true;
                if let Some(k) = wake {
                    b.facing = if hero_s >= b.s { 1.0 } else { -1.0 };
                    b.ai = Ai::Blow { i: k, t: 0, hit: false, from: b.s, aim: hero_s };
                    out.push(Event::WindsUp { enemy: i, attack: b.breed.blows[k].name });
                }
            }
            Ai::Wait(n) => {
                b.facing = if hero_s >= b.s { 1.0 } else { -1.0 };
                let gap = (hero_s - b.s).abs();
                // Closing in at a walk while it waits, unless another stands in its way.
                if gap > b.breed.keep && !held { b.s += step; }
                // A boss does not wait on a hero beating at it, nor on one drinking within a rush
                // of it.
                let reads = b.breed.enrages;
                let pressed = reads && b.pressed >= TEMPER;
                let n = if reads && (pressed || (drinking && gap < 3.0 * b.breed.near)) { 0 } else { n };
                if n == 0 && busy {
                    // Its turn comes when the other's blow is over.
                    b.ai = Ai::Wait(10);
                } else if n == 0 {
                    let near = gap < b.breed.near;
                    let (pick, read) = (self.roll(10), self.roll(10));
                    let b = &mut self.foes[i];
                    let k = (b.breed.pick)(near, pick, b.enraged());
                    // Its answers: to a beating, the reply; to a guard held up close, half the
                    // time the grab.
                    let k = match (pressed && near, near && guarding && reads && read < 5) {
                        (true, _) => b.breed.reply.unwrap_or(k),
                        (_, true) => b.breed.grab.unwrap_or(k),
                        _ => k,
                    }.min(b.breed.blows.len() - 1);
                    b.count += 1;
                    b.pressed = 0;
                    b.ai = Ai::Blow { i: k, t: 0, hit: false, from: b.s, aim: hero_s };
                    out.push(Event::WindsUp { enemy: i, attack: b.breed.blows[k].name });
                } else {
                    b.ai = Ai::Wait(n - 1);
                }
            }
            Ai::Blow { i: k, t, hit, from, aim } => {
                let blow = b.timed(k);
                let (w, live, rec) = blow.frames;
                let tell = Foe::tell_at(&blow);
                if t == tell { out.push(Event::Commits { enemy: i, kind: blow.kind, parry: blow.parry }); }
                // A leap, a shot or a rune follows the hero until its tell; then it is thrown where
                // they were.
                let tracks = matches!(blow.motion, Motion::Leap | Motion::Shot(_) | Motion::Rune);
                let aim = if tracks && t < tell { hero_s } else { aim };
                if tracks && t < tell { b.facing = if hero_s >= b.s { 1.0 } else { -1.0 }; }
                // A charge runs while live, and stops at the wall.
                if let Motion::Charge(v) = blow.motion {
                    if t >= w && t < w + live && !charge_held { b.s = (b.s + b.facing * v).clamp(lo, hi); }
                }
                if leap != 0.0 && !leap_held { b.s = (b.s + leap).clamp(lo, hi); }
                // Launched from where it stood when its shot left the hand.
                let from = if matches!(blow.motion, Motion::Shot(_)) && t == w { b.s } else { from };
                // The shot leaving the hand, or the rune going off, for the effects to show.
                if t == w {
                    match blow.motion {
                        Motion::Shot(_) => out.push(Event::Loosed { enemy: i }),
                        Motion::Rune => out.push(Event::Burst { enemy: i, at: aim as f64 }),
                        Motion::Ring => out.push(Event::Rung { enemy: i, at: (b.s + b.facing * 1.4) as f64 }),
                        // A boss's blow meeting the ground: where its reach ends, or where it lands.
                        Motion::Strike | Motion::Leap if b.breed.kind.is_none() => {
                            let at = if blow.motion == Motion::Leap { b.s } else { b.s + b.facing * blow.reach.1.max(0.5) * 0.85 };
                            out.push(Event::Slam { enemy: i, at: at as f64, heavy: blow.damage >= 30.0 });
                        }
                        _ => {}
                    }
                }
                b.ai = Ai::Blow { i: k, t, hit, from, aim };
                if t == w && blow.motion == Motion::Raise { self.raise_risen(i, out); }
                let live_now = t >= w && t < w + live;
                let mut hit = hit;
                if live_now && !hit { hit = self.blow_lands(i, blow, out); }
                let seed = self.seed;
                // A blow that met nothing leaves it stuck in its recovery a while (a dodge's reward).
                let stuck = if hit { 0 } else { blow.stuck };
                // What follows, if anything: the next of a string goes on only sometimes.
                let ending = t + 1 >= w + live + rec + stuck;
                let wild = if self.foes[i].enraged() { 4 } else { 0 };
                // (Not while another presses: one at a time, as ever.)
                let go_on = blow.follow.filter(|f| ending && !busy && self.roll(10) < f.odds + wild);
                let b = &mut self.foes[i];
                b.ai = if matches!(b.ai, Ai::Open(_) | Ai::Dead) { b.ai } else if t + 1 >= w + live + rec + stuck {
                    match go_on.filter(|_| stuck == 0 || hit) {
                        Some(f) => {
                            b.count += 1;
                            b.facing = if hero_s >= b.s { 1.0 } else { -1.0 };
                            out.push(Event::WindsUp { enemy: i, attack: b.breed.blows[f.to].name });
                            Ai::Blow { i: f.to, t: f.at, hit: false, from: b.s, aim: hero_s }
                        }
                        None => Ai::Wait(if b.enraged() { b.breed.rest * 3 / 5 } else { b.breed.rest } + seed % 20),
                    }
                } else {
                    Ai::Blow { i: k, t: t + 1, hit, from, aim }
                };
            }
        }
    }

    /// Foe `i` struck by the art (`hit`'s reckoning, `damage` and `stance` times the hero's
    /// strike): what it took.
    fn art_hit(&mut self, i: usize, damage: f32, stance: f32, out: &mut Vec<Event>) -> f32 {
        let h = &self.hero;
        let b = &mut self.foes[i];
        let open = matches!(b.ai, Ai::Open(_));
        let dealt = (damage * h.stats.damage() * if open { OPEN_DAMAGE * h.stats.kit.opener } else { 1.0 }).min(b.hp.max(0.0));
        b.hp -= dealt;
        let h = &mut self.hero;
        h.hp = (h.hp + dealt * h.stats.kit.leech).min(h.stats.max_hp());
        let b = &mut self.foes[i];
        (b.hurt, b.stance_wait) = (10, 0);
        out.push(Event::Hit { enemy: i, mv: Move::Art });
        if b.hp <= 0.0 {
            b.hp = 0.0;
            b.ai = Ai::Dead;
            out.push(Event::Felled { enemy: i });
        } else if !open {
            b.stance -= stance;
            if b.stance <= 0.0 {
                b.stance = b.breed.stance;
                b.ai = Ai::Open(b.breed.open);
                out.push(Event::StanceBroken { enemy: i });
            }
        }
        dealt
    }

    /// Foe `i` thrown `arts::SHOVE` back from the hero: off balance (open half its usual time)
    /// when `opens`, else only its blow broken off.
    fn throw_back(&mut self, i: usize, opens: bool) {
        let hero = self.hero.s;
        let b = &mut self.foes[i];
        if !b.alive() || matches!(b.ai, Ai::Open(_)) { return; }
        b.s = (b.s + crate::arts::SHOVE * if b.s >= hero { 1.0 } else { -1.0 }).clamp(-HALF + b.breed.half, HALF - b.breed.half);
        b.ai = if opens { Ai::Open((b.breed.open / 2).max(30)) } else { Ai::Wait(crate::arts::REELS) };
    }

    /// Living foes within `reach` of the hero: in front only, or either side (a slam, a scream).
    fn art_reach(&self, reach: f32, both: bool) -> Vec<usize> {
        let (hs, hf) = (self.hero.s, self.hero.facing);
        (0..self.foes.len()).filter(|&i| {
            let f = &self.foes[i];
            let ahead = (f.s - hs) * hf;
            f.alive() && if both { ahead.abs() <= reach + f.breed.half } else { ahead >= -f.breed.half && ahead <= reach + f.breed.half }
        }).collect()
    }

    /// What `art` does `t` ticks into it (as `fight::Fight::art_fires`, across the yard).
    fn art_fires(&mut self, art: crate::arts::WeaponArt, t: u32, out: &mut Vec<Event>) {
        use crate::arts::*;
        let spec = Act::art_spec(&self.hero.stats.kit);
        let (w, live, _) = spec.frames;
        let (d, s) = (spec.damage, spec.stance);
        if t == art.fires() {
            self.hero.art = (self.hero.art - art.cost()).max(0.0);
            out.push(Event::Art { art });
            match art {
                WeaponArt::Toll => for i in self.art_reach(spec.reach, true) {
                    let open = matches!(self.foes[i].ai, Ai::Open(_));
                    self.art_hit(i, if open { d * 1.5 } else { d }, s, out);
                    self.throw_back(i, true);
                },
                WeaponArt::Wail => {
                    let mut drunk = 0.0;
                    for i in self.art_reach(spec.reach, true) {
                        drunk += self.art_hit(i, d, s, out);
                        // A windup broken off; a shot in the air falls.
                        let f = &mut self.foes[i];
                        match f.blow() {
                            Some((_, crate::fight::Phase::Windup, _)) => f.ai = Ai::Open((f.breed.open / 2).max(30)),
                            Some((b, crate::fight::Phase::Active, _)) if matches!(b.motion, Motion::Shot(_)) => f.ai = Ai::Wait(REELS),
                            _ => {}
                        }
                    }
                    // Shots already loosed from beyond its reach fall too.
                    for f in self.foes.iter_mut().filter(|f| f.shot().is_some()) { f.ai = Ai::Wait(REELS); }
                    let h = &mut self.hero;
                    h.hp = (h.hp + drunk * art.drinks()).min(h.stats.max_hp());
                }
                WeaponArt::Hatchet => {
                    let h = &self.hero;
                    self.hatchets.push(SideHatchet { s: h.s + h.facing * 0.4, dir: h.facing, left: spec.reach, back: false, bit: (0, 0), age: 0 });
                }
                WeaponArt::Sentence => {
                    let hs = self.hero.s;
                    let pick = self.art_reach(spec.reach, false).into_iter().min_by(|&a, &b| (self.foes[a].s - hs).abs().total_cmp(&(self.foes[b].s - hs).abs()));
                    if let Some(i) = pick { self.hero.mark = Some((i, SENTENCE_DELAY)); }
                }
                WeaponArt::Reap => {
                    let mut drunk = 0.0;
                    for i in self.art_reach(spec.reach, true) { drunk += self.art_hit(i, d, s, out); }
                    let h = &mut self.hero;
                    h.hp = (h.hp + drunk * art.drinks()).min(h.stats.max_hp());
                }
                WeaponArt::Lance => {}
                WeaponArt::StoneSkin => {
                    self.hero.stone = STONE_TICKS;
                    for i in self.art_reach(spec.reach, true) { self.art_hit(i, d, s, out); self.throw_back(i, false); }
                }
                WeaponArt::Wingbeat => for i in self.art_reach(spec.reach, false) { self.throw_back(i, false); },
            }
        }
        match art {
            // The dash: on through the live frames, piercing each foe it passes once.
            WeaponArt::Lance if t >= w && t < w + live => {
                let h = &mut self.hero;
                h.s = (h.s + h.facing * LANCE_DASH / live as f32).clamp(-HALF + HERO_HALF, HALF - HERO_HALF);
                for i in self.art_reach(1.0, true) {
                    if self.hero.pierced & (1 << (i % 64)) != 0 { continue; }
                    self.hero.pierced |= 1 << (i % 64);
                    self.art_hit(i, d, s, out);
                }
            }
            // The dive: across to the nearest ahead (bodies still stop it), striking as it lands.
            WeaponArt::Wingbeat if t >= w && t < DIVE_AT => {
                let h = &mut self.hero;
                h.s += h.facing * DIVE / (DIVE_AT - w) as f32;
            }
            WeaponArt::Wingbeat if t == DIVE_AT => {
                let hs = self.hero.s;
                let pick = self.art_reach(DIVE_REACH, false).into_iter().min_by(|&a, &b| (self.foes[a].s - hs).abs().total_cmp(&(self.foes[b].s - hs).abs()));
                if let Some(i) = pick { self.art_hit(i, d, s, out); }
            }
            _ => {}
        }
    }

    /// The thrown Hatchet one tick on: out the way it was thrown biting the first foe it meets,
    /// then back to the hero's hand, biting what it passes again.
    fn hatchets_tick(&mut self, out: &mut Vec<Event>) {
        let spec = Act::art_spec(&self.hero.stats.kit);
        let step = crate::arts::HATCHET_SPEED / TPS;
        let mut k = 0;
        while k < self.hatchets.len() {
            let mut t = self.hatchets[k];
            t.age += 1;
            let was = t.s;
            if t.back { t.s += (self.hero.s - t.s).signum() * step; } else { t.s += t.dir * step; t.left -= step; }
            let (lo, hi) = (was.min(t.s), was.max(t.s));
            let bit = if t.back { t.bit.1 } else { t.bit.0 };
            let hit: Vec<usize> = (0..self.foes.len()).filter(|&i| {
                let f = &self.foes[i];
                f.alive() && bit & (1 << (i % 64)) == 0 && f.s + f.breed.half >= lo && f.s - f.breed.half <= hi
            }).collect();
            for i in hit {
                if t.back { t.bit.1 |= 1 << (i % 64); } else { t.bit.0 |= 1 << (i % 64); }
                self.art_hit(i, spec.damage, spec.stance, out);
                if !t.back { t.back = true; break; }
            }
            if !t.back && (t.left <= 0.0 || t.s.abs() >= HALF) { t.back = true; }
            if t.back && (self.hero.s - t.s).abs() <= step {
                self.hatchets.remove(k);
                out.push(Event::Caught);
                continue;
            }
            self.hatchets[k] = t;
            k += 1;
        }
    }

    /// Foe `i`'s raise going off: the risen out of the ground either side of it, as many as the
    /// foe objects have room for and no more than `MOST_RISEN` standing.
    fn raise_risen(&mut self, i: usize, out: &mut Vec<Event>) {
        let c = &self.foes[i];
        let (s, facing, ratio, power, variant) = (c.s, c.facing, c.top / c.breed.hp, c.power, c.variant);
        let standing = self.foes.iter().filter(|f| std::ptr::eq(f.breed, &RISEN) && f.alive()).count();
        for side in [1.0_f32, -1.0].into_iter().take(MOST_RISEN.saturating_sub(standing)) {
            if self.foes.len() >= crate::world::MOST_FOES { break; }
            let at = (s + facing * side * 1.6).clamp(-HALF + RISEN.half, HALF - RISEN.half);
            out.push(Event::Raised { enemy: self.foes.len() });
            self.foes.push(Foe::new(&RISEN, at, facing, RISE_WAIT).scaled((ratio, power)).of(variant));
        }
    }

    /// Foe `i`'s blow live now: does it meet the hero, and how? True once it has struck (one hit a
    /// blow).
    fn blow_lands(&mut self, i: usize, blow: Blow, out: &mut Vec<Event>) -> bool {
        let blow = Blow { damage: blow.damage * self.foes[i].power, ..blow };
        let b = &self.foes[i];
        let h = &self.hero;
        let reaches = match blow.motion {
            Motion::Ring => b.ring().is_some_and(|(at, r)| ((h.s - at).abs() - r).abs() < RING_HALF + HERO_HALF),
            Motion::Charge(_) => (h.s - b.s).abs() < b.breed.half + HERO_HALF + 0.1,
            Motion::Strike | Motion::Leap => { let ahead = (h.s - b.s) * b.facing; ahead >= blow.reach.0 - HERO_HALF && ahead <= blow.reach.1 + HERO_HALF }
            Motion::Shot(_) => b.shot().is_some_and(|at| (h.s - at).abs() < SHOT_HALF + HERO_HALF),
            Motion::Rune => match b.ai { Ai::Blow { aim, .. } => h.s >= aim + blow.reach.0 - HERO_HALF && h.s <= aim + blow.reach.1 + HERO_HALF, _ => false },
            Motion::Raise => false,
        };
        let high_enough = h.y + 1.7 >= blow.height.0 && h.y <= blow.height.1;
        if !reaches || !high_enough || h.dead { return false; }
        if h.invulnerable() {
            if matches!(h.act, Some((Act::Roll(_), t)) if t <= PERFECT) { out.push(Event::JustDodge { enemy: i }); }
            return false;
        }
        let toward = if self.foes[i].s >= h.s { 1.0 } else { -1.0 };
        let facing_it = h.facing == toward;
        let h = &mut self.hero;
        // A parry, live (and the kit's extra ticks), meets a blow that can be parried.
        if let Some((Act::Parry, t)) = h.act {
            let f = Act::Parry.frames(&h.stats.kit);
            let live = t >= f.0 && t < f.0 + f.1 + h.stats.kit.parry;
            if live && blow.parry && facing_it {
                h.act = None;
                h.bulwark.parried(&h.stats.kit, blow.guard);
                h.mend();
                out.push(Event::Parried { enemy: i });
                let f = &mut self.foes[i];
                f.ai = Ai::Open(f.breed.open);
                out.push(Event::StanceBroken { enemy: i });
                return true;
            }
            let damage = blow.damage * 0.5 * h.stats.taken();
            h.hp -= damage;
            h.spend(blow.guard * 0.5);
            h.reel(Reel::Fumble);
            out.push(Event::ParryMissed { damage });
            return true;
        }
        if h.guarding && facing_it && blow.kind != Kind::Unblockable {
            let guarded = crate::fight::guard_blow(&h.stats, h.stamina, &mut h.bulwark, blow.guard, blow.damage, blow.kind == Kind::GuardBreak);
            if !matches!(guarded, Guarded::Broken { .. }) { self.thorns(i, blow.damage, out); }
            let h = &mut self.hero;
            match guarded {
                Guarded::Perfect { burst } => {
                    h.mend();
                    out.push(Event::PerfectBlock);
                    if let Some(w) = burst { self.ward_let(i, w, blow.damage, out); }
                }
                Guarded::Broken { cost, damage } => {
                    h.spend(cost);
                    h.hp -= damage;
                    h.reel(Reel::GuardBreak);
                    out.push(Event::GuardBroken { damage });
                }
                Guarded::Held { cost, chip, breaker } => {
                    h.spend(cost);
                    h.hp -= chip;
                    h.reel(if breaker { Reel::ChipJolt } else { Reel::BlockJolt });
                    out.push(Event::Blocked { chip });
                }
            }
            return true;
        }
        let damage = blow.damage * h.stats.taken();
        if h.stone > 0 {
            // Stone Skin: less gets through, and nothing knocks the hero about.
            let damage = damage * crate::arts::STONE_TAKEN;
            h.hp -= damage;
            h.spend(self.foes[i].variant.sap(damage));
            out.push(Event::Struck { damage, heavy: false });
            return true;
        }
        h.hp -= damage;
        // A blighted foe's blow saps stamina too.
        h.spend(self.foes[i].variant.sap(damage));
        h.poise -= damage * 1.3;
        h.poise_wait = 0;
        // Knocked away from the foe, and out of the air.
        h.s += -toward * if blow.kind == Kind::Physical { 0.4 } else { 0.9 };
        if h.airborne() { h.vy = h.vy.min(0.0); }
        // A low blow takes the legs out; a huge guard-breaker floors the hero too.
        if blow.height.1 < 1.0 || (blow.kind != Kind::Physical && blow.damage >= 28.0) {
            h.reel(Reel::Down);
            h.poise = h.stats.poise();
            out.push(Event::KnockedDown { damage });
        } else if h.poise <= 0.0 {
            h.reel(Reel::Stagger);
            h.poise = h.stats.poise();
            out.push(Event::Staggered { damage });
        } else {
            let heavy = blow.damage >= crate::fight::HEAVY_HIT;
            // An armoured swing (an ultra greatsword's) rides it out.
            let armoured = match h.act {
                Some((a @ (Act::Light | Act::Heavy), t)) => t < a.struck(&h.stats.kit).armour,
                Some((Act::Art, t)) => t < Act::art_spec(&h.stats.kit).armour,
                _ => false,
            };
            if !armoured { h.reel(if heavy { Reel::Recoil } else { Reel::Flinch }); }
            out.push(Event::Struck { damage, heavy });
        }
        true
    }
}

/// Ticks the hero lies floored before getting up starts (for the animation).
pub const FALL: u32 = DOWN_FALL;

#[cfg(test)]
mod tests {
    use super::*;

    fn fight() -> SideFight { SideFight::new(Stats::default(), 100.0, 100.0, 3) }
    fn idle() -> SideInput { SideInput::default() }
    fn took(e: &[Event]) -> bool { e.iter().any(|e| matches!(e, Event::Struck { .. } | Event::Staggered { .. } | Event::KnockedDown { .. })) }
    /// Tick until `until` holds (at most `n` ticks); whether it did.
    fn run(f: &mut SideFight, n: u32, input: SideInput, mut until: impl FnMut(&SideFight, &[Event]) -> bool) -> bool {
        for _ in 0..n { let e = f.tick(&input); if until(f, &e) { return true; } }
        false
    }
    /// The Warden throwing blow `i` at once, `gap` metres in front of the hero.
    fn throwing(i: usize, gap: f32) -> SideFight {
        let mut f = fight();
        f.foes[0].s = f.hero.s + gap;
        f.foes[0].ai = Ai::Blow { i, t: 0, hit: false, from: f.foes[0].s, aim: f.hero.s };
        f
    }

    /// How a bot plays a boss: mashing, hiding behind a guard, or reading every blow.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Bot { Masher, Turtle, Reader }

    /// What `bot` presses this tick against the first foe.
    fn bot_input(f: &SideFight, bot: Bot) -> SideInput {
        let (h, b) = (&f.hero, &f.foes[0]);
        let mut i = SideInput::default();
        let toward = if b.s >= h.s { 1.0 } else { -1.0 };
        let gap = (b.s - h.s).abs() - b.breed.half;
        let reach = Act::Light.struck(&h.stats.kit).reach;
        let walk = |i: &mut SideInput, dir: f32| if dir > 0.0 { i.right = true } else { i.left = true };
        let low = h.hp < 0.4 * h.stats.max_hp();
        let open = matches!(b.ai, Ai::Open(_));
        let recovering = b.blow().is_some_and(|(_, ph, _)| ph == crate::fight::Phase::Recover);
        match bot {
            Bot::Masher => {
                if low && h.flasks > 0 { i.flask = true; }
                else if gap > reach - 0.2 { walk(&mut i, toward); } else { i.light = true; }
            }
            Bot::Turtle => {
                if low && h.flasks > 0 && gap > 2.5 { i.flask = true; }
                else if gap > reach - 0.2 { walk(&mut i, toward); i.guard = true; }
                else if (open || recovering) && h.stamina > 25.0 { i.light = true; } else { i.guard = true; }
            }
            Bot::Reader => {
                if let (Some((bl, ph, _)), Ai::Blow { t, .. }) = (b.blow(), b.ai) {
                    if ph != crate::fight::Phase::Recover {
                        let to_live = bl.frames.0 as i32 - t as i32;
                        let (lo, hi, ..) = b.warning().unwrap();
                        let covers = h.s + 0.8 > lo && h.s - 0.8 < hi;
                        // Ticks until it reaches the hero: a jump needs about five to clear a low
                        // blow; a roll's i-frames start at once.
                        let jump = bl.height.1 < 1.0;
                        let k = match bl.motion {
                            Motion::Ring => match b.ring() {
                                Some((at, r)) => ((h.s - at).abs() - r - RING_HALF - HERO_HALF) / RING_V,
                                None => to_live as f32 + ((h.s - b.s - b.facing * 1.4).abs() - RING_HALF - HERO_HALF).max(0.0) / RING_V,
                            },
                            Motion::Shot(v) => match b.shot() {
                                Some(at) => ((at - h.s).abs() - SHOT_HALF - HERO_HALF) / v,
                                None => to_live as f32 + ((h.s - b.s - b.facing * SHOT_FROM).abs() - SHOT_HALF - HERO_HALF).max(0.0) / v,
                            },
                            _ => to_live as f32,
                        };
                        // Ticks a jump takes to rise past the blow's top.
                        let up = { let (v, g) = (JUMP_V, GRAVITY); (v - (v * v - 2.0 * g * bl.height.1).max(0.0).sqrt()) / g };
                        let soon = if jump { (up..=up + 2.0).contains(&k) } else { (0.5..=3.0).contains(&k) };
                        let parry_at = Act::Parry.frames(&h.stats.kit).0 as i32;
                        if bl.parry && bl.motion == Motion::Strike && covers && (parry_at - 2..=parry_at).contains(&to_live) && h.act.is_none() { i.parry = true; return i; }
                        if covers && soon && !(bl.parry && bl.motion == Motion::Strike) {
                            if jump { i.jump = true; } else { i.roll = true; walk(&mut i, if matches!(bl.motion, Motion::Charge(_) | Motion::Ring | Motion::Shot(_)) { toward } else { -toward }); }
                            return i;
                        }
                        // Out of the way of what is coming from afar: wait for it.
                        if gap > reach { return i; }
                        // Too close to a big swing that is not parried: back off.
                        if covers && !bl.parry { walk(&mut i, -toward); return i; }
                        if !covers && gap > reach - 0.2 { walk(&mut i, toward); }
                        return i;
                    }
                }
                if low && h.flasks > 0 && gap > 3.0 { i.flask = true; }
                else if gap > reach - 0.2 { walk(&mut i, toward); }
                else if h.stamina > 20.0 && (open || recovering || matches!(b.ai, Ai::Wait(n) if n > 18)) { i.light = true; }
            }
        }
        i
    }

    /// `bot` against `breed` as the world scales it at `tier`, the hero levelled for that tier:
    /// (won, seconds, hero health left of full, flasks drunk).
    fn bout(breed: &'static Breed, tier: u32, bot: Bot, seed: u32) -> (bool, f32, f32, u32) {
        let scale = crate::world::World::scale_at(tier, 0);
        let l = 2 * tier;
        let mut stats = Stats { vigor: l, endurance: l, might: l, ..Stats::default() };
        // As reinforced as a hero is expected to be by then.
        let (main, armour) = crate::world::World::forged_at(tier);
        (stats.kit.main, stats.kit.forged_armour) = (main, armour as f32 * crate::gear::ARMOUR_STEP);
        let mut f = SideFight::boss(stats, stats.max_hp(), stats.max_stamina(), 3, breed, scale);
        f.seed = seed;
        let mut n = 0;
        while n < 60 * 240 && !f.won() && !f.hero.dead {
            let i = bot_input(&f, bot);
            let was = f.hero.hp;
            let by = f.foes[0].blow().map(|b| b.0.name);
            f.tick(&i);
            if f.hero.hp < was && bot == Bot::Reader { HURT.with(|h| *h.borrow_mut().entry(by.unwrap_or("?")).or_insert(0.0) += was - f.hero.hp); }
            n += 1;
        }
        (f.won(), n as f32 / 60.0, f.hero.hp / f.hero.stats.max_hp(), 3 - f.hero.flasks)
    }
    thread_local! { static HURT: std::cell::RefCell<std::collections::BTreeMap<&'static str, f32>> = Default::default(); }

    /// Every boss against the three bots: `cargo test --release boss_bots -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn boss_bots() {
        let roster: [(&'static Breed, u32); 15] = [(&COLLECTOR, 0), (&ALPHA, 2), (&WARDEN, 1), (&SCREAMER, 1), (&AGENT, 3), (&CHIEF, 2),
            (&MAGISTRATE, 3), (&SENTINEL, 4), (&WRAITH, 4), (&BUTCHER, 5), (&PRIEST, 5), (&STALKER, 6), (&STATUE, 6), (&ARCHANGEL, 7), (&HUSK, 0)];
        for (breed, tier) in roster {
            let mut line = format!("{:<26} t{tier}", breed.name);
            for bot in [Bot::Masher, Bot::Turtle, Bot::Reader] {
                let runs: Vec<_> = (0..12u32).map(|k| bout(breed, tier, bot, 0x51DE + 977 * k)).collect();
                let wins = runs.iter().filter(|r| r.0).count();
                let secs = runs.iter().map(|r| r.1).sum::<f32>() / runs.len() as f32;
                let left = runs.iter().filter(|r| r.0).map(|r| r.2).sum::<f32>() / wins.max(1) as f32;
                let drunk = runs.iter().map(|r| r.3).sum::<u32>() as f32 / runs.len() as f32;
                line += &format!(" | {bot:?} {wins:>2}/12 {secs:>5.1}s hp{:>3.0}% fl{drunk:.1}", 100.0 * left);
            }
            println!("{line}");
            let hurt = HURT.with(|h| std::mem::take(&mut *h.borrow_mut()));
            let mut v: Vec<_> = hurt.into_iter().collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            println!("    reader hurt by: {}", v.iter().map(|(n, d)| format!("{n} {d:.0}")).collect::<Vec<_>>().join(", "));
        }
    }

    /// Each boss beats a player who mashes or hides behind a shield, falls to one who reads its
    /// blows, and still costs that player: (breed, tier, most of the reader's health it may leave).
    #[test]
    fn bosses_beat_a_masher_and_a_turtle_and_fall_to_a_reader_at_a_cost() {
        // The Collector's blows are mostly low: a jump timed well clears them, so the first boss
        // leaves a reader more than the others do.
        for (breed, tier, most) in [(&COLLECTOR, 0, 0.8), (&WARDEN, 1, 0.85), (&CHIEF, 2, 0.75), (&BUTCHER, 5, 0.75), (&STATUE, 6, 0.75)] {
            let runs = |bot| (0..12u32).map(|k| bout(breed, tier, bot, 0x51DE + 977 * k)).collect::<Vec<_>>();
            let wins = |r: &[(bool, f32, f32, u32)]| r.iter().filter(|r| r.0).count();
            let (mash, turtle, read) = (runs(Bot::Masher), runs(Bot::Turtle), runs(Bot::Reader));
            let left = read.iter().filter(|r| r.0).map(|r| r.2).sum::<f32>() / wins(&read).max(1) as f32;
            assert!(wins(&mash) <= 2 && wins(&turtle) <= 2 && wins(&read) >= 8 && left <= most,
                "{}: masher {}/12, turtle {}/12, reader {}/12 with {:.0}% left", breed.name, wins(&mash), wins(&turtle), wins(&read), 100.0 * left);
        }
    }

    /// The Collector squared up (`wait` ticks to go) a metre and a half in front of the hero.
    fn collector(wait: u32) -> SideFight {
        let mut f = SideFight::boss(Stats::default(), 1e6, 100.0, 3, &COLLECTOR, (1.0, 1.0));
        f.foes[0].s = f.hero.s + 1.5;
        f.foes[0].ai = Ai::Wait(wait);
        f
    }
    fn wound_up(e: &[Event]) -> Option<&'static str> { e.iter().find_map(|e| match e { Event::WindsUp { attack, .. } => Some(*attack), _ => None }) }

    #[test]
    fn a_boss_beaten_at_while_squared_up_answers_at_once() {
        let mut f = collector(400);
        let mut answer = None;
        run(&mut f, 300, SideInput { light: true, ..idle() }, |_, e| { answer = wound_up(e); answer.is_some() });
        assert_eq!(answer, Some("rap"));
        assert!(f.ticks < 200, "it waited {} ticks", f.ticks);
        // A road foe does not.
        let mut f = SideFight::ambush(Stats::default(), 1e6, 100.0, 3, &[(&HUSK, 1.5)]);
        f.foes[0].ai = Ai::Wait(400);
        f.hero.s = 0.0;
        assert!(!run(&mut f, 150, SideInput { light: true, ..idle() }, |_, e| wound_up(e).is_some()));
    }

    #[test]
    fn a_flask_raised_in_reach_brings_the_boss_on_and_a_guard_held_close_gets_grabbed() {
        let mut f = collector(400);
        let mut came = None;
        run(&mut f, 10, SideInput { flask: true, ..idle() }, |_, e| { came = wound_up(e); came.is_some() });
        assert!(came.is_some(), "it let the hero drink");
        let grabs = (0..20u32).filter(|&k| {
            let mut f = collector(0);
            f.seed = 0x51DE + 977 * k;
            let mut what = None;
            run(&mut f, 5, SideInput { guard: true, ..idle() }, |_, e| { what = wound_up(e); what.is_some() });
            what == Some("collar")
        }).count();
        assert!((6..=16).contains(&grabs), "{grabs} of 20 grabs at a guard held up close");
    }

    #[test]
    fn a_jump_rises_about_a_metre_and_a_half_and_floats_over_the_top() {
        let mut f = fight();
        f.tick(&SideInput { jump: true, ..idle() });
        let mut top = 0.0f32;
        let mut ticks = 0;
        while f.hero.airborne() && ticks < 200 { f.tick(&idle()); top = top.max(f.hero.y); ticks += 1; }
        assert!((1.3..1.8).contains(&top), "{top}");
        assert!((45..60).contains(&ticks), "{ticks} ticks up");
        assert!(f.hero.stamina < 100.0);
    }

    #[test]
    fn a_roll_with_a_direction_travels_and_passes_through_the_boss() {
        let mut f = fight();
        f.foes[0].s = f.hero.s + 1.5;
        f.foes[0].ai = Ai::Wait(500);
        let start = f.hero.s;
        f.tick(&SideInput { roll: true, right: true, ..idle() });
        run(&mut f, 60, SideInput { right: true, ..idle() }, |f, _| f.hero.act.is_none());
        assert!(f.hero.s > f.foes[0].s, "rolled through to the far side: {} vs boss {}", f.hero.s, f.foes[0].s);
        // Walking does not.
        let mut g = fight();
        g.foes[0].s = g.hero.s + 1.5;
        g.foes[0].ai = Ai::Wait(500);
        run(&mut g, 120, SideInput { right: true, ..idle() }, |_, _| false);
        assert!(g.hero.s < g.foes[0].s, "walked into its body and stopped");
        let _ = start;
    }

    #[test]
    fn a_sweep_passes_under_a_jump_and_hits_a_hero_on_the_ground() {
        let w = BLOWS[1].frames.0;
        let mut f = throwing(1, 2.0);
        run(&mut f, w - 12, idle(), |_, _| false);
        f.tick(&SideInput { jump: true, ..idle() });
        let hurt = run(&mut f, 40, idle(), |_, e| took(e));
        assert!(!hurt, "jumped it");
        let mut g = throwing(1, 2.0);
        assert!(run(&mut g, 80, idle(), |_, e| e.iter().any(|e| matches!(e, Event::KnockedDown { .. }))), "a sweep floors a hero standing on it");
    }

    #[test]
    fn the_toll_is_parried_and_opens_the_boss_to_heavy_damage() {
        let w = BLOWS[0].frames.0;
        let mut f = throwing(0, 2.0);
        run(&mut f, w - 6, idle(), |_, _| false);
        f.tick(&SideInput { parry: true, ..idle() });
        assert!(run(&mut f, 30, idle(), |_, e| e.contains(&Event::Parried { enemy: 0 })));
        assert!(matches!(f.foes[0].ai, Ai::Open(_)));
        let hp = f.foes[0].hp;
        f.tick(&SideInput { light: true, ..idle() });
        run(&mut f, 30, idle(), |f, _| f.foes[0].hp < hp);
        assert!(hp - f.foes[0].hp > 20.0, "an open boss takes a strike two and a half times over: {}", hp - f.foes[0].hp);
    }

    #[test]
    fn the_knell_rolls_out_and_a_roll_slips_it() {
        let w = BLOWS[2].frames.0;
        let mut f = throwing(2, 3.0);
        assert!(run(&mut f, 200, idle(), |_, e| e.contains(&Event::KnockedDown { damage: 28.0 })), "the ring floors a hero who stands");
        let mut g = throwing(2, 3.0);
        // Roll as the ring arrives: it reaches 3 - 1.4 = 1.6 m out about 14 ticks into its life.
        run(&mut g, w + 10, idle(), |_, _| false);
        g.tick(&SideInput { roll: true, left: true, ..idle() });
        let hurt = run(&mut g, 80, idle(), |_, e| took(e));
        assert!(!hurt, "rolled through the ring");
    }

    #[test]
    fn strikes_wear_the_boss_down_to_its_death() {
        let mut f = fight();
        f.foes[0].s = f.hero.s + 1.5;
        f.foes[0].ai = Ai::Wait(u32::MAX / 2);
        f.foes[0].hp = 15.0;
        f.tick(&SideInput { heavy: true, ..idle() });
        assert!(run(&mut f, 60, idle(), |_, e| e.contains(&Event::Felled { enemy: 0 })));
        assert!(f.won());
    }

    fn ambushed() -> SideFight {
        SideFight::ambush(Stats::default(), 100.0, 100.0, 3, &[(&HUSK, -3.0), (&LANCER, 3.0), (&WRETCH, 4.5)])
    }

    #[test]
    fn in_the_yard_a_bloater_bursts_where_it_fell_and_a_gravecaller_raises_the_risen() {
        // Felled beside the hero, it bursts on them; felled across the yard, it misses.
        for (at, near) in [(1.0, true), (5.0, false)] {
            let mut f = SideFight::ambush(Stats::default(), 100.0, 100.0, 3, &[(&BLOATER, at)]);
            (f.foes[0].hp, f.foes[0].ai) = (0.0, Ai::Dead);
            let mut popped = false;
            assert!(run(&mut f, 80, idle(), |_, e| { popped |= e.iter().any(|e| matches!(e, Event::Popped { .. })); popped }));
            assert_eq!(f.hero.hp < 100.0, near, "burst at {at}");
        }
        // The raise: two of the risen either side of it, and no more while they stand.
        let mut f = SideFight::ambush(Stats::default(), 100.0, 100.0, 3, &[(&GRAVECALLER, 3.0)]);
        f.foes[0].ai = Ai::Blow { i: 0, t: 0, hit: false, from: 3.0, aim: 0.0 };
        assert!(run(&mut f, 120, idle(), |f, _| f.foes.len() == 3));
        assert!(f.foes[1..].iter().all(|r| std::ptr::eq(r.breed, &RISEN) && r.alive()));
        f.foes[0].ai = Ai::Blow { i: 0, t: 0, hit: false, from: 3.0, aim: 0.0 };
        run(&mut f, 120, idle(), |_, _| false);
        assert_eq!(f.foes.len(), 3, "two standing already");
        assert!(!f.won());
    }

    #[test]
    fn an_ambush_never_has_two_foes_winding_up_at_once() {
        let mut f = ambushed();
        f.hero.hp = 1e9;
        let mut blows = 0;
        run(&mut f, 4000, idle(), |f, e| {
            blows += e.iter().filter(|e| matches!(e, Event::WindsUp { .. })).count();
            assert!(f.foes.iter().filter(|x| x.pressing()).count() <= 1, "two at once at tick {}", f.ticks);
            false
        });
        assert!(blows > 20, "they still fight: {blows} blows");
        let thrown: std::collections::HashSet<_> = (0..3).filter(|&i| f.foes[i].count > 0).collect();
        assert_eq!(thrown.len(), 3, "each takes its turn");
    }

    #[test]
    fn foes_close_in_without_walking_through_one_another() {
        let mut f = ambushed();
        f.hero.hp = 1e9;
        run(&mut f, 1500, idle(), |f, _| {
            let (l, w) = (&f.foes[1], &f.foes[2]);
            assert!(w.s - l.s > l.breed.half + w.breed.half, "the wretch stays behind the lancer: {} {} {:?} {:?}", l.s, w.s, l.ai, w.ai);
            false
        });
        // The hero is knocked about the yard; wherever they end up, the foes have closed in on them.
        assert!(f.foes.iter().all(|x| (x.s - f.hero.s).abs() < 4.0), "closed in on {}: {:?}", f.hero.s, f.foes.iter().map(|x| x.s).collect::<Vec<_>>());
    }

    #[test]
    fn the_yard_swings_the_road_weapons_quicker_and_shorter() {
        use crate::gear::{Arms, Guard, Off, SWORD};
        // The Wayfarer's sword keeps the numbers the yard was tuned with.
        let sword = Kit::default();
        assert_eq!(Act::Light.frames(&sword), (12, 4, 16));
        assert_eq!(Act::Heavy.frames(&sword), (28, 6, 26));
        assert!((Act::Light.strike_spec(Class::Sword).reach - 1.7).abs() < 1e-5);
        assert!((Act::Heavy.strike_spec(Class::Sword).reach - 2.0).abs() < 1e-5);
        assert!(Act::Light.strike_spec(Class::Spear).reach > Act::Light.strike_spec(Class::Sword).reach);
        // The parry is the offhand's: a buckler's flick, a kite shield's slower turn, a bare blade's deflect.
        let great = Kit { arms: Arms { class: Class::Greatsword, ..SWORD }, off: None, ..Kit::default() };
        let kite = Kit { off: Some(Off { guard: Guard::Kite, art: "kite_shield" }), ..Kit::default() };
        assert_eq!(Act::Parry.frames(&sword), Move::Parry.spec().frames);
        assert_eq!(Act::Parry.frames(&great), Move::Parry.spec_for(Class::Greatsword, None).frames);
        assert!(Act::Parry.frames(&kite).0 > Act::Parry.frames(&sword).0 && Act::Parry.frames(&great).1 < Act::Parry.frames(&sword).1);
    }

    #[test]
    fn a_heavy_cleaves_both_foes_in_reach_and_a_light_only_the_nearest() {
        let mk = || {
            let mut f = SideFight::ambush(Stats::default(), 100.0, 100.0, 3, &[(&HUSK, 0.9), (&HUSK, 1.9)]);
            f.hero.facing = 1.0;
            for x in &mut f.foes { x.ai = Ai::Wait(u32::MAX / 2); }
            f
        };
        let mut f = mk();
        f.tick(&SideInput { heavy: true, ..idle() });
        run(&mut f, 60, idle(), |f, _| f.hero.act.is_none());
        assert!(f.foes.iter().all(|x| x.hp < x.max_hp()), "{:?}", f.foes.iter().map(|x| x.hp).collect::<Vec<_>>());
        let mut g = mk();
        g.tick(&SideInput { light: true, ..idle() });
        run(&mut g, 60, idle(), |g, _| g.hero.act.is_none());
        assert!(g.foes[0].hp < g.foes[0].max_hp() && g.foes[1].hp == g.foes[1].max_hp());
    }

    #[test]
    fn an_ambush_is_won_when_the_last_foe_falls() {
        let mut f = ambushed();
        for x in &mut f.foes[..2] { x.ai = Ai::Dead; }
        assert!(!f.won());
        let w = &mut f.foes[2];
        (w.s, w.hp, w.ai) = (0.9, 1.0, Ai::Wait(u32::MAX / 2));
        f.hero.facing = 1.0;
        f.tick(&SideInput { light: true, ..idle() });
        assert!(run(&mut f, 60, idle(), |_, e| e.contains(&Event::Felled { enemy: 2 })));
        assert!(f.won());
    }

    #[test]
    fn the_boss_picks_blows_by_distance_and_always_warns_first() {
        let mut f = fight();
        f.hero.hp = 1e9;
        let mut names = std::collections::HashSet::new();
        let mut warned = 0;
        run(&mut f, 6000, idle(), |f, e| {
            for ev in e { match ev { Event::WindsUp { attack, .. } => { names.insert(*attack); } Event::Commits { .. } => warned += 1, _ => {} } }
            let _ = f;
            false
        });
        assert!(names.len() >= 3, "{names:?}");
        assert!(warned > 5);
    }
    /// `breed` throwing blow `i` at once, `gap` metres in front of the hero.
    fn boss_throwing(breed: &'static Breed, i: usize, gap: f32) -> SideFight {
        let mut f = SideFight::boss(Stats::default(), 1e6, 100.0, 3, breed, (1.0, 1.0));
        f.foes[0].s = f.hero.s + gap;
        f.foes[0].facing = -1.0;
        f.foes[0].ai = Ai::Blow { i, t: 0, hit: false, from: f.foes[0].s, aim: f.hero.s };
        f
    }

    #[test]
    fn a_draining_weapon_heals_and_an_opener_hits_the_open_harder() {
        let strike = |kit: crate::gear::Kit, open: bool| {
            let mut f = fight();
            f.hero.stats.kit = kit;
            f.hero.hp = 50.0;
            f.foes[0].s = f.hero.s + 1.0;
            if open { f.foes[0].ai = Ai::Open(100); }
            let hp = f.foes[0].hp;
            f.hit(0, Act::Light, &mut Vec::new());
            (hp - f.foes[0].hp, f.hero.hp)
        };
        let plain = crate::gear::Kit::default();
        let (dealt, healed) = strike(crate::gear::Kit { leech: 0.1, ..plain }, false);
        assert!((healed - (50.0 + 0.1 * dealt)).abs() < 1e-3, "{healed} after {dealt}");
        let (open_plain, _) = strike(plain, true);
        let (open_more, _) = strike(crate::gear::Kit { opener: 1.5, ..plain }, true);
        assert!((open_more / open_plain - 1.5).abs() < 1e-3);
    }

    #[test]
    fn every_breed_picks_and_follows_only_blows_it_has() {
        let all: [&Breed; 28] = [&WARDEN, &SCREAMER, &CHIEF, &MAGISTRATE, &WRAITH, &PRIEST, &STATUE, &ARCHANGEL,
            &COLLECTOR, &ALPHA, &AGENT, &SENTINEL, &BUTCHER, &STALKER, &HUSK, &LANCER, &WRETCH, &ARCHER, &HEXER, &LANCER, &WRETCH, &HUSK, &WARDEN,
            &BRUTE, &HOUND, &BLOATER, &GRAVECALLER, &RISEN];
        for b in all {
            for near in [false, true] { for wild in [false, true] { for r in 0..10 { assert!((b.pick)(near, r, wild) < b.blows.len(), "{} picks past its blows", b.name); } } }
            for bl in b.blows {
                if let Some(f) = bl.follow { assert!(f.to < b.blows.len() && f.at < b.blows[f.to].frames.0 + 1, "{}: {} follows nothing", b.name, bl.name); }
                assert!(bl.tell > 0.0 && bl.tell < 1.0, "{}", bl.name);
            }
            assert!(b.wake.is_none_or(|k| k < b.blows.len()), "{}", b.name);
        }
        // Every region boss has something to parry, something no shield holds, and something to
        // meet at range.
        for b in [&SCREAMER, &CHIEF, &MAGISTRATE, &WRAITH, &PRIEST, &STATUE, &ARCHANGEL] {
            assert!(b.blows.iter().any(|x| x.parry) || b.name == "The Wraith Lord", "{} has nothing to parry", b.name);
            assert!(b.blows.iter().any(|x| x.kind == Kind::Unblockable), "{} can be turtled", b.name);
            assert!(b.blows.iter().any(|x| matches!(x.motion, Motion::Shot(_) | Motion::Leap | Motion::Charge(_) | Motion::Ring)), "{} has no reach", b.name);
        }
    }

    #[test]
    fn every_boss_and_mini_boss_has_a_clip_for_each_blow() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/pixellab");
        for b in [&WARDEN, &SCREAMER, &CHIEF, &MAGISTRATE, &WRAITH, &PRIEST, &STATUE, &ARCHANGEL,
            &COLLECTOR, &ALPHA, &AGENT, &SENTINEL, &BUTCHER, &STALKER] {
            let (folder, _) = b.sprite.expect(b.name);
            for clip in ["idle", "walk", "stagger", "death"].into_iter().chain(b.blows.iter().map(|x| x.art)) {
                assert!(dir.join(folder).join(clip).join("000.png").is_file(), "{} has no {clip} clip in {folder}", b.name);
            }
        }
    }

    #[test]
    fn a_delayed_blow_punishes_a_parry_at_its_tell_and_a_late_one_catches_it() {
        let late = BLOWS[4];
        let tell = Foe::tell_at(&late);
        assert!(tell + 30 < late.frames.0, "it holds well past its tell");
        let parried = |at: u32| {
            let mut f = boss_throwing(&WARDEN, 4, 2.0);
            f.hero.facing = 1.0;
            run(&mut f, at, idle(), |_, _| false);
            f.tick(&SideInput { parry: true, ..idle() });
            let mut out = None;
            run(&mut f, late.frames.0 + 20, idle(), |_, e| {
                out = e.iter().find_map(|e| match e { Event::Parried { .. } => Some(true), Event::Struck { .. } | Event::KnockedDown { .. } | Event::Staggered { .. } | Event::ParryMissed { .. } => Some(false), _ => None });
                out.is_some()
            });
            out
        };
        // Parrying the moment it tells: the parry is over long before the hammer comes down.
        assert_eq!(parried(tell), Some(false), "parried too soon");
        // Parrying just before it lands catches it.
        let lead = Act::Parry.frames(&Stats::default().kit).0;
        assert_eq!(parried(late.frames.0 - lead - 1), Some(true), "a late parry catches it");
    }

    #[test]
    fn a_leap_follows_the_hero_until_its_tell_and_a_dodged_one_is_stuck() {
        let leap = SCREAMER_BLOWS[4];
        let tell = Foe::tell_at(&leap);
        let mut f = boss_throwing(&SCREAMER, 4, 4.5);
        // The hero walks right while it follows; at its tell the aim is where they stand.
        run(&mut f, tell - 1, SideInput { right: true, ..idle() }, |_, _| false);
        f.tick(&idle());
        let Ai::Blow { aim, .. } = f.foes[0].ai else { panic!("{:?}", f.foes[0].ai) };
        assert!((aim - f.hero.s).abs() < 0.2, "aimed at {aim}, the hero at {}", f.hero.s);
        // Standing on the aim: struck where it lands.
        assert!(run(&mut f, leap.frames.0 + 10, idle(), |_, e| took(e)), "the leap lands on the hero");
        assert!((f.foes[0].s - aim).abs() < 0.6, "it came down at {} for {aim}", f.foes[0].s);
        // Dodged after its tell: it lands on nothing and is stuck far longer.
        let mut g = boss_throwing(&SCREAMER, 4, 4.5);
        run(&mut g, tell + 2, idle(), |_, _| false);
        // Rolled out from under it, toward where it leapt from (behind, the wall is near).
        g.tick(&SideInput { roll: true, right: true, ..idle() });
        let mut t = 0;
        while matches!(g.foes[0].ai, Ai::Blow { i: 4, .. }) && t < 400 { g.tick(&idle()); t += 1; }
        assert!(t as u32 >= leap.frames.2 + leap.stuck - 20, "stuck only {t} ticks after the miss");
    }

    #[test]
    fn a_shot_flies_the_yard_a_low_one_is_jumped_and_a_high_one_blocked() {
        let chains = MAGISTRATE_BLOWS[2];
        // Thrown from afar, it reaches a hero who stands.
        let mut f = boss_throwing(&MAGISTRATE, 2, 7.0);
        assert!(run(&mut f, chains.frames.0 + chains.frames.1, idle(), |_, e| took(e)), "the chains reach the hero 7 m off");
        // Jumped as it arrives, it passes under.
        let mut g = boss_throwing(&MAGISTRATE, 2, 7.0);
        let mut hurt = false;
        for _ in 0..chains.frames.0 + chains.frames.1 {
            let near = g.foes[0].shot().is_some_and(|at| (at - g.hero.s).abs() < 1.4);
            let e = g.tick(&SideInput { jump: near && !g.hero.airborne(), ..idle() });
            hurt |= took(&e);
        }
        assert!(!hurt, "jumped the chains");
        // The hatchet is thrown high: a guard holds it.
        let hatchet = CHIEF_BLOWS[3];
        let mut h = boss_throwing(&CHIEF, 3, 6.0);
        h.hero.facing = 1.0;
        let blocked = run(&mut h, hatchet.frames.0 + hatchet.frames.1, SideInput { guard: true, ..idle() }, |_, e| e.iter().any(|e| matches!(e, Event::Blocked { .. })));
        assert!(blocked, "the hatchet is blocked");
    }

    #[test]
    fn in_the_yard_a_last_moment_block_on_the_bulwark_bursts_its_ward_into_the_boss() {
        let mut l = crate::gear::Loadout::starting();
        l.warden = true;
        let b = crate::gear::find("Warden's Bulwark");
        l.gain(b);
        assert!(l.wear(1, Some(b)));
        // The toll lands as its windup ends; the guard comes up four ticks before.
        let mut f = throwing(0, 2.0);
        f.hero.stats.kit = l.kit();
        f.hero.bulwark.ward = 50.0;
        let lands = BLOWS[0].frames.0;
        let mut seen = Vec::new();
        for t in 0..lands + 20 {
            seen.extend(f.tick(&SideInput { guard: t + 4 >= lands, ..idle() }));
            if seen.iter().any(|e| matches!(e, Event::WardBurst { .. })) { break; }
        }
        assert!(seen.contains(&Event::PerfectBlock) && seen.iter().any(|e| matches!(e, Event::WardBurst { enemy: 0, .. })), "{seen:?}");
        assert!(matches!(f.foes[0].ai, Ai::Open(_)), "{:?}", f.foes[0].ai);
        assert!(f.foes[0].hp < WARDEN.hp && f.hero.hp == f.hero.stats.max_hp(), "{} {}", f.foes[0].hp, f.hero.hp);
        assert_eq!(f.hero.bulwark.ward, 0.0);
    }

    /// The kite's ward in the yard: a perfect block bashes the boss open longer than a stagger.
    #[test]
    fn in_the_yard_a_kites_ward_bashes_the_boss_open_longer() {
        let mut l = crate::gear::Loadout::starting();
        l.warden = true;
        let k = crate::gear::find("Kite Shield");
        l.gain(k);
        assert!(l.wear(1, Some(k)));
        let mut f = throwing(0, 2.0);
        f.hero.stats.kit = l.kit();
        f.hero.bulwark.ward = 60.0;
        let lands = BLOWS[0].frames.0;
        let mut seen = Vec::new();
        for t in 0..lands + 20 {
            seen.extend(f.tick(&SideInput { guard: t + 4 >= lands, ..idle() }));
            if seen.iter().any(|e| matches!(e, Event::WardBurst { .. })) { break; }
        }
        assert!(seen.iter().any(|e| matches!(e, Event::WardBurst { enemy: 0, ward: crate::gear::Ward::Kite, .. })), "{seen:?}");
        assert!(matches!(f.foes[0].ai, Ai::Open(t) if t > f.foes[0].breed.open), "{:?}", f.foes[0].ai);
    }

    #[test]
    fn a_grab_goes_through_a_guard() {
        let mut f = boss_throwing(&CHIEF, 4, 1.2);
        f.hero.facing = 1.0;
        assert!(run(&mut f, 80, SideInput { guard: true, ..idle() }, |_, e| took(e)), "the throttle takes a guarding hero");
    }

    #[test]
    fn a_string_goes_on_only_sometimes() {
        let (mut on, mut stopped) = (0, 0);
        for seed in 0..40u32 {
            let mut f = boss_throwing(&CHIEF, 0, 1.6);
            f.seed = seed.wrapping_mul(2654435761);
            f.hero.hp = 1e9;
            let mut next = None;
            run(&mut f, 120, idle(), |f, _| { if !matches!(f.foes[0].ai, Ai::Blow { i: 0, .. }) { next = Some(f.foes[0].ai); true } else { false } });
            match next { Some(Ai::Blow { i: 1, .. }) => on += 1, _ => stopped += 1 }
        }
        assert!(on > 5 && stopped > 2, "went on {on}, stopped {stopped}");
    }

    #[test]
    fn past_half_its_health_a_boss_opens_its_second_phase_with_its_waking_blow() {
        let mut f = SideFight::boss(Stats::default(), 1e6, 100.0, 3, &MAGISTRATE, (1.0, 1.0));
        f.foes[0].hp = 0.4 * f.foes[0].top;
        let mut woke = None;
        run(&mut f, 200, idle(), |_, e| { woke = e.iter().find_map(|e| match e { Event::WindsUp { attack, .. } => Some(*attack), _ => None }); woke.is_some() });
        assert_eq!(woke, Some("contempt"));
        assert!(f.foes[0].woke);
    }

    #[test]
    fn an_archer_shoots_high_and_low_and_a_hexer_opens_a_rune_under_the_hero() {
        // The loose reaches a hero who stands 6 m off; the low arrow is jumped.
        let loose = ARCHER.blows[0];
        let mut f = boss_throwing(&ARCHER, 0, 6.0);
        assert!(run(&mut f, loose.frames.0 + loose.frames.1, idle(), |_, e| took(e)));
        // An arrow flies faster than the chains, so the jump goes up a little sooner.
        let mut g = boss_throwing(&ARCHER, 1, 6.0);
        let mut hurt = false;
        for _ in 0..loose.frames.0 + loose.frames.1 {
            let near = g.foes[0].shot().is_some_and(|at| (at - g.hero.s).abs() < 2.4);
            hurt |= took(&g.tick(&SideInput { jump: near && !g.hero.airborne(), ..idle() }));
        }
        assert!(!hurt, "jumped the low arrow");
        // The rune opens where the hero stood at its tell: it catches one who stays, not one who
        // rolls off it.
        let rune = HEXER.blows[1];
        let mut f = boss_throwing(&HEXER, 1, 5.0);
        assert!(run(&mut f, rune.frames.0 + rune.frames.1, SideInput { guard: true, ..idle() }, |_, e| took(e)), "no guard holds a rune");
        let mut g = boss_throwing(&HEXER, 1, 5.0);
        let tell = Foe::tell_at(&rune);
        let mut hurt = false;
        for t in 0..rune.frames.0 + rune.frames.1 + 4 {
            hurt |= took(&g.tick(&SideInput { roll: t == tell + 4, right: t == tell + 4, ..idle() }));
        }
        assert!(!hurt, "rolled off the rune");
    }

    #[test]
    fn an_arrow_crosses_the_yard_into_the_foe_and_a_bolt_cracks_its_stance() {
        use crate::gear::Ranged;
        for r in [Ranged::Bow, Ranged::Frost] {
            let mut f = fight();
            match r {
                Ranged::Frost => { f.hero.stats.kit.catalyst = crate::gear::ITEMS[crate::gear::find("Frostweaver's Focus")].catalyst; f.hero.stats.focus = 3; }
                r => { f.hero.stats.kit.ranged = Some(r); f.hero.quiver.arrows = 12; }
            }
            f.foes[0].ai = Ai::Wait(10_000);
            f.hero.facing = if f.foes[0].s > f.hero.s { 1.0 } else { -1.0 };
            let (hp, stance) = (f.foes[0].hp, f.foes[0].stance);
            let mut hit = false;
            for t in 0..200 {
                let e = f.tick(&SideInput { loose: t == 0 && r != Ranged::Frost, cast: t == 0 && r == Ranged::Frost, ..idle() });
                hit |= e.iter().any(|e| matches!(e, Event::Hit { enemy: 0, mv: Move::Loose }));
                if hit { break; }
            }
            assert!(hit && f.foes[0].hp < hp, "{r:?}");
            match r {
                Ranged::Bow => assert_eq!(f.hero.quiver.arrows, 11),
                _ => assert!(f.foes[0].stance <= stance - Move::loose_spec(r).stance + 0.5 && f.hero.charges < f.hero.stats.max_charges()),
            }
        }
    }


    /// Blood Imbue in the yard as on the road: health for five imbued shots, the next loosed one of them.
    #[test]
    fn blood_imbue_in_the_yard_pays_health_and_imbues_the_next_shots() {
        use crate::gear::Ranged;
        let mut f = fight();
        f.hero.stats.kit.ranged = Some(Ranged::BloodCrossbow);
        f.hero.quiver = crate::fight::Quiver::SETTING_OUT;
        f.foes.clear();
        let hp = f.hero.hp;
        let mut imbued = false;
        for t in 0..80 { imbued |= f.tick(&SideInput { cast: t == 0, ..idle() }).contains(&Event::Imbued); }
        assert!(imbued && (f.hero.hp - (hp - f.hero.stats.max_hp() * crate::fight::IMBUE_COST)).abs() < 1e-3, "{}", f.hero.hp);
        assert_eq!(f.hero.quiver.imbued, crate::fight::IMBUE_SHOTS);
        for t in 0..60 { f.tick(&SideInput { loose: t == 0, ..idle() }); if !f.bolts.is_empty() { break; } }
        assert!(f.bolts.first().is_some_and(|b| b.imbued && b.pierces()));
        assert_eq!((f.hero.quiver.imbued, f.hero.quiver.bolts), (crate::fight::IMBUE_SHOTS - 1, crate::fight::Quiver::SETTING_OUT.bolts - 1));
    }

    /// The Warden squaring up `gap` in front of a hero holding forged weapon `name`, the meter at `meter`.
    fn forged(name: &str, meter: f32, gap: f32) -> SideFight {
        let mut l = crate::gear::Loadout::starting();
        let i = crate::gear::find(name);
        l.gain(i);
        assert!(l.wear(0, Some(i)));
        let mut f = fight();
        f.hero.stats.kit = l.kit();
        (f.hero.stats.might, f.hero.stats.dexterity) = (10, 10);
        f.hero.art = meter;
        f.foes[0].s = f.hero.s + gap;
        f.foes[0].ai = Ai::Wait(10_000);
        f
    }
    fn art_events(f: &mut SideFight, n: u32) -> Vec<Event> {
        let mut ev = f.tick(&SideInput { art: true, ..idle() });
        for _ in 1..n { ev.extend(f.tick(&idle())); }
        ev
    }
    fn art_hits(ev: &[Event]) -> usize { ev.iter().filter(|e| matches!(e, Event::Hit { mv: Move::Art, .. })).count() }

    #[test]
    fn in_the_yard_an_art_waits_for_its_meter_and_the_toll_throws_the_boss_open() {
        let mut f = forged("Bell-Hammer", 0.0, 1.6);
        assert!(!art_events(&mut f, 5).contains(&Event::Began(Move::Art)));
        f.hero.art = 70.0;
        let ev = art_events(&mut f, 40);
        assert!(ev.contains(&Event::Art { art: crate::arts::WeaponArt::Toll }) && art_hits(&ev) == 1);
        assert_eq!(f.hero.art, 20.0);
        assert!(matches!(f.foes[0].ai, Ai::Open(_)), "{:?}", f.foes[0].ai);
    }

    #[test]
    fn in_the_yard_the_light_lance_dashes_through_the_boss() {
        let mut f = forged("Lance of Light", 100.0, 1.6);
        let ev = art_events(&mut f, 30);
        assert_eq!(art_hits(&ev), 1);
        assert!(f.hero.s > f.foes[0].s, "{} {}", f.hero.s, f.foes[0].s);
    }

    #[test]
    fn in_the_yard_the_hatchet_comes_back_to_the_hand() {
        let mut f = forged("Chieftain's Cleaver", 100.0, 4.0);
        let ev = art_events(&mut f, 120);
        assert_eq!(art_hits(&ev), 2);
        assert!(ev.contains(&Event::Caught) && f.hatchets.is_empty());
    }

    #[test]
    fn in_the_yard_stone_skin_shrugs_off_a_sweep_that_floors_anyone_else() {
        let mut f = throwing(1, 2.0);
        f.hero.stone = 300;
        let mut ev = Vec::new();
        assert!(run(&mut f, 80, idle(), |_, e| { ev.extend_from_slice(e); took(e) }));
        assert!(!ev.iter().any(|e| matches!(e, Event::KnockedDown { .. })) && f.hero.reeling == 0, "{ev:?}");
    }

    #[test]
    fn in_the_yard_the_wail_breaks_a_windup_and_sentence_falls_later() {
        let mut f = forged("Drowned Wail", 100.0, 3.0);
        f.foes[0].ai = Ai::Blow { i: 0, t: 0, hit: false, from: f.foes[0].s, aim: f.hero.s };
        art_events(&mut f, 22);
        assert!(matches!(f.foes[0].ai, Ai::Open(_)), "{:?}", f.foes[0].ai);
        let mut g = forged("Gavel of Sentence", 100.0, 4.0);
        let ev = art_events(&mut g, crate::arts::WeaponArt::Sentence.fires() + 1 + crate::arts::SENTENCE_DELAY);
        assert!(ev.contains(&Event::Sentenced { enemy: 0 }) && art_hits(&ev) == 1, "{ev:?}");
    }
}
