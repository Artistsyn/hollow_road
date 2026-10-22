//! The fight in road space: where the hero and the enemies stand along the road and across it,
//! and what every move does, tick by tick (60 a second). No drawing and no engine here, so it is
//! tested on its own; `game` reads it each tick to place sprites, paint warnings and drive the road.
//!
//! Positions along the road are metres walked (`WalkState::walked`, which never comes round):
//! something at `s` stands `s - walked` metres ahead of the camera. Across the road, `lane` -1, 0,
//! 1 is left, middle, right, `LANE_W` metres apart.
//!
//! Frame data starts from FromSoftware's (doubled from their 30 fps): a roll (every lane change)
//! is invulnerable for 20 ticks from its first, a parry is live for 10 after 6 of start-up, a normal enemy's stance is 15-65 and
//! comes back at 13 a second once it has gone a while unhit. Everything is a starting value to
//! tune in play.

/// How far ahead of the camera the hero stands, metres (the camera looks over their shoulder).
pub const HERO_D: f32 = 3.5;
/// How far left of their lane the hero is drawn, metres, so the lane they fight in stays in view.
pub const SHOULDER: f32 = -0.3;
/// Metres between lanes (the road is 4.8 m wide: three lanes with room to roll between them).
pub const LANE_W: f32 = 1.4;
/// Walking pace forward and stepping back, m/s.
pub const WALK: f32 = 2.2;
pub const BACK: f32 = 1.1;
/// Walking forward stops this far short of an enemy standing in the way, metres.
pub const ENGAGE: f32 = 2.3;
/// Outside a roll (the road putting the hero back in the middle at a fork) the hero eases
/// across at about this many ticks.
pub const LANE_TICKS: f32 = 9.0;
/// A roll carries the hero a lane across in this many ticks: off at once and most of the way in
/// the first half, settling at the end (a Souls roll's root motion is front-loaded).
pub const TRAVEL: u32 = 18;

pub const HP: f32 = 100.0;
pub const STAMINA: f32 = 100.0;
/// Stamina back per tick once `REGEN_DELAY` ticks have passed since it was last spent; a third
/// of that while guarding.
const REGEN: f32 = 55.0 / 60.0;
const REGEN_DELAY: u32 = 30;
/// A press waits this many ticks for the hero to be free, then is dropped. Only the last is kept.
pub const BUFFER: u32 = 18;
pub const FLASKS: u32 = 3;
/// A full quiver, and the arrows taken back from each foe felled while a bow is held.
pub const QUIVER: u32 = 12;
pub(crate) const SALVAGE: u32 = 2;
/// What a Bloodrunner's arrow costs in health.
pub const BLOOD_COST: f32 = 5.0;
/// Of a frost charge, what each melee strike landed fills: the catalyst is fed by fighting up close.
pub(crate) const CHARGE_PER_HIT: f32 = 0.34;
/// How far an arrow or a bolt flies before it falls, metres.
pub(crate) const BOLT_RANGE: f32 = 18.0;
const FLASK_HEAL: f32 = 40.0;
/// The Warden's bulwark (a shield whose kit has a `ward`, design doc §8.4): blocks fill a ward
/// up to `WARD_MAX`. A guard raised fresh this few ticks before a blow lands is a perfect block:
/// it costs nothing and lets nothing through, and with `WARD_BURST` or more stored it lets the
/// ward go, the attacker staggered and hurt by the ward's share of full and foes close by
/// thrown back.
pub const WARD_MAX: f32 = 100.0;
pub const WARD_BURST: f32 = 35.0;
pub const PERFECT_BLOCK: u32 = 8;
/// Ticks the guard must be down before a raise counts as fresh: a guard flicked up and down
/// again and again blocks perfectly no more often than one held up and lowered with thought.
pub(crate) const REARM: u32 = 20;
/// Ward a block fills for each point of stamina the blow asks of a guard (before the kit's share).
const WARD_PER_STAMINA: f32 = 0.7;
/// Out of a fight the ward runs down, once this many ticks pass without a block.
const WARD_HOLD: u32 = 240;
const WARD_FADE: f32 = 10.0 / 60.0;
/// What a full ward's burst hits its attacker for (less as less is stored), what it hits foes
/// close by for, of that, and how near they must be and how far they are thrown, metres.
pub(crate) const BURST_DAMAGE: f32 = 30.0;
pub(crate) const BURST_SPLASH: f32 = 0.4;
pub(crate) const BURST_REACH: f32 = 2.5;
pub(crate) const BURST_SHOVE: f32 = 1.5;
/// A roll or backstep that began this few ticks before a blow fell on the lane it left is
/// perfect: the attacker stumbles.
pub const PERFECT: u32 = 9;
/// How long a stumble leaves an enemy unable to act, ticks.
pub const STUMBLE: u32 = 70;
/// Damage and stance damage a stumbling enemy takes, times the usual.
const STUMBLE_HURT: f32 = 1.5;
/// Running pace, m/s, and its stamina per tick.
pub const RUN: f32 = 4.0;
const RUN_COST: f32 = 12.0 / 60.0;
/// An enemy follows the hero's lane through this much of its wind-up, then commits.
const COMMIT: f32 = 0.7;
/// Ticks an enemy takes to shift one lane while it aims.
const SHIFT: u32 = 16;
/// How long a broken stance leaves an enemy open, ticks.
const STAGGER: u32 = 150;
/// The hero's poise: what hits wear down (1.3 a point of damage) before one staggers them. It
/// comes back after `POISE_DELAY` ticks without a hit.
pub const POISE: f32 = 40.0;
const POISE_DELAY: u32 = 90;
const POISE_REGEN: f32 = 30.0 / 60.0;
/// A blow this hard is a heavy hit (a longer recoil), not a flinch.
pub const HEAVY_HIT: f32 = 20.0;
/// A guard-breaking blow this hard, taken unguarded, knocks the hero down.
const KNOCKDOWN: f32 = 28.0;

/// How the hero is reeling from what last met them, each read at a glance: what the body does,
/// for how long, and (in the game) the burst, the hitstop and the camera with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reel {
    None,
    /// A light blow: a short flinch.
    Flinch,
    /// A heavy blow: knocked back a step.
    Recoil,
    /// Poise broken: stumbling, open.
    Stagger,
    /// Floored by a huge blow: down, then up (unhurt while on the ground).
    Down,
    /// A guard that held completely.
    BlockJolt,
    /// A guard that held but let some through (a guard-breaking blow).
    ChipJolt,
    /// A guard smashed open.
    GuardBreak,
    /// A parry out of time: the buckler half-caught it.
    Fumble,
    /// A light strike turned by a shield: the blade bounces back.
    Bounce,
}

impl Reel {
    /// Ticks it lasts.
    pub fn ticks(self) -> u32 {
        match self {
            Reel::None => 0, Reel::Flinch => 14, Reel::Recoil => 30, Reel::Stagger => 50, Reel::Down => DOWN_FALL + DOWN_LIE + DOWN_RISE,
            Reel::BlockJolt => 10, Reel::ChipJolt => 18, Reel::GuardBreak => 56, Reel::Fumble => 34, Reel::Bounce => 18,
        }
    }
}
/// A knockdown's fall, the time on the ground, and getting up.
pub const DOWN_FALL: u32 = 26;
pub const DOWN_LIE: u32 = 30;
pub const DOWN_RISE: u32 = 40;

/// One tick's input, already read from whatever device. `forward`, `back`, `run` and `guard`
/// are held; the rest are presses this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    /// Walk at a run (out of a fight's reach; it costs stamina).
    pub run: bool,
    pub guard: bool,
    /// A roll one lane across: -1 left, 1 right, 0 none.
    pub step: i32,
    /// A backstep out of reach, staying in the lane.
    pub backstep: bool,
    pub light: bool,
    pub heavy: bool,
    pub parry: bool,
    pub flask: bool,
    /// An arrow or a bolt from what the other hand holds.
    pub loose: bool,
}

impl Input {
    /// The press this tick, if any (the last of them wins).
    fn press(&self) -> Option<Move> {
        let mut m = None;
        if self.step != 0 { m = Some(Move::Step(self.step.signum())); }
        if self.backstep { m = Some(Move::Backstep); }
        if self.flask { m = Some(Move::Flask); }
        if self.loose { m = Some(Move::Loose); }
        if self.parry { m = Some(Move::Parry); }
        if self.light { m = Some(Move::Light); }
        if self.heavy { m = Some(Move::Heavy); }
        m
    }
}

/// What the hero does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Light,
    Heavy,
    /// A roll one lane across (-1, 1): every lane change is the dodge, invulnerable while it
    /// travels, and perfect (the attacker stumbles) when it leaves a lane just before a blow.
    Step(i32),
    /// A quick step back down the road, staying in the lane: invulnerable for most of it.
    Backstep,
    Parry,
    Flask,
    /// The strike on an enemy whose stance is broken (a light or heavy in reach becomes this).
    Riposte,
    /// An arrow or a frost bolt from the bow or catalyst in the other hand (`gear::Ranged`).
    Loose,
}

/// A move's timing and price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    /// Ticks of wind-up, of being live, and of recovery.
    pub frames: (u32, u32, u32),
    pub stamina: f32,
    /// Ticks (from, to) the hero cannot be hurt.
    pub iframes: Option<(u32, u32)>,
    /// How far past the hero a strike reaches, metres; its damage and stance damage.
    pub reach: f32,
    pub damage: f32,
    pub stance: f32,
    /// Lanes either side it also sweeps (0: its own only; past 0 it strikes every enemy it
    /// reaches there, not just the nearest).
    pub wide: i32,
    /// Ticks from its start that a hit does not knock the hero out of it (still hurt; a stagger
    /// or knockdown still does). A heavy rides out a flinch anyway (`Move::armoured`).
    pub armour: u32,
}

impl Move {
    /// As swung with the Wayfarer's sword.
    pub fn spec(self) -> Spec { self.spec_for(crate::gear::Class::Sword, Some(crate::gear::Guard::Buckler)) }

    /// As swung with a weapon of `class` and `off` in the other hand: the light and heavy are the
    /// weapon's own, the parry the offhand's; the rest are the body's.
    pub fn spec_for(self, class: crate::gear::Class, off: Option<crate::gear::Guard>) -> Spec {
        use crate::gear::{Class, Guard};
        let s = |frames, stamina, iframes, reach, damage, stance| Spec { frames, stamina, iframes, reach, damage, stance, wide: 0, armour: 0 };
        match (self, class) {
            (Move::Light, Class::Thrust) => return s((10, 3, 14), 9.0, None, 2.9, 8.0, 5.0),
            (Move::Heavy, Class::Thrust) => return s((24, 5, 24), 22.0, None, 3.3, 20.0, 14.0),
            (Move::Light, Class::Spear) => return s((16, 4, 18), 13.0, None, 3.4, 10.0, 7.0),
            (Move::Heavy, Class::Spear) => return s((32, 6, 30), 30.0, None, 3.6, 24.0, 20.0),
            (Move::Light, Class::Axe) => return s((18, 5, 20), 15.0, None, 2.4, 13.0, 12.0),
            (Move::Heavy, Class::Axe) => return s((36, 7, 32), 34.0, None, 2.6, 34.0, 30.0),
            (Move::Light, Class::Greatsword) => return s((22, 6, 24), 20.0, None, 3.0, 16.0, 16.0),
            (Move::Heavy, Class::Greatsword) => return Spec { wide: 1, ..s((42, 8, 36), 40.0, None, 3.2, 42.0, 40.0) },
            // Swung by the whole body: a long windup to commit to, half the stamina bar for the
            // heavy, both sweeping the lanes beside; armoured from the first tick until it lands.
            (Move::Light, Class::Ultra) => return Spec { wide: 1, armour: 37, ..s((30, 7, 34), 30.0, None, 3.3, 26.0, 30.0) },
            (Move::Heavy, Class::Ultra) => return Spec { wide: 1, armour: 67, ..s((58, 9, 50), 50.0, None, 3.5, 66.0, 75.0) },
            _ => {}
        }
        match (self, off) {
            // No shield to flick: the blade turns the blow, in a tighter window.
            (Move::Parry, None) => return s((8, 6, 26), 0.0, None, 0.0, 0.0, 0.0),
            // A tall shield swings round slower and stays out a little longer.
            (Move::Parry, Some(Guard::Kite)) => return s((9, 9, 25), 0.0, None, 0.0, 0.0, 0.0),
            _ => {}
        }
        match self {
            Move::Light => s((14, 4, 16), 12.0, None, 2.6, 10.0, 8.0),
            Move::Heavy => s((30, 6, 28), 28.0, None, 2.9, 26.0, 22.0),
            // Elden Ring's light roll: 13 of 30 fps i-frames and 8 of recovery; here 20 and 13 of 60.
            Move::Step(_) => s((3, 18, 13), 12.0, Some((1, 21)), 0.0, 0.0, 0.0),
            Move::Backstep => s((2, 16, 6), 14.0, Some((2, 18)), 0.0, 0.0, 0.0),
            Move::Parry => s((6, 10, 22), 0.0, None, 0.0, 0.0, 0.0),
            Move::Flask => s((36, 1, 17), 0.0, None, 0.0, 0.0, 0.0),
            Move::Riposte => s((18, 4, 28), 0.0, Some((0, 50)), 2.9, 60.0, 0.0),
            Move::Loose => Move::loose_spec(crate::gear::Ranged::Bow),
        }
    }

    /// An arrow's or a bolt's timing and price, kept well short of melee's: a long draw with no
    /// armour (any hit cuts it off, the shot not spent), stamina that stays unspent while it
    /// draws, and about 40% of a sword's light in damage a second with almost none of its stance
    /// damage, so a foe is only opened from range with a frost bolt, which a charge pays for.
    /// None of it is raised at the smith, nothing it lands drinks or opens a riposte, and
    /// a shield raised in front catches it.
    pub fn loose_spec(r: crate::gear::Ranged) -> Spec {
        use crate::gear::Ranged;
        let s = |frames, stamina, damage, stance| Spec { frames, stamina, iframes: None, reach: BOLT_RANGE, damage, stance, wide: 0, armour: 0 };
        match r {
            Ranged::Bow => s((36, 2, 20), 14.0, 7.0, 3.0),
            Ranged::BloodBow => s((34, 2, 20), 10.0, 8.0, 3.0),
            Ranged::Catalyst => s((42, 3, 24), 8.0, 10.0, 16.0),
        }
    }
    fn strikes(self) -> bool { matches!(self, Move::Light | Move::Heavy | Move::Riposte) }
    /// The hero is not knocked out of it by a hit (still hurt): a heavy's swing.
    fn armoured(self, t: u32) -> bool { self == Move::Heavy && t < 36 }
}

/// Where a move or an enemy attack is in its frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Windup, Active, Recover }

fn phase(frames: (u32, u32, u32), t: u32) -> Phase {
    if t < frames.0 { Phase::Windup } else if t < frames.0 + frames.1 { Phase::Active } else { Phase::Recover }
}

