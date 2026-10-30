//! Help called to a boss, the way the Souls games let a phantom in: Krix with his axe, Olekeen
//! with his bow. Each fights as its `Style` says: stands where its weapon wants it, on the boss's
//! far side from the hero (Krix close, Olekeen off at range), strikes when the boss is busy elsewhere or open,
//! gets out of the way of the blows it sees coming (most, never all), and keeps off when hurt.
//! The boss's eye goes between it and the hero by who has hurt it lately (`SideFight::eyes_for`),
//! and it comes in stronger for the help. Who can be called, and where, is `summons`.

use crate::fight::Event;
use crate::side::{Ai, Blow, SideFight, HALF};

/// How much more a boss can take with someone called to it.
pub const BOSS_HP: f32 = 1.4;
/// How fast the hurt that draws a boss's eye fades, a tick.
pub const HEAT_FADE: f32 = 0.995;
/// The hurt the hero must have done the boss lately, of its whole, for an ally to fight at its
/// own pace rather than hold back.
const LEAD: f32 = 0.02;
/// What of a blow an ally takes.
const ALLY_TAKEN: f32 = 0.8;
/// A dodge's ticks, the first of them that nothing touches, and its pace (metres a tick).
const DODGE_T: u32 = 20;
const DODGE_SAFE: u32 = 15;
const DODGE_V: f32 = 0.15;
/// Ticks rocked by a blow taken.
const REEL_T: u32 = 18;
/// How many ticks out an ally sees a blow coming at it.
const SEES: f32 = 5.0;
/// How near the yard's ends an ally may stand.
const EDGE: f32 = HALF - 0.4;
/// An arrow's half length.
const ARROW_HALF: f32 = 0.25;
const TPS: f32 = 60.0;

/// Who can be called to a boss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Summoner { Krix, Olekeen, Exodus, Atrael }

/// How a summoner fights.
pub struct Style {
    /// Its whole health, against the hero's.
    pub hp: f32,
    /// How far off the boss's body it stands (metres), and how far its blow reaches.
    pub keep: f32,
    pub reach: f32,
    /// What one blow takes, as shares of the boss's whole health and stance (as it was before
    /// the call made it stronger).
    pub damage: f32,
    pub stance: f32,
    /// A blow's windup and the rest of it, then the rest after it, ticks.
    pub frames: (u32, u32, u32),
    /// Its walk, metres a second.
    pub walk: f32,
    /// Of ten blows it sees coming at it, how many it gets clear of.
    pub reads: u32,
    /// An arrow's pace (metres a tick) for one who shoots; None for one who strikes.
    pub arrow: Option<f32>,
    /// Its side-on art: folder, metres tall drawn, and its blow's and dodge's clips.
    pub sprite: (&'static str, f32),
    pub clips: (&'static str, &'static str),
    /// For one who comes for a phase and no more: the share of its foe's health below which the
    /// foe throws it out of the fight.
    pub flung_below: Option<f32>,
}

const KRIX: Style = Style {
    hp: 1.1, keep: 0.35, reach: 1.5, damage: 0.04, stance: 0.12, frames: (20, 22, 46), walk: 2.4, reads: 7,
    arrow: None, sprite: ("krix_echo", 1.6), clips: ("sweep", "leap"), flung_below: None,
};
const OLEKEEN: Style = Style {
    hp: 0.85, keep: 4.5, reach: 9.0, damage: 0.045, stance: 0.04, frames: (26, 16, 40), walk: 2.6, reads: 8,
    arrow: Some(0.3), sprite: ("olekeen_side", 1.6), clips: ("shoot", "dodge"), flung_below: None,
};
/// Exodus fights bare-handed, quick and close, and is rarely where a blow lands.
const EXODUS: Style = Style {
    hp: 1.0, keep: 0.3, reach: 1.3, damage: 0.04, stance: 0.1, frames: (16, 20, 46), walk: 2.8, reads: 8,
    arrow: None, sprite: ("exodus_side", 2.2), clips: ("strike", "dodge"), flung_below: None,
};
/// Atrael comes down on the Hollowmere Screamer in his ravens, as he does in the book: a heavy
/// mace, slow to get out of the way, and thrown through a shack wall once it is half beaten.
const ATRAEL: Style = Style {
    hp: 1.3, keep: 0.4, reach: 1.7, damage: 0.05, stance: 0.14, frames: (24, 22, 44), walk: 2.2, reads: 5,
    arrow: None, sprite: ("atrael_side", 2.3), clips: ("strike", "walk"), flung_below: Some(0.45),
};

impl Summoner {
    pub fn name(self) -> &'static str {
        match self { Summoner::Krix => "Krix", Summoner::Olekeen => "Olekeen", Summoner::Exodus => "Exodus", Summoner::Atrael => "Atrael" }
    }
    pub fn style(self) -> &'static Style {
        match self { Summoner::Krix => &KRIX, Summoner::Olekeen => &OLEKEEN, Summoner::Exodus => &EXODUS, Summoner::Atrael => &ATRAEL }
    }
}

