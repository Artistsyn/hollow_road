//! The fight in road space: where the hero and the enemies stand along the road and across it,
//! and what every move does, tick by tick (60 a second). No drawing and no engine here, so it is
//! tested on its own; `game` reads it each tick to place sprites, paint warnings and drive the road.
//!
//! Positions along the road are metres walked (`WalkState::walked`, which never comes round):
//! something at `s` stands `s - walked` metres ahead of the camera. Across the road, `lane` -1, 0,
//! 1 is left, middle, right, `LANE_W` metres apart.
//!
//! Frame data starts from FromSoftware's (doubled from their 30 fps): a dodge is invulnerable for
//! 26 ticks, a parry is live for 10 after 6 of start-up, a normal enemy's stance is 15-65 and
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
/// A lane dodge carries the hero across in this many ticks.
pub const LANE_TICKS: f32 = 9.0;

pub const HP: f32 = 100.0;
pub const STAMINA: f32 = 100.0;
/// Stamina back per tick once `REGEN_DELAY` ticks have passed since it was last spent; a third
/// of that while guarding.
const REGEN: f32 = 55.0 / 60.0;
const REGEN_DELAY: u32 = 30;
/// A press waits this many ticks for the hero to be free, then is dropped. Only the last is kept.
pub const BUFFER: u32 = 18;
pub const FLASKS: u32 = 3;
const FLASK_HEAL: f32 = 40.0;
/// A dodge that began this few ticks before an attack would have struck is a just dodge.
const JUST: u32 = 14;
/// A step this young turns into a roll when a dodge is pressed (a direction, then the dodge key).
const STEP_TO_ROLL: u32 = 6;
/// Running pace, m/s, and its stamina per tick.
pub const RUN: f32 = 4.0;
const RUN_COST: f32 = 12.0 / 60.0;
/// An enemy follows the hero's lane through this much of its wind-up, then commits.
const COMMIT: f32 = 0.7;
/// Ticks an enemy takes to shift one lane while it aims.
const SHIFT: u32 = 16;
/// How long a broken stance leaves an enemy open, ticks.
const STAGGER: u32 = 150;

/// One tick's input, already read from whatever device. `forward`, `back`, `run` and `guard`
/// are held; the rest are presses this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    /// Walk at a run (out of a fight's reach; it costs stamina).
    pub run: bool,
    pub guard: bool,
    /// A step one lane across: -1 left, 1 right, 0 none.
    pub step: i32,
    /// A dodge: Some(-1 or 1) rolls a lane across, Some(0) hops back.
    pub dodge: Option<i32>,
    pub light: bool,
    pub heavy: bool,
    pub parry: bool,
    pub flask: bool,
}

