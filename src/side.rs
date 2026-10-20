//! The side-on boss fight. At the gate the camera swings round to look across the road and the
//! fight runs along it: the hero walks left and right, rolls with Space and a direction (through
//! the boss, too, inside the roll's i-frames), jumps, strikes, guards and parries as on the road,
//! against one great enemy whose blows come high, low along the ground, or as a ring rolling out
//! across the stones. What jumping is for: a low sweep or the ring passes under a hero in the air.
//!
//! Positions are metres along the road from the arena's middle (`s`), and up (`y`). Events are
//! the road fight's own (`fight::Event`, the boss as enemy 0), so hitstop, callouts, bursts and
//! sound all follow it unchanged.

use crate::fight::{Event, Kind, Move, Reel, Stats, DOWN_FALL, PERFECT};

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
/// Half the hero's width and the boss's, metres; the boss's height.
const HERO_HALF: f32 = 0.3;
pub const BOSS_HALF: f32 = 0.95;
const BOSS_HEIGHT: f32 = 3.2;
const BOSS_HP: f32 = 520.0;
const BOSS_STANCE: f32 = 110.0;
/// Ticks the boss stays open after its stance breaks or a parry, and what a strike on it then does.
const OPEN: u32 = 150;
const OPEN_DAMAGE: f32 = 2.5;

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

/// One of the boss's blows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub name: &'static str,
    pub frames: (u32, u32, u32),
    /// Metres in front of the boss's middle it covers (from, to), and heights (from, to).
    pub reach: (f32, f32),
    pub height: (f32, f32),
    pub damage: f32,
    pub kind: Kind,
    pub parry: bool,
    /// Stamina a guard pays to hold it.
    pub guard: f32,
}

const fn blow(name: &'static str, frames: (u32, u32, u32), reach: (f32, f32), height: (f32, f32), damage: f32, kind: Kind, parry: bool, guard: f32) -> Blow {
    Blow { name, frames, reach, height, damage, kind, parry, guard }
}

/// The Bell Warden's blows. `knell`'s reach is the ring's: it rolls out both ways along the
/// ground from where the hammer struck, so it is placed by `ring` rather than by `reach`.
pub const BLOWS: [Blow; 4] = [
    // The bell-hammer brought down overhead: parry it.
    blow("toll", (48, 6, 42), (0.3, 3.3), (0.0, 3.6), 34.0, Kind::Physical, true, 40.0),
    // Swung low across the ground: jump it (or block, paying for it).
    blow("sweep", (40, 8, 38), (-0.7, 3.7), (0.0, 0.75), 24.0, Kind::GuardBreak, false, 45.0),
    // The hammer rung on the stones: a ring rolls out both ways, low. Jump it or roll through it.
    blow("knell", (62, 66, 40), (0.0, 0.0), (0.0, 0.55), 28.0, Kind::Unblockable, false, 0.0),
    // A charge across the yard, shoulder first: roll through it.
    blow("charge", (34, 42, 38), (-BOSS_HALF, BOSS_HALF), (0.0, 2.6), 30.0, Kind::GuardBreak, false, 50.0),
];
/// How fast the knell's ring rolls out, m/tick, and how wide its crest is.
const RING_V: f32 = 7.0 / TPS;
const RING_HALF: f32 = 0.35;
/// How fast the charge runs, m/tick.
const CHARGE_V: f32 = 8.5 / TPS;

/// What the boss is doing.
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
pub struct Boss {
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
}