/// What an ally is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Doing {
    /// Ready, with this many ticks of rest to go before its next blow.
    Ready(u32),
    /// A blow or a shot, this many ticks in.
    Swing(u32),
    /// Getting out of the way: ticks in, and which way.
    Dodge(u32, f32),
    /// Rocked by a blow taken: ticks left.
    Reel(u32),
    /// Down for good: ticks since.
    Down(u32),
}

/// Someone called to the fight.
#[derive(Clone, Debug, PartialEq)]
pub struct Ally {
    pub who: Summoner,
    /// Where it stands along the yard.
    pub at: f32,
    pub facing: f32,
    pub hp: f32,
    pub top: f32,
    pub doing: Doing,
    /// Walking this tick (for the drawing); ticks of the flash of the last blow taken.
    pub walking: bool,
    pub hurt: u32,
    /// Hurt done to the boss lately, which draws its eye.
    pub heat: f32,
    /// An arrow in flight: where, and its pace along the yard.
    pub arrow: Option<(f32, f32)>,
    /// What one blow takes, health and stance, set by the boss it was called to.
    blow: (f32, f32),
    wits: Wits,
}

/// What an ally makes of the blows coming at it: the last one it saw, so each is read once (foe,
/// where it began, whether it got clear), and a count that stands in for chance.
#[derive(Clone, Debug, PartialEq)]
struct Wits { seen: Option<(usize, u32, bool)>, seed: u32 }

impl Ally {
    pub fn standing(&self) -> bool { !matches!(self.doing, Doing::Down(_)) }
    /// In the part of a dodge that nothing touches.
    fn slipping(&self) -> bool { matches!(self.doing, Doing::Dodge(t, _) if t < DODGE_SAFE) }
    fn hurt_badly(&self) -> bool { self.hp < 0.35 * self.top }
    fn roll(&mut self, n: u32) -> u32 {
        let w = &mut self.wits;
        w.seed = w.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (w.seed >> 16) % n.max(1)
    }
}

impl SideFight {
    /// `who` called to this fight, in from the boss's far side (an archer backs off from there
    /// as the room opens); the boss can take `BOSS_HP` times more for it. `hero_top` is the
    /// hero's whole health.
    pub fn summon(&mut self, who: Summoner, hero_top: f32) {
        let Some(foe) = self.foes.first_mut() else { return };
        (foe.hp, foe.top) = (foe.hp * BOSS_HP, foe.top * BOSS_HP);
        self.join(who, hero_top, 1.0 / BOSS_HP);
    }

