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

use crate::fight::{Archetype, Event, Kind, Move, Reel, Stats, DOWN_FALL, PERFECT};

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
    /// Held.
    pub guard: bool,
}

/// What the hero is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act { Light, Heavy, Roll(i8), Parry, Flask }

impl Act {
    /// Windup, live and recovery ticks.
    pub fn frames(self) -> (u32, u32, u32) {
        match self {
            Act::Light => (12, 4, 16),
            Act::Heavy => (28, 6, 26),
            Act::Roll(_) => (0, ROLL_T, 0),
            Act::Parry => Move::Parry.spec().frames,
            Act::Flask => Move::Flask.spec().frames,
        }
    }
    fn total(self) -> u32 { let f = self.frames(); f.0 + f.1 + f.2 }
    /// As the road fight costs and names it.
    fn as_move(self) -> Move {
        match self { Act::Light => Move::Light, Act::Heavy => Move::Heavy, Act::Roll(d) => Move::Step(d as i32), Act::Parry => Move::Parry, Act::Flask => Move::Flask }
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
}

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
}

#[allow(clippy::too_many_arguments)]
const fn blow(name: &'static str, frames: (u32, u32, u32), reach: (f32, f32), height: (f32, f32), damage: f32, kind: Kind, parry: bool, guard: f32, motion: Motion) -> Blow {
    Blow { name, frames, reach, height, damage, kind, parry, guard, motion }
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
    /// Which blow next: given whether the hero is within `near` metres and a roll of 0..10.
    pub pick: fn(bool, u32) -> usize,
    pub near: f32,
    /// How fast it closes in while it waits, m/s, and from how far off it stops.
    pub walk: f32,
    pub keep: f32,
    /// Ticks it squares up between blows (plus up to 20 by chance), and how long it stays open.
    pub rest: u32,
    pub open: u32,
    /// Whether past half its health it is quicker and follows its first blow with its second.
    pub enrages: bool,
}

/// How fast a ring rolls out, m/tick, and how wide its crest is.
const RING_V: f32 = 7.0 / TPS;
const RING_HALF: f32 = 0.35;

/// The Bell Warden's blows.
pub const BLOWS: [Blow; 4] = [
    // The bell-hammer brought down overhead: parry it.
    blow("toll", (48, 6, 42), (0.3, 3.3), (0.0, 3.6), 34.0, Kind::Physical, true, 40.0, Motion::Strike),
    // Swung low across the ground: jump it (or block, paying for it).
    blow("sweep", (40, 8, 38), (-0.7, 3.7), (0.0, 0.75), 24.0, Kind::GuardBreak, false, 45.0, Motion::Strike),
    // The hammer rung on the stones: a ring rolls out both ways, low. Jump it or roll through it.
    blow("knell", (62, 66, 40), (0.0, 0.0), (0.0, 0.55), 28.0, Kind::Unblockable, false, 0.0, Motion::Ring),
    // A charge across the yard, shoulder first: roll through it.
    blow("charge", (34, 42, 38), (-0.95, 0.95), (0.0, 2.6), 30.0, Kind::GuardBreak, false, 50.0, Motion::Charge(8.5 / TPS)),
];

/// The boss at the gate.
pub static WARDEN: Breed = Breed {
    name: "The Bell Warden", kind: None, hp: 520.0, stance: 110.0, half: 0.95, height: 3.2, blows: &BLOWS,
    pick: |near, r| match (near, r) { (true, 0..=4) => 0, (true, 5..=8) => 1, (true, _) => 2, (false, 0..=5) => 3, (false, _) => 2 },
    near: 3.4, walk: 1.3, keep: 3.0, rest: 46, open: 150, enrages: true,
};

/// The breed a road enemy fights as when it springs an ambush (shieldbearers do not).
pub fn breed_of(kind: Archetype) -> &'static Breed {
    match kind { Archetype::Lancer => &LANCER, Archetype::Wretch => &WRETCH, _ => &HUSK }
}

