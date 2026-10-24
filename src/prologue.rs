//! The prologue: a remembered fight on a road at night, one lesson at a time, before the
//! pilgrimage proper. Nothing in it is needed later (the story begins at the first shrine either
//! way), and Esc leaves it at any moment.
//!
//! It runs over the ordinary road fight: one husk, held still or made to swing the blow a lesson
//! is about, and the hero kept on their feet. A side-on bout comes last, then the dark and the
//! pilgrimage.

use crate::fight::{Ai, Enemy, Event, Fight, Archetype, Move, HERO_D};
use crate::shrine::{Key, Panel};

/// The stop the memory walks: the Blackwood's edge under a full moon, far from where the
/// pilgrimage begins (a stop with no fork, so the walk never waits at a signpost).
pub(crate) const STOP: &str = "blackwood@cathedral";

/// The steps, in order. A card stops the road for words; the rest are played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lesson { Memory, Walk, Run, Lanes, Strike, Pace, Guard, Roll, Late, Parry, Flask, Hand, Turn, Side, End }

/// The husk's blows by index (`Archetype::Husk.attacks()`).
const CHOP: usize = 0;
const SWEEP: usize = 1;
const LATE_CHOP: usize = 2;
/// The memory's husk: hard to fell, soft in the arm.
const DUMMY: (f32, f32) = (8.0, 0.35);
/// Ticks between the blows a lesson asks the husk for (the first comes a little sooner).
const BETWEEN: u32 = 80;
/// The hero is set back on their feet below this share of health (but not in the flask lesson).
const STEADY: f32 = 0.35;
/// Ticks a word on the last try stays up.
const WORD_T: u32 = 200;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Prologue {
    pub lesson: Lesson,
    /// Ticks into the lesson, and the road where it began.
    pub t: u32,
    pub from: f64,
    /// How far into the goal: blows landed, blocks, rolls; and a second count where a lesson has
    /// two (heavies landed, the riposte after a parry).
    pub got: u32,
    pub also: u32,
    /// Lanes stepped into, left and right.
    pub went: (bool, bool),
    /// A word on how the last try went, and its ticks left.
    pub word: (String, u32),
    /// Asked whether to leave the prologue.
    pub leaving: bool,
    /// Ticks until the husk is made to swing; and of the blow under way, whether it hurt the hero,
    /// whether a roll began in it, whether it already counted.
    pub cool: u32,
    pub swing: Option<(bool, bool, bool)>,
    /// Whether the hero's kit has something to loose or cast (the lesson on it is skipped if not).
    pub hand: bool,
    /// The road as of the last tick (the walk lesson counts from `from`).
    pub at: f64,
    /// Played on a touch screen: the lessons name the buttons and the thumb, not keys.
    pub touch: bool,
}

/// What the game should do after a tick of the prologue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step { Side, Done }

impl Lesson {
    fn card(self) -> bool { matches!(self, Lesson::Memory | Lesson::Pace | Lesson::Turn | Lesson::End) }
    fn next(self) -> Lesson {
        use Lesson::*;
        match self { Memory => Walk, Walk => Run, Run => Lanes, Lanes => Strike, Strike => Pace, Pace => Guard, Guard => Roll, Roll => Late, Late => Parry, Parry => Flask, Flask => Hand, Hand => Turn, Turn => Side, Side | End => End }
    }
}

impl Prologue {
    pub fn new(walked: f64, hand: bool) -> Prologue {
        Prologue { lesson: Lesson::Memory, t: 0, from: walked, got: 0, also: 0, went: (false, false), word: (String::new(), 0), leaving: false, cool: 0, swing: None, hand, at: walked, touch: false }
    }

    /// Whether the road stands still for words (a card, or the question of leaving).
    pub fn still(&self) -> bool { self.leaving || self.lesson.card() }