    /// `who` into this fight as it stands, not called to it: the boss is no stronger for it.
    /// `scale` is what the boss's health was before a call made it stronger, of what it is now.
    pub fn join(&mut self, who: Summoner, hero_top: f32, scale: f32) {
        let st = who.style();
        let hero_s = self.hero.s;
        let Some(foe) = self.foes.first() else { return };
        let blow = (foe.top * scale * st.damage, foe.breed.stance * st.stance);
        // Across the boss from the hero, so what it turns on one it turns its back on the other.
        let away = if foe.s >= hero_s { 1.0 } else { -1.0 };
        let at = (foe.s + away * (foe.breed.half + 1.2)).clamp(-EDGE, EDGE);
        let facing = if foe.s >= at { 1.0 } else { -1.0 };
        let top = hero_top * st.hp;
        let seed = 0x0A11 + 31 * self.allies.len() as u32;
        self.allies.push(Ally { who, at, facing, hp: top, top, doing: Doing::Ready(40), walking: false, hurt: 0, heat: 0.0, arrow: None, blow, wits: Wits { seen: None, seed } });
    }

    /// Whom foe `i` means its next blow for, `roll` one of a hundred: the hero, or the standing
    /// ally whose hurt it feels most, by shares of what each has done to it lately. One much
    /// nearer than the hero draws it more. (`look_round` never lets it be three running.)
    pub(crate) fn eyes_for(&self, i: usize, roll: u32) -> Option<usize> {
        let foe = &self.foes[i];
        let unit = 0.04 * foe.top.max(1.0);
        let hero = 1.0 + self.heat / unit;
        let near = |x: f32| (x - foe.s).abs() + 1.5 < (self.hero.s - foe.s).abs();
        let (k, pull) = self.allies.iter().enumerate().filter(|(_, ally)| ally.standing())
            .map(|(k, ally)| (k, (0.6 + ally.heat / unit) * if near(ally.at) { 1.5 } else { 1.0 }))
            .max_by(|x, y| x.1.total_cmp(&y.1))?;
        ((roll as f32) < 100.0 * pull / (pull + hero)).then_some(k)
    }

    /// Every ally's tick, after the foes': its arrow, then whatever it does.
    pub(crate) fn allies_tick(&mut self, out: &mut Vec<Event>) {
        for k in 0..self.allies.len() {
            self.arrow_tick(k, out);
            self.ally_tick(k, out);
        }
    }

    /// The foe ally `k` fights: the living one nearest it.
    fn mark_of(&self, k: usize) -> Option<usize> {
        let at = self.allies[k].at;
        (0..self.foes.len()).filter(|&i| self.foes[i].alive()).min_by(|&a, &b| (self.foes[a].s - at).abs().total_cmp(&(self.foes[b].s - at).abs()))
    }

    /// Whether ally `k`, here for a phase only, is thrown out now: standing, and its foe `mark`
    /// beaten below the share it stays for.
    fn is_thrown_out(&self, k: usize, mark: Option<usize>) -> bool {
        let ally = &self.allies[k];
        let Some(below) = ally.who.style().flung_below else { return false };
        ally.standing() && mark.is_some_and(|i| self.foes[i].hp < below * self.foes[i].top)
    }

    fn ally_tick(&mut self, k: usize, out: &mut Vec<Event>) {
        let mark = self.mark_of(k);
        if self.is_thrown_out(k, mark) { self.allies[k].doing = Doing::Down(0); }
        let ally = &mut self.allies[k];
        ally.hurt = ally.hurt.saturating_sub(1);
        ally.heat *= HEAT_FADE;
        ally.walking = false;
        match ally.doing {
            Doing::Down(t) => ally.doing = Doing::Down(t.saturating_add(1)),
            Doing::Reel(0) => ally.doing = Doing::Ready(10),
            Doing::Reel(n) => ally.doing = Doing::Reel(n - 1),
            Doing::Dodge(t, _) if t >= DODGE_T => ally.doing = Doing::Ready(8),
            Doing::Dodge(t, way) => (ally.at, ally.doing) = ((ally.at + way * DODGE_V).clamp(-EDGE, EDGE), Doing::Dodge(t + 1, way)),
            Doing::Swing(t) => if let Some(i) = mark { self.swing_tick(k, i, t, out) } else { ally.doing = Doing::Ready(0) },
            Doing::Ready(_) => if let Some(i) = mark { self.ready_tick(k, i, out) },
        }
    }