/// The road's dead, as they spring an ambush: weaker than the Warden and quicker to fall, each
/// with one blow to parry or block and one to jump or roll.
pub static HUSK: Breed = Breed {
    name: "Husk", kind: Some(Archetype::Husk), hp: 70.0, stance: 34.0, half: 0.4, height: 1.9,
    blows: &[
        blow("chop", (34, 5, 30), (0.2, 1.6), (0.0, 2.2), 16.0, Kind::Physical, true, 22.0, Motion::Strike),
        blow("sweep", (30, 6, 28), (-0.3, 1.8), (0.0, 0.7), 14.0, Kind::GuardBreak, false, 26.0, Motion::Strike),
    ],
    pick: |_, r| if r < 6 { 0 } else { 1 }, near: 1.8, walk: 0.9, keep: 1.3, rest: 70, open: 110, enrages: false,
};
pub static LANCER: Breed = Breed {
    name: "Lancer", kind: Some(Archetype::Lancer), hp: 62.0, stance: 30.0, half: 0.35, height: 1.9,
    blows: &[
        blow("thrust", (28, 4, 26), (0.3, 2.6), (0.6, 1.8), 15.0, Kind::Physical, true, 20.0, Motion::Strike),
        blow("lunge", (32, 22, 30), (-0.4, 0.4), (0.0, 1.9), 18.0, Kind::GuardBreak, false, 30.0, Motion::Charge(7.5 / TPS)),
    ],
    pick: |near, r| if near && r < 7 { 0 } else { 1 }, near: 2.6, walk: 1.1, keep: 2.2, rest: 64, open: 100, enrages: false,
};
pub static WRETCH: Breed = Breed {
    name: "Wretch", kind: Some(Archetype::Wretch), hp: 44.0, stance: 22.0, half: 0.35, height: 1.5,
    blows: &[
        blow("rake", (18, 6, 22), (0.1, 1.3), (0.0, 1.6), 10.0, Kind::Physical, false, 16.0, Motion::Strike),
        blow("pounce", (26, 18, 28), (-0.4, 0.4), (0.0, 1.4), 14.0, Kind::GuardBreak, false, 24.0, Motion::Charge(8.0 / TPS)),
    ],
    pick: |near, r| if near && r < 6 { 0 } else { 1 }, near: 1.4, walk: 1.8, keep: 1.0, rest: 48, open: 90, enrages: false,
};

/// What a foe is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    /// Squaring up for this many ticks.
    Wait(u32),
    /// Blow `i`, `t` ticks in; `hit` once it has struck the hero; `from` where it began.
    Blow { i: usize, t: u32, hit: bool, from: f32 },
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
}

impl Foe {
    /// A foe of `breed` at `s`, facing `facing`, squaring up for `wait` ticks.
    pub fn new(breed: &'static Breed, s: f32, facing: f32, wait: u32) -> Foe {
        Foe { breed, s, facing, hp: breed.hp, stance: breed.stance, stance_wait: 0, ai: Ai::Wait(wait), hurt: 0, count: 0, down: 0 }
    }
    pub fn max_hp(&self) -> f32 { self.breed.hp }
    pub fn max_stance(&self) -> f32 { self.breed.stance }
    pub fn alive(&self) -> bool { self.ai != Ai::Dead }
    /// A breed that enrages, past half its health: quicker, and its first blow followed by its second.
    pub fn enraged(&self) -> bool { self.breed.enrages && self.hp < 0.5 * self.breed.hp }
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
        };
        Some((span.0, span.1, b.kind, b.parry, progress))
    }
}

/// The whole side-on fight.
#[derive(Clone, Debug, PartialEq)]
pub struct SideFight {
    pub hero: SideHero,
    pub foes: Vec<Foe>,
    pub ticks: u64,
    /// A count that stands in for chance.
    seed: u32,
}