impl Boss {
    pub fn max_hp() -> f32 { BOSS_HP }
    pub fn max_stance() -> f32 { BOSS_STANCE }
    pub fn alive(&self) -> bool { self.ai != Ai::Dead }
    /// Past half its health it is quicker, and follows a toll with a sweep.
    pub fn enraged(&self) -> bool { self.hp < 0.5 * BOSS_HP }
    /// The blow under way: which, its phase, how far through the windup (0..1).
    pub fn blow(&self) -> Option<(Blow, crate::fight::Phase, f32)> {
        let Ai::Blow { i, t, .. } = self.ai else { return None };
        let b = self.timed(i);
        let ph = if t < b.frames.0 { crate::fight::Phase::Windup } else if t < b.frames.0 + b.frames.1 { crate::fight::Phase::Active } else { crate::fight::Phase::Recover };
        Some((b, ph, (t as f32 / b.frames.0 as f32).min(1.0)))
    }
    /// Blow `i` as this boss throws it now (quicker windups when enraged).
    pub fn timed(&self, i: usize) -> Blow {
        let mut b = BLOWS[i];
        if self.enraged() { b.frames.0 = (b.frames.0 as f32 * 0.8) as u32; }
        b
    }
    /// The knell's ring now: how far out from where it struck, while live.
    pub fn ring(&self) -> Option<(f32, f32)> {
        let Ai::Blow { i: 2, t, from, .. } = self.ai else { return None };
        let b = self.timed(2);
        (t >= b.frames.0 && t < b.frames.0 + b.frames.1).then(|| (from + self.facing * 1.4, (t - b.frames.0) as f32 * RING_V))
    }
    /// Where on the ground the blow under way will land, (from, to) metres, for its warning.
    pub fn warning(&self) -> Option<(f32, f32, Kind, bool, f32)> {
        let (b, ph, progress) = self.blow()?;
        if ph == crate::fight::Phase::Recover { return None; }
        let Ai::Blow { i, .. } = self.ai else { return None };
        let span = match i {
            2 => { let (at, r) = self.ring().unwrap_or((self.s + self.facing * 1.4, 0.0)); (at - r - RING_HALF, at + r + RING_HALF) }
            3 => { let end = (self.s + self.facing * 2.0 * HALF).clamp(-HALF, HALF); (self.s.min(end), self.s.max(end)) }
            _ => { let (a, z) = (self.s + self.facing * b.reach.0, self.s + self.facing * b.reach.1); (a.min(z), a.max(z)) }
        };
        Some((span.0, span.1, b.kind, b.parry, progress))
    }
}

/// The whole side-on fight.
#[derive(Clone, Debug, PartialEq)]
pub struct SideFight {
    pub hero: SideHero,
    pub boss: Boss,
    pub ticks: u64,
    /// A count that stands in for chance.
    seed: u32,
}

impl SideFight {
    /// The hero, as the road left them, at the yard's left; the boss at its right, facing them.
    pub fn new(stats: Stats, hp: f32, stamina: f32, flasks: u32) -> SideFight {
        SideFight {
            hero: SideHero {
                stats, hp, stamina, rested: 30, flasks, s: -HALF + 1.5, y: 0.0, vy: 0.0, facing: 1.0, act: None, guarding: false,
                reel: Reel::None, reeling: 0, poise: stats.poise(), poise_wait: 0, from: 0.0, landed: false, dead: false, walking: false, buffer: None,
            },
            boss: Boss { s: HALF - 2.0, facing: -1.0, hp: BOSS_HP, stance: BOSS_STANCE, stance_wait: 0, ai: Ai::Wait(90), hurt: 0, count: 0 },
            ticks: 0,
            seed: 0x51DE,
        }
    }

    pub fn won(&self) -> bool { !self.boss.alive() }