/// A move under way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Act {
    pub mv: Move,
    /// Its timing and reach, as the weapon held when it began swings it.
    pub spec: Spec,
    /// Ticks since it began.
    pub t: u32,
    /// Something was struck already (one hit per strike).
    pub landed: bool,
    /// The lane the hero was in when it began (a perfect dodge is judged against it), and their
    /// place across the road then (a roll travels from it).
    pub from_lane: i32,
    pub from_x: f32,
}

impl Act {
    pub fn phase(&self) -> Phase { phase(self.spec.frames, self.t) }
    fn invulnerable(&self) -> bool { self.spec.iframes.is_some_and(|(a, b)| (a..b).contains(&self.t)) }
    fn total(&self) -> u32 { let (a, b, c) = self.spec.frames; a + b + c }
}

/// What an enemy attack is like to meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Blocked, parried or dodged.
    Physical,
    /// Blocked at a high stamina price; not parried.
    GuardBreak,
    /// Only dodged.
    Unblockable,
}

/// One of an enemy's attacks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attack {
    pub name: &'static str,
    pub frames: (u32, u32, u32),
    /// How far in front of the enemy it strikes, metres.
    pub reach: f32,
    /// The lanes it covers besides the enemy's own: 0 none, -1 or 1 one more that way.
    pub wide: i32,
    pub damage: f32,
    /// Stamina a guard pays to stop it.
    pub guard: f32,
    pub kind: Kind,
    pub parry: bool,
    /// The attack that follows straight on, without a rest (a combo's next blow).
    pub then: Option<usize>,
    /// Only ever a follow-up, never chosen to open with.
    pub follow: bool,
    /// The clip it is drawn with (its own name, or one it borrows).
    pub art: &'static str,
    /// How far through its windup it flashes its warning, and how far it follows the hero's lane.
    /// A delayed blow flashes early and goes on following: a roll at the flash is a roll too soon.
    pub tell: f32,
    pub lock: f32,
    /// Loosed rather than swung: it flies down the lane it was aimed at (`Missile`) and strikes
    /// whoever stands there when it arrives.
    pub shot: Option<Flight>,
    /// Lands on the ground where the hero stood when it locked on, not in front of the enemy: a
    /// rune or a burst, its patch marked on the road all through the windup.
    pub area: bool,
}

/// How something loosed flies: metres a second, whether it follows the hero's lane until it is
/// close (a hex does; an arrow flies straight), and its picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight { pub speed: f32, pub homing: bool, pub look: Look }

/// What a missile looks like in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look { Arrow, Hex }

/// The hero's arrow or frost bolt in flight down their lane, away from them: where it is along
/// the road (metres walked) and across it, what loosed it, what it deals, metres of flight left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bolt { pub s: f64, pub lane: i32, pub x: f32, pub ranged: crate::gear::Ranged, pub damage: f32, pub stance: f32, pub left: f32, pub age: u32 }

/// Something loosed down the road, in flight toward the hero: by enemy `from`, with what it
/// does when it arrives; where it is along the road and across it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile { pub from: usize, pub attack: Attack, pub s: f64, pub lane: i32, pub x: f32, pub age: u32 }

/// How near (metres in front of the hero) a hex stops following their lane.
const HOMING_UNTIL: f32 = 3.5;
/// Where an area attack's patch runs, metres either side of the spot it locked on.
pub const AREA_HALF: f32 = 1.1;

/// An attack with no follow-up, warning and locking on at the usual point.
const fn att(name: &'static str, frames: (u32, u32, u32), reach: f32, wide: i32, damage: f32, guard: f32, kind: Kind, parry: bool) -> Attack {
    Attack { name, frames, reach, wide, damage, guard, kind, parry, then: None, follow: false, art: name, tell: COMMIT, lock: COMMIT, shot: None, area: false }
}

impl Attack {
    const fn art(self, art: &'static str) -> Attack { Attack { art, ..self } }
    /// Warns at `tell` and follows the hero's lane until `lock` (fractions of its windup).
    const fn delayed(self, tell: f32, lock: f32) -> Attack { Attack { tell, lock, ..self } }
    const fn then(self, next: usize) -> Attack { Attack { then: Some(next), ..self } }
    const fn follow_up(self) -> Attack { Attack { follow: true, ..self } }
    const fn shot(self, speed: f32, homing: bool, look: Look) -> Attack { Attack { shot: Some(Flight { speed, homing, look }), ..self } }
    const fn area(self) -> Attack { Attack { area: true, ..self } }
}

/// The shieldbearer's bash back when its shield turns a light strike (its attack of that index).
const SHIELD_COUNTER: usize = 3;

/// The kinds of enemy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Archetype {
    /// Slow approach, an overhead chop and a sweep across two lanes: teaches dodge and parry.
    Husk,
    /// Keeps its distance and thrusts from out of the hero's reach: teaches stepping aside
    /// (the lane) against rolling, and closing in between thrusts.
    Lancer,
    /// A tower shield turns light strikes from the front: teaches heavies, parries and patience.
    Shieldbearer,
    /// Fast and frail, with a three-blow frenzy: teaches reading a combo to its end.
    Wretch,
    /// Hangs back and shoots down the lane, backing off as the hero comes: teaches stepping
    /// out of the lane at the loose, and closing in between draws. Close, it kicks and backs away.
    Archer,
    /// Hangs back and casts: a hex that follows the hero's lane until late, and a rune that
    /// opens under their feet. Teaches leaving the marked ground and rolling late; close, it
    /// throws them back.
    Hexer,
}

impl Archetype {
    pub const ALL: [Archetype; 6] = [Archetype::Husk, Archetype::Lancer, Archetype::Shieldbearer, Archetype::Wretch, Archetype::Archer, Archetype::Hexer];
    pub fn hp(self) -> f32 {
        match self { Archetype::Husk => 80.0, Archetype::Lancer => 70.0, Archetype::Shieldbearer => 110.0, Archetype::Wretch => 50.0, Archetype::Archer => 52.0, Archetype::Hexer => 58.0 }
    }
    pub fn stance(self) -> f32 {
        match self { Archetype::Husk => 30.0, Archetype::Lancer => 28.0, Archetype::Shieldbearer => 45.0, Archetype::Wretch => 18.0, Archetype::Archer => 16.0, Archetype::Hexer => 20.0 }
    }
    /// Approach pace, m/s, and how far off it notices the hero, metres.
    fn pace(self) -> (f32, f32) {
        match self { Archetype::Husk => (0.9, 9.0), Archetype::Lancer => (0.8, 11.0), Archetype::Shieldbearer => (0.7, 8.0), Archetype::Wretch => (1.6, 10.0), Archetype::Archer => (0.8, 17.0), Archetype::Hexer => (0.7, 15.0) }
    }
    /// Souls it leaves when felled.
    pub fn souls(self) -> u32 {
        match self { Archetype::Husk => 30, Archetype::Lancer => 40, Archetype::Shieldbearer => 60, Archetype::Wretch => 25, Archetype::Archer => 45, Archetype::Hexer => 55 }
    }
    /// Whether it turns light strikes aside while it stands guard (not mid-attack, not staggered).
    pub fn shielded(self) -> bool { self == Archetype::Shieldbearer }
    /// The gap it tries to hold, metres: the ranged kinds hang back and give ground as the hero
    /// comes (slower than the hero walks, so they can be caught); the rest close to strike.
    pub fn keep(self) -> Option<f32> {
        match self { Archetype::Archer => Some(9.0), Archetype::Hexer => Some(7.5), _ => None }
    }
    pub fn attacks(self) -> &'static [Attack] {
        const HUSK: [Attack; 3] = [
            att("chop", (42, 6, 40), 2.9, 0, 28.0, 30.0, Kind::Physical, true),
            att("sweep", (50, 8, 44), 2.8, 1, 20.0, 45.0, Kind::GuardBreak, false),
            // The cleaver held up, and held: it flashes early and follows the hero's lane almost
            // to the end. Roll late, or parry late.
            att("late chop", (74, 6, 42), 2.9, 0, 32.0, 34.0, Kind::Physical, true).art("chop").delayed(0.38, 0.9),
        ];
        // From well outside the hero's reach (a light reaches 2.6 m): step aside or close in.
        const LANCER: [Attack; 4] = [
            att("thrust", (34, 5, 36), 4.2, 0, 24.0, 28.0, Kind::Physical, true),
            att("lunge", (46, 6, 44), 5.0, 0, 30.0, 40.0, Kind::GuardBreak, false),
            // Looks like the thrust; a second comes straight after.
            att("thrust", (34, 5, 10), 4.2, 0, 24.0, 28.0, Kind::Physical, true).then(3),
            att("thrust again", (16, 5, 38), 4.2, 0, 22.0, 26.0, Kind::Physical, true).art("thrust").follow_up(),
        ];
        const SHIELDBEARER: [Attack; 4] = [
            att("bash", (30, 5, 34), 2.6, 0, 16.0, 25.0, Kind::Physical, true),
            att("smash", (52, 7, 46), 3.0, 0, 34.0, 50.0, Kind::GuardBreak, false),
            // Behind the shield at a run: it follows the hero's lane nearly to the end, and no
            // block holds it for long.
            att("shield rush", (44, 8, 46), 3.6, 0, 26.0, 55.0, Kind::GuardBreak, false).art("bash").delayed(0.6, 0.92),
            // Its shield turned a light strike: the bash back, quick (`SHIELD_COUNTER`).
            att("counter bash", (16, 5, 34), 2.6, 0, 18.0, 25.0, Kind::Physical, true).art("bash").follow_up(),
        ];
        // The frenzy: three rakes, each its own warning, the last the hardest; parry any one.
        const WRETCH: [Attack; 5] = [
            att("rake", (22, 4, 22), 2.4, 0, 12.0, 14.0, Kind::Physical, true),
            att("frenzy", (18, 4, 8), 2.4, 0, 10.0, 12.0, Kind::Physical, true).then(2),
            att("frenzy", (12, 4, 8), 2.4, 0, 10.0, 12.0, Kind::Physical, true).then(3).follow_up(),
            att("frenzy", (16, 5, 30), 2.6, 0, 16.0, 20.0, Kind::Physical, true).follow_up(),
            // From afar, onto the hero: it follows their lane until late. Roll late; a block pays.
            att("pounce", (40, 6, 36), 4.6, 0, 18.0, 30.0, Kind::GuardBreak, false).art("rake").delayed(0.5, 0.88),
        ];
        // The bow drawn, aimed down the hero's lane until late, and loosed: step out of the lane
        // as it looses; a shield turns an arrow cheaply, a parry cannot. The long draw glints
        // early and holds: roll late. Close, a kick, and it backs off.
        const ARCHER: [Attack; 5] = [
            att("loose", (52, 3, 30), 14.0, 0, 18.0, 16.0, Kind::Physical, false).delayed(0.6, 0.85).shot(16.0, false, Look::Arrow),
            att("aimed shot", (92, 3, 40), 16.0, 0, 32.0, 34.0, Kind::GuardBreak, false).art("loose").delayed(0.4, 0.93).shot(24.0, false, Look::Arrow),
            att("loose", (40, 3, 8), 14.0, 0, 16.0, 16.0, Kind::Physical, false).delayed(0.6, 0.85).shot(16.0, false, Look::Arrow).then(3),
            att("second arrow", (22, 3, 30), 14.0, 0, 16.0, 16.0, Kind::Physical, false).art("loose").delayed(0.6, 0.8).shot(16.0, false, Look::Arrow).follow_up(),
            att("kick", (24, 5, 30), 2.4, 0, 12.0, 14.0, Kind::Physical, true),
        ];
        // A hex that follows the hero's lane until it is nearly on them (roll at the last
        // moment; a shield pays dearly for it); a rune that opens under the hero's feet (its
        // ground glows the whole time: get off it); close, a burst that throws them back.
        const HEXER: [Attack; 3] = [
            att("hex", (60, 3, 40), 13.0, 0, 22.0, 34.0, Kind::GuardBreak, false).delayed(0.55, 0.8).shot(7.5, true, Look::Hex),
            att("rune", (78, 8, 44), 12.0, 0, 30.0, 0.0, Kind::Unblockable, false).delayed(0.35, 0.7).area(),
            att("repel", (30, 6, 40), 2.6, 1, 16.0, 30.0, Kind::GuardBreak, false),
        ];
        match self {
            Archetype::Husk => &HUSK, Archetype::Lancer => &LANCER, Archetype::Shieldbearer => &SHIELDBEARER, Archetype::Wretch => &WRETCH,
            Archetype::Archer => &ARCHER, Archetype::Hexer => &HEXER,
        }
    }
    /// Ticks it waits between attacks, at least and at most.
    fn rest(self) -> (u32, u32) {
        match self { Archetype::Husk => (50, 110), Archetype::Lancer => (60, 120), Archetype::Shieldbearer => (70, 130), Archetype::Wretch => (25, 60), Archetype::Archer => (50, 100), Archetype::Hexer => (70, 130) }
    }
}

/// What an enemy is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    /// Closing in or waiting `n` more ticks before its next attack.
    Wait(u32),
    /// Attack `i` of its archetype, `t` ticks in; `wide` is the side it covers this time; `aim`
    /// the road metre an area attack lands on (the hero's, followed until it locks).
    Attack { i: usize, t: u32, wide: i32, landed: bool, aim: f64 },
    /// Stance broken: open to a riposte for `n` more ticks.
    Staggered(u32),
    /// Thrown off balance by a perfect dodge for `n` more ticks: it cannot act, and strikes
    /// hurt it and its stance more (no riposte unless the stance breaks).
    Stumbled(u32),
    Dead,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enemy {
    pub kind: Archetype,
    /// Where it stands along the road (metres walked) and in which lane.
    pub s: f64,
    pub lane: i32,
    pub hp: f32,
    pub stance: f32,
    /// Ticks before its stance starts to come back.
    pub stance_wait: u32,
    pub ai: Ai,
    /// Ticks left of the flash of the last hit taken.
    pub hurt: u32,
    /// Its whole health, and what its attacks do against what they are drawn as: its kind's, and
    /// more deeper in the world (`scaled`).
    pub top: f32,
    pub power: f32,
}

