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
/// A jump: up at this speed (m/tick), pulled down by this (m/tick²); about a metre high and
/// 0.6 s in the air.
const JUMP_V: f32 = 7.0 / TPS;
const GRAVITY: f32 = 24.0 / (TPS * TPS);
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

/// What the player pressed this tick, and what is held.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SideInput {
    /// Held: walk left or right.
    pub left: bool,
    pub right: bool,
    /// Pressed: a roll (Space with a direction held, or the way faced), a jump, and the rest.
    pub roll: bool,
    pub jump: bool,
    pub light: bool,
    pub heavy: bool,
    pub parry: bool,
    pub flask: bool,
    /// An arrow or a bolt from what the other hand holds.
    pub loose: bool,
    /// Held.
    pub guard: bool,
}

/// The hero's arrow or frost bolt in flight across the yard: where it is, which way it goes,
/// how high, what loosed it, what it deals, metres left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideBolt { pub s: f32, pub dir: f32, pub y: f32, pub ranged: crate::gear::Ranged, pub damage: f32, pub stance: f32, pub left: f32, pub age: u32 }

/// What the hero is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act { Light, Heavy, Roll(i8), Parry, Flask, Loose }

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

    /// Windup, live and recovery ticks, with what `kit` holds (its weapon's class; the parry, its
    /// offhand).
    pub fn frames(self, kit: &Kit) -> (u32, u32, u32) {
        match self {
            Act::Light | Act::Heavy => self.strike_spec(kit.arms.class).frames,
            Act::Roll(_) => (0, ROLL_T, 0),
            Act::Parry | Act::Flask => self.as_move().spec_for(kit.arms.class, kit.off.map(|o| o.guard)).frames,
            Act::Loose => Move::loose_spec(kit.ranged.unwrap_or(crate::gear::Ranged::Bow)).frames,
        }
    }
    fn total(self, kit: &Kit) -> u32 { let f = self.frames(kit); f.0 + f.1 + f.2 }
    /// As the road fight costs and names it.
    fn as_move(self) -> Move {
        match self { Act::Light => Move::Light, Act::Heavy => Move::Heavy, Act::Roll(d) => Move::Step(d as i32), Act::Parry => Move::Parry, Act::Flask => Move::Flask, Act::Loose => Move::Loose }
    }
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
    /// Arrows in the quiver and frost charges, brought in from the road and taken back to it.
    pub arrows: u32,
    pub charges: f32,
    /// A Warden's ward, brought in from the road and taken back to it.
    pub bulwark: Bulwark,
}