    /// On to `l`, setting the road and the husk up for it.
    pub fn begin(&mut self, l: Lesson, fight: &mut Fight, walked: f64) {
        let l = if l == Lesson::Hand && !self.hand { l.next() } else { l };
        (self.lesson, self.t, self.from, self.got, self.also, self.went, self.cool, self.swing) = (l, 0, walked, 0, 0, (false, false), BETWEEN / 2, None);
        let lane = fight.hero.lane;
        if l == Lesson::Strike {
            // The husk steps out ahead, in the hero's lane, and stands to be struck.
            fight.enemies = vec![Enemy { ai: Ai::Wait(u32::MAX), ..Enemy::new(Archetype::Husk, walked + (HERO_D + 6.0) as f64, lane).scaled(DUMMY) }];
        }
        let h = &mut fight.hero;
        match l {
            Lesson::Flask => { h.hp = h.stats.max_hp() * 0.4; h.flasks = h.flasks.max(1); }
            // From where it stands to the far end of a shot.
            Lesson::Hand => {
                if let Some(e) = fight.enemies.first_mut() { e.s = walked + (HERO_D + 7.0) as f64; e.lane = h.lane; e.ai = Ai::Wait(u32::MAX); }
                (h.arrows, h.charges) = (h.arrows.max(3), h.stats.max_charges());
            }
            _ => {}
        }
    }

    /// A word on the last try, said for a while.
    fn say(&mut self, w: &str) { self.word = (w.to_owned(), WORD_T); }

    /// The words for the keyboard, or for the touch screen.
    fn k(&self, keys: &'static str, thumb: &'static str) -> &'static str { if self.touch { thumb } else { keys } }

    /// One drawn tick, after the fight's: the menu keys pressed, the fight's events, and whether
    /// the hero ran this tick. What the game should do next, if anything.
    pub fn tick(&mut self, fight: &mut Fight, walked: f64, events: &[Event], keys: &[Key], running: bool) -> Option<Step> {
        self.t += 1;
        self.word.1 = self.word.1.saturating_sub(1);
        // Leaving: asked once, answered with E (leave) or Esc (stay).
        for k in keys {
            match (self.leaving, k) {
                (false, Key::Back) => { self.leaving = true; return None; }
                (true, Key::Use) => return Some(Step::Done),
                (true, Key::Back) => { self.leaving = false; return None; }
                (false, Key::Use) if self.lesson.card() => {
                    return match self.lesson {
                        Lesson::End => Some(Step::Done),
                        Lesson::Turn => { self.lesson = Lesson::Side; Some(Step::Side) }
                        l => { self.begin(l.next(), fight, walked); None }
                    };
                }
                _ => {}
            }
        }
        self.at = walked;
        if self.still() { return None; }
        self.keep_up(fight);
        if matches!(self.lesson, Lesson::Roll | Lesson::Late) { self.rolls(events); }
        self.drive(fight, walked);
        let h = &fight.hero;
        let done = match self.lesson {
            Lesson::Walk => walked - self.from >= 6.0,
            Lesson::Run => { if running { self.got += 1; } self.got >= 60 }
            Lesson::Lanes => {
                for e in events { if let Event::Began(Move::Lane(d)) = e { if *d < 0 { self.went.0 = true; } else { self.went.1 = true; } } }
                self.went.0 && self.went.1 && h.act.is_none()
            }
            Lesson::Strike => {
                for e in events {
                    match e {
                        Event::Hit { mv: Move::Light, .. } => self.got += 1,
                        Event::Hit { mv: Move::Heavy, .. } => self.also += 1,
                        Event::Whiffed(_) => self.say("Missed: stand in its lane, close enough that it stops your walk."),
                        _ => {}
                    }
                }
                self.got >= 3 && self.also >= 1 && h.act.is_none()
            }
            Lesson::Guard => {
                for e in events {
                    match e {
                        Event::Blocked { .. } | Event::PerfectBlock | Event::Parried { .. } => { self.got += 1; self.say("Blocked. A guard costs stamina for every blow it stops."); }
                        Event::Struck { .. } => self.say(self.k("Hold the right mouse button before the blow lands.", "Hold GUARD before the blow lands.")),
                        Event::GuardBroken { .. } => self.say("Out of stamina, the guard gives way. Let it come back between blows."),
                        _ => {}
                    }
                }
                self.got >= 2
            }
            Lesson::Roll => self.got >= 2,
            Lesson::Late => self.got >= 1,
            Lesson::Parry => {
                for e in events {
                    match e {
                        Event::Parried { .. } => { self.got = 1; self.say("Parried! It is open: strike now for a riposte."); }
                        Event::ParryMissed { .. } => self.say("Out of time: parry just as the gold mark fills, not before."),
                        Event::Struck { .. } if self.got == 0 => self.say(self.k("Shift + right click as the gold mark fills.", "Tap PARRY as the gold mark fills.")),
                        Event::Hit { mv: Move::Riposte, .. } => self.also += 1,
                        _ => {}
                    }
                }
                // The opening passes if it is not taken.
                if self.got == 1 && self.also == 0 && fight.enemies.first().is_some_and(|e| !matches!(e.ai, Ai::Staggered(_))) {
                    self.got = 0;
                    self.say("Too slow: the opening after a parry is short. Strike at once.");
                }
                self.also >= 1 && h.act.is_none()
            }
            Lesson::Flask => { if events.contains(&Event::Healed) { self.got = 1; } self.got > 0 && h.act.is_none() }
            Lesson::Hand => {
                if events.iter().any(|e| matches!(e, Event::Shot(_))) { self.got += 1; }
                self.got >= 2 && h.act.is_none()
            }
            _ => false,
        };
        if done {
            // What the lessons spent is given back.
            if self.lesson == Lesson::Flask { fight.hero.flasks = crate::fight::FLASKS; }
            let next = self.lesson.next();
            self.say("");
            self.begin(next, fight, walked);
        }
        None
    }