impl Enemy {
    pub fn new(kind: Archetype, s: f64, lane: i32) -> Enemy {
        Enemy { kind, s, lane, hp: kind.hp(), stance: kind.stance(), stance_wait: 0, ai: Ai::Wait(30), hurt: 0, top: kind.hp(), power: 1.0 }
    }
    /// As strong as the region it is met in makes it: (health, attacks) times its kind's.
    pub fn scaled(mut self, (hp, power): (f32, f32)) -> Enemy {
        self.hp *= hp;
        self.top *= hp;
        self.power *= power;
        self
    }
    pub fn alive(&self) -> bool { self.ai != Ai::Dead }
    /// Metres ahead of the camera, as of `walked`.
    pub fn ahead(&self, walked: f64) -> f32 { (self.s - walked) as f32 }
    /// Metres between the hero and it (negative once it is behind the hero).
    pub fn gap(&self, walked: f64) -> f32 { self.ahead(walked) - HERO_D }
    /// The attack under way and where it is in it.
    pub fn attack(&self) -> Option<(Attack, Phase, f32)> {
        match self.ai {
            Ai::Attack { i, t, .. } => {
                let a = self.kind.attacks()[i];
                Some((a, phase(a.frames, t), (t as f32 / a.frames.0 as f32).min(1.0)))
            }
            _ => None,
        }
    }
    /// The lanes its attack under way covers.
    pub fn lanes(&self) -> (i32, i32) {
        match self.ai {
            Ai::Attack { wide, .. } if wide != 0 => { let o = (self.lane + wide).clamp(-1, 1); (self.lane.min(o), self.lane.max(o)) }
            _ => (self.lane, self.lane),
        }
    }
}

/// A patch of road an attack will strike: lanes (from, to), metres ahead (near, far).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Threat {
    pub enemy: usize,
    pub lanes: (i32, i32),
    pub d: (f32, f32),
    pub kind: Kind,
    /// Whether a parry can meet it.
    pub parry: bool,
    pub phase: Phase,
    /// How far through its wind-up, 0..1.
    pub progress: f32,
    /// A rune's patch, which opens under whoever stands on it (drawn as its circle).
    pub area: bool,
}

/// A bulwark's ward, and the timing of the guard that decides a perfect block (`guard_blow`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bulwark {
    pub ward: f32,
    /// Ticks the guard has been up (0 while it is down), and down (0 while it is up).
    up: u32,
    down: u32,
    /// The guard up now was raised after `REARM` ticks down, and has not yet blocked perfectly.
    fresh: bool,
    /// Ticks since the last block.
    idle: u32,
}

impl Bulwark {
    /// One tick of the guard, up or down; `calm` while no foe is near enough to fight.
    pub fn tick(&mut self, guarding: bool, calm: bool) {
        if guarding {
            if self.up == 0 { self.fresh = self.down >= REARM; }
            self.up = self.up.saturating_add(1);
            self.down = 0;
        } else {
            self.up = 0;
            self.down = self.down.saturating_add(1);
        }
        self.idle = self.idle.saturating_add(1);
        if calm && self.idle > WARD_HOLD { self.ward = (self.ward - WARD_FADE).max(0.0); }
    }
    /// The ward's share of full.
    pub fn share(&self) -> f32 { self.ward / WARD_MAX }
}

/// How a guard met a blow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Guarded {
    /// A bulwark raised fresh just before it landed: nothing spent, nothing through, and if the
    /// ward was let go, its share of full.
    Perfect { burst: Option<f32> },
    /// Held: this much stamina, and this much through (a blow built to break guards, `breaker`).
    Held { cost: f32, chip: f32, breaker: bool },
    /// Given way: the stamina it took, and half the blow through.
    Broken { cost: f32, damage: f32 },
}

/// How a guard with `stats`, `stamina` left and `bulwark` meets a blow that asks `guard` of it
/// and deals `damage`, `breaker` if it is built to break guards. A bulwark's ward fills from each
/// block, by what the blow asked rather than what the shield paid, and empties into a burst.
pub fn guard_blow(stats: &Stats, stamina: f32, bulwark: &mut Bulwark, guard: f32, damage: f32, breaker: bool) -> Guarded {
    let kit = stats.kit;
    let gain = guard * WARD_PER_STAMINA * kit.ward;
    if kit.ward > 0.0 { bulwark.idle = 0; }
    if kit.ward > 0.0 && bulwark.fresh && bulwark.up <= PERFECT_BLOCK {
        // Once a raise: a second blow on the same guard is blocked as any other.
        bulwark.fresh = false;
        if bulwark.ward >= WARD_BURST {
            let w = bulwark.share();
            bulwark.ward = 0.0;
            return Guarded::Perfect { burst: Some(w) };
        }
        bulwark.ward = (bulwark.ward + gain).min(WARD_MAX);
        return Guarded::Perfect { burst: None };
    }
    let cost = guard * kit.guard * if breaker { 1.5 } else { 1.0 };
    if cost > stamina { return Guarded::Broken { cost, damage: damage * 0.5 * stats.taken() }; }
    bulwark.ward = (bulwark.ward + gain).min(WARD_MAX);
    Guarded::Held { cost, chip: if breaker { damage * 0.15 * kit.chip * stats.taken() } else { 0.0 }, breaker }
}

/// What a tick did that the game should show or sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The hero began a move.
    Began(Move),
    /// Enemy `enemy` loosed an arrow or a hex.
    Loosed { enemy: usize },
    /// The hero loosed an arrow or cast a bolt.
    Shot(crate::gear::Ranged),
    /// The hero's arrow or bolt caught on enemy `enemy`'s raised shield.
    Glanced { enemy: usize },
    /// Enemy `enemy`'s rune opened, `at` metres along the road.
    Burst { enemy: usize, at: f64 },
    /// A strike landed on enemy `enemy`.
    Hit { enemy: usize, mv: Move },
    /// Its stance broke: it is open to a riposte.
    StanceBroken { enemy: usize },
    Felled { enemy: usize },
    /// A strike's live frames passed and struck nothing.
    Whiffed(Move),
    /// A light strike turned aside by enemy `enemy`'s shield: the hero is thrown back.
    Deflected { enemy: usize },
    /// An enemy began an attack.
    WindsUp { enemy: usize, attack: &'static str },
    /// Enemy `enemy`'s attack stopped following the hero's lane: from here it lands where its
    /// warning lies (the moment its weapon glints).
    Commits { enemy: usize, kind: Kind, parry: bool },
    /// The hero took a hit: a flinch, or a heavy one that knocks them back.
    Struck { damage: f32, heavy: bool },
    /// Hits wore the hero's poise through: they stumble, open.
    Staggered { damage: f32 },
    /// A huge guard-breaking blow floored the hero.
    KnockedDown { damage: f32 },
    /// A guard stopped a hit: completely (chip 0), or letting this much through.
    Blocked { chip: f32 },
    /// The guard gave way, and half the blow came through.
    GuardBroken { damage: f32 },
    Parried { enemy: usize },
    /// A bulwark met a blow raised at the last moment: nothing spent, nothing through.
    PerfectBlock,
    /// ... and let its ward go into enemy `enemy`, at `power` of a full ward.
    WardBurst { enemy: usize, power: f32 },
    /// A parry out of time: half the blow came through.
    ParryMissed { damage: f32 },
    /// A roll or backstep slipped enemy `enemy`'s blow at the last moment: it stumbles.
    JustDodge { enemy: usize },
    Healed,
    /// Wanted to move with no stamina.
    Spent,
    Died,
}

/// Levels bought with souls at a shrine, each over the start (design doc §8.3; Finesse, Insight
/// and Resolve come with the callings that use them).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Health: 8 a level.
    pub vigor: u32,
    /// Stamina: 6 a level.
    pub endurance: u32,
    /// The attributes a weapon asks for and scales with (`gear::Arms`): heavy weapons on
    /// might, quick ones on dexterity, rune-worked ones on focus.
    pub might: u32,
    pub dexterity: u32,
    pub focus: u32,
    /// The weapon's and the armour's upgrade at the smith (+0 to `GEAR_MAX`): 8% more strike
    /// damage, and 5% less damage taken, a step. Not levels: they do not raise a level's cost.
    pub weapon: u32,
    pub armour: u32,
    /// What everything worn does (`gear::Loadout::kit`).
    pub kit: crate::gear::Kit,
}

/// What a weapon held without the attributes it needs hits for, of itself, and what its strikes
/// then cost, of their usual.
pub const WEAK: f32 = 0.6;
pub const WEAK_STRIKE: f32 = 1.25;

/// The highest upgrade the smith can forge.
pub const GEAR_MAX: u32 = 5;

impl Stats {
    pub fn level(&self) -> u32 { self.vigor + self.endurance + self.might + self.dexterity + self.focus }
    /// Might, dexterity and focus, in the order a weapon names them.
    pub fn attributes(&self) -> [u32; 3] { [self.might, self.dexterity, self.focus] }
    /// What `level`s of an attribute add to a weapon graded B in it: 6% a level to 10, 3% to 20,
    /// 1% past that.
    pub fn soft(level: u32) -> f32 {
        let l = level as f32;
        0.06 * l.min(10.0) + 0.03 * (l - 10.0).clamp(0.0, 10.0) + 0.01 * (l - 20.0).max(0.0)
    }
    /// The attributes (by `gear::ATTRIBUTES` index) the weapon held needs more of.
    pub fn lacking(&self) -> Vec<usize> { (0..3).filter(|&i| self.attributes()[i] < self.kit.arms.needs[i]).collect() }
    /// Held without what it needs: it hits for `WEAK` of itself and its strikes cost more.
    pub fn weak(&self) -> bool { !self.lacking().is_empty() }
    /// The weapon's strike before the smith and the rest of the kit: its base, raised by each
    /// attribute it scales with.
    pub fn arms_damage(&self) -> f32 {
        let a = &self.kit.arms;
        let at = self.attributes();
        a.base * (1.0 + (0..3).map(|i| a.scales[i].share() * Self::soft(at[i])).sum::<f32>()) * if self.weak() { WEAK } else { 1.0 }
    }
    /// Souls the next level costs.
    pub fn level_cost(&self) -> u32 { 60 + 20 * self.level() }
    pub fn max_hp(&self) -> f32 { HP + 8.0 * self.vigor as f32 + self.kit.hp }
    pub fn max_stamina(&self) -> f32 { STAMINA + 6.0 * self.endurance as f32 + self.kit.stamina }
    /// Frost charges at full: three, and one more for every four levels of focus.
    pub fn max_charges(&self) -> f32 { 3.0 + (self.focus / 4) as f32 }
    /// What an arrow or a bolt hits for, of its own: raised by dexterity (a bow) or focus (a
    /// catalyst) as a B-grade weapon is, and by what is worn; never by the smith.
    pub fn ranged_damage(&self, r: crate::gear::Ranged) -> f32 {
        let at = match r { crate::gear::Ranged::Catalyst => self.focus, _ => self.dexterity };
        (1.0 + crate::gear::Grade::B.share() * Self::soft(at)) * self.kit.damage
    }
    pub fn damage(&self) -> f32 { self.arms_damage() * (1.0 + 0.08 * self.weapon as f32) * self.kit.damage }
    /// What is left of a blow after the armour.
    pub fn taken(&self) -> f32 { (1.0 - 0.05 * self.armour as f32) * self.kit.taken }
    /// Poise at full: hits' worth before a stagger.
    pub fn poise(&self) -> f32 { POISE + self.kit.poise }
    /// Weight the hero carries without slowing (`burdened` past 70% of it): endurance raises it.
    pub fn capacity(&self) -> f32 { 25.0 + 2.5 * self.endurance as f32 }
    pub fn burdened(&self) -> bool { self.kit.load > 0.7 * self.capacity() }
    /// What a move's stamina costs, as worn: rolls dearer when burdened.
    pub fn cost(&self, m: Move, base: f32) -> f32 {
        match m {
            Move::Step(_) | Move::Backstep => base * self.kit.roll * if self.burdened() { 1.4 } else { 1.0 },
            Move::Light | Move::Heavy => base * self.kit.strike * if self.weak() { WEAK_STRIKE } else { 1.0 },
            _ => base,
        }
    }
    /// How fast stamina comes back, against the unencumbered rate.
    pub fn regen(&self) -> f32 { self.kit.regen * if self.burdened() { 0.85 } else { 1.0 } }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hero {
    pub stats: Stats,
    pub hp: f32,
    pub stamina: f32,
    /// Ticks since stamina was last spent.
    pub rested: u32,
    pub flasks: u32,
    /// The lane the hero is in, and their place across the road (easing toward it), metres.
    pub lane: i32,
    pub x: f32,
    pub act: Option<Act>,
    pub guarding: bool,
    /// Ticks left reeling from a hit, unable to act, and how (with how long it lasts in all).
    pub reeling: u32,
    pub reel: Reel,
    pub poise: f32,
    /// Ticks since the last hit wore poise down.
    pub poise_wait: u32,
    /// The last press, and how many ticks it has waited.
    pub buffer: Option<(Move, u32)>,
    pub dead: bool,
    /// Arrows in the quiver, and frost charges (whole ones cast).
    pub arrows: u32,
    pub charges: f32,
    /// A Warden's ward, filled by blocking (`Bulwark`).
    pub bulwark: Bulwark,
}

impl Hero {
    fn new() -> Hero { Hero::with(Stats::default()) }
    /// Whole, with these levels: as they rise from a shrine.
    pub fn with(stats: Stats) -> Hero {
        Hero { stats, hp: stats.max_hp(), stamina: stats.max_stamina(), rested: REGEN_DELAY, flasks: FLASKS, lane: 0, x: 0.0, act: None, guarding: false, reeling: 0, reel: Reel::None, poise: stats.poise(), poise_wait: 0, buffer: None, dead: false, arrows: QUIVER, charges: stats.max_charges(), bulwark: Bulwark::default() }
    }
    fn free(&self) -> bool { self.act.is_none() && self.reeling == 0 && !self.dead }
    /// Whether what the other hand holds has a shot to give: an arrow, the blood for one, a charge.
    pub fn can_loose(&self) -> bool {
        use crate::gear::Ranged;
        match self.stats.kit.ranged {
            Some(Ranged::Bow) => self.arrows > 0,
            Some(Ranged::BloodBow) => self.hp > BLOOD_COST + 1.0,
            Some(Ranged::Catalyst) => self.charges >= 1.0,
            None => false,
        }
    }
    /// Start reeling `r`'s way, whatever was under way cut off.
    fn reel(&mut self, r: Reel) { self.reel = r; self.reeling = r.ticks(); self.act = None; }
    /// On the ground after a knockdown (past the first moment of the fall): blows pass over.
    pub(crate) fn floored(&self) -> bool { self.reel == Reel::Down && self.reeling > 0 && self.reeling < Reel::Down.ticks() - 8 }
    fn spend(&mut self, n: f32) { if n > 0.0 { self.stamina = (self.stamina - n).max(0.0); self.rested = 0; } }
    /// The lane the hero is in for being struck: where they are now on the way across.
    fn standing_lane(&self) -> i32 { (self.x / LANE_W).round() as i32 }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fight {
    pub hero: Hero,
    pub enemies: Vec<Enemy>,
    /// Arrows and hexes in flight.
    pub missiles: Vec<Missile>,
    /// The hero's arrows and bolts in flight.
    pub bolts: Vec<Bolt>,
    rng: u64,
}

impl Fight {
    pub fn new(enemies: Vec<Enemy>) -> Fight { Fight { hero: Hero::new(), enemies, missiles: Vec::new(), bolts: Vec::new(), rng: 0x9E37_79B9_7F4A_7C15 } }