impl SideHero {
    pub fn airborne(&self) -> bool { self.y > 0.0 }
    pub fn rising(&self) -> bool { self.vy > 0.0 }
    fn free(&self) -> bool { self.act.is_none() && self.reeling == 0 && !self.dead }
    fn invulnerable(&self) -> bool {
        matches!(self.act, Some((Act::Roll(_), t)) if (ROLL_IFRAMES.0..ROLL_IFRAMES.1).contains(&t))
            || (self.reel == Reel::Down && self.reeling < Reel::Down.ticks() - 8)
    }
    fn spend(&mut self, n: f32) { if n > 0.0 { self.stamina = (self.stamina - n).max(0.0); self.rested = 0; } }
    fn reel(&mut self, r: Reel) { self.reel = r; self.reeling = r.ticks(); self.act = None; }
    /// Whether what the other hand holds has a shot to give (as `fight::Hero::can_loose`).
    fn can_loose(&self) -> bool {
        use crate::gear::Ranged;
        match self.stats.kit.ranged {
            Some(Ranged::Bow) => self.arrows > 0,
            Some(Ranged::BloodBow) => self.hp > crate::fight::BLOOD_COST + 1.0,
            Some(Ranged::Catalyst) => self.charges >= 1.0,
            None => false,
        }
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
pub const BLOWS: [Blow; 6] = [
    // The bell-hammer brought down overhead: parry it. Enraged, the sweep may come straight after.
    blow("toll", (48, 6, 42), (0.3, 3.3), (0.0, 3.6), 34.0, Physical, true, 40.0, Motion::Strike).then(1, 12, 0),
    // Swung low across the ground: jump it (or block, paying for it).
    blow("sweep", (40, 8, 38), (-0.7, 3.7), (0.0, 0.75), 24.0, GuardBreak, false, 45.0, Motion::Strike),
    // The hammer rung on the stones: a ring rolls out both ways, low. Jump it or roll through it.
    blow("knell", (62, 66, 40), (0.0, 0.0), (0.0, 0.55), 28.0, Unblockable, false, 0.0, Motion::Ring),
    // A charge across the yard, shoulder first: roll through it.
    blow("charge", (34, 42, 38), (-0.95, 0.95), (0.0, 2.6), 30.0, GuardBreak, false, 50.0, Motion::Charge(8.5 / TPS)),
    // Second phase: the hammer held high far longer than the toll, then down. It tells early: a
    // parry at the tell is a parry too soon.
    blow("late toll", (84, 6, 44), (0.3, 3.3), (0.0, 3.6), 38.0, Physical, true, 44.0, Motion::Strike).art("toll").tell(0.4),
    // Its waking: the bell rung twice, both ways.
    blow("double knell", (40, 66, 20), (0.0, 0.0), (0.0, 0.55), 26.0, Unblockable, false, 0.0, Motion::Ring).art("knell").then(2, 30, 10),
];

/// The boss at the gate.
pub static WARDEN: Breed = Breed {
    name: "The Bell Warden", kind: None, hp: 520.0, stance: 110.0, half: 0.95, height: 3.2, blows: &BLOWS,
    pick: |near, r, wild| match (near, r, wild) {
        (true, 0..=2, true) => 4,
        (true, 0..=4, _) => 0, (true, 5..=8, _) => 1, (true, _, _) => 2,
        (false, 0..=5, _) => 3, (false, _, _) => 2,
    },
    near: 3.4, walk: 1.3, keep: 3.0, rest: 46, open: 150, enrages: true, power: 1.0, wake: Some(5), sprite: Some(("bell_warden", 3.4)),
};

/// A region boss on the Warden's frame and art (until each has its own): its own blows, how it
/// picks them, and its own pace.
#[allow(clippy::too_many_arguments)]
const fn boss(name: &'static str, hp: f32, stance: f32, rest: u32, power: f32, blows: &'static [Blow], pick: fn(bool, u32, bool) -> usize, near: f32, wake: Option<usize>, sprite: (&'static str, f32)) -> Breed {
    Breed { name, hp, stance, rest, power, blows, pick, near, wake, sprite: Some(sprite), ..WARDEN_LIKE }
}
const WARDEN_LIKE: Breed = Breed {
    name: "", kind: None, hp: 520.0, stance: 110.0, half: 0.95, height: 3.2, blows: &BLOWS,
    pick: |_, _, _| 0, near: 3.4, walk: 1.3, keep: 3.0, rest: 46, open: 150, enrages: true, power: 1.0, wake: None, sprite: None,
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
const CHIEF_BLOWS: [Blow; 6] = [
    blow("chop", (28, 5, 16), (0.3, 3.0), (0.0, 2.8), 22.0, Physical, true, 30.0, Motion::Strike).art("toll").then(1, 6, 8),
    blow("chop again", (20, 5, 18), (0.3, 3.0), (0.0, 2.8), 22.0, Physical, true, 30.0, Motion::Strike).art("toll").then(2, 0, 6),
    blow("headsman's drop", (70, 6, 46), (0.3, 3.2), (0.0, 3.4), 40.0, Physical, true, 46.0, Motion::Strike).art("toll").tell(0.35),
    blow("hatchet", (32, 80, 30), (0.0, 0.0), (0.6, 1.9), 18.0, Physical, false, 24.0, Motion::Shot(9.0 / TPS)).art("throw"),
    blow("throttle", (34, 6, 44), (0.2, 1.7), (0.0, 2.2), 36.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30),
    blow("bull rush", (30, 40, 40), (-0.95, 0.95), (0.0, 2.6), 30.0, GuardBreak, false, 50.0, Motion::Charge(9.0 / TPS)).art("charge"),
];
pub static CHIEF: Breed = boss("The Bandit Chief", 760.0, 130.0, 42, 1.3, &CHIEF_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=6, _) => 4, (true, 7, _) => 2, (true, _, true) => 2, (true, _, _) => 0, (false, 0..=5, _) => 3, (false, _, _) => 5 },
    3.2, Some(5), ("bandit_chief", 3.3));

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
pub static WRAITH: Breed = boss("The Wraith Lord", 980.0, 140.0, 38, 1.6, &WRAITH_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=7, _) => 4, (true, _, _) => 5, (false, 0..=5, _) => 1, (false, _, true) => 5, (false, _, _) => 1 },
    3.0, Some(1), ("wraith_lord", 3.8));

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
const STATUE_BLOWS: [Blow; 5] = [
    blow("stone blade", (80, 8, 50), (0.3, 3.8), (0.0, 3.8), 44.0, Physical, true, 50.0, Motion::Strike).art("toll").tell(0.45).then(1, 10, 4),
    blow("stone sweep", (50, 10, 50), (-0.9, 4.0), (0.0, 0.8), 34.0, GuardBreak, false, 56.0, Motion::Strike).art("sweep"),
    blow("stomp", (54, 70, 40), (0.0, 0.0), (0.0, 0.55), 30.0, Unblockable, false, 0.0, Motion::Ring).art("knell"),
    blow("falling statue", (70, 10, 40), (-2.0, 2.0), (0.0, 3.4), 46.0, GuardBreak, false, 60.0, Motion::Leap).art("leap").tell(0.5).stuck(90),
    blow("march", (40, 50, 50), (-1.1, 1.1), (0.0, 3.0), 36.0, GuardBreak, false, 56.0, Motion::Charge(6.5 / TPS)).art("charge"),
];
pub static STATUE: Breed = boss("The Statue Archangel", 1250.0, 170.0, 36, 2.0, &STATUE_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=7, _) => 1, (true, _, _) => 2, (false, 0..=4, _) => 3, (false, _, true) => 3, (false, _, _) => 4 },
    3.8, Some(2), ("statue_archangel", 4.4));

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
pub static ARCHANGEL: Breed = boss("The Archangel", 1500.0, 190.0, 32, 2.3, &ARCHANGEL_BLOWS,
    |near, r, wild| match (near, r, wild) { (true, 0..=4, _) => 0, (true, 5..=6, _) => 7, (true, 7, _) => 5, (true, _, true) => 5, (true, _, _) => 0, (false, 0..=4, _) => 3, (false, _, _) => 6 },
    3.4, Some(5), ("the_archangel", 4.0));