    /// The hero kept on their feet: a memory cannot kill.
    fn keep_up(&mut self, fight: &mut Fight) {
        let h = &mut fight.hero;
        if h.dead || (h.hp < h.stats.max_hp() * STEADY && self.lesson != Lesson::Flask) {
            (h.dead, h.hp) = (false, h.stats.max_hp());
        }
        if self.lesson == Lesson::Flask { h.hp = h.hp.max(1.0); }
    }

    /// The husk: never left to choose for itself; made to swing this lesson's blow once it is in
    /// reach and has stood a while.
    fn drive(&mut self, fight: &mut Fight, walked: f64) {
        let blow = match self.lesson { Lesson::Guard | Lesson::Parry => Some(CHOP), Lesson::Roll => Some(SWEEP), Lesson::Late => Some(LATE_CHOP), _ => None };
        let lane = fight.hero.lane;
        let Some(e) = fight.enemies.first_mut() else { return };
        // Felled after all: up again, as a memory would have it.
        if !e.alive() { *e = Enemy { ai: Ai::Wait(u32::MAX), ..Enemy::new(Archetype::Husk, e.s, e.lane).scaled(DUMMY) }; }
        if let Ai::Wait(n) = e.ai { if n < u32::MAX / 2 { e.ai = Ai::Wait(u32::MAX); } }
        let swinging = matches!(e.ai, Ai::Attack { .. });
        // Standing idle, it steps across to the hero's lane (a lesson is not about finding it).
        if matches!(e.ai, Ai::Wait(_)) && e.lane != lane && self.t % 30 == 0 { e.lane += (lane - e.lane).signum(); }
        if !swinging && self.swing.is_some() { self.passed(); }
        let Some(i) = blow else { return };
        if swinging || !matches!(e.ai, Ai::Wait(_)) { return; }
        let a = Archetype::Husk.attacks()[i];
        let gap = e.gap(walked);
        if gap > a.reach + 0.2 || gap < -0.3 { return; }
        if self.cool > 0 { self.cool -= 1; return; }
        // A sweep from the middle lane turns toward the hero.
        let wide = if a.wide == 0 { 0 } else if e.lane == 1 { -1 } else if e.lane == -1 { 1 } else if lane < 0 { -1 } else { 1 };
        e.ai = Ai::Attack { i, t: 0, wide, landed: false, aim: walked + HERO_D as f64 };
        self.cool = BETWEEN;
        self.swing = Some((false, false, false));
    }