    fn roll(&mut self, n: u32) -> u32 {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17;
        (self.rng % n.max(1) as u64) as u32
    }

    /// The nearest living enemy standing in the hero's way: ahead, close enough to block a step.
    fn in_the_way(&self, walked: f64) -> Option<usize> {
        self.enemies.iter().enumerate()
            .filter(|(_, e)| e.alive() && e.gap(walked) > -0.2)
            .min_by(|a, b| a.1.gap(walked).total_cmp(&b.1.gap(walked)))
            .map(|(i, _)| i)
    }

    /// No living enemy near enough to have noticed the hero.
    fn at_peace(&self, walked: f64) -> bool {
        !self.enemies.iter().any(|e| e.alive() && (-1.0..=e.kind.pace().1).contains(&e.gap(walked)))
    }

    /// How fast the road walks this tick, m/s: forward unless an enemy stands within `ENGAGE`,
    /// back when asked, half pace behind a guard, a quick hop back, and not at all mid-move.
    pub fn speed(&self, input: &Input, walked: f64) -> f32 {
        let h = &self.hero;
        if h.dead || h.reeling > 0 { return 0.0; }
        if let Some(a) = h.act {
            return if a.mv == Move::Backstep && a.t < 18 { -3.6 } else { 0.0 };
        }
        let k = if input.guard { 0.5 } else { 1.0 };
        if input.back { return -BACK * k; }
        if !input.forward { return 0.0; }
        match self.in_the_way(walked) {
            Some(i) if self.enemies[i].gap(walked) <= ENGAGE => 0.0,
            // A run only with stamina, behind no guard, and well short of the next enemy.
            Some(i) if input.run && k == 1.0 && h.stamina > 0.0 && self.enemies[i].gap(walked) > 4.0 => RUN,
            None if input.run && k == 1.0 && h.stamina > 0.0 => RUN,
            _ => WALK * k,
        }
    }

    /// Where every attack under way will strike.
    pub fn threats(&self, walked: f64) -> Vec<Threat> {
        let swung = self.enemies.iter().enumerate().filter_map(|(i, e)| {
            let (a, phase, progress) = e.attack()?;
            let d = e.ahead(walked);
            let d = match (e.ai, a.shot) {
                // A rune: the patch it will open on.
                (Ai::Attack { aim, .. }, _) if a.area => { let at = (aim - walked) as f32; (at - AREA_HALF, at + AREA_HALF) }
                // A bow drawn: the lane it will fly down, all the way to the hero. Once loosed, the
                // missile is the threat.
                (_, Some(_)) if phase == Phase::Windup => (HERO_D - 0.3, d),
                (_, Some(_)) => return None,
                _ => (d - a.reach, d),
            };
            Some(Threat { enemy: i, lanes: e.lanes(), d, kind: a.kind, parry: a.parry, phase, progress, area: a.area })
        });
        let flying = self.missiles.iter().map(|m| {
            Threat { enemy: m.from, lanes: (m.lane, m.lane), d: (HERO_D - 0.3, (m.s - walked) as f32), kind: m.attack.kind, parry: m.attack.parry, phase: Phase::Active, progress: 1.0, area: false }
        });
        swung.chain(flying).collect()
    }

    /// One tick at 60 Hz, with the road `walked` metres along as drawn.
    pub fn tick(&mut self, input: &Input, walked: f64) -> Vec<Event> {
        let mut out = Vec::new();
        for e in &mut self.enemies { e.hurt = e.hurt.saturating_sub(1); }
        if self.hero.dead { return out; }
        self.hero_tick(input, walked, &mut out);
        for i in 0..self.enemies.len() { self.enemy_tick(i, walked, &mut out); }
        self.missiles_tick(walked, &mut out);
        self.bolts_tick(&mut out);
        // Arrows pulled back out of the fallen.
        if self.hero.stats.kit.ranged == Some(crate::gear::Ranged::Bow) {
            let felled = out.iter().filter(|e| matches!(e, Event::Felled { .. })).count() as u32;
            self.hero.arrows = (self.hero.arrows + SALVAGE * felled).min(QUIVER);
        }
        if self.hero.hp <= 0.0 && !self.hero.dead {
            self.hero.dead = true;
            self.hero.act = None;
            out.push(Event::Died);
        }
        out
    }

    fn hero_tick(&mut self, input: &Input, walked: f64, out: &mut Vec<Event>) {
        let calm = self.at_peace(walked);
        let h = &mut self.hero;
        h.reeling = h.reeling.saturating_sub(1);
        if h.reeling == 0 { h.reel = Reel::None; }
        // Poise comes back once the hits stop.
        h.poise_wait = h.poise_wait.saturating_add(1);
        if h.poise_wait > POISE_DELAY { h.poise = (h.poise + POISE_REGEN).min(h.stats.poise()); }
        // Presses wait in the buffer for the hero to be free.
        if let Some(m) = input.press() { h.buffer = Some((m, 0)); }
        if let Some((m, age)) = h.buffer { h.buffer = (age < BUFFER).then_some((m, age + 1)); }
        h.guarding = input.guard && h.free();
        h.bulwark.tick(h.guarding, calm);
        if h.free() {
            if let Some((m, _)) = h.buffer {
                let m = self.riposte_for(m, walked).unwrap_or(m);
                let kit = self.hero.stats.kit;
                let spec = match (m, kit.ranged) { (Move::Loose, Some(r)) => Move::loose_spec(r), _ => m.spec_for(kit.arms.class, kit.off.map(|o| o.guard)) };
                // Out of any fight a roll is only a change of lane (leaning at a fork): it is free.
                let cost = if matches!(m, Move::Step(_)) && self.at_peace(walked) { 0.0 } else { self.hero.stats.cost(m, spec.stamina) };
                let h = &mut self.hero;
                if cost > 0.0 && h.stamina <= 0.0 {
                    out.push(Event::Spent);
                    h.buffer = None;
                } else if m == Move::Flask && h.flasks == 0 {
                    h.buffer = None;
                } else if m == Move::Loose && !h.can_loose() {
                    h.buffer = None;
                } else if matches!(m, Move::Step(dir) if !(-1..=1).contains(&(h.lane + dir))) {
                    // No rolling off the road's edge.
                    h.buffer = None;
                } else {
                    h.buffer = None;
                    h.spend(cost);
                    let from = h.lane;
                    if let Move::Step(dir) = m { h.lane += dir; }
                    h.act = Some(Act { mv: m, spec, t: 0, landed: false, from_lane: from, from_x: h.x });
                    h.guarding = false;
                    out.push(Event::Began(m));
                }
            }
        }
        // A run spends stamina while it lasts.
        let h = &mut self.hero;
        if input.run && h.act.is_none() && h.stamina > 0.0 && self.speed(input, walked) > WALK {
            let h = &mut self.hero;
            h.stamina = (h.stamina - RUN_COST).max(0.0);
            h.rested = 0;
        }
        self.hero_rest(walked, out);
    }

    /// The rest of the hero's tick: across to the lane, stamina back, the move under way.
    fn hero_rest(&mut self, walked: f64, out: &mut Vec<Event>) {
        let h = &mut self.hero;
        // Across to the lane: a roll on its own ease, the same every time, so the drawing and the
        // travel agree; otherwise an ease toward it.
        let target = h.lane as f32 * LANE_W;
        match h.act {
            Some(a) if matches!(a.mv, Move::Step(_)) => {
                let k = ((a.t + 1) as f32 / TRAVEL as f32).min(1.0).powf(0.75);
                h.x = a.from_x + (target - a.from_x) * k * k * (3.0 - 2.0 * k);
            }
            _ => {
                h.x += (target - h.x) / LANE_TICKS.max(1.0) * 2.0;
                if (target - h.x).abs() < 0.01 { h.x = target; }
            }
        }
        // Stamina comes back once it has gone unspent a while, slower behind a guard.
        h.rested = h.rested.saturating_add(1);
        if h.rested > REGEN_DELAY && h.act.is_none_or(|a| a.mv == Move::Flask) {
            let k = if h.guarding { 1.0 / 3.0 } else { 1.0 };
            h.stamina = (h.stamina + REGEN * k * h.stats.regen()).min(h.stats.max_stamina());
        }
        let Some(mut a) = h.act else { return };
        let spec = a.spec;
        if a.mv == Move::Flask && a.t == spec.frames.0 {
            h.flasks -= 1;
            h.hp = (h.hp + FLASK_HEAL * h.stats.kit.heal).min(h.stats.max_hp());
            out.push(Event::Healed);
        }
        // Loosed at the end of the draw, paid for then: a draw cut off spends nothing.
        if a.mv == Move::Loose && a.t == spec.frames.0 {
            if let Some(r) = h.stats.kit.ranged {
                use crate::gear::Ranged;
                match r {
                    Ranged::Bow => h.arrows = h.arrows.saturating_sub(1),
                    Ranged::BloodBow => h.hp = (h.hp - BLOOD_COST).max(1.0),
                    Ranged::Catalyst => h.charges = (h.charges - 1.0).max(0.0),
                }
                let damage = spec.damage * h.stats.ranged_damage(r);
                let bolt = Bolt { s: walked + HERO_D as f64 + 0.4, lane: h.lane, x: h.x, ranged: r, damage, stance: spec.stance, left: spec.reach, age: 0 };
                self.bolts.push(bolt);
                out.push(Event::Shot(r));
            }
        }
        let h = &mut self.hero;
        if a.mv.strikes() && a.phase() == Phase::Active && !a.landed {
            let lane = h.lane;
            let mut hit: Vec<usize> = self.enemies.iter().enumerate()
                .filter(|(_, e)| e.alive() && (e.lane - lane).abs() <= spec.wide && (-0.3..=spec.reach).contains(&e.gap(walked)))
                .map(|(i, _)| i).collect();
            // The nearest in its own lane; a sweep takes every one it reaches.
            hit.sort_by(|&x, &y| self.enemies[x].gap(walked).total_cmp(&self.enemies[y].gap(walked)));
            if spec.wide == 0 { hit.truncate(1); }
            if !hit.is_empty() { a.landed = true; }
            for i in hit { self.strike(i, a.mv, spec, out); }
        }
        a.t += 1;
        let (w, act, _) = spec.frames;
        if a.mv.strikes() && a.t == w + act && !a.landed { out.push(Event::Whiffed(a.mv)); }
        self.hero.act = (a.t < a.total()).then_some(a);
    }

    /// A light or heavy pressed beside a staggered enemy in reach becomes a riposte.
    fn riposte_for(&self, m: Move, walked: f64) -> Option<Move> {
        if !matches!(m, Move::Light | Move::Heavy) { return None; }
        let lane = self.hero.lane;
        self.enemies.iter()
            .any(|e| matches!(e.ai, Ai::Staggered(_)) && e.lane == lane && (-0.3..=Move::Riposte.spec().reach).contains(&e.gap(walked)))
            .then_some(Move::Riposte)
    }

    fn strike(&mut self, i: usize, mv: Move, spec: Spec, out: &mut Vec<Event>) {
        let e = &mut self.enemies[i];
        // A shield standing guard turns a light strike aside and throws the hero back a step.
        if mv == Move::Light && e.kind.shielded() && matches!(e.ai, Ai::Wait(_)) {
            e.stance -= 2.0;
            let h = &mut self.hero;
            h.spend(8.0);
            h.reel(Reel::Bounce);
            out.push(Event::Deflected { enemy: i });
            // And bashes back at once, while the hero is thrown off balance.
            let e = &mut self.enemies[i];
            e.ai = Ai::Attack { i: SHIELD_COUNTER, t: 0, wide: 0, landed: false, aim: 0.0 };
            out.push(Event::WindsUp { enemy: i, attack: e.kind.attacks()[SHIELD_COUNTER].name });
            return;
        }
        let kit = self.hero.stats.kit;
        let open = match e.ai { Ai::Stumbled(_) => STUMBLE_HURT * kit.opener, Ai::Staggered(_) => kit.opener, _ => 1.0 };
        let dealt = (spec.damage * self.hero.stats.damage() * open).min(e.hp.max(0.0));
        e.hp -= dealt;
        // A weapon that drinks gives some of it back.
        let h = &mut self.hero;
        h.hp = (h.hp + dealt * kit.leech).min(h.stats.max_hp());
        // A blow landed up close fills the catalyst.
        if kit.ranged == Some(crate::gear::Ranged::Catalyst) { h.charges = (h.charges + CHARGE_PER_HIT).min(h.stats.max_charges()); }
        let e = &mut self.enemies[i];
        e.hurt = 12;
        out.push(Event::Hit { enemy: i, mv });
        if e.hp <= 0.0 {
            e.ai = Ai::Dead;
            out.push(Event::Felled { enemy: i });
            return;
        }
        if mv == Move::Riposte {
            // Back on its feet, stance whole.
            e.ai = Ai::Wait(40);
            e.stance = e.kind.stance();
            return;
        }
        if matches!(e.ai, Ai::Staggered(_)) { return; }
        e.stance -= spec.stance * open;
        e.stance_wait = (e.kind.stance() / 13.0 * 60.0) as u32;
        if e.stance <= 0.0 {
            e.ai = Ai::Staggered(STAGGER);
            out.push(Event::StanceBroken { enemy: i });
        }
    }