    /// A blow under way, `t` ticks in: it lands (or the arrow leaves the string) at the end of
    /// its windup; after the rest of it, a rest, longer when hurt.
    fn swing_tick(&mut self, k: usize, i: usize, t: u32, out: &mut Vec<Event>) {
        let st = self.allies[k].who.style();
        let (w, after, rest) = st.frames;
        // Early in its windup it can still pull out of a blow it sees coming.
        if t < w / 2 {
            if let Some(way) = self.sees_coming(k, i) {
                self.allies[k].doing = Doing::Dodge(0, way);
                out.push(Event::AllyDodged { ally: k });
                return;
            }
        }
        if t == w {
            let foe = &self.foes[i];
            let ally = &mut self.allies[k];
            match st.arrow {
                Some(v) => {
                    ally.arrow = Some((ally.at + ally.facing * 0.5, ally.facing * v));
                    out.push(Event::AllyLoosed { ally: k });
                }
                None if (ally.at - foe.s).abs() - foe.breed.half <= st.reach + 0.2 => {
                    let (damage, stance) = ally.blow;
                    let dealt = self.struck_by_ally(i, damage, stance, out);
                    self.allies[k].heat += dealt;
                }
                None => {}
            }
        }
        // It follows the hero's lead: while they are not hurting the boss, it holds back too.
        let led = self.heat >= LEAD * self.foes[i].top;
        let ally = &mut self.allies[k];
        ally.doing = if t >= w + after {
            let rest = rest + ally.roll(20);
            let rest = if ally.hurt_badly() { rest * 8 / 5 } else { rest };
            Doing::Ready(if led { rest } else { rest * 5 / 2 })
        } else {
            Doing::Swing(t + 1)
        };
    }

    /// An arrow in flight: on to the first foe's body it meets, or off the yard's end.
    fn arrow_tick(&mut self, k: usize, out: &mut Vec<Event>) {
        let Some((was, pace)) = self.allies[k].arrow else { return };
        let now = was + pace;
        let struck = (0..self.foes.len()).find(|&i| { let foe = &self.foes[i]; foe.alive() && (now - foe.s).abs() < foe.breed.half + ARROW_HALF });
        let ally = &mut self.allies[k];
        ally.arrow = (struck.is_none() && now.abs() < HALF).then_some((now, pace));
        if let Some(i) = struck {
            let (damage, stance) = ally.blow;
            let dealt = self.struck_by_ally(i, damage, stance, out);
            self.allies[k].heat += dealt;
        }
    }

    /// Ready, against foe `i`: out of the way of what it sees coming; else to where its weapon
    /// wants it (further off while the boss winds up at it, or when hurt); then, rested and in
    /// reach, a blow, unless that would walk into one.
    fn ready_tick(&mut self, k: usize, i: usize, out: &mut Vec<Event>) {
        if let Some(way) = self.sees_coming(k, i) {
            self.allies[k].doing = Doing::Dodge(0, way);
            out.push(Event::AllyDodged { ally: k });
            return;
        }
        let foe = &self.foes[i];
        let (fs, half, open) = (foe.s, foe.breed.half, matches!(foe.ai, Ai::Open(_)));
        let winding = foe.blow().is_some_and(|(_, ph, _)| ph != crate::fight::Phase::Recover);
        let at_me = foe.eyes == Some(k);
        let covered = foe.warning().is_some_and(|(lo, hi, ..)| self.allies[k].at + 0.6 > lo && self.allies[k].at - 0.6 < hi);
        let ally = &mut self.allies[k];
        let st = ally.who.style();
        let side = if ally.at >= fs { 1.0 } else { -1.0 };
        let wary = ally.hurt_badly() || (at_me && winding && !open);
        let keep = st.keep + if wary { if st.arrow.is_some() { 1.5 } else { 2.2 } } else { 0.0 };
        let want = (fs + side * (half + keep)).clamp(-EDGE, EDGE);
        let step = st.walk / TPS;
        if (want - ally.at).abs() > 0.1 {
            ally.at += (want - ally.at).clamp(-step, step);
            ally.walking = true;
        }
        ally.facing = -side;
        let Doing::Ready(rest) = ally.doing else { return };
        if rest > 0 { ally.doing = Doing::Ready(rest - 1); return; }
        let in_reach = (ally.at - fs).abs() - half <= st.reach;
        let melee_wary = wary && st.arrow.is_none() && !open;
        if in_reach && !covered && !melee_wary && self.opening(k, i) { self.allies[k].doing = Doing::Swing(0); }
    }