    /// The rolling lessons: what the blow under way has met so far; a roll at the last moment
    /// counts at once.
    fn rolls(&mut self, events: &[Event]) {
        let Some((hurt, rolled, counted)) = &mut self.swing else { return };
        let mut perfect = false;
        for e in events {
            match e {
                Event::Began(Move::Step(_) | Move::Tumble | Move::Backstep) => *rolled = true,
                Event::Struck { .. } | Event::Staggered { .. } | Event::KnockedDown { .. } | Event::GuardBroken { .. } | Event::ParryMissed { .. } => *hurt = true,
                Event::JustDodge { .. } if !*counted => { *counted = true; perfect = true; }
                _ => {}
            }
        }
        if perfect { self.got += 1; self.say("Perfect timing: it stumbles. Strike it while it reels."); }
    }

    /// The blow under way has passed: a roll that slipped it counts, and one that found the hero
    /// says why.
    fn passed(&mut self) {
        let Some((hurt, rolled, counted)) = self.swing.take() else { return };
        if !matches!(self.lesson, Lesson::Roll | Lesson::Late) { return; }
        if counted { return; }
        match (hurt, rolled) {
            (false, true) => { self.got += 1; self.say("Rolled clear."); }
            (true, true) => self.say(if self.lesson == Lesson::Late { "Too soon: this blow hangs. Wait for the glint, then roll." } else { "Too soon: roll as the mark fills, not when it appears." }),
            (true, false) => self.say(self.k("Roll with Shift + A or D (or W) as the mark under you fills.", "Tap ROLL as the mark under you fills.")),
            (false, false) => self.say("Stepping out of its way or blocking works too, but try a roll."),
        }
    }