    fn enemy_tick(&mut self, i: usize, walked: f64, out: &mut Vec<Event>) {
        let e = &mut self.enemies[i];
        if !e.alive() { return; }
        // Stance comes back after a while unhit.
        if e.stance_wait > 0 { e.stance_wait -= 1; } else { e.stance = (e.stance + 13.0 / 60.0).min(e.kind.stance()); }
        let gap = e.gap(walked);
        let (pace, notice) = e.kind.pace();
        match e.ai {
            Ai::Dead => {}
            Ai::Staggered(n) => {
                e.ai = if n > 1 { Ai::Staggered(n - 1) } else { e.stance = e.kind.stance(); Ai::Wait(20) };
            }
            Ai::Stumbled(n) => {
                e.ai = if n > 1 { Ai::Stumbled(n - 1) } else { Ai::Wait(30) };
            }
            Ai::Wait(n) => {
                let reach = e.kind.attacks().iter().map(|a| a.reach).fold(f32::MAX, f32::min);
                let far = e.kind.attacks().iter().filter(|a| !a.follow).map(|a| a.reach).fold(0.0, f32::max);
                match e.kind.keep() {
                    // A ranged kind holds its gap: in while too far, back (slower) while too near.
                    Some(k) if gap <= notice => {
                        if gap > k + 0.5 { e.s -= (pace / 60.0) as f64; } else if gap < k - 1.0 && gap > 0.6 { e.s += (pace * 0.75 / 60.0) as f64; }
                    }
                    Some(_) => {}
                    // Closes to just inside its shortest reach, never through the hero.
                    None => if gap <= notice && gap > reach - 0.4 { e.s -= (pace / 60.0) as f64; },
                }
                if n > 0 { e.ai = Ai::Wait(n - 1); return; }
                if gap > far + 0.2 || gap < -0.3 { return; }
                // What reaches from here: from afar only its long attacks.
                let openers: Vec<usize> = (0..e_attacks(self.enemies[i].kind) as usize)
                    // Nothing loosed or cast from closer than 3 m: up close a ranged kind fights hand to hand.
                    .filter(|&k| { let a = self.enemies[i].kind.attacks()[k]; !a.follow && gap <= a.reach + 0.2 && (gap > 3.0 || (a.shot.is_none() && !a.area)) }).collect();
                if openers.is_empty() { return; }
                let pick = openers[self.roll(openers.len() as u32) as usize];
                let side = if self.roll(2) == 0 { -1 } else { 1 };
                let e = &mut self.enemies[i];
                let a = e.kind.attacks()[pick];
                // A sweep from the edge lane can only go inward.
                let wide = if a.wide == 0 { 0 } else if e.lane == 1 { -1 } else if e.lane == -1 { 1 } else { side };
                e.ai = Ai::Attack { i: pick, t: 0, wide, landed: false, aim: walked + HERO_D as f64 };
                out.push(Event::WindsUp { enemy: i, attack: a.name });
            }
            Ai::Attack { i: ai, t, wide, landed, aim } => {
                let a = e.kind.attacks()[ai];
                // Aim: follow the hero's lane, a lane at a time, until it commits.
                let target = self.hero.lane;
                let e = &mut self.enemies[i];
                if (t as f32) < a.frames.0 as f32 * a.lock && t % SHIFT == SHIFT - 1 && e.lane != target {
                    e.lane += (target - e.lane).signum();
                }
                // An area attack follows where the hero stands until it locks on.
                let aim = if a.area && (t as f32) < a.frames.0 as f32 * a.lock { walked + HERO_D as f64 } else { aim };
                if t == (a.frames.0 as f32 * a.tell).ceil() as u32 { out.push(Event::Commits { enemy: i, kind: a.kind, parry: a.parry }); }
                let mut landed = landed;
                if phase(a.frames, t) == Phase::Active && !landed {
                    let (l0, l1) = e.lanes();
                    if a.shot.is_some() {
                        // Loosed down its lane, from just in front of it.
                        landed = true;
                        let (s, lane) = (e.s - 0.4, e.lane);
                        self.missiles.push(Missile { from: i, attack: a, s, lane, x: lane as f32 * LANE_W, age: 0 });
                        out.push(Event::Loosed { enemy: i });
                    } else if a.area {
                        // The rune opens: whoever stands on its patch is caught.
                        landed = true;
                        out.push(Event::Burst { enemy: i, at: aim });
                        if ((walked + HERO_D as f64) - aim).abs() <= AREA_HALF as f64 { self.strike_hero(i, a, (l0, l1), out); }
                    } else if (-0.3..=a.reach).contains(&gap) {
                        landed = true;
                        self.strike_hero(i, a, (l0, l1), out);
                    }
                }
                if !matches!(self.enemies[i].ai, Ai::Attack { .. }) { return; }
                let t = t + 1;
                let (w, act, rec) = a.frames;
                let (lo, hi) = self.enemies[i].kind.rest();
                let rest = lo + self.roll(hi - lo + 1);
                self.enemies[i].ai = if t < w + act + rec {
                    Ai::Attack { i: ai, t, wide, landed, aim }
                } else if let Some(next) = a.then {
                    // A combo's next blow, straight on.
                    out.push(Event::WindsUp { enemy: i, attack: self.enemies[i].kind.attacks()[next].name });
                    Ai::Attack { i: next, t: 0, wide: 0, landed: false, aim }
                } else {
                    Ai::Wait(rest)
                };
            }
        }
    }

    /// Every missile one tick on: a hex turning toward the hero's lane until it is close, and
    /// each one arriving at the hero striking if they stand in its lane (a roll through it is
    /// only a miss: the one who loosed it is too far off to stumble).
    /// The hero's arrows and bolts fly on down their lanes; each strikes the first foe it reaches.
    fn bolts_tick(&mut self, out: &mut Vec<Event>) {
        let mut k = 0;
        while k < self.bolts.len() {
            let b = &mut self.bolts[k];
            let step = b.ranged.speed() / 60.0;
            b.age += 1;
            b.s += step as f64;
            b.left -= step;
            b.x += (b.lane as f32 * LANE_W - b.x) * 0.15;
            let (s, lane) = (b.s, b.lane);
            // Passed through this tick (or standing right on it).
            let reached = |e: &Enemy| e.alive() && e.lane == lane && e.s <= s + 0.2 && e.s > s - step as f64 - 0.4;
            let hit = (0..self.enemies.len()).filter(|&i| reached(&self.enemies[i])).min_by(|&x, &y| self.enemies[x].s.total_cmp(&self.enemies[y].s));
            if let Some(i) = hit {
                let b = self.bolts.remove(k);
                self.strike_bolt(i, b, out);
                continue;
            }
            if self.bolts[k].left <= 0.0 { self.bolts.remove(k); continue; }
            k += 1;
        }
    }

    /// The hero's arrow or bolt `b` reaching enemy `i`: caught on a raised shield, or it bites,
    /// with none of a blow's extras (no drinking, no more against the open).
    fn strike_bolt(&mut self, i: usize, b: Bolt, out: &mut Vec<Event>) {
        let e = &mut self.enemies[i];
        if e.kind.shielded() && matches!(e.ai, Ai::Wait(_)) {
            e.stance -= 1.0;
            out.push(Event::Glanced { enemy: i });
            return;
        }
        let dealt = b.damage.min(e.hp.max(0.0));
        e.hp -= dealt;
        e.hurt = 12;
        out.push(Event::Hit { enemy: i, mv: Move::Loose });
        if e.hp <= 0.0 {
            e.ai = Ai::Dead;
            out.push(Event::Felled { enemy: i });
            return;
        }
        if matches!(e.ai, Ai::Staggered(_)) { return; }
        e.stance -= b.stance;
        e.stance_wait = (e.kind.stance() / 13.0 * 60.0) as u32;
        if e.stance <= 0.0 {
            e.ai = Ai::Staggered(STAGGER);
            out.push(Event::StanceBroken { enemy: i });
        }
    }

    fn missiles_tick(&mut self, walked: f64, out: &mut Vec<Event>) {
        let target = self.hero.lane;
        let mut k = 0;
        while k < self.missiles.len() {
            let m = &mut self.missiles[k];
            let Some(f) = m.attack.shot else { self.missiles.remove(k); continue };
            m.age += 1;
            m.s -= (f.speed / 60.0) as f64;
            let ahead = (m.s - walked) as f32 - HERO_D;
            if f.homing && ahead > HOMING_UNTIL && m.age % SHIFT == 0 && m.lane != target { m.lane += (target - m.lane).signum(); }
            m.x += (m.lane as f32 * LANE_W - m.x) * 0.15;
            if ahead > 0.2 { k += 1; continue; }
            let m = self.missiles.remove(k);
            let h = &self.hero;
            if h.dead || h.standing_lane() != m.lane || h.act.is_some_and(|x| x.invulnerable()) { continue; }
            self.strike_hero(m.from, m.attack, (m.lane, m.lane), out);
        }
    }

    /// Enemy `i`'s attack `a`, live now over lanes `lanes`: dodged, parried, blocked or taken.
    fn strike_hero(&mut self, i: usize, a: Attack, lanes: (i32, i32), out: &mut Vec<Event>) {
        let a = Attack { damage: a.damage * self.enemies[i].power, ..a };
        let h = &mut self.hero;
        let covers = |l: i32| (lanes.0..=lanes.1).contains(&l);
        if h.floored() { return; }
        if !covers(h.standing_lane()) || h.act.is_some_and(|x| x.invulnerable()) {
            // Slipped: perfect if a roll or backstep began a moment ago from a lane it covers,
            // and the attacker, swinging through where the hero was, stumbles.
            if h.act.is_some_and(|x| matches!(x.mv, Move::Step(_) | Move::Backstep) && x.t <= PERFECT && covers(x.from_lane)) {
                out.push(Event::JustDodge { enemy: i });
                self.enemies[i].ai = Ai::Stumbled(STUMBLE);
            }
            return;
        }
        if let Some(x) = h.act.filter(|x| x.mv == Move::Parry) {
            // A parrying shield keeps it live a few ticks into the recovery.
            let f = x.spec.frames;
            let live = x.phase() == Phase::Active || (x.phase() == Phase::Recover && x.t < f.0 + f.1 + h.stats.kit.parry);
            match live {
                true if a.parry => {
                    // A clean parry is over at once, so the riposte can follow.
                    h.act = None;
                    out.push(Event::Parried { enemy: i });
                    let e = &mut self.enemies[i];
                    e.ai = Ai::Staggered(STAGGER);
                    out.push(Event::StanceBroken { enemy: i });
                    return;
                }
                // Too early or too late: the buckler half-catches it, and the hero fumbles.
                _ => {
                    let h = &mut self.hero;
                    let damage = a.damage * 0.5 * h.stats.taken();
                    h.hp -= damage;
                    h.spend(a.guard * 0.5);
                    h.reel(Reel::Fumble);
                    out.push(Event::ParryMissed { damage });
                    return;
                }
            }
        }
        if h.guarding && a.kind != Kind::Unblockable { return self.block(i, a, out); }
        let h = &mut self.hero;
        let damage = a.damage * h.stats.taken();
        h.hp -= damage;
        h.poise -= damage * 1.3;
        h.poise_wait = 0;
        if a.kind != Kind::Physical && a.damage >= KNOCKDOWN {
            // Floored, poise whole again for when they are up.
            h.reel(Reel::Down);
            h.poise = h.stats.poise();
            out.push(Event::KnockedDown { damage });
        } else if h.poise <= 0.0 {
            h.reel(Reel::Stagger);
            h.poise = h.stats.poise();
            out.push(Event::Staggered { damage });
        } else {
            let heavy = a.damage >= HEAVY_HIT;
            out.push(Event::Struck { damage, heavy });
            // A heavy strike's armour rides out a flinch or a recoil (not a stagger).
            if !h.act.is_some_and(|x| x.mv.armoured(x.t) || x.t < x.spec.armour) { h.reel(if heavy { Reel::Recoil } else { Reel::Flinch }); }
        }
    }

    fn block(&mut self, i: usize, a: Attack, out: &mut Vec<Event>) {
        let h = &mut self.hero;
        match guard_blow(&h.stats, h.stamina, &mut h.bulwark, a.guard, a.damage, a.kind == Kind::GuardBreak) {
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
    }

    /// A perfect block's ward let go at enemy `i`, at `w` of a full ward: it staggers, open to a
    /// riposte, hurt by its share of `BURST_DAMAGE`; foes near it in the lanes beside are hurt by
    /// less and thrown back from it, stumbling.
    fn ward_burst(&mut self, i: usize, w: f32, out: &mut Vec<Event>) {
        let dealt = BURST_DAMAGE * w * self.hero.stats.damage();
        let (at, lane) = (self.enemies[i].s, self.hero.lane);
        out.push(Event::WardBurst { enemy: i, power: w });
        for (j, e) in self.enemies.iter_mut().enumerate() {
            let near = j == i || ((e.s - at).abs() < BURST_REACH as f64 && (e.lane - lane).abs() <= 1);
            if !e.alive() || !near { continue; }
            e.hp -= (if j == i { dealt } else { dealt * BURST_SPLASH }).min(e.hp.max(0.0));
            e.hurt = 12;
            if e.hp <= 0.0 {
                e.ai = Ai::Dead;
                out.push(Event::Felled { enemy: j });
            } else if j == i {
                e.ai = Ai::Staggered(STAGGER);
                out.push(Event::StanceBroken { enemy: j });
            } else {
                e.s += BURST_SHOVE as f64 * if e.s >= at { 1.0 } else { -1.0 };
                e.ai = Ai::Stumbled(STUMBLE / 2);
            }
        }
    }
}

fn e_attacks(k: Archetype) -> u32 { k.attacks().len() as u32 }

#[cfg(test)]
mod tests {
    use super::*;

    fn husk(gap: f32, lane: i32) -> Enemy { Enemy::new(Archetype::Husk, (HERO_D + gap) as f64, lane) }
    fn idle() -> Input { Input::default() }

    /// Walk forward a tick at a time (as the road would) until stopped; returns metres walked.
    fn walk_up(f: &mut Fight, mut walked: f64) -> f64 {
        let go = Input { forward: true, ..idle() };
        for _ in 0..10_000 {
            let v = f.speed(&go, walked);
            if v == 0.0 { break; }
            walked += (v * 0.016) as f64;
            f.tick(&go, walked);
        }
        walked
    }

    /// Tick with `input` held until `until` says stop (at most `n` ticks); every event.
    fn run(f: &mut Fight, n: u32, mut input: impl FnMut(&Fight, u32) -> Input, mut until: impl FnMut(&Fight, &[Event]) -> bool) -> Vec<Event> {
        let mut ev = Vec::new();
        for t in 0..n {
            let i = input(f, t);
            ev.extend(f.tick(&i, 0.0));
            if until(f, &ev) { break; }
        }
        ev
    }

    /// An enemy that will not attack (to test the hero alone).
    fn dummy(gap: f32, lane: i32) -> Enemy { Enemy { ai: Ai::Wait(u32::MAX), ..husk(gap, lane) } }

    /// Tick until enemy 0 begins attack `name`.
    fn until_winds_up(f: &mut Fight, name: &str) {
        for _ in 0..2_000 {
            let ev = f.tick(&idle(), 0.0);
            if ev.iter().any(|e| matches!(e, Event::WindsUp { enemy: 0, attack } if *attack == name)) { return; }
            // Re-roll an unwanted attack straight away.
            if let Some((a, _, _)) = f.enemies[0].attack() { if a.name != name { f.enemies[0].ai = Ai::Wait(0); } }
        }
        panic!("never wound up {name}");
    }

    fn of(kind: Archetype, gap: f32, lane: i32) -> Enemy { Enemy::new(kind, (HERO_D + gap) as f64, lane) }