    fn roll(&mut self, n: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.seed >> 16) % n.max(1)
    }

    pub fn tick(&mut self, input: &SideInput) -> Vec<Event> {
        let mut out = Vec::new();
        self.ticks += 1;
        if !self.hero.dead { self.hero_tick(input, &mut out); }
        if self.boss.alive() { self.boss_tick(&mut out); }
        if self.hero.hp <= 0.0 && !self.hero.dead {
            self.hero.dead = true;
            self.hero.hp = 0.0;
            out.push(Event::Died);
        }
        out
    }

    fn hero_tick(&mut self, input: &SideInput, out: &mut Vec<Event>) {
        let boss_s = self.boss.s;
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
            } else if dir == 0 && !h.airborne() {
                // Standing, they face the boss.
                h.facing = if boss_s >= h.s { 1.0 } else { -1.0 };
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
        // The yard's walls; and the boss's body, which only a roll passes through.
        let h = &mut self.hero;
        h.s = h.s.clamp(-HALF + HERO_HALF, HALF - HERO_HALF);
        let rolling = matches!(h.act, Some((Act::Roll(_), _)));
        let gap = h.s - self.boss.s;
        if self.boss.alive() && !rolling && h.y < BOSS_HEIGHT * 0.8 && gap.abs() < HERO_HALF + BOSS_HALF {
            let side = if gap == 0.0 { -self.boss.facing } else { gap.signum() };
            let s = self.boss.s + side * (HERO_HALF + BOSS_HALF);
            h.s = if s.abs() > HALF - HERO_HALF { self.boss.s - side * (HERO_HALF + BOSS_HALF) } else { s };
        }
    }

    /// The hero's strike `a` in its live frames: does it reach the boss?
    fn strike(&mut self, a: Act, out: &mut Vec<Event>) {
        let (reach, damage, stance) = match a { Act::Heavy => (2.0, 26.0, 22.0), _ => (1.7, 10.0, 8.0) };
        let h = &mut self.hero;
        let b = &mut self.boss;
        let ahead = (b.s - h.s) * h.facing;
        if !b.alive() || ahead < -BOSS_HALF || ahead > reach + BOSS_HALF { return; }
        h.landed = true;
        let open = matches!(b.ai, Ai::Open(_));
        b.hp -= damage * h.stats.damage() * if open { OPEN_DAMAGE } else { 1.0 };
        b.hurt = 10;
        b.stance_wait = 0;
        out.push(Event::Hit { enemy: 0, mv: a.as_move() });
        if b.hp <= 0.0 {
            b.hp = 0.0;
            b.ai = Ai::Dead;
            out.push(Event::Felled { enemy: 0 });
            return;
        }
        if !open {
            b.stance -= stance;
            if b.stance <= 0.0 {
                b.stance = BOSS_STANCE;
                b.ai = Ai::Open(OPEN);
                out.push(Event::StanceBroken { enemy: 0 });
            }
        }
    }

    fn boss_tick(&mut self, out: &mut Vec<Event>) {
        let hero_s = self.hero.s;
        let b = &mut self.boss;
        b.hurt = b.hurt.saturating_sub(1);
        b.stance_wait += 1;
        if b.stance_wait > 120 { b.stance = (b.stance + 0.4).min(BOSS_STANCE); }
        match b.ai {
            Ai::Dead => {}
            Ai::Open(0) => b.ai = Ai::Wait(30),
            Ai::Open(n) => b.ai = Ai::Open(n - 1),
            Ai::Wait(n) => {
                b.facing = if hero_s >= b.s { 1.0 } else { -1.0 };
                let gap = (hero_s - b.s).abs();
                // Closing in at a walk while it waits.
                if gap > 3.0 { b.s += b.facing * 1.3 / TPS; }
                if n == 0 {
                    let near = gap < 3.4;
                    let pick = self.roll(10);
                    let b = &mut self.boss;
                    let i = match (near, pick) { (true, 0..=4) => 0, (true, 5..=8) => 1, (true, _) => 2, (false, 0..=5) => 3, (false, _) => 2 };
                    b.count += 1;
                    b.ai = Ai::Blow { i, t: 0, hit: false, from: b.s };
                    out.push(Event::WindsUp { enemy: 0, attack: BLOWS[i].name });
                } else {
                    b.ai = Ai::Wait(n - 1);
                }
            }
            Ai::Blow { i, t, hit, from } => {
                let blow = b.timed(i);
                let (w, live, rec) = blow.frames;
                if t == ((w as f32) * 0.7).ceil() as u32 { out.push(Event::Commits { enemy: 0, kind: blow.kind, parry: blow.parry }); }
                // The charge runs while live, and stops at the wall.
                if i == 3 && t >= w && t < w + live { b.s = (b.s + b.facing * CHARGE_V).clamp(-HALF + BOSS_HALF, HALF - BOSS_HALF); }
                let live_now = t >= w && t < w + live;
                let mut hit = hit;
                if live_now && !hit { hit = self.blow_lands(i, blow, out); }
                let b = &mut self.boss;
                b.ai = if t + 1 >= w + live + rec {
                    // Enraged, a toll comes back as a sweep.
                    if i == 0 && b.enraged() && b.count % 2 == 1 {
                        b.count += 1;
                        out.push(Event::WindsUp { enemy: 0, attack: BLOWS[1].name });
                        Ai::Blow { i: 1, t: 12, hit: false, from: b.s }
                    } else {
                        Ai::Wait(if b.enraged() { 28 } else { 46 } + self.seed % 20)
                    }
                } else if matches!(b.ai, Ai::Open(_)) { b.ai } else {
                    Ai::Blow { i, t: t + 1, hit, from }
                };
            }
        }
    }

    /// Blow `i` live now: does it meet the hero, and how? True once it has struck (one hit a blow).
    fn blow_lands(&mut self, i: usize, blow: Blow, out: &mut Vec<Event>) -> bool {
        let b = &self.boss;
        let h = &self.hero;
        let reaches = match i {
            2 => self.boss.ring().is_some_and(|(at, r)| ((h.s - at).abs() - r).abs() < RING_HALF + HERO_HALF),
            3 => (h.s - b.s).abs() < BOSS_HALF + HERO_HALF + 0.1,
            _ => { let ahead = (h.s - b.s) * b.facing; ahead >= blow.reach.0 - HERO_HALF && ahead <= blow.reach.1 + HERO_HALF }
        };
        let high_enough = h.y + 1.7 >= blow.height.0 && h.y <= blow.height.1;
        if !reaches || !high_enough || h.dead { return false; }
        if h.invulnerable() {
            if matches!(h.act, Some((Act::Roll(_), t)) if t <= PERFECT) { out.push(Event::JustDodge { enemy: 0 }); }
            return false;
        }
        let toward = if self.boss.s >= h.s { 1.0 } else { -1.0 };
        let facing_it = h.facing == toward;
        let h = &mut self.hero;
        // A parry, live (and the kit's extra ticks), meets a blow that can be parried.
        if let Some((Act::Parry, t)) = h.act {
            let f = Act::Parry.frames();
            let live = t >= f.0 && t < f.0 + f.1 + h.stats.kit.parry;
            if live && blow.parry && facing_it {
                h.act = None;
                out.push(Event::Parried { enemy: 0 });
                self.boss.ai = Ai::Open(OPEN);
                out.push(Event::StanceBroken { enemy: 0 });
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
        // Knocked away from the boss, and out of the air.
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
    /// The boss throwing blow `i` at once, `gap` metres in front of the hero.
    fn throwing(i: usize, gap: f32) -> SideFight {
        let mut f = fight();
        f.boss.s = f.hero.s + gap;
        f.boss.ai = Ai::Blow { i, t: 0, hit: false, from: f.boss.s };
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
        f.boss.s = f.hero.s + 1.5;
        f.boss.ai = Ai::Wait(500);
        let start = f.hero.s;
        f.tick(&SideInput { roll: true, right: true, ..idle() });
        run(&mut f, 60, SideInput { right: true, ..idle() }, |f, _| f.hero.act.is_none());
        assert!(f.hero.s > f.boss.s, "rolled through to the far side: {} vs boss {}", f.hero.s, f.boss.s);
        // Walking does not.
        let mut g = fight();
        g.boss.s = g.hero.s + 1.5;
        g.boss.ai = Ai::Wait(500);
        run(&mut g, 120, SideInput { right: true, ..idle() }, |_, _| false);
        assert!(g.hero.s < g.boss.s, "walked into its body and stopped");
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
        assert!(matches!(f.boss.ai, Ai::Open(_)));
        let hp = f.boss.hp;
        f.tick(&SideInput { light: true, ..idle() });
        run(&mut f, 30, idle(), |f, _| f.boss.hp < hp);
        assert!(hp - f.boss.hp > 20.0, "an open boss takes a strike two and a half times over: {}", hp - f.boss.hp);
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
        f.boss.s = f.hero.s + 1.5;
        f.boss.ai = Ai::Wait(u32::MAX / 2);
        f.boss.hp = 15.0;
        f.tick(&SideInput { heavy: true, ..idle() });
        assert!(run(&mut f, 60, idle(), |_, e| e.contains(&Event::Felled { enemy: 0 })));
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