impl SideFight {
    /// The boss fight: the hero, as the road left them, at the yard's left; the Warden at its
    /// right, facing them.
    pub fn new(stats: Stats, hp: f32, stamina: f32, flasks: u32) -> SideFight {
        Self::with(stats, hp, stamina, flasks, -HALF + 1.5, vec![Foe::new(&WARDEN, HALF - 2.0, -1.0, 90)])
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
            },
            foes,
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
        h.walking = false;
        // Presses wait a moment for the hero to be free, as on the road.
        if input.roll || input.jump || input.light || input.heavy || input.parry || input.flask { h.buffer = Some((*input, 0)); }
        else if let Some((b, age)) = h.buffer { h.buffer = (age < crate::fight::BUFFER).then_some((b, age + 1)); }
        let p = h.buffer.map(|b| b.0).unwrap_or_default();
        let pdir = (p.right as i8) - (p.left as i8);
        // What was pressed, if the hero is free to do it (in the air: only a strike).
        let want = if p.roll { Some(Act::Roll(if pdir != 0 { pdir } else if dir != 0 { dir } else { h.facing as i8 })) }
            else if p.flask { Some(Act::Flask) } else if p.parry { Some(Act::Parry) }
            else if p.heavy { Some(Act::Heavy) } else if p.light { Some(Act::Light) } else { None };
        if h.free() {
            if p.jump && !h.airborne() && !h.guarding {
                h.buffer = None;
                if h.stamina <= 0.0 { out.push(Event::Spent); } else { h.spend(h.stats.cost(Move::Backstep, JUMP_COST)); h.vy = JUMP_V; h.y = 1e-4; }
            } else if let Some(a) = want.filter(|a| !h.airborne() || matches!(a, Act::Light | Act::Heavy)) {
                h.buffer = None;
                let cost = h.stats.cost(a.as_move(), match a { Act::Light => 12.0, Act::Heavy => 28.0, Act::Roll(_) => 12.0, _ => 0.0 });
                if cost > 0.0 && h.stamina <= 0.0 {
                    out.push(Event::Spent);
                } else if a == Act::Flask && h.flasks == 0 {
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
            let f = a.frames();
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
                _ => {}
            }
            h.act = (t + 1 < a.total()).then_some((a, t + 1));
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
        let (reach, cleave) = match a { Act::Heavy => (2.0, true), _ => (1.7, false) };
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
    fn hit(&mut self, i: usize, a: Act, out: &mut Vec<Event>) {
        let (damage, stance) = match a { Act::Heavy => (26.0, 22.0), _ => (10.0, 8.0) };
        let h = &self.hero;
        let b = &mut self.foes[i];
        let open = matches!(b.ai, Ai::Open(_));
        b.hp -= damage * h.stats.damage() * if open { OPEN_DAMAGE } else { 1.0 };
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
        let charge_held = charge != 0.0 && blocked(charge);
        let b = &mut self.foes[i];
        b.hurt = b.hurt.saturating_sub(1);
        b.stance_wait += 1;
        if b.stance_wait > 120 { b.stance = (b.stance + 0.4).min(b.breed.stance); }
        match b.ai {
            Ai::Dead => {}
            Ai::Open(0) => b.ai = Ai::Wait(30),
            Ai::Open(n) => b.ai = Ai::Open(n - 1),
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
                    let k = (b.breed.pick)(near, pick).min(b.breed.blows.len() - 1);
                    b.count += 1;
                    b.ai = Ai::Blow { i: k, t: 0, hit: false, from: b.s };
                    out.push(Event::WindsUp { enemy: i, attack: b.breed.blows[k].name });
                } else {
                    b.ai = Ai::Wait(n - 1);
                }
            }
            Ai::Blow { i: k, t, hit, from } => {
                let blow = b.timed(k);
                let (w, live, rec) = blow.frames;
                if t == ((w as f32) * 0.7).ceil() as u32 { out.push(Event::Commits { enemy: i, kind: blow.kind, parry: blow.parry }); }
                // A charge runs while live, and stops at the wall.
                if let Motion::Charge(v) = blow.motion {
                    if t >= w && t < w + live && !charge_held { b.s = (b.s + b.facing * v).clamp(-HALF + b.breed.half, HALF - b.breed.half); }
                }
                let live_now = t >= w && t < w + live;
                let mut hit = hit;
                if live_now && !hit { hit = self.blow_lands(i, blow, out); }
                let seed = self.seed;
                let b = &mut self.foes[i];
                b.ai = if t + 1 >= w + live + rec {
                    // Enraged, its first blow comes back as its second.
                    if k == 0 && b.enraged() && b.count % 2 == 1 && b.breed.blows.len() > 1 {
                        b.count += 1;
                        out.push(Event::WindsUp { enemy: i, attack: b.breed.blows[1].name });
                        Ai::Blow { i: 1, t: 12, hit: false, from: b.s }
                    } else {
                        Ai::Wait(if b.enraged() { b.breed.rest * 3 / 5 } else { b.breed.rest } + seed % 20)
                    }
                } else if matches!(b.ai, Ai::Open(_)) { b.ai } else {
                    Ai::Blow { i: k, t: t + 1, hit, from }
                };
            }
        }
    }

    /// Foe `i`'s blow live now: does it meet the hero, and how? True once it has struck (one hit a
    /// blow).
    fn blow_lands(&mut self, i: usize, blow: Blow, out: &mut Vec<Event>) -> bool {
        let b = &self.foes[i];
        let h = &self.hero;
        let reaches = match blow.motion {
            Motion::Ring => b.ring().is_some_and(|(at, r)| ((h.s - at).abs() - r).abs() < RING_HALF + HERO_HALF),
            Motion::Charge(_) => (h.s - b.s).abs() < b.breed.half + HERO_HALF + 0.1,
            Motion::Strike => { let ahead = (h.s - b.s) * b.facing; ahead >= blow.reach.0 - HERO_HALF && ahead <= blow.reach.1 + HERO_HALF }
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
            let f = Act::Parry.frames();
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
            let cost = blow.guard * h.stats.kit.guard * if blow.kind == Kind::GuardBreak { 1.5 } else { 1.0 };
            let short = cost > h.stamina;
            h.spend(cost);
            if short {
                let damage = blow.damage * 0.5 * h.stats.taken();
                h.hp -= damage;
                h.reel(Reel::GuardBreak);
                out.push(Event::GuardBroken { damage });
            } else if blow.kind == Kind::GuardBreak {
                let chip = blow.damage * 0.15 * h.stats.kit.chip * h.stats.taken();
                h.hp -= chip;
                h.reel(Reel::ChipJolt);
                out.push(Event::Blocked { chip });
            } else {
                h.reel(Reel::BlockJolt);
                out.push(Event::Blocked { chip: 0.0 });
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
            h.reel(if heavy { Reel::Recoil } else { Reel::Flinch });
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
        f.foes[0].ai = Ai::Blow { i, t: 0, hit: false, from: f.foes[0].s };
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
            assert!(w.s - l.s > l.breed.half + w.breed.half, "the wretch stays behind the lancer: {} {}", l.s, w.s);
            false
        });
        // The hero is knocked about the yard; wherever they end up, the foes have closed in on them.
        assert!(f.foes.iter().all(|x| (x.s - f.hero.s).abs() < 4.0), "closed in on {}: {:?}", f.hero.s, f.foes.iter().map(|x| x.s).collect::<Vec<_>>());
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
}