    /// The words to show now: (panel, whether it is a full card).
    pub fn panel(&self, fight: &Fight) -> (Panel, bool) {
        let skip = self.k("Esc skip the prologue", "II skip the prologue").to_owned();
        if self.leaving {
            return (Panel { title: "Leave the prologue?".into(), text: "Nothing of the pilgrimage is lost: it begins at the first shrine either way.".into(), hint: self.k("E leave   Esc stay", "Tap to leave   BACK to stay").into(), ..Panel::default() }, true);
        }
        let n = |got: u32, of: u32| format!("{} / {of}", got.min(of));
        let (title, text, progress): (&str, String, Vec<String>) = match self.lesson {
            Lesson::Memory => ("Prologue", "Years before the pilgrimage, on a forest road under the moon, an old soldier taught you how the dead fight. You remember it now, as you wake.".into(), vec![]),
            Lesson::Walk => ("Walk", self.k("Hold W to walk the road, S to step back.", "Push the left thumb up to walk the road, down to step back.").into(), vec![format!("Walked  {:.0} / 6 m", (self.at - self.from).clamp(0.0, 6.0))]),
            Lesson::Run => ("Run", self.k("Tap W twice and hold it to run.", "Push the left thumb all the way up to run.").to_owned() + " Running spends stamina (the green bar).", vec![format!("Run  {}", n(self.got / 6, 10))]),
            Lesson::Lanes => ("Lanes", self.k("A and D step", "Push the left thumb left or right to step").to_owned() + " a lane across, free, but with no dodge in it: a blow aimed at your lane still finds you mid-step.",
                vec![format!("Left  {}", if self.went.0 { "done" } else { "-" }), format!("Right  {}", if self.went.1 { "done" } else { "-" })]),
            Lesson::Strike => ("Strike", "Walk up until the husk stops you. ".to_owned() + self.k("Left click strikes; Shift + left click is", "STRIKE strikes; HEAVY is") + " a heavy blow, slower but it breaks stance.",
                vec![format!("Light  {}", n(self.got, 3)), format!("Heavy  {}", n(self.also, 1))]),
            Lesson::Pace => ("Every weapon has its own pace", format!(
                "Your {} swings the way you just felt. A dagger starts and recovers quickly; a greatsword winds up long, hits hard and rides out a flinch. Learn when your weapon's blow lands and how long it leaves you open: timing on the road is the weapon's as much as yours.",
                fight.hero.stats.kit.arms.class.name()), vec![]),
            Lesson::Guard => ("Guard", self.k("Hold the right mouse button", "Hold GUARD").to_owned() + " to guard. A block costs stamina; run out and the guard breaks.", vec![format!("Blocked  {}", n(self.got, 2))]),
            Lesson::Roll => ("Roll", "Warning marks show where a blow will land, and fill as it comes. Orange: roll or block. ".to_owned() + self.k("Roll with Shift + A or D", "Tap ROLL (lean the left thumb to pick the way)") + " as the mark fills: too early and the roll is over before the blow.", vec![format!("Rolled clear  {}", n(self.got, 2))]),
            Lesson::Late => ("A blow that hangs", "Some blows are held up before they fall: this one flashes early and waits. Do not roll at the flash; wait for the glint, then roll.".into(), vec![format!("Rolled clear  {}", n(self.got, 1))]),
            Lesson::Parry => ("Parry", "Gold marks can be parried: ".to_owned() + self.k("Shift + right click", "tap PARRY") + " just as the mark fills. A parried foe is open; strike at once for a riposte.",
                vec![format!("Parry  {}", if self.got > 0 || self.also > 0 { "done" } else { "-" }), format!("Riposte  {}", n(self.also, 1))]),
            Lesson::Flask => ("Flask", self.k("R", "FLASK").to_owned() + " drinks a flask to heal. Drinking takes a moment and leaves you open: step back out of reach first.", vec![]),
            Lesson::Hand => {
                let what = if fight.hero.stats.kit.spell_cast().is_some() { self.k("C", "CAST").to_owned() + " casts the spell your catalyst holds; two blows landed up close fill a charge." } else { self.k("F", "SHOOT").to_owned() + " is your ranged attack: it flies down your lane to the first foe in it." };
                ("From afar", what, vec![format!("Shots  {}", n(self.got, 2))])
            }
            Lesson::Turn => ("Side-on", "Some fights turn the road to the side: ".to_owned() + self.k("A and D move, W or Space jumps", "the left thumb moves, JUMP jumps") + ", the same strikes, guard, rolls and parries. Fell the husk to finish.", vec![]),
            Lesson::Side => ("Side-on", self.k("A / D move   W / Space jump", "Left thumb move   JUMP jump").to_owned() + "   strike, guard, roll and parry as before", vec![]),
            Lesson::End => ("The memory fades", "The old soldier's road is long behind you. You wake on the road to Hollowmere.".into(), vec![]),
        };
        let hint = if self.lesson.card() { format!("{}   {skip}", self.k("E go on", "Tap to go on")) } else if self.word.1 > 0 && !self.word.0.is_empty() { self.word.0.clone() } else { skip };
        (Panel { title: title.into(), text, options: progress.into_iter().map(|p| (p, true)).collect(), choice: usize::MAX, hint }, self.lesson.card())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::Input;

    fn at(l: Lesson) -> (Prologue, Fight) {
        let mut f = Fight::new(Vec::new());
        let mut p = Prologue::new(0.0, false);
        p.begin(Lesson::Strike, &mut f, 0.0);
        f.enemies[0].s = (HERO_D + 2.0) as f64;
        p.begin(l, &mut f, 0.0);
        (p, f)
    }

    /// Ticks the fight and the prologue together with `input`; every event.
    fn run(p: &mut Prologue, f: &mut Fight, n: u32, input: Input) -> Vec<Event> {
        let mut all = Vec::new();
        for _ in 0..n {
            let ev = f.tick(&input, 0.0);
            p.tick(f, 0.0, &ev, &[], false);
            all.extend(ev);
        }
        all
    }

    #[test]
    fn the_husk_swings_only_the_lessons_blow_and_never_of_its_own_accord() {
        let (mut p, mut f) = at(Lesson::Strike);
        run(&mut p, &mut f, 600, Input::default());
        assert!(f.enemies[0].attack().is_none(), "a training husk stands to be struck");
        for (l, name) in [(Lesson::Guard, "chop"), (Lesson::Roll, "sweep"), (Lesson::Late, "late chop"), (Lesson::Parry, "chop")] {
            let (mut p, mut f) = at(l);
            let mut seen = Vec::new();
            for _ in 0..600 {
                run(&mut p, &mut f, 1, Input::default());
                if let Some((a, _, _)) = f.enemies[0].attack() { if !seen.contains(&a.name) { seen.push(a.name); } }
            }
            assert_eq!(seen, vec![name], "{l:?}");
        }
    }

    #[test]
    fn a_memory_cannot_kill_and_a_card_waits_for_e() {
        let (mut p, mut f) = at(Lesson::Guard);
        run(&mut p, &mut f, 3_000, Input::default());
        assert!(!f.hero.dead && f.hero.hp > 0.0, "kept on their feet through every chop");
        let mut p = Prologue::new(0.0, false);
        assert!(p.still());
        p.tick(&mut f, 0.0, &[], &[], false);
        assert_eq!(p.lesson, Lesson::Memory);
        p.tick(&mut f, 0.0, &[], &[Key::Use], false);
        assert_eq!(p.lesson, Lesson::Walk);
        // Esc asks; Esc again stays, E leaves.
        p.tick(&mut f, 0.0, &[], &[Key::Back], false);
        assert!(p.leaving && p.still());
        p.tick(&mut f, 0.0, &[], &[Key::Back], false);
        assert!(!p.leaving);
        p.tick(&mut f, 0.0, &[], &[Key::Back], false);
        assert_eq!(p.tick(&mut f, 0.0, &[], &[Key::Use], false), Some(Step::Done));
    }

    #[test]
    fn a_roll_too_soon_is_told_so_and_one_at_the_mark_counts() {
        let (mut p, mut f) = at(Lesson::Roll);
        // Too soon: rolled the moment the sweep began.
        run(&mut p, &mut f, 400, Input::default());
        let (mut p, mut f) = at(Lesson::Roll);
        let mut rolled = false;
        for _ in 0..400 {
            let t = match f.enemies[0].ai { Ai::Attack { t, .. } => Some(t), _ => None };
            let i = Input { step: if t == Some(2) && !rolled { rolled = true; 1 } else { 0 }, ..Input::default() };
            run(&mut p, &mut f, 1, i);
            if rolled && f.enemies[0].attack().is_none() { break; }
        }
        assert_eq!(p.got, 0);
        assert!(p.word.0.starts_with("Too soon"), "{:?}", p.word);
        // At the mark: rolled six ticks before it lands.
        let (mut p, mut f) = at(Lesson::Roll);
        let mut rolled = false;
        for _ in 0..400 {
            let t = match f.enemies[0].ai { Ai::Attack { t, .. } => Some(t), _ => None };
            let i = Input { step: if t == Some(44) && !rolled { rolled = true; 1 } else { 0 }, ..Input::default() };
            run(&mut p, &mut f, 1, i);
            if p.got > 0 { break; }
        }
        assert_eq!(p.got, 1, "{:?}", p.word);
    }
}