    #[test]
    fn a_lancer_strikes_from_beyond_the_heros_reach() {
        let thrust = Archetype::Lancer.attacks()[0];
        assert!(thrust.reach > Move::Light.spec().reach + 1.0);
        // Standing at 4 m, out of reach of any strike, it still lands its thrust.
        let mut f = Fight::new(vec![of(Archetype::Lancer, 4.0, 0)]);
        until_winds_up(&mut f, "thrust");
        let ev = run(&mut f, 200, |_, _| idle(), |_, ev| ev.iter().any(|e| matches!(e, Event::Struck { .. })));
        assert!(ev.contains(&Event::Struck { damage: thrust.damage, heavy: true }), "{ev:?}");
        // A step to the side once it has committed (it follows the hero's lane until then) slips it.
        let mut g = Fight::new(vec![of(Archetype::Lancer, 4.0, 0)]);
        until_winds_up(&mut g, "thrust");
        let late = (thrust.frames.0 as f32 * COMMIT) as u32 + 1;
        let ev = run(&mut g, 200, |_, t| Input { step: if t == late { 1 } else { 0 }, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(!ev.iter().any(|e| matches!(e, Event::Struck { .. })), "{ev:?}");
    }

    #[test]
    fn a_shield_turns_a_light_strike_but_not_a_heavy() {
        let guard = |gap| Enemy { ai: Ai::Wait(u32::MAX), ..of(Archetype::Shieldbearer, gap, 0) };
        let mut f = Fight::new(vec![guard(1.0)]);
        let ev = run(&mut f, 80, |_, t| Input { light: t == 0, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Deflected { .. } | Event::Hit { .. })));
        assert!(ev.contains(&Event::Deflected { enemy: 0 }) && f.enemies[0].hp == Archetype::Shieldbearer.hp(), "{ev:?}");
        assert!(f.hero.reeling > 0, "thrown back");
        let mut g = Fight::new(vec![guard(1.0)]);
        let ev = run(&mut g, 120, |_, t| Input { heavy: t == 0, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Deflected { .. } | Event::Hit { .. })));
        assert!(ev.contains(&Event::Hit { enemy: 0, mv: Move::Heavy }), "{ev:?}");
    }

    #[test]
    fn a_wretchs_frenzy_runs_three_blows_without_a_rest() {
        let mut f = Fight::new(vec![of(Archetype::Wretch, 1.0, 0)]);
        f.hero.hp = 1_000.0;
        until_winds_up(&mut f, "frenzy");
        let ev = run(&mut f, 300, |_, _| idle(), |f, _| matches!(f.enemies[0].ai, Ai::Wait(_)));
        let winds = ev.iter().filter(|e| matches!(e, Event::WindsUp { attack: "frenzy", .. })).count();
        let struck = ev.iter().filter(|e| matches!(e, Event::Struck { heavy: false, .. } | Event::Staggered { .. })).count();
        assert_eq!((winds, struck), (2, 3), "the two follow-ups wound up, and all three blows landed: {ev:?}");
        // Each a flinch, and the three together wear the poise through: the last staggers.
        assert!(matches!(ev.iter().rev().find(|e| matches!(e, Event::Struck { .. } | Event::Staggered { .. })), Some(Event::Staggered { .. })), "{ev:?}");
        // A follow-up is never chosen to open with.
        assert!(Archetype::Wretch.attacks().iter().filter(|a| a.follow).count() == 2);
    }

    #[test]
    fn walking_stops_short_of_an_enemy_and_steps_back_when_asked() {
        let mut f = Fight::new(vec![dummy(20.0, 0)]);
        let w = walk_up(&mut f, 0.0);
        let gap = f.enemies[0].gap(w);
        assert!((ENGAGE - 0.05..=ENGAGE).contains(&gap), "stopped {gap} m from it");
        assert_eq!(f.speed(&Input { back: true, ..idle() }, w), -BACK);
        // An enemy in another lane still stands in the road.
        let mut g = Fight::new(vec![dummy(20.0, 1)]);
        let gw = walk_up(&mut g, 0.0);
        assert!(g.enemies[0].gap(gw) <= ENGAGE);
    }

    #[test]
    fn a_light_lands_on_its_live_frames_in_the_same_lane_only() {
        let (w, a, r) = Move::Light.spec().frames;
        let mut f = Fight::new(vec![dummy(1.0, 0)]);
        let mut ev = f.tick(&Input { light: true, ..idle() }, 0.0);
        assert_eq!(f.hero.stamina, STAMINA - 12.0, "paid up front");
        for _ in 0..(w + a + r + 2) { ev.extend(f.tick(&idle(), 0.0)); }
        assert_eq!(ev.iter().filter(|e| matches!(e, Event::Hit { .. })).count(), 1, "one hit per strike: {ev:?}");
        assert_eq!(f.enemies[0].hp, Archetype::Husk.hp() - 10.0);
        let mut g = Fight::new(vec![dummy(1.0, 1)]);
        let mut ev = g.tick(&Input { light: true, ..idle() }, 0.0);
        for _ in 0..(w + a + 2) { ev.extend(g.tick(&idle(), 0.0)); }
        assert!(ev.contains(&Event::Whiffed(Move::Light)), "{ev:?}");
    }

    #[test]
    fn a_move_commits_and_a_press_waits_in_the_buffer() {
        let mut f = Fight::new(vec![dummy(1.0, 0)]);
        f.tick(&Input { heavy: true, ..idle() }, 0.0);
        assert_eq!(f.speed(&Input { forward: true, ..idle() }, 0.0), 0.0, "no walking mid-swing");
        let total = { let (a, b, c) = Move::Heavy.spec().frames; a + b + c };
        // A light pressed near the end of the heavy runs as soon as it is over.
        for t in 1..total {
            let i = Input { light: t == total - BUFFER + 2, ..idle() };
            f.tick(&i, 0.0);
        }
        f.tick(&idle(), 0.0);
        assert_eq!(f.hero.act.map(|a| a.mv), Some(Move::Light), "buffered light ran");
        // One pressed too early is dropped.
        let mut g = Fight::new(vec![dummy(1.0, 0)]);
        g.tick(&Input { heavy: true, ..idle() }, 0.0);
        g.tick(&Input { light: true, ..idle() }, 0.0);
        for _ in 0..total { g.tick(&idle(), 0.0); }
        assert!(g.hero.act.is_none(), "{:?}", g.hero.act);
    }

    #[test]
    fn a_chop_strikes_a_hero_who_stands_and_misses_one_who_dodges_out_of_its_lane() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "chop");
        let ev = run(&mut f, 200, |_, _| idle(), |f, _| f.enemies[0].attack().is_none());
        assert!(ev.contains(&Event::Struck { damage: 28.0, heavy: true }), "{ev:?}");
        let mut r = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut r, "chop");
        run(&mut r, 200, |_, _| idle(), |_, ev| ev.iter().any(|e| matches!(e, Event::Struck { .. })));
        assert_eq!(r.hero.reel, Reel::Recoil, "a heavy blow knocks the hero back");
        assert_eq!(f.hero.hp, HP - 28.0);

        let (w, _, _) = Archetype::Husk.attacks()[0].frames;
        // Roll right once it has committed but well before the chop falls: slipped, nothing more.
        let mut g = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut g, "chop");
        let ev = run(&mut g, 200, |_, t| Input { step: if t == w - 12 { 1 } else { 0 }, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert_eq!(g.hero.hp, HP, "{ev:?}");
        assert!(!ev.iter().any(|e| matches!(e, Event::JustDodge { .. })), "an early roll is only a roll: {ev:?}");
        assert!(!matches!(g.enemies[0].ai, Ai::Stumbled(_)));
        // Roll at the last moment: perfect, and the husk stumbles through where the hero was.
        let mut h = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut h, "chop");
        let ev = run(&mut h, 200, |_, t| Input { step: if t == w - 6 { 1 } else { 0 }, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert_eq!(h.hero.hp, HP, "{ev:?}");
        assert!(ev.contains(&Event::JustDodge { enemy: 0 }), "{ev:?}");
        assert!(matches!(h.enemies[0].ai, Ai::Stumbled(n) if n > STUMBLE - 5), "{:?}", h.enemies[0].ai);
        // It cannot act until it has its feet back.
        let ev = run(&mut h, STUMBLE - 6, |_, _| idle(), |_, _| false);
        assert!(!ev.iter().any(|e| matches!(e, Event::WindsUp { .. })), "{ev:?}");
    }

    #[test]
    fn a_stumbling_enemy_takes_more_from_every_strike_and_its_shield_is_down() {
        let stumbling = |kind| Enemy { ai: Ai::Stumbled(u32::MAX), ..of(kind, 1.0, 0) };
        let mut f = Fight::new(vec![stumbling(Archetype::Husk)]);
        run(&mut f, 80, |_, t| Input { light: t == 0, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Hit { .. })));
        assert_eq!(f.enemies[0].hp, Archetype::Husk.hp() - 10.0 * STUMBLE_HURT);
        assert_eq!(f.enemies[0].stance, Archetype::Husk.stance() - 8.0 * STUMBLE_HURT);
        let mut g = Fight::new(vec![stumbling(Archetype::Shieldbearer)]);
        let ev = run(&mut g, 80, |_, t| Input { light: t == 0, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Hit { .. } | Event::Deflected { .. })));
        assert!(ev.contains(&Event::Hit { enemy: 0, mv: Move::Light }), "{ev:?}");
    }