/// A mini-boss: a named road enemy, stronger, enraging, with blows and art of its own.
const fn elite(base: &Breed, name: &'static str, hp: f32, power: f32, blows: &'static [Blow], pick: fn(bool, u32, bool) -> usize, sprite: (&'static str, f32)) -> Breed {
    Breed { name, hp, stance: base.stance * 2.2, half: base.half * 1.2, height: base.height * 1.2, rest: base.rest * 3 / 4, enrages: true, power, blows, pick, sprite: Some(sprite), ..*base }
}
/// The husks' chop and sweep, as their elites keep them (indices 0 and 1 of theirs).
const HUSK_CHOP: Blow = blow("chop", (34, 5, 30), (0.2, 1.6), (0.0, 2.2), 16.0, Physical, true, 22.0, Motion::Strike);
const HUSK_SWEEP: Blow = blow("sweep", (30, 6, 28), (-0.3, 1.8), (0.0, 0.7), 14.0, GuardBreak, false, 26.0, Motion::Strike);
/// The Debt Collector: chop, sweep, and a hand on the collar for whoever turtles.
pub static COLLECTOR: Breed = elite(&HUSK_BASE, "The Debt Collector", 220.0, 1.4,
    &[HUSK_CHOP.art("toll").then(1, 8, 4), HUSK_SWEEP, blow("collar", (30, 6, 40), (0.1, 1.4), (0.0, 2.2), 24.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30)],
    |near, r, _| match (near, r) { (true, 0..=4) => 0, (true, 5..=7) => 2, (true, _) => 1, (false, _) => 0 }, ("debt_collector", 2.5));
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
/// The Butcher: a cleaver held up too long, and a hook for the turtle.
pub static BUTCHER: Breed = elite(&HUSK_BASE, "The Butcher", 420.0, 2.7,
    &[HUSK_CHOP.art("toll").then(2, 0, 5), HUSK_SWEEP,
      blow("cleave", (60, 6, 40), (0.2, 1.9), (0.0, 2.4), 24.0, Physical, true, 30.0, Motion::Strike).art("toll").tell(0.38),
      blow("meathook", (30, 6, 40), (0.1, 1.6), (0.0, 2.2), 26.0, Unblockable, false, 0.0, Motion::Strike).art("grab").stuck(30)],
    |near, r, _| match (near, r) { (true, 0..=3) => 0, (true, 4..=6) => 2, (true, 7..=8) => 3, (true, _) => 1, (false, _) => 0 }, ("the_butcher", 2.9));
/// The Void Stalker: leaps, and bolts of the void from afar.
pub static STALKER: Breed = elite(&WRETCH_BASE, "The Void Stalker", 420.0, 3.0,
    &[blow("rake", (18, 6, 18), (0.1, 1.3), (0.0, 1.6), 10.0, Physical, false, 16.0, Motion::Strike).art("sweep").then(2, 6, 5),
      blow("void bolt", (30, 90, 26), (0.0, 0.0), (0.4, 1.6), 14.0, Physical, false, 20.0, Motion::Shot(8.0 / TPS)).art("throw"),
      blow("void leap", (40, 8, 24), (-1.0, 1.0), (0.0, 1.8), 18.0, GuardBreak, false, 30.0, Motion::Leap).art("leap").tell(0.5).stuck(50)],
    |near, r, _| match (near, r) { (true, 0..=6) => 0, (true, _) => 2, (false, 0..=5) => 1, (false, _) => 2 }, ("void_stalker", 2.6));

/// The breed a road enemy fights as when it springs an ambush (shieldbearers do not).
pub fn breed_of(kind: Archetype) -> &'static Breed {
    match kind { Archetype::Lancer => &LANCER, Archetype::Wretch => &WRETCH, Archetype::Archer => &ARCHER, Archetype::Hexer => &HEXER, _ => &HUSK }
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
    pick: |_, r, _| match r { 0..=4 => 0, 5..=7 => 1, _ => 2 }, near: 1.8, walk: 0.9, keep: 1.3, rest: 70, open: 110, enrages: false, power: 1.0, wake: None, sprite: None,
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
    pick: |near, r, _| if near && r < 7 { 0 } else { 1 }, near: 2.6, walk: 1.1, keep: 2.2, rest: 64, open: 100, enrages: false, power: 1.0, wake: None, sprite: None,
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
    pick: |near, r, _| match (near, r) { (true, 0..=5) => 0, (true, _) => 1, (false, 0..=5) => 1, (false, _) => 2 }, near: 1.4, walk: 1.8, keep: 1.0, rest: 48, open: 90, enrages: false, power: 1.0, wake: None, sprite: None,
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
    pick: |near, r, _| match (near, r) { (true, 0..=5) => 2, (_, 0..=5) => 0, _ => 1 }, near: 1.6, walk: 1.0, keep: 6.0, rest: 60, open: 90, enrages: false, power: 1.0, wake: None, sprite: None,
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
    pick: |near, r, _| match (near, r) { (true, 0..=6) => 2, (_, 0..=5) => 0, _ => 1 }, near: 1.8, walk: 0.8, keep: 5.5, rest: 80, open: 100, enrages: false, power: 1.0, wake: None, sprite: None,
};

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
}