impl Input {
    /// The press this tick, if any (the last of them wins).
    fn press(&self) -> Option<Move> {
        let mut m = None;
        if self.step != 0 { m = Some(Move::Step(self.step.signum())); }
        if let Some(d) = self.dodge { m = Some(Move::Dodge(d.signum())); }
        if self.flask { m = Some(Move::Flask); }
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
    /// A plain step one lane across (-1, 1): quick, free, and no protection.
    Step(i32),
    /// A roll one lane across (-1, 1), or a hop back (0): invulnerable for most of it.
    Dodge(i32),
    Parry,
    Flask,
    /// The strike on an enemy whose stance is broken (a light or heavy in reach becomes this).
    Riposte,
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
}

impl Move {
    pub fn spec(self) -> Spec {
        let s = |frames, stamina, iframes, reach, damage, stance| Spec { frames, stamina, iframes, reach, damage, stance };
        match self {
            Move::Light => s((14, 4, 16), 12.0, None, 2.6, 10.0, 8.0),
            Move::Heavy => s((30, 6, 28), 28.0, None, 2.9, 26.0, 22.0),
            Move::Step(_) => s((2, 8, 4), 0.0, None, 0.0, 0.0, 0.0),
            Move::Dodge(0) => s((2, 16, 6), 16.0, Some((2, 18)), 0.0, 0.0, 0.0),
            Move::Dodge(_) => s((3, 26, 4), 18.0, Some((3, 29)), 0.0, 0.0, 0.0),
            Move::Parry => s((6, 10, 22), 0.0, None, 0.0, 0.0, 0.0),
            Move::Flask => s((36, 1, 17), 0.0, None, 0.0, 0.0, 0.0),
            Move::Riposte => s((18, 4, 28), 0.0, Some((0, 50)), 2.9, 60.0, 0.0),
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
    /// Ticks since it began.
    pub t: u32,
    /// Something was struck already (one hit per strike).
    pub landed: bool,
    /// The lane the hero was in when it began (a just dodge is judged against it).
    pub from_lane: i32,
}

impl Act {
    pub fn phase(&self) -> Phase { phase(self.mv.spec().frames, self.t) }
    fn invulnerable(&self) -> bool { self.mv.spec().iframes.is_some_and(|(a, b)| (a..b).contains(&self.t)) }
    fn total(&self) -> u32 { let (a, b, c) = self.mv.spec().frames; a + b + c }
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
}

/// The kinds of enemy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    /// Slow approach, an overhead chop and a sweep across two lanes: teaches dodge and parry.
    Husk,
}

impl Archetype {
    pub fn hp(self) -> f32 { match self { Archetype::Husk => 80.0 } }
    pub fn stance(self) -> f32 { match self { Archetype::Husk => 30.0 } }
    /// Approach pace, m/s, and how far off it notices the hero, metres.
    fn pace(self) -> (f32, f32) { match self { Archetype::Husk => (0.9, 9.0) } }
    pub fn attacks(self) -> &'static [Attack] {
        const HUSK: [Attack; 2] = [
            Attack { name: "chop", frames: (42, 6, 40), reach: 2.9, wide: 0, damage: 28.0, guard: 30.0, kind: Kind::Physical, parry: true },
            Attack { name: "sweep", frames: (50, 8, 44), reach: 2.8, wide: 1, damage: 20.0, guard: 45.0, kind: Kind::GuardBreak, parry: false },
        ];
        match self { Archetype::Husk => &HUSK }
    }
    /// Ticks it waits between attacks, at least and at most.
    fn rest(self) -> (u32, u32) { match self { Archetype::Husk => (50, 110) } }
}

/// What an enemy is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ai {
    /// Closing in or waiting `n` more ticks before its next attack.
    Wait(u32),
    /// Attack `i` of its archetype, `t` ticks in; `wide` is the side it covers this time.
    Attack { i: usize, t: u32, wide: i32, landed: bool },
    /// Stance broken: open to a riposte for `n` more ticks.
    Staggered(u32),
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
}