    #[test]
    fn a_sweep_covers_two_lanes_and_breaks_a_short_guard() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "sweep");
        let (l0, l1) = f.enemies[0].lanes();
        assert_eq!(l1 - l0, 1, "two lanes");
        // Stand in the other lane it covers, guarding with too little stamina.
        f.hero.lane = if l0 == 0 { 1 } else { -1 };
        f.hero.x = f.hero.lane as f32 * LANE_W;
        f.hero.stamina = 30.0;
        let ev = run(&mut f, 200, |_, _| Input { guard: true, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.iter().any(|e| matches!(e, Event::GuardBroken { .. })), "{ev:?}");
        assert_eq!(f.hero.reel, Reel::GuardBreak);
        assert!(f.hero.hp < HP);
    }

    #[test]
    fn a_full_guard_stops_a_chop_for_stamina_and_a_sweep_for_some_chip() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "chop");
        let ev = run(&mut f, 200, |_, _| Input { guard: true, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.contains(&Event::Blocked { chip: 0.0 }), "{ev:?}");
        assert_eq!(f.hero.hp, HP, "a chop is stopped completely");
        assert!(f.hero.stamina < STAMINA - 25.0);
        // A guard-breaking sweep held with stamina to spare still gets some through.
        let mut g = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut g, "sweep");
        g.hero.lane = g.enemies[0].lanes().0;
        g.hero.x = g.hero.lane as f32 * LANE_W;
        let mut seen = None;
        let ev = run(&mut g, 200, |_, _| Input { guard: true, ..idle() }, |f, ev| { if seen.is_none() && ev.iter().any(|e| matches!(e, Event::Blocked { .. })) { seen = Some(f.hero.reel); } f.enemies[0].attack().is_none() });
        assert!(ev.iter().any(|e| matches!(e, Event::Blocked { chip } if (*chip - 3.0).abs() < 0.01)), "{ev:?}");
        assert_eq!(seen, Some(Reel::ChipJolt));
    }

    /// A fight against a husk 1.2 m off with the hero holding the Warden's Bulwark.
    fn warden(enemies: Vec<Enemy>) -> Fight {
        let mut l = crate::gear::Loadout::starting();
        let b = crate::gear::find("Warden's Bulwark");
        l.gain(b);
        assert!(l.wear(1, Some(b)));
        let mut f = Fight::new(enemies);
        f.hero.stats.kit = l.kit();
        f.hero = Hero::with(f.hero.stats);
        f
    }

    /// Ticks from a husk's chop winding up to it landing on a guard held all along.
    fn chop_lands_after(f: &Fight) -> u32 {
        let mut g = f.clone();
        until_winds_up(&mut g, "chop");
        let mut t = 0;
        run(&mut g, 200, |_, _| Input { guard: true, ..idle() }, |_, ev| { t += 1; ev.iter().any(|e| matches!(e, Event::Blocked { .. } | Event::PerfectBlock)) });
        t
    }

    #[test]
    fn a_bulwark_fills_its_ward_by_blocking_and_a_last_moment_block_lets_it_burst() {
        // Held up all along, a chop is blocked as any shield would, and the ward fills.
        let mut f = warden(vec![husk(1.2, 0)]);
        assert!(f.hero.stats.poise() >= POISE + 20.0, "the steadiest on their feet");
        let lands = chop_lands_after(&f);
        until_winds_up(&mut f, "chop");
        let ev = run(&mut f, 200, |_, _| Input { guard: true, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.contains(&Event::Blocked { chip: 0.0 }) && !ev.contains(&Event::PerfectBlock), "{ev:?}");
        let filled = f.hero.bulwark.ward;
        assert!(filled > 0.0 && filled < WARD_BURST, "{filled}");
        // Raised a moment before it lands: free, nothing through, and the ward fills more.
        let mut g = warden(vec![husk(1.2, 0)]);
        assert!(lands > REARM + 4, "{lands}");
        until_winds_up(&mut g, "chop");
        let ev = run(&mut g, 200, |_, t| Input { guard: t + 4 >= lands, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.contains(&Event::PerfectBlock) && !ev.iter().any(|e| matches!(e, Event::Blocked { .. })), "{ev:?}");
        assert_eq!((g.hero.hp, g.hero.reel), (HP, Reel::None));
        assert!(g.hero.stamina >= STAMINA - 0.01, "nothing spent: {}", g.hero.stamina);
        // With ward enough stored, the perfect block lets it go into the husk: staggered, hurt,
        // and the ward spent.
        let mut k = warden(vec![husk(1.2, 0)]);
        k.hero.bulwark.ward = 60.0;
        until_winds_up(&mut k, "chop");
        let ev = run(&mut k, 200, |_, t| Input { guard: t + 4 >= lands, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::WardBurst { .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::WardBurst { enemy: 0, power } if (*power - 0.6).abs() < 1e-4)), "{ev:?}");
        assert!(matches!(k.enemies[0].ai, Ai::Staggered(_)), "{:?}", k.enemies[0].ai);
        let hurt = Archetype::Husk.hp() - k.enemies[0].hp;
        assert!((hurt - BURST_DAMAGE * 0.6 * k.hero.stats.damage()).abs() < 0.01, "{hurt}");
        assert_eq!(k.hero.bulwark.ward, 0.0);
    }

    #[test]
    fn a_guard_flicked_up_and_down_never_blocks_perfectly_and_a_plain_shield_has_no_ward() {
        // Flicked every few ticks, the guard is up when the chop lands but never fresh.
        let mut f = warden(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "chop");
        let ev = run(&mut f, 200, |_, t| Input { guard: t % 6 < 3, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(!ev.contains(&Event::PerfectBlock), "{ev:?}");
        // The Wayfarer's buckler, raised at the same last moment: an ordinary block, no ward.
        let lands = chop_lands_after(&warden(vec![husk(1.2, 0)]));
        let mut g = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut g, "chop");
        let ev = run(&mut g, 200, |_, t| Input { guard: t + 4 >= lands, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.contains(&Event::Blocked { chip: 0.0 }) && !ev.contains(&Event::PerfectBlock), "{ev:?}");
        assert_eq!(g.hero.bulwark.ward, 0.0);
    }

    #[test]
    fn a_guard_breaking_blow_taken_unguarded_floors_the_hero_who_is_safe_while_down() {
        let mut f = Fight::new(vec![of(Archetype::Shieldbearer, 1.2, 0)]);
        f.hero.hp = 1_000.0;
        until_winds_up(&mut f, "smash");
        let ev = run(&mut f, 200, |_, _| idle(), |_, ev| ev.iter().any(|e| matches!(e, Event::KnockedDown { .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::KnockedDown { .. })), "{ev:?}");
        // On the ground a blow passes over; the fall's first moment is not yet safe.
        run(&mut f, 12, |_, _| idle(), |_, _| false);
        let hp = f.hero.hp;
        f.strike_hero(0, Archetype::Shieldbearer.attacks()[0], (0, 0), &mut Vec::new());
        assert_eq!(f.hero.hp, hp, "unhurt while down");
        // And up again, free to act, once the getting up is done.
        run(&mut f, Reel::Down.ticks(), |_, _| idle(), |f, _| f.hero.reel == Reel::None);
        assert!(f.hero.reel == Reel::None && f.hero.reeling == 0);
    }

    #[test]
    fn a_parry_on_time_opens_a_riposte_and_one_out_of_time_half_catches_the_blow() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "chop");
        let (w, _, _) = Archetype::Husk.attacks()[0].frames;
        let (pw, pa, _) = Move::Parry.spec().frames;
        // Live from the press + start-up: press so the chop falls mid-window.
        let press = w - 1 - pw - pa / 2;
        let ev = run(&mut f, 200, |_, t| Input { parry: t == press, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Parried { .. } | Event::Struck { .. } | Event::Blocked { .. })));
        assert!(ev.contains(&Event::Parried { enemy: 0 }), "{ev:?}");
        assert!(matches!(f.enemies[0].ai, Ai::Staggered(_)));
        assert_eq!(f.hero.hp, HP);
        // The next strike pressed is a riposte, and it hurts.
        let ev = run(&mut f, 120, |f, _| Input { heavy: f.hero.act.is_none(), ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Hit { .. })));
        assert!(ev.contains(&Event::Hit { enemy: 0, mv: Move::Riposte }), "{ev:?}");
        assert_eq!(f.enemies[0].hp, Archetype::Husk.hp() - 60.0);

        let mut g = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut g, "chop");
        // Early enough that the chop falls in the parry's recovery.
        let early = w - 1 - pw - pa - 6;
        let ev = run(&mut g, 200, |_, t| Input { parry: t == early, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::ParryMissed { .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::ParryMissed { damage } if (*damage - 14.0).abs() < 0.01)), "early: {ev:?}");
        assert_eq!(g.hero.reel, Reel::Fumble);
        assert!(!ev.iter().any(|e| matches!(e, Event::Parried { .. })));
    }

    #[test]
    fn heavies_break_stance_which_comes_back_after_a_while() {
        let mut f = Fight::new(vec![dummy(1.0, 0)]);
        let ev = run(&mut f, 400, |f, _| Input { heavy: f.hero.act.is_none(), ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::StanceBroken { .. })));
        assert_eq!(ev.iter().filter(|e| matches!(e, Event::Hit { .. })).count(), 2, "30 stance, 22 a heavy: {ev:?}");
        // A single light leaves a dent that fills back in.
        let mut g = Fight::new(vec![dummy(1.0, 0)]);
        g.tick(&Input { light: true, ..idle() }, 0.0);
        for _ in 0..40 { g.tick(&idle(), 0.0); }
        assert_eq!(g.enemies[0].stance, 22.0);
        for _ in 0..400 { g.tick(&idle(), 0.0); }
        assert_eq!(g.enemies[0].stance, 30.0);
    }

    #[test]
    fn stamina_gates_moves_and_comes_back_slower_behind_a_guard() {
        let mut f = Fight::new(vec![dummy(5.0, 0)]);
        f.hero.stamina = 0.0;
        let ev = f.tick(&Input { light: true, ..idle() }, 0.0);
        assert!(ev.contains(&Event::Spent) && f.hero.act.is_none(), "{ev:?}");
        let mut g = f.clone();
        for _ in 0..(REGEN_DELAY + 31) { f.tick(&idle(), 0.0); g.tick(&Input { guard: true, ..idle() }, 0.0); }
        assert!(f.hero.stamina > 2.5 * g.hero.stamina && g.hero.stamina > 0.0, "{} vs {}", f.hero.stamina, g.hero.stamina);
    }

    #[test]
    fn a_backstep_walks_the_road_back_and_a_flask_heals() {
        let mut f = Fight::new(vec![dummy(5.0, 0)]);
        f.tick(&Input { backstep: true, ..idle() }, 0.0);
        assert!(f.speed(&idle(), 0.0) < -3.0);
        assert_eq!(f.hero.lane, 0);
        let mut g = Fight::new(vec![dummy(5.0, 0)]);
        g.hero.hp = 30.0;
        let ev = run(&mut g, 80, |_, t| Input { flask: t == 0, ..idle() }, |f, _| f.hero.act.is_none());
        assert!(ev.contains(&Event::Healed));
        assert_eq!((g.hero.hp, g.hero.flasks), (70.0, FLASKS - 1));
    }

    #[test]
    fn the_hero_dies_and_is_still_after() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        f.hero.hp = 10.0;
        let ev = run(&mut f, 2_000, |_, _| idle(), |f, _| f.hero.dead);
        assert!(ev.contains(&Event::Died), "{ev:?}");
        assert!(f.tick(&Input { light: true, ..idle() }, 0.0).is_empty());
    }

    #[test]
    fn a_lane_change_is_a_roll_that_costs_stamina_travels_smoothly_and_stops_at_the_edge() {
        let mut f = Fight::new(vec![dummy(5.0, 0)]);
        f.tick(&Input { step: 1, ..idle() }, 0.0);
        assert_eq!((f.hero.lane, f.hero.stamina), (1, STAMINA - Move::Step(1).spec().stamina));
        assert!(f.hero.act.unwrap().invulnerable());
        // Across on an ease in and out: slow, quick, slow; there exactly when the travel ends.
        let mut xs = vec![f.hero.x];
        for _ in 1..TRAVEL { f.tick(&idle(), 0.0); xs.push(f.hero.x); }
        let steps: Vec<f32> = xs.windows(2).map(|w| w[1] - w[0]).collect();
        let mid = steps[steps.len() / 2];
        assert!(steps[0] < mid && steps[steps.len() - 1] < mid && steps.iter().all(|s| *s >= 0.0), "{xs:?}");
        assert_eq!(f.hero.x, LANE_W);
        // Out of any fight a lane change is free (leaning at a fork after a run).
        let mut g = Fight::new(vec![dummy(40.0, 0)]);
        g.hero.stamina = 0.0;
        g.tick(&Input { step: -1, ..idle() }, 0.0);
        assert_eq!((g.hero.lane, g.hero.stamina), (-1, 0.0));
        // No roll off the road: the press is dropped and nothing is spent.
        for _ in 0..40 { f.tick(&idle(), 0.0); }
        let st = f.hero.stamina;
        f.tick(&Input { step: 1, ..idle() }, 0.0);
        assert_eq!((f.hero.lane, f.hero.act, f.hero.stamina >= st), (1, None, true));
    }

    #[test]
    fn a_run_is_quicker_costs_stamina_and_stops_short_of_a_fight() {
        let mut f = Fight::new(vec![dummy(20.0, 0)]);
        let go = Input { forward: true, run: true, ..idle() };
        assert_eq!(f.speed(&go, 0.0), RUN);
        f.tick(&go, 0.0);
        assert!(f.hero.stamina < STAMINA);
        // Near an enemy it is a walk again.
        assert_eq!(f.speed(&go, 17.0), WALK);
    }

    #[test]
    fn an_attack_commits_once_at_its_commit_point_saying_how_to_meet_it() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        f.hero.hp = 1_000.0;
        until_winds_up(&mut f, "chop");
        let ev = run(&mut f, 200, |_, _| idle(), |f, _| f.enemies[0].attack().is_none());
        let commits: Vec<_> = ev.iter().filter(|e| matches!(e, Event::Commits { .. })).collect();
        assert_eq!(commits, [&Event::Commits { enemy: 0, kind: Kind::Physical, parry: true }], "{ev:?}");
        // Before the blow lands.
        let at = |p: &dyn Fn(&Event) -> bool| ev.iter().position(|e| p(e));
        assert!(at(&|e| matches!(e, Event::Commits { .. })) < at(&|e| matches!(e, Event::Struck { .. })));
    }

    #[test]
    fn a_chop_follows_the_heros_lane_until_it_commits() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "chop");
        // Step right early: it follows.
        f.tick(&Input { step: 1, ..idle() }, 0.0);
        for _ in 0..(SHIFT + 1) { f.tick(&idle(), 0.0); }
        assert_eq!(f.enemies[0].lane, 1, "followed");
        // After it commits, stepping away is too late for it to follow.
        let w = Archetype::Husk.attacks()[0].frames.0;
        while matches!(f.enemies[0].ai, Ai::Attack { t, .. } if (t as f32) < w as f32 * COMMIT) { f.tick(&idle(), 0.0); }
        f.tick(&Input { step: -1, ..idle() }, 0.0);
        let ev = run(&mut f, 100, |_, _| idle(), |f, _| f.enemies[0].attack().is_some_and(|a| a.1 == Phase::Recover));
        assert_eq!(f.enemies[0].lane, 1, "committed");
        assert!(!ev.iter().any(|e| matches!(e, Event::Struck { .. })), "and the step took the hero out of it: {ev:?}");
    }

    /// A fight against `enemies` with the hero holding the weapon `name`, strong enough for it.
    fn armed(name: &str, enemies: Vec<Enemy>) -> Fight {
        let mut l = crate::gear::Loadout::starting();
        let i = crate::gear::find(name);
        l.gain(i);
        assert!(l.wear(0, Some(i)));
        let mut f = Fight::new(enemies);
        f.hero.stats.might = 10;
        f.hero.stats.dexterity = 10;
        f.hero.stats.kit = l.kit();
        f
    }

    /// Hits landed by one `mv` thrown from rest.
    fn hits(f: &mut Fight, mv: Input) -> usize {
        let mut ev = f.tick(&mv, 0.0);
        for _ in 0..120 { ev.extend(f.tick(&idle(), 0.0)); if f.hero.act.is_none() { break; } }
        ev.iter().filter(|e| matches!(e, Event::Hit { .. })).count()
    }

    #[test]
    fn each_kind_of_weapon_swings_its_own_way() {
        use crate::gear::Class;
        let light = Input { light: true, ..idle() };
        let heavy = Input { heavy: true, ..idle() };
        // A spear reaches where a sword falls short.
        assert_eq!(hits(&mut armed("Wayfarer's Sword", vec![dummy(3.2, 0)]), light), 0);
        assert_eq!(hits(&mut armed("Lancer's Shortspear", vec![dummy(3.2, 0)]), light), 1);
        // A greatsword's heavy sweeps the lanes either side; its light, and any sword's heavy, do not.
        let three = || vec![dummy(1.0, 0), dummy(1.0, 1), dummy(1.0, -1)];
        assert_eq!(hits(&mut armed("Executioner's Greatsword", three()), heavy), 3);
        assert_eq!(hits(&mut armed("Executioner's Greatsword", three()), light), 1);
        assert_eq!(hits(&mut armed("Wayfarer's Sword", three()), heavy), 1);
        // Quick against slow, and the slow one breaks more stance.
        let (rap, axe) = (Move::Light.spec_for(Class::Thrust, None), Move::Light.spec_for(Class::Axe, None));
        let total = |s: Spec| s.frames.0 + s.frames.1 + s.frames.2;
        assert!(total(rap) < total(Move::Light.spec()) && total(Move::Light.spec()) < total(axe));
        assert!(axe.stance > Move::Light.spec().stance && rap.stance < Move::Light.spec().stance);
        // The time a strike takes is the weapon's it began with.
        let mut f = armed("Husk Cleaver", vec![dummy(1.0, 0)]);
        f.tick(&light, 0.0);
        assert_eq!(f.hero.act.unwrap().spec, axe);
    }

    #[test]
    fn an_ultra_greatsword_is_slow_wide_and_rides_out_a_flinch() {
        use crate::gear::Class;
        let light = Input { light: true, ..idle() };
        let heavy = Input { heavy: true, ..idle() };
        let three = || vec![dummy(1.0, 0), dummy(1.0, 1), dummy(1.0, -1)];
        let slab = |enemies| { let mut f = armed("Gravewarden's Slab", enemies); f.hero.stats.might = 20; f };
        // Both its swings sweep all three lanes.
        assert_eq!(hits(&mut slab(three()), light), 3);
        assert_eq!(hits(&mut slab(three()), heavy), 3);
        // Slower to start than a greatsword's, and far heavier when it lands.
        let (great, ultra) = (Move::Heavy.spec_for(Class::Greatsword, None), Move::Heavy.spec_for(Class::Ultra, None));
        assert!(ultra.frames.0 > great.frames.0 + 10 && ultra.damage > great.damage * 1.5 && ultra.stance > great.stance);
        // A hit in the windup hurts but does not knock it out of the swing; a greatsword's light
        // is knocked out of it.
        let struck = |mut f: Fight| {
            f.tick(&light, 0.0);
            for _ in 0..5 { f.tick(&idle(), 0.0); }
            let hp = f.hero.hp;
            f.strike_hero(0, Archetype::Husk.attacks()[0], (0, 0), &mut Vec::new());
            assert!(f.hero.hp < hp);
            f.hero.act.is_some_and(|a| a.mv == Move::Light)
        };
        assert!(struck(slab(vec![dummy(4.0, 0)])));
        assert!(!struck(armed("Executioner's Greatsword", vec![dummy(4.0, 0)])));
    }

    #[test]
    fn a_weapon_scales_with_what_it_names_and_is_weak_without_what_it_needs() {
        let mut st = Stats::default();
        let mut l = crate::gear::Loadout::starting();
        let rapier = crate::gear::find("Duelist's Rapier");
        l.gain(rapier);
        l.wear(0, Some(rapier));
        st.kit = l.kit();
        assert!(st.weak() && st.lacking() == vec![1]);
        assert!((st.damage() - WEAK).abs() < 1e-5 && st.cost(Move::Light, 10.0) > 10.0);
        st.dexterity = 6;
        let at6 = st.damage();
        assert!((at6 - (1.0 + 1.2 * 0.36)).abs() < 1e-5, "{at6}");
        // Might does nothing for it; dexterity keeps paying, more slowly past 10.
        st.might = 10;
        assert_eq!(st.damage(), at6);
        let step = |st: &mut Stats| { let d = st.damage(); st.dexterity += 1; st.damage() - d };
        st.dexterity = 9;
        let early = step(&mut st);
        st.dexterity = 15;
        let late = step(&mut st);
        st.dexterity = 25;
        let past = step(&mut st);
        assert!(early > late && late > past && past > 0.0, "{early} {late} {past}");
    }

    #[test]
    fn what_is_worn_changes_the_numbers_and_a_heavy_load_makes_rolls_dear() {
        let mut st = Stats::default();
        let roll = Move::Step(1).spec().stamina;
        assert_eq!(st.cost(Move::Step(1), roll), roll);
        let mut l = crate::gear::Loadout::starting();
        for n in ["Chain Hauberk", "Kite Shield", "Husk Cleaver", "Keeper's Sigil"] {
            let i = crate::gear::find(n);
            l.gain(i);
            let p = crate::gear::SLOTS.iter().position(|s| s.0 == crate::gear::ITEMS[i].slot).unwrap();
            assert!(l.wear(p, Some(i)));
        }
        st.kit = l.kit();
        assert!(st.burdened(), "{} of {}", st.kit.load, st.capacity());
        assert!(st.cost(Move::Step(1), roll) > 1.3 * roll && st.cost(Move::Light, 12.0) > 12.0);
        // The cleaver wants might 4: short of it, it is swung weakly.
        assert!(st.weak() && st.damage() < 0.75, "{}", st.damage());
        st.might = 4;
        assert!(!st.weak());
        assert!(st.max_hp() > HP && st.poise() > POISE && st.taken() < 1.0 && st.damage() > 1.2);
        // Endurance carries it.
        st.endurance = 10;
        assert!(!st.burdened());
    }
    #[test]
    fn a_late_chop_follows_a_roll_made_at_its_flash_and_misses_a_late_one() {
        let late = Archetype::Husk.attacks()[2];
        let (w, _, _) = late.frames;
        let tell = (w as f32 * late.tell).ceil() as u32;
        // Rolled at the flash: the husk turns after the hero, a lane, and the chop lands.
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "late chop");
        let ev = run(&mut f, 200, |_, t| Input { step: if t == tell { 1 } else { 0 }, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.iter().any(|e| matches!(e, Event::Struck { .. } | Event::Staggered { .. } | Event::KnockedDown { .. })), "followed and struck: {ev:?}");
        // Rolled at the last moment: slipped.
        let mut g = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut g, "late chop");
        let ev = run(&mut g, 200, |_, t| Input { step: if t == w - 6 { 1 } else { 0 }, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert_eq!(g.hero.hp, HP, "{ev:?}");
    }

    #[test]
    fn a_shield_that_turns_a_light_strike_bashes_back() {
        let mut f = Fight::new(vec![of(Archetype::Shieldbearer, 1.2, 0)]);
        f.enemies[0].ai = Ai::Wait(200);
        let ev = run(&mut f, 60, |_, t| Input { light: t == 0, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::WindsUp { .. })));
        assert!(ev.contains(&Event::Deflected { enemy: 0 }), "{ev:?}");
        assert!(ev.contains(&Event::WindsUp { enemy: 0, attack: "counter bash" }), "{ev:?}");
    }

    #[test]
    fn a_wretch_pounces_from_afar() {
        let mut f = Fight::new(vec![of(Archetype::Wretch, 4.3, 0)]);
        let mut first = None;
        run(&mut f, 400, |_, _| idle(), |_, ev| { first = ev.iter().find_map(|e| match e { Event::WindsUp { attack, .. } => Some(*attack), _ => None }); first.is_some() });
        assert_eq!(first, Some("pounce"), "from 4.3 m only the pounce reaches");
    }

    /// Enemy 0 starts its attack `i` now.
    fn attacking(mut f: Fight, i: usize) -> Fight { f.enemies[0].ai = Ai::Attack { i, t: 0, wide: 0, landed: false, aim: HERO_D as f64 }; f }
    fn hurt(ev: &[Event]) -> bool { ev.iter().any(|e| matches!(e, Event::Struck { .. } | Event::Staggered { .. } | Event::KnockedDown { .. })) }

    #[test]
    fn an_arrow_flies_down_its_lane_and_misses_a_hero_who_steps_out_of_it() {
        // Stood in the lane, it arrives a good while after the loose: 9 m at 16 m/s.
        let mut f = attacking(Fight::new(vec![of(Archetype::Archer, 9.0, 0)]), 0);
        let (mut loosed, mut struck) = (None, None);
        run(&mut f, 300, |_, _| idle(), |f, ev| {
            if loosed.is_none() && ev.iter().any(|e| matches!(e, Event::Loosed { .. })) { loosed = Some(f.hero.hp); }
            if hurt(ev) { struck = Some(()); }
            struck.is_some()
        });
        assert!(loosed.is_some() && struck.is_some());
        let mut f = attacking(Fight::new(vec![of(Archetype::Archer, 9.0, 0)]), 0);
        let ev = run(&mut f, 300, |_, _| idle(), |_, ev| ev.iter().any(|e| matches!(e, Event::Loosed { .. })));
        assert!(!hurt(&ev) && f.missiles.len() == 1, "nothing lands at the loose; the arrow is in flight");
        let ticks_to_hit = { let mut g = f.clone(); run(&mut g, 200, |_, _| idle(), |_, ev| hurt(ev)); g };
        assert!(ticks_to_hit.missiles.is_empty());
        // A step out of the lane as it flies: it passes.
        let ev = run(&mut f, 200, |_, t| Input { step: if t == 0 { 1 } else { 0 }, ..idle() }, |f, _| f.missiles.is_empty());
        assert!(!hurt(&ev) && f.hero.hp == f.hero.stats.max_hp());
        // A shield turns it, cheaply.
        let mut f = attacking(Fight::new(vec![of(Archetype::Archer, 9.0, 0)]), 0);
        let ev = run(&mut f, 300, |_, _| Input { guard: true, ..idle() }, |f, ev| ev.iter().any(|e| matches!(e, Event::Loosed { .. })) && f.missiles.is_empty());
        assert!(!hurt(&ev) && ev.iter().any(|e| matches!(e, Event::Blocked { .. })));
        assert!(f.hero.stamina > f.hero.stats.max_stamina() - 25.0);
    }

    #[test]
    fn a_hex_follows_the_heros_lane_until_late_and_a_rune_catches_only_who_stays_on_it() {
        // A roll early is followed; a roll once it is close gets away.
        let hex = || attacking(Fight::new(vec![of(Archetype::Hexer, 9.0, 0)]), 0);
        let mut f = hex();
        run(&mut f, 300, |_, _| idle(), |_, ev| ev.iter().any(|e| matches!(e, Event::Loosed { .. })));
        let ev = run(&mut f, 400, |_, t| Input { step: if t == 2 { 1 } else { 0 }, ..idle() }, |f, _| f.missiles.is_empty());
        assert!(hurt(&ev), "it turned after the hero");
        let mut f = hex();
        run(&mut f, 300, |_, _| idle(), |_, ev| ev.iter().any(|e| matches!(e, Event::Loosed { .. })));
        run(&mut f, 400, |_, _| idle(), |f, _| f.missiles.first().is_none_or(|m| (m.s as f32) - HERO_D < HOMING_UNTIL - 0.5));
        let ev = run(&mut f, 400, |_, t| Input { step: if t == 0 { 1 } else { 0 }, ..idle() }, |f, _| f.missiles.is_empty());
        assert!(!hurt(&ev), "too late to turn");
        // The rune: no guard holds it; off its lane once it has locked on, it misses.
        let mut f = attacking(Fight::new(vec![of(Archetype::Hexer, 8.0, 0)]), 1);
        let ev = run(&mut f, 200, |_, _| Input { guard: true, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Burst { .. })));
        assert!(hurt(&ev));
        let mut f = attacking(Fight::new(vec![of(Archetype::Hexer, 8.0, 0)]), 1);
        let lock = (Archetype::Hexer.attacks()[1].frames.0 as f32 * 0.72) as u32;
        let ev = run(&mut f, 200, |_, t| Input { step: if t == lock { 1 } else { 0 }, ..idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Burst { .. })));
        assert!(!hurt(&ev));
    }

    #[test]
    fn the_ranged_kinds_hold_their_distance_and_give_ground_slower_than_the_hero_walks() {
        let gap = |f: &Fight| f.enemies[0].gap(0.0);
        let mut near = Fight::new(vec![Enemy { ai: Ai::Wait(1000), ..of(Archetype::Archer, 4.0, 0) }]);
        run(&mut near, 60, |_, _| idle(), |_, _| false);
        assert!(gap(&near) > 4.4 && gap(&near) < 4.0 + RUN, "backs off, slower than a walk: {}", gap(&near));
        let mut far = Fight::new(vec![Enemy { ai: Ai::Wait(1000), ..of(Archetype::Archer, 15.0, 0) }]);
        run(&mut far, 60, |_, _| idle(), |_, _| false);
        assert!(gap(&far) < 14.6);
        // Close in, it kicks rather than shoots.
        let mut f = Fight::new(vec![of(Archetype::Archer, 2.0, 0)]);
        let mut first = None;
        run(&mut f, 400, |_, _| idle(), |_, ev| { first = ev.iter().find_map(|e| match e { Event::WindsUp { attack, .. } => Some(*attack), _ => None }); first.is_some() });
        assert_eq!(first, Some("kick"));
    }

    /// A hero holding `r` in the other hand (no shield), full.
    fn holding(r: crate::gear::Ranged, enemies: Vec<Enemy>) -> Fight {
        let mut f = Fight::new(enemies);
        f.hero.stats.kit.off = None;
        f.hero.stats.kit.ranged = Some(r);
        f.hero = Hero::with(f.hero.stats);
        f
    }
    fn press_loose() -> Input { Input { loose: true, ..idle() } }

    #[test]
    fn an_arrow_flies_down_the_heros_lane_strikes_the_first_foe_in_it_and_costs_one_from_the_quiver() {
        use crate::gear::Ranged;
        let mut f = holding(Ranged::Bow, vec![of(Archetype::Husk, 9.0, 1), of(Archetype::Husk, 12.0, 0)]);
        for e in &mut f.enemies { e.ai = Ai::Wait(10_000); }
        let ev = run(&mut f, 120, |_, t| if t == 0 { press_loose() } else { idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Hit { .. })));
        assert!(ev.contains(&Event::Shot(Ranged::Bow)) && ev.contains(&Event::Hit { enemy: 1, mv: Move::Loose }), "{ev:?}");
        assert_eq!(f.hero.arrows, QUIVER - 1);
        assert!(f.enemies[0].hp == Archetype::Husk.hp(), "the one in the next lane is untouched");
        // A hit in the draw cuts it off, and nothing is spent.
        let mut g = holding(Ranged::Bow, vec![of(Archetype::Husk, 2.0, 0)]);
        let late = Archetype::Husk.attacks()[0].frames.0 - 12;
        g.enemies[0].ai = Ai::Attack { i: 0, t: late, wide: 0, landed: false, aim: HERO_D as f64 };
        let ev = run(&mut g, 90, |_, t| if t == 0 { press_loose() } else { idle() }, |_, ev| hurt(ev));
        assert!(hurt(&ev) && !ev.iter().any(|e| matches!(e, Event::Shot(_))), "{ev:?}");
        assert_eq!(g.hero.arrows, QUIVER);
        // An empty quiver looses nothing.
        let mut h = holding(Ranged::Bow, Vec::new());
        h.hero.arrows = 0;
        let ev = run(&mut h, 60, |_, t| if t == 0 { press_loose() } else { idle() }, |_, _| false);
        assert!(h.hero.act.is_none() && !ev.iter().any(|e| matches!(e, Event::Began(Move::Loose))));
    }

    #[test]
    fn range_pays_less_than_melee_and_breaks_no_stance_but_frost() {
        use crate::gear::{Class, Ranged};
        let rate = |s: Spec| s.damage / (s.frames.0 + s.frames.1 + s.frames.2) as f32;
        let light = Move::Light.spec_for(Class::Sword, None);
        for r in [Ranged::Bow, Ranged::BloodBow, Ranged::Catalyst] {
            let l = Move::loose_spec(r);
            assert!(rate(l) < 0.5 * rate(light), "{r:?}: {} against {}", rate(l), rate(light));
            assert!(l.frames.0 >= 2 * light.frames.0, "{r:?} draws long");
            assert_eq!(l.armour, 0);
        }
        assert!(Move::loose_spec(Ranged::Bow).stance < 0.5 * light.stance);
        assert!(Move::loose_spec(Ranged::Catalyst).stance > light.stance, "frost cracks a stance");
        // The smith's levels raise a blow, not a shot.
        let mut st = Stats::default();
        let before = st.ranged_damage(Ranged::Bow);
        st.weapon = 5;
        assert_eq!(st.ranged_damage(Ranged::Bow), before);
        st.dexterity = 10;
        assert!(st.ranged_damage(Ranged::Bow) > before);
    }

    #[test]
    fn frost_charges_run_out_and_blows_landed_up_close_fill_them() {
        use crate::gear::Ranged;
        let mut f = holding(Ranged::Catalyst, Vec::new());
        assert_eq!(f.hero.charges, 3.0);
        for _ in 0..3 { run(&mut f, 100, |_, t| if t == 0 { press_loose() } else { idle() }, |f, _| f.hero.act.is_none()); f.hero.stamina = f.hero.stats.max_stamina(); }
        assert!(f.hero.charges < 1.0 && !f.hero.can_loose());
        // Three blows landed on a husk give one back.
        let mut g = holding(Ranged::Catalyst, vec![of(Archetype::Husk, 1.5, 0)]);
        g.hero.charges = 0.0;
        g.enemies[0].ai = Ai::Wait(10_000);
        g.enemies[0].hp = 1e6;
        for _ in 0..3 {
            run(&mut g, 80, |_, t| Input { light: t == 0, ..idle() }, |f, _| f.hero.act.is_none());
            g.hero.stamina = g.hero.stats.max_stamina();
        }
        assert!(g.hero.can_loose(), "charges {}", g.hero.charges);
    }

    #[test]
    fn a_raised_shield_catches_an_arrow_the_blood_bow_costs_health_and_the_fallen_give_arrows_back() {
        use crate::gear::Ranged;
        let mut f = holding(Ranged::Bow, vec![of(Archetype::Shieldbearer, 8.0, 0)]);
        f.enemies[0].ai = Ai::Wait(10_000);
        let ev = run(&mut f, 120, |_, t| if t == 0 { press_loose() } else { idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Glanced { .. } | Event::Hit { .. })));
        assert!(ev.contains(&Event::Glanced { enemy: 0 }) && f.enemies[0].hp == Archetype::Shieldbearer.hp(), "{ev:?}");
        let mut b = holding(Ranged::BloodBow, Vec::new());
        let hp = b.hero.hp;
        run(&mut b, 100, |_, t| if t == 0 { press_loose() } else { idle() }, |f, _| f.hero.act.is_none());
        assert_eq!(b.hero.hp, hp - BLOOD_COST);
        let mut k = holding(Ranged::Bow, vec![of(Archetype::Husk, 8.0, 0)]);
        k.enemies[0].ai = Ai::Wait(10_000);
        k.enemies[0].hp = 1.0;
        k.hero.arrows = 5;
        run(&mut k, 120, |_, t| if t == 0 { press_loose() } else { idle() }, |_, ev| ev.iter().any(|e| matches!(e, Event::Felled { .. })));
        assert_eq!(k.hero.arrows, 5 - 1 + SALVAGE);
    }
}