impl Foe {
    /// A foe of `breed` at `s`, facing `facing`, squaring up for `wait` ticks.
    pub fn new(breed: &'static Breed, s: f32, facing: f32, wait: u32) -> Foe {
        Foe { breed, s, facing, hp: breed.hp, stance: breed.stance, stance_wait: 0, ai: Ai::Wait(wait), hurt: 0, count: 0, down: 0, top: breed.hp, power: breed.power, woke: false }
    }
    /// As strong as the region it is met in makes it: (health, blows) times its breed's.
    pub fn scaled(mut self, (hp, power): (f32, f32)) -> Foe {
        self.hp *= hp;
        self.top *= hp;
        self.power *= power;
        self
    }
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
                arrows: crate::fight::QUIVER, charges: stats.max_charges(), bulwark: Bulwark::default(),
            },
            foes,
            bolts: Vec::new(),
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
        for i in 0..self.foes.len() { if self.foes[i].alive() { self.foe_tick(i, &mut out); } else { self.foes[i].down += 1; } }
        self.bolts_tick(&mut out);
        if self.hero.stats.kit.ranged == Some(crate::gear::Ranged::Bow) {
            let felled = out.iter().filter(|e| matches!(e, Event::Felled { .. })).count() as u32;
            self.hero.arrows = (self.hero.arrows + crate::fight::SALVAGE * felled).min(crate::fight::QUIVER);
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
        if input.roll || input.jump || input.light || input.heavy || input.parry || input.flask || input.loose { h.buffer = Some((*input, 0)); }
        else if let Some((b, age)) = h.buffer { h.buffer = (age < crate::fight::BUFFER).then_some((b, age + 1)); }
        let p = h.buffer.map(|b| b.0).unwrap_or_default();
        let pdir = (p.right as i8) - (p.left as i8);
        // What was pressed, if the hero is free to do it (in the air: only a strike).
        let want = if p.roll { Some(Act::Roll(if pdir != 0 { pdir } else if dir != 0 { dir } else { h.facing as i8 })) }
            else if p.flask { Some(Act::Flask) } else if p.parry { Some(Act::Parry) } else if p.loose { Some(Act::Loose) }
            else if p.heavy { Some(Act::Heavy) } else if p.light { Some(Act::Light) } else { None };
        if h.free() {
            if p.jump && !h.airborne() && !h.guarding {
                h.buffer = None;
                if h.stamina <= 0.0 { out.push(Event::Spent); } else { h.spend(h.stats.cost(Move::Backstep, JUMP_COST)); h.vy = JUMP_V; h.y = 1e-4; }
            } else if let Some(a) = want.filter(|a| !h.airborne() || matches!(a, Act::Light | Act::Heavy)) {
                h.buffer = None;
                let class = h.stats.kit.arms.class;
                let cost = h.stats.cost(a.as_move(), match a {
                    Act::Light | Act::Heavy => a.strike_spec(class).stamina,
                    Act::Roll(_) => 12.0,
                    Act::Loose => Move::loose_spec(h.stats.kit.ranged.unwrap_or(crate::gear::Ranged::Bow)).stamina,
                    _ => 0.0,
                });
                if cost > 0.0 && h.stamina <= 0.0 {
                    out.push(Event::Spent);
                } else if a == Act::Flask && h.flasks == 0 {
                } else if a == Act::Loose && !h.can_loose() {
                } else {
                    h.spend(cost);
                    if let Act::Roll(d) = a { h.facing = d as f32; }
                    h.act = Some((a, 0));
                    (h.from, h.landed) = (h.s, false);
                    out.push(Event::Began(a.as_move()));
                }
            } else if dir != 0 && !h.guarding {
                // Walking turns the hero the way they go.
                h.facing = dir as f32;
                h.s += dir as f32 * WALK;
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
            h.vy -= GRAVITY;
            if h.airborne() && h.act.is_none() && dir != 0 { h.facing = dir as f32; }
            if h.y <= 0.0 { (h.y, h.vy) = (0.0, 0.0); }
        }
        // Stamina back once it has gone unspent a while.
        h.rested = h.rested.saturating_add(1);
        if h.rested > 30 && h.act.is_none_or(|(a, _)| a == Act::Flask) {
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
                Act::Loose if t == f.0 => {
                    if let Some(r) = kit.ranged {
                        use crate::gear::Ranged;
                        match r {
                            Ranged::Bow => h.arrows = h.arrows.saturating_sub(1),
                            Ranged::BloodBow => h.hp = (h.hp - crate::fight::BLOOD_COST).max(1.0),
                            Ranged::Catalyst => h.charges = (h.charges - 1.0).max(0.0),
                        }
                        let spec = Move::loose_spec(r);
                        let bolt = SideBolt { s: h.s + h.facing * 0.4, dir: h.facing, y: h.y + 1.3, ranged: r, damage: spec.damage * h.stats.ranged_damage(r), stance: spec.stance, left: crate::fight::BOLT_RANGE, age: 0 };
                        self.bolts.push(bolt);
                        out.push(Event::Shot(r));
                    }
                }
                _ => {}
            }
            h.act = (t + 1 < a.total(&kit)).then_some((a, t + 1));
            if matches!(a, Act::Light | Act::Heavy) && t >= f.0 && t < f.0 + f.1 && !h.landed { self.strike(a, out); }
        }
        // The yard's walls; and the foes' bodies, which only a roll (or a jump over a small
        // one) passes through.
        let h = &mut self.hero;
        h.s = h.s.clamp(-HALF + HERO_HALF, HALF - HERO_HALF);
        let rolling = matches!(h.act, Some((Act::Roll(_), _)));
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
        let reach = a.strike_spec(self.hero.stats.kit.arms.class).reach;
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
    /// A perfect block's ward let go at foe `i`, at `w` of a full ward: it is thrown open, hurt
    /// by its share of `fight::BURST_DAMAGE`; foes near it are hurt by less and thrown back.
    fn ward_burst(&mut self, i: usize, w: f32, out: &mut Vec<Event>) {
        use crate::fight::{BURST_DAMAGE, BURST_REACH, BURST_SHOVE, BURST_SPLASH};
        let dealt = BURST_DAMAGE * w * self.hero.stats.damage();
        let (at, hero) = (self.foes[i].s, self.hero.s);
        out.push(Event::WardBurst { enemy: i, power: w });
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
        let Spec { damage, stance, .. } = a.strike_spec(h.stats.kit.arms.class);
        let b = &mut self.foes[i];
        let open = matches!(b.ai, Ai::Open(_));
        let dealt = (damage * h.stats.damage() * if open { OPEN_DAMAGE * h.stats.kit.opener } else { 1.0 }).min(b.hp.max(0.0));
        b.hp -= dealt;
        // A weapon that drinks gives some of it back.
        let h = &mut self.hero;
        h.hp = (h.hp + dealt * h.stats.kit.leech).min(h.stats.max_hp());
        // A blow landed up close fills the catalyst.
        if h.stats.kit.ranged == Some(crate::gear::Ranged::Catalyst) { h.charges = (h.charges + crate::fight::CHARGE_PER_HIT).min(h.stats.max_charges()); }
        let b = &mut self.foes[i];
        b.hurt = 10;
        b.stance_wait = 0;
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
            let (s, y, dir) = (b.s, b.y, b.dir);
            let reached = |f: &Foe| f.alive() && y < f.breed.height + f.lift() && y > f.lift() && ((f.s - s) * dir) <= f.breed.half && ((f.s - s) * dir) > -f.breed.half - step;
            let hit = (0..self.foes.len()).filter(|&i| reached(&self.foes[i])).min_by(|&a, &c| ((self.foes[a].s - s) * dir).total_cmp(&((self.foes[c].s - s) * dir)));
            if let Some(i) = hit {
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
        f.hp -= b.damage.min(f.hp.max(0.0));
        f.hurt = 10;
        f.stance_wait = 0;
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
        let hero_s = self.hero.s;
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
        b.stance_wait += 1;
        if b.stance_wait > 120 { b.stance = (b.stance + 0.4).min(b.breed.stance); }
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
                if n == 0 && busy {
                    // Its turn comes when the other's blow is over.
                    b.ai = Ai::Wait(10);
                } else if n == 0 {
                    let near = gap < b.breed.near;
                    let pick = self.roll(10);
                    let b = &mut self.foes[i];
                    let k = (b.breed.pick)(near, pick, b.enraged()).min(b.breed.blows.len() - 1);
                    b.count += 1;
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
                    match blow.motion { Motion::Shot(_) => out.push(Event::Loosed { enemy: i }), Motion::Rune => out.push(Event::Burst { enemy: i, at: aim as f64 }), _ => {} }
                }
                b.ai = Ai::Blow { i: k, t, hit, from, aim };
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
            match crate::fight::guard_blow(&h.stats, h.stamina, &mut h.bulwark, blow.guard, blow.damage, blow.kind == Kind::GuardBreak) {
                Guarded::Perfect { burst } => {
                    out.push(Event::PerfectBlock);
                    if let Some(w) = burst { self.ward_burst(i, w, out); }
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
        h.hp -= damage;
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
            let armoured = match h.act { Some((a @ (Act::Light | Act::Heavy), t)) => t < a.strike_spec(h.stats.kit.arms.class).armour, _ => false };
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

    #[test]
    fn a_jump_rises_about_a_metre_and_comes_down() {
        let mut f = fight();
        f.tick(&SideInput { jump: true, ..idle() });
        let mut top = 0.0f32;
        let mut ticks = 0;
        while f.hero.airborne() && ticks < 200 { f.tick(&idle()); top = top.max(f.hero.y); ticks += 1; }
        assert!((0.8..1.3).contains(&top), "{top}");
        assert!((25..50).contains(&ticks), "{ticks} ticks up");
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
        let all: [&Breed; 23] = [&WARDEN, &SCREAMER, &CHIEF, &MAGISTRATE, &WRAITH, &PRIEST, &STATUE, &ARCHANGEL,
            &COLLECTOR, &ALPHA, &AGENT, &SENTINEL, &BUTCHER, &STALKER, &HUSK, &LANCER, &WRETCH, &ARCHER, &HEXER, &LANCER, &WRETCH, &HUSK, &WARDEN];
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
        for r in [Ranged::Bow, Ranged::Catalyst] {
            let mut f = fight();
            f.hero.stats.kit.ranged = Some(r);
            f.foes[0].ai = Ai::Wait(10_000);
            f.hero.facing = if f.foes[0].s > f.hero.s { 1.0 } else { -1.0 };
            let (hp, stance) = (f.foes[0].hp, f.foes[0].stance);
            let mut hit = false;
            for t in 0..200 {
                let e = f.tick(&SideInput { loose: t == 0, ..idle() });
                hit |= e.iter().any(|e| matches!(e, Event::Hit { enemy: 0, mv: Move::Loose }));
                if hit { break; }
            }
            assert!(hit && f.foes[0].hp < hp, "{r:?}");
            match r {
                Ranged::Bow => assert_eq!(f.hero.arrows, crate::fight::QUIVER - 1),
                _ => assert!(f.foes[0].stance <= stance - Move::loose_spec(r).stance + 0.5 && f.hero.charges < f.hero.stats.max_charges()),
            }
        }
    }
}