    /// Whether foe `i` leaves ally `k` time for a blow: open, squared up for longer than its
    /// windup, or in a recovery that will outlast it, or busy with the hero.
    fn opening(&self, k: usize, i: usize) -> bool {
        let foe = &self.foes[i];
        let windup = self.allies[k].who.style().frames.0;
        match foe.ai {
            Ai::Open(_) => true,
            Ai::Wait(n) => n > windup + 6 || foe.eyes != Some(k),
            Ai::Blow { i: bi, t, .. } => {
                let blow = foe.timed(bi);
                let (w, live, rec) = blow.frames;
                t >= w + live && w + live + rec + blow.stuck > t + windup * 3 / 5 || (t < w && foe.eyes != Some(k))
            }
            Ai::Dead => false,
        }
    }

    /// Whether ally `k` sees foe `i`'s blow about to come to it, and gets clear: which way it
    /// goes (away from the foe). Each blow is read once, and it does not always see it.
    fn sees_coming(&mut self, k: usize, i: usize) -> Option<f32> {
        let foe = &self.foes[i];
        let Ai::Blow { from, .. } = foe.ai else { return None };
        let soon = foe.reaches_in(self.allies[k].at)?;
        if soon > SEES { return None; }
        let blow = (i, from.to_bits(), true);
        let away = if self.allies[k].at >= foe.s { 1.0 } else { -1.0 };
        let ally = &mut self.allies[k];
        // One it read and got clear of it keeps clear of, for as long as it lasts.
        if let Some((j, at, cleared)) = ally.wits.seen { if (j, at) == (blow.0, blow.1) { return cleared.then_some(away); } }
        let reads = ally.who.style().reads + u32::from(ally.hurt_badly());
        let cleared = ally.roll(10) < reads;
        ally.wits.seen = Some((i, blow.1, cleared));
        // Never past the boss: with its back to a wall, the dodge's slip is all it has.
        cleared.then_some(away)
    }