impl Enemy {
    pub fn new(kind: Archetype, s: f64, lane: i32) -> Enemy {
        Enemy { kind, s, lane, hp: kind.hp(), stance: kind.stance(), stance_wait: 0, ai: Ai::Wait(30), hurt: 0 }
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
    pub phase: Phase,
    /// How far through its wind-up, 0..1.
    pub progress: f32,
}

/// What a tick did that the game should show or sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The hero began a move.
    Began(Move),
    /// A strike landed on enemy `enemy`.
    Hit { enemy: usize, mv: Move },
    /// Its stance broke: it is open to a riposte.
    StanceBroken { enemy: usize },
    Felled { enemy: usize },
    /// A strike's live frames passed and struck nothing.
    Whiffed(Move),
    /// An enemy began an attack.
    WindsUp { enemy: usize, attack: &'static str },
    /// The hero took a hit.
    Struck { damage: f32 },
    /// A guard stopped a hit (for this much chip damage), or gave way.
    Blocked { chip: f32 },
    GuardBroken,
    Parried { enemy: usize },
    /// A dodge slipped a hit at the last moment.
    JustDodge,
    Healed,
    /// Wanted to move with no stamina.
    Spent,
    Died,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hero {
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
    /// Ticks left reeling from a hit, unable to act.
    pub reeling: u32,
    /// The last press, and how many ticks it has waited.
    pub buffer: Option<(Move, u32)>,
    pub dead: bool,
}

impl Hero {
    fn new() -> Hero {
        Hero { hp: HP, stamina: STAMINA, rested: REGEN_DELAY, flasks: FLASKS, lane: 0, x: 0.0, act: None, guarding: false, reeling: 0, buffer: None, dead: false }
    }
    fn free(&self) -> bool { self.act.is_none() && self.reeling == 0 && !self.dead }
    fn spend(&mut self, n: f32) { if n > 0.0 { self.stamina = (self.stamina - n).max(0.0); self.rested = 0; } }
    /// The lane the hero is in for being struck: where they are now on the way across.
    fn standing_lane(&self) -> i32 { (self.x / LANE_W).round() as i32 }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fight {
    pub hero: Hero,
    pub enemies: Vec<Enemy>,
    rng: u64,
}

impl Fight {
    pub fn new(enemies: Vec<Enemy>) -> Fight { Fight { hero: Hero::new(), enemies, rng: 0x9E37_79B9_7F4A_7C15 } }

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

    /// How fast the road walks this tick, m/s: forward unless an enemy stands within `ENGAGE`,
    /// back when asked, half pace behind a guard, a quick hop back, and not at all mid-move.
    pub fn speed(&self, input: &Input, walked: f64) -> f32 {
        let h = &self.hero;
        if h.dead || h.reeling > 0 { return 0.0; }
        if let Some(a) = h.act {
            return if a.mv == Move::Dodge(0) && a.t < 18 { -3.6 } else { 0.0 };
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
        self.enemies.iter().enumerate().filter_map(|(i, e)| {
            let (a, phase, progress) = e.attack()?;
            let d = e.ahead(walked);
            Some(Threat { enemy: i, lanes: e.lanes(), d: (d - a.reach, d), kind: a.kind, phase, progress })
        }).collect()
    }

    /// One tick at 60 Hz, with the road `walked` metres along as drawn.
    pub fn tick(&mut self, input: &Input, walked: f64) -> Vec<Event> {
        let mut out = Vec::new();
        for e in &mut self.enemies { e.hurt = e.hurt.saturating_sub(1); }
        if self.hero.dead { return out; }
        self.hero_tick(input, walked, &mut out);
        for i in 0..self.enemies.len() { self.enemy_tick(i, walked, &mut out); }
        if self.hero.hp <= 0.0 && !self.hero.dead {
            self.hero.dead = true;
            self.hero.act = None;
            out.push(Event::Died);
        }
        out
    }

    fn hero_tick(&mut self, input: &Input, walked: f64, out: &mut Vec<Event>) {
        let h = &mut self.hero;
        h.reeling = h.reeling.saturating_sub(1);
        // Presses wait in the buffer for the hero to be free.
        // A dodge pressed early in a step rolls on from where the step began, the same way.
        if let (Some(_), Some(a)) = (input.dodge, h.act) {
            if let Move::Step(dir) = a.mv { if a.t < STEP_TO_ROLL && h.stamina > 0.0 {
                h.spend(Move::Dodge(dir).spec().stamina);
                h.act = Some(Act { mv: Move::Dodge(dir), t: a.t, landed: false, from_lane: a.from_lane });
                out.push(Event::Began(Move::Dodge(dir)));
                return self.hero_rest(walked, out);
            } }
        }
        if let Some(m) = input.press() { h.buffer = Some((m, 0)); }
        if let Some((m, age)) = h.buffer { h.buffer = (age < BUFFER).then_some((m, age + 1)); }
        h.guarding = input.guard && h.free();
        if h.free() {
            if let Some((m, _)) = h.buffer {
                let m = self.riposte_for(m, walked).unwrap_or(m);
                let h = &mut self.hero;
                let spec = m.spec();
                if spec.stamina > 0.0 && h.stamina <= 0.0 {
                    out.push(Event::Spent);
                    h.buffer = None;
                } else if m == Move::Flask && h.flasks == 0 {
                    h.buffer = None;
                } else {
                    h.buffer = None;
                    h.spend(spec.stamina);
                    let from = h.lane;
                    if let Move::Dodge(dir) | Move::Step(dir) = m { h.lane = (h.lane + dir).clamp(-1, 1); }
                    h.act = Some(Act { mv: m, t: 0, landed: false, from_lane: from });
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
        // Across to the lane: most of the way in the dodge's first ticks.
        let target = h.lane as f32 * LANE_W;
        h.x += (target - h.x) / LANE_TICKS.max(1.0) * 2.0;
        if (target - h.x).abs() < 0.01 { h.x = target; }
        // Stamina comes back once it has gone unspent a while, slower behind a guard.
        h.rested = h.rested.saturating_add(1);
        if h.rested > REGEN_DELAY && h.act.is_none_or(|a| a.mv == Move::Flask) {
            let k = if h.guarding { 1.0 / 3.0 } else { 1.0 };
            h.stamina = (h.stamina + REGEN * k).min(STAMINA);
        }
        let Some(mut a) = h.act else { return };
        let spec = a.mv.spec();
        if a.mv == Move::Flask && a.t == spec.frames.0 {
            h.flasks -= 1;
            h.hp = (h.hp + FLASK_HEAL).min(HP);
            out.push(Event::Healed);
        }
        if a.mv.strikes() && a.phase() == Phase::Active && !a.landed {
            let lane = h.lane;
            let hit = self.enemies.iter().enumerate()
                .filter(|(_, e)| e.alive() && e.lane == lane && (-0.3..=spec.reach).contains(&e.gap(walked)))
                .min_by(|x, y| x.1.gap(walked).total_cmp(&y.1.gap(walked)))
                .map(|(i, _)| i);
            if let Some(i) = hit {
                a.landed = true;
                self.strike(i, a.mv, out);
            }
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

    fn strike(&mut self, i: usize, mv: Move, out: &mut Vec<Event>) {
        let spec = mv.spec();
        let e = &mut self.enemies[i];
        e.hp -= spec.damage;
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
        e.stance -= spec.stance;
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
            Ai::Wait(n) => {
                let reach = e.kind.attacks().iter().map(|a| a.reach).fold(f32::MAX, f32::min);
                // Closes to just inside its shortest reach, never through the hero.
                if gap <= notice && gap > reach - 0.4 { e.s -= (pace / 60.0) as f64; }
                if n > 0 { e.ai = Ai::Wait(n - 1); return; }
                if gap > reach + 0.2 || gap < -0.3 { return; }
                let pick = self.roll(e_attacks(self.enemies[i].kind)) as usize;
                let side = if self.roll(2) == 0 { -1 } else { 1 };
                let e = &mut self.enemies[i];
                let a = e.kind.attacks()[pick];
                // A sweep from the edge lane can only go inward.
                let wide = if a.wide == 0 { 0 } else if e.lane == 1 { -1 } else if e.lane == -1 { 1 } else { side };
                e.ai = Ai::Attack { i: pick, t: 0, wide, landed: false };
                out.push(Event::WindsUp { enemy: i, attack: a.name });
            }
            Ai::Attack { i: ai, t, wide, landed } => {
                let a = e.kind.attacks()[ai];
                // Aim: follow the hero's lane, a lane at a time, until it commits.
                let target = self.hero.lane;
                let e = &mut self.enemies[i];
                if (t as f32) < a.frames.0 as f32 * COMMIT && t % SHIFT == SHIFT - 1 && e.lane != target {
                    e.lane += (target - e.lane).signum();
                }
                let mut landed = landed;
                if phase(a.frames, t) == Phase::Active && !landed {
                    let (l0, l1) = e.lanes();
                    if (-0.3..=a.reach).contains(&gap) {
                        landed = true;
                        self.strike_hero(i, a, (l0, l1), out);
                    }
                }
                if !matches!(self.enemies[i].ai, Ai::Attack { .. }) { return; }
                let t = t + 1;
                let (w, act, rec) = a.frames;
                let (lo, hi) = self.enemies[i].kind.rest();
                let rest = lo + self.roll(hi - lo + 1);
                self.enemies[i].ai = if t < w + act + rec { Ai::Attack { i: ai, t, wide, landed } } else { Ai::Wait(rest) };
            }
        }
    }

    /// Enemy `i`'s attack `a`, live now over lanes `lanes`: dodged, parried, blocked or taken.
    fn strike_hero(&mut self, i: usize, a: Attack, lanes: (i32, i32), out: &mut Vec<Event>) {
        let h = &mut self.hero;
        let covers = |l: i32| (lanes.0..=lanes.1).contains(&l);
        if !covers(h.standing_lane()) || h.act.is_some_and(|x| x.invulnerable()) {
            // Slipped: a just dodge if a dodge began a moment ago from a lane it covers.
            if h.act.is_some_and(|x| matches!(x.mv, Move::Dodge(_)) && x.t <= JUST && covers(x.from_lane)) {
                out.push(Event::JustDodge);
            }
            return;
        }
        if let Some(x) = h.act.filter(|x| x.mv == Move::Parry) {
            match x.phase() {
                Phase::Active if a.parry => {
                    // A clean parry is over at once, so the riposte can follow.
                    h.act = None;
                    out.push(Event::Parried { enemy: i });
                    let e = &mut self.enemies[i];
                    e.ai = Ai::Staggered(STAGGER);
                    out.push(Event::StanceBroken { enemy: i });
                    return;
                }
                // Too early or too late: a clumsy block.
                _ => return self.block(a, 1.5, out),
            }
        }
        if h.guarding && a.kind != Kind::Unblockable { return self.block(a, 1.0, out); }
        let h = &mut self.hero;
        h.hp -= a.damage;
        out.push(Event::Struck { damage: a.damage });
        if !h.act.is_some_and(|x| x.mv.armoured(x.t)) { h.act = None; h.reeling = 24; }
    }

    fn block(&mut self, a: Attack, k: f32, out: &mut Vec<Event>) {
        let h = &mut self.hero;
        let cost = a.guard * k * if a.kind == Kind::GuardBreak { 1.5 } else { 1.0 };
        let short = cost > h.stamina;
        h.spend(cost);
        h.act = None;
        if short {
            h.hp -= a.damage * 0.5;
            h.reeling = 50;
            out.push(Event::GuardBroken);
        } else {
            let chip = a.damage * 0.1;
            h.hp -= chip;
            h.reeling = 10;
            out.push(Event::Blocked { chip });
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
    fn run(f: &mut Fight, n: u32, mut input: impl FnMut(&Fight, u32) -> Input, until: impl Fn(&Fight, &[Event]) -> bool) -> Vec<Event> {
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
        assert!(ev.contains(&Event::Struck { damage: 28.0 }), "{ev:?}");
        assert_eq!(f.hero.hp, HP - 28.0);

        let mut g = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut g, "chop");
        let (w, _, _) = Archetype::Husk.attacks()[0].frames;
        // Dodge right a few ticks before the chop falls.
        let ev = run(&mut g, 200, |_, t| Input { dodge: (t == w - 8).then_some(1), ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert_eq!(g.hero.hp, HP, "{ev:?}");
        assert!(ev.contains(&Event::JustDodge), "a late dodge is a just dodge: {ev:?}");
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
        assert!(ev.contains(&Event::GuardBroken), "{ev:?}");
        assert!(f.hero.hp < HP);
    }

    #[test]
    fn a_full_guard_stops_a_chop_for_stamina_and_chip_damage() {
        let mut f = Fight::new(vec![husk(1.2, 0)]);
        until_winds_up(&mut f, "chop");
        let ev = run(&mut f, 200, |_, _| Input { guard: true, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.iter().any(|e| matches!(e, Event::Blocked { .. })), "{ev:?}");
        assert!((f.hero.hp - (HP - 2.8)).abs() < 0.01, "{}", f.hero.hp);
        assert!(f.hero.stamina < STAMINA - 25.0);
    }

    #[test]
    fn a_parry_on_time_opens_a_riposte_and_one_out_of_time_is_a_clumsy_block() {
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
        let ev = run(&mut g, 200, |_, t| Input { parry: t == early, ..idle() }, |f, _| f.enemies[0].attack().is_none());
        assert!(ev.iter().any(|e| matches!(e, Event::Blocked { .. } | Event::GuardBroken)), "early: {ev:?}");
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
    fn a_hop_back_walks_the_road_back_and_a_flask_heals() {
        let mut f = Fight::new(vec![dummy(5.0, 0)]);
        f.tick(&Input { dodge: Some(0), ..idle() }, 0.0);
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
    fn a_step_is_free_and_unprotected_and_a_dodge_soon_after_turns_it_into_a_roll() {
        let mut f = Fight::new(vec![dummy(5.0, 0)]);
        f.tick(&Input { step: 1, ..idle() }, 0.0);
        assert_eq!((f.hero.lane, f.hero.stamina), (1, STAMINA));
        assert!(!f.hero.act.unwrap().invulnerable());
        f.tick(&Input { dodge: Some(1), ..idle() }, 0.0);
        let a = f.hero.act.unwrap();
        assert_eq!((a.mv, a.from_lane, f.hero.lane), (Move::Dodge(1), 0, 1), "rolled on the way it stepped");
        assert!(f.hero.stamina < STAMINA);
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
}