    /// Foe `i`'s blow, live now, on every ally it reaches who is not slipping it: true if it
    /// struck one. A blow taken rocks it back; one too many puts it down for good.
    pub(crate) fn allies_struck(&mut self, i: usize, blow: &Blow, out: &mut Vec<Event>) -> bool {
        let mut struck = false;
        for k in 0..self.allies.len() {
            let ally = &self.allies[k];
            if !ally.standing() || ally.slipping() || !self.reaches(i, blow, ally.at, 0.0) { continue; }
            let away = if ally.at >= self.foes[i].s { 1.0 } else { -1.0 };
            let damage = blow.damage * ALLY_TAKEN;
            let ally = &mut self.allies[k];
            (ally.hp, ally.hurt, struck) = (ally.hp - damage, 10, true);
            out.push(Event::AllyStruck { ally: k, damage });
            if ally.hp <= 0.0 {
                (ally.hp, ally.doing) = (0.0, Doing::Down(0));
                out.push(Event::AllyDown { ally: k });
            } else {
                ally.at = (ally.at + away * 0.5).clamp(-EDGE, EDGE);
                ally.doing = Doing::Reel(REEL_T);
            }
        }
        struck
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::Stats;
    use crate::side::tests::{bot_input, Bot};
    use crate::side::{Breed, SideInput, BUTCHER, CHIEF, MAGISTRATE, SCREAMER};

    /// One bout, as `side::tests::bout` sets it up, with `who` called (or no one), and whether
    /// the hero stands back and only guards: (won, seconds, blows meant for the ally of all, the
    /// hero's health lost to blows, of their whole).
    fn bout(breed: &'static Breed, tier: u32, bot: Option<Bot>, who: Option<Summoner>, seed: u32) -> (bool, f32, f32, f32) {
        let scale = crate::world::World::scale_at(tier, 0);
        let l = 2 * tier;
        let mut stats = Stats { vigor: l, endurance: l, might: l, ..Stats::default() };
        let (main, armour) = crate::world::World::forged_at(tier);
        (stats.kit.main, stats.kit.forged_armour) = (main, armour as f32 * crate::gear::ARMOUR_STEP);
        let mut fight = SideFight::boss(stats, stats.max_hp(), stats.max_stamina(), 3, breed, scale);
        fight.reseed(seed);
        if let Some(w) = who { fight.summon(w, stats.max_hp()); }
        let (mut n, mut blows, mut at_ally) = (0, 0, 0);
        let (mut by_ally, mut hurt) = (0.0, 0.0);
        while n < 60 * 240 && !fight.won() && !fight.hero.dead {
            let i = bot.map_or_else(|| SideInput { guard: true, left: fight.hero.s > -HALF + 0.6, ..SideInput::default() }, |b| bot_input(&fight, b));
            let (was, hero_was) = (fight.foes[0].hp, fight.hero.hp);
            let events = fight.tick(&i);
            hurt += (hero_was - fight.hero.hp).max(0.0);
            if events.iter().any(|e| matches!(e, Event::AllyHit { .. })) { by_ally += was - fight.foes[0].hp; }
            for e in &events {
                if let Event::WindsUp { .. } = e { blows += 1; if fight.foes[0].eyes.is_some() { at_ally += 1; } }
            }
            n += 1;
        }
        if std::env::var("ALLY_DEBUG").is_ok() {
            let a = fight.allies.first();
            println!("    seed {seed:x}: won {} in {:.0}s, ally did {:.0}% of the boss, ally hp {:.0}%", fight.won(), n as f32 / 60.0, 100.0 * by_ally / fight.foes[0].top, a.map_or(0.0, |a| 100.0 * a.hp / a.top));
        }
        (fight.won(), n as f32 / 60.0, at_ally as f32 / blows.max(1) as f32, hurt / fight.hero.stats.max_hp())
    }

    /// Twenty-four bouts: wins, and the mean seconds, share of blows at the ally, and health lost.
    fn wins(breed: &'static Breed, tier: u32, bot: Option<Bot>, who: Option<Summoner>) -> (usize, f32, f32, f32) {
        let runs: Vec<_> = (0..24u32).map(|k| bout(breed, tier, bot, who, 0x51DE + 977 * k)).collect();
        let won = runs.iter().filter(|r| r.0).count();
        let mean = |f: fn(&(bool, f32, f32, f32)) -> f32| runs.iter().map(f).sum::<f32>() / 24.0;
        (won, mean(|r| r.1), mean(|r| r.2), mean(|r| r.3))
    }

    #[test]
    fn an_ally_draws_some_of_the_bosss_blows_and_shortens_a_readers_fight_but_a_masher_still_loses() {
        let (mut lost_alone, mut lost_helped) = (0.0, 0.0);
        for (breed, tier) in [(&CHIEF, 2), (&MAGISTRATE, 3), (&BUTCHER, 5)] {
            let alone = wins(breed, tier, Some(Bot::Reader), None);
            for who in [Summoner::Krix, Summoner::Olekeen, Summoner::Exodus] {
                let (read, secs, share, hurt) = wins(breed, tier, Some(Bot::Reader), Some(who));
                let (mash, ..) = wins(breed, tier, Some(Bot::Masher), Some(who));
                println!("{} {who:?}: reader {}/24 {:.0}s lost {:.0}% alone, {read}/24 {secs:.0}s lost {:.0}% helped, {:.0}% at the ally; masher {mash}/24",
                    breed.name, alone.0, alone.1, 100.0 * alone.3, 100.0 * hurt, 100.0 * share);
                assert!((0.15..=0.6).contains(&share), "{}: {:.0}% of the blows went to {who:?}", breed.name, 100.0 * share);
                // A reader rolling through the boss to its far side meets blows meant for the ally,
                // as a player would, and a quick boss with more to take presses them longer: one
                // boss may cost a little more, so long as the three together cost less.
                (lost_alone, lost_helped) = (lost_alone + alone.3, lost_helped + hurt);
                assert!(hurt < 1.3 * alone.3 + 0.1 && read + 3 >= alone.0, "{}: {who:?} cost the reader ({read}/24, lost {:.0}%)", breed.name, 100.0 * hurt);
                assert!(secs < alone.1, "{}: with {who:?} the fight was no shorter ({secs:.0}s against {:.0}s)", breed.name, alone.1);
                assert!(mash <= 8, "{}: with {who:?} a masher won {mash}/24", breed.name);
            }
        }
        assert!(lost_helped < lost_alone, "help cost the reader more over all ({lost_helped:.2} against {lost_alone:.2})");
    }

    #[test]
    fn an_ally_cannot_carry_a_hero_who_only_stands_back_and_guards() {
        for (breed, tier) in [(&CHIEF, 2), (&MAGISTRATE, 3)] {
            for who in [Summoner::Krix, Summoner::Olekeen, Summoner::Exodus] {
                let (won, ..) = wins(breed, tier, None, Some(who));
                assert!(won <= 2, "{}: {who:?} won {won}/24 for a hero who never struck", breed.name);
            }
        }
    }

    #[test]
    fn an_ally_gets_clear_of_most_blows_it_sees_but_not_all() {
        let (mut dodged, mut struck) = (0, 0);
        for k in 0..12u32 {
            let mut fight = SideFight::boss(Stats::default(), 1e6, 100.0, 3, &CHIEF, (1.0, 1.0));
            fight.reseed(0x51DE + 977 * k);
            fight.summon(Summoner::Krix, 1e6);
            for _ in 0..60 * 60 {
                for e in fight.tick(&SideInput { guard: true, left: true, ..SideInput::default() }) {
                    match e { Event::AllyDodged { .. } => dodged += 1, Event::AllyStruck { .. } => struck += 1, _ => {} }
                }
            }
        }
        let share = dodged as f32 / (dodged + struck).max(1) as f32;
        assert!((0.45..=0.95).contains(&share), "Krix got clear of {dodged} and took {struck}");
    }

    #[test]
    fn atrael_joins_without_making_the_screamer_stronger_and_is_thrown_out_at_half() {
        let stats = Stats::default();
        let mut fight = SideFight::boss(stats, stats.max_hp(), stats.max_stamina(), 3, &SCREAMER, crate::world::World::scale_at(1, 0));
        let top = fight.foes[0].top;
        fight.join(Summoner::Atrael, stats.max_hp(), 1.0);
        assert_eq!(fight.foes[0].top, top, "joining is not a call");
        fight.foes[0].hp = 0.5 * top;
        let mut events = Vec::new();
        fight.allies_tick(&mut events);
        assert!(fight.allies[0].standing(), "still with it above the half");
        fight.foes[0].hp = 0.44 * top;
        fight.allies_tick(&mut events);
        assert!(!fight.allies[0].standing(), "thrown out below it");
        // Those called stay to the end.
        let mut called = SideFight::boss(stats, stats.max_hp(), stats.max_stamina(), 3, &SCREAMER, crate::world::World::scale_at(1, 0));
        called.summon(Summoner::Krix, stats.max_hp());
        called.foes[0].hp = 0.1 * called.foes[0].top;
        called.allies_tick(&mut events);
        assert!(called.allies[0].standing());
    }
}
