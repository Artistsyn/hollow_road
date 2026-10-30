//! The controls for a touch screen. Fingers come in from the canvas (`Canvas::on_touch`, each
//! under its own id, so a thumb can steer while another strikes) and go out as what the keyboard
//! and mouse would have given: presses (`Out`) and what is held (`Held`). The game reads those
//! the same way whichever they came from.
//!
//! On the road and in the side-on fight the left of the screen is a stick: a thumb put down
//! anywhere there is its middle, and pushed from it is a way (walk on, back, a lane; left and
//! right in the yard, where a jump is its own button), pushed far is a run. The right holds the buttons, the
//! strike under the thumb where it rests. In a menu a tap on a choice picks it; in a cutscene a
//! tap skips it.
//!
//! Everything is in the canvas's virtual units (3840 x 2160). A phone wider than 16:9 shows the
//! picture between bars, and a thumb on a bar is still read: its x is below 0 or past 3840.

use std::collections::HashMap;

use quartz::TouchPhase;

/// What the controls are for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ui {
    /// On the road: the stick and the fight's buttons, `Use` when there is something to use,
    /// `Turn` when the hero can turn round.
    Road { prompt: bool, ranged: bool, cast: bool, art: bool, turn: bool },
    /// The side-on fight.
    /// `Use` when the rune can be thrown (`game::Pilgrimage::can_bait`).
    Side { prompt: bool, ranged: bool, cast: bool, art: bool },
    /// A menu or words to read: a tap picks, `Back` goes back.
    Menu,
    /// A cutscene, or the title: a tap goes on.
    Tap,
    /// Nothing to press (a fade, a fall, getting up).
    None,
}

/// The buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Btn { Light, Heavy, Roll, Guard, Parry, Flask, Loose, Cast, Art, Jump, Use, Turn, Map, Pause, Back }

impl Btn {
    pub const ALL: [Btn; 15] = [Btn::Light, Btn::Heavy, Btn::Roll, Btn::Guard, Btn::Parry, Btn::Flask, Btn::Loose, Btn::Cast, Btn::Art, Btn::Jump, Btn::Use, Btn::Turn, Btn::Map, Btn::Pause, Btn::Back];

    /// What is written on it.
    pub fn label(self) -> &'static str {
        match self {
            Btn::Light => "STRIKE", Btn::Heavy => "HEAVY", Btn::Roll => "ROLL", Btn::Guard => "GUARD", Btn::Parry => "PARRY",
            Btn::Flask => "FLASK", Btn::Loose => "SHOOT", Btn::Cast => "CAST", Btn::Art => "ART", Btn::Jump => "JUMP", Btn::Use => "USE",
            Btn::Turn => "TURN", Btn::Map => "MAP", Btn::Pause => "II", Btn::Back => "BACK",
        }
    }
}

/// A button where it stands: its middle and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Button { pub btn: Btn, pub at: (f32, f32), pub r: f32 }

/// What a press is, for the game's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tap { Light, Heavy, Roll, Parry, Flask, Loose, Cast, Art, Jump }

/// What the fingers did this tick, as the game takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Out {
    /// A fighting press.
    Tap(Tap),
    /// A key, as if typed: a way when the stick is pushed onto it ('w', 'a', 's', 'd'), `e` to use,
    /// `m` for the map, `t` to turn, `q` to pause or go back (given to the game as Esc).
    Key(char),
    /// A tap in a menu, where (the game finds the choice under it).
    Pick((f32, f32)),
}

/// What is held now: the stick's way, a run, the guard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held { pub up: bool, pub down: bool, pub left: bool, pub right: bool, pub run: bool, pub guard: bool }

impl Held {
    /// Whether way `w` ('w', 'a', 's', 'd') is held.
    pub fn way(&self, w: char) -> bool { match w { 'w' => self.up, 's' => self.down, 'a' => self.left, 'd' => self.right, _ => false } }
}

/// Where the stick takes a thumb: anywhere left of this, below the bars at the top.
pub const STICK_RIGHT: f32 = 1500.0;
pub const STICK_TOP: f32 = 560.0;
/// Pushed this far from where it came down is a way on or back; this far aside, a lane step;
/// this far, near the rim, a run (so a firm walk does not spend stamina). A thumb pushed past
/// the rim drags the middle after it, so coming back the other way answers at once.
pub const DEAD: f32 = 110.0;
pub const SIDE: f32 = 170.0;
pub const RUN: f32 = 350.0;
pub const RIM: f32 = 380.0;
/// A tap in a menu is a finger that came down and up without moving further than this; a swipe,
/// one that went this far one way (twice as far as the other), and is the arrow that way.
const STILL: f32 = 60.0;
const SWIPE: f32 = 240.0;

/// The buttons for `ui`, laid out for the right thumb: the strike biggest, where it rests, the
/// others round it nearest first by how often they are pressed.
pub fn layout(ui: Ui) -> Vec<Button> {
    let b = |btn, x, y, r| Button { btn, at: (x, y), r };
    // Below the foe's bar and name (top right, game.rs `bar_foe`), not over them.
    let top = [b(Btn::Pause, 3700.0, 330.0, 95.0)];
    let mut v: Vec<Button> = match ui {
        Ui::Road { ranged, cast, art, .. } | Ui::Side { ranged, cast, art, .. } => {
            let side = matches!(ui, Ui::Side { .. });
            let mut v = vec![
                b(Btn::Light, 3420.0, 1760.0, 210.0),
                b(Btn::Heavy, 3060.0, 1960.0, 140.0),
                b(Btn::Roll, 3680.0, 1400.0, 140.0),
                b(Btn::Guard, 3000.0, 1600.0, 140.0),
                b(Btn::Parry, 3330.0, 1360.0, 120.0),
                b(Btn::Flask, 2700.0, 1960.0, 110.0),
            ];
            if side { v.push(b(Btn::Jump, 3700.0, 1060.0, 120.0)); }
            if ranged { v.push(b(Btn::Loose, 2650.0, 1690.0, 105.0)); }
            if cast { v.push(b(Btn::Cast, 3030.0, 1290.0, 105.0)); }
            if art { v.push(b(Btn::Art, 2700.0, 1380.0, 110.0)); }
            if let Ui::Side { prompt: true, .. } = ui { v.push(b(Btn::Use, 1920.0, 1940.0, 210.0)); }
            if let Ui::Road { prompt, turn, .. } = ui {
                // The prompt plate itself (game.rs PROMPT_Y, low in the middle) is the button.
if prompt { v.push(b(Btn::Use, 1920.0, 1940.0, 210.0)); }
                if turn { v.push(b(Btn::Turn, 3420.0, 330.0, 95.0)); }
                v.push(b(Btn::Map, 3140.0, 330.0, 95.0));
            }
            v
        }
        Ui::Menu => vec![b(Btn::Back, 3640.0, 1960.0, 130.0)],
        _ => return Vec::new(),
    };
    if !matches!(ui, Ui::Menu) { v.extend(top); }
    v
}

/// What a finger is on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum On { Stick { mid: (f32, f32) }, Button(Btn), Screen }

#[derive(Clone, Copy, Debug)]
struct Finger { on: On, from: (f32, f32), at: (f32, f32), far: f32 }

/// The fingers down and what they hold.
#[derive(Clone, Debug, Default)]
pub struct Pad {
    fingers: HashMap<u64, Finger>,
    /// The stick's ways held, last tick (a way newly held is a press, as a key is).
    ways: [bool; 4],
    /// Whether a finger has ever touched: the controls show from then on.
    pub seen: bool,
}

impl Pad {
    /// The finger on the stick, if any: where it came down (the stick's middle now) and where it is.
    pub fn stick(&self) -> Option<((f32, f32), (f32, f32))> {
        self.fingers.values().find_map(|f| match f.on { On::Stick { mid } => Some((mid, f.at)), _ => None })
    }

    /// Whether `btn` is held down now.
    pub fn pressed(&self, btn: Btn) -> bool { self.fingers.values().any(|f| f.on == On::Button(btn)) }

    /// One finger's event, with the controls laid out for `ui`: what it presses.
    pub fn feed(&mut self, id: u64, phase: TouchPhase, at: (f32, f32), ui: Ui, buttons: &[Button]) -> Vec<Out> {
        self.seen = true;
        let mut out = Vec::new();
        match phase {
            TouchPhase::Started => {
                let on = match buttons.iter().filter(|b| dist(b.at, at) <= b.r * 1.15).min_by(|a, b| dist(a.at, at).total_cmp(&dist(b.at, at))) {
                    Some(b) => On::Button(b.btn),
                    None if matches!(ui, Ui::Road { .. } | Ui::Side { .. }) && at.0 < STICK_RIGHT && at.1 > STICK_TOP && self.stick().is_none() => On::Stick { mid: at },
                    None => On::Screen,
                };
                if let On::Button(b) = on { out.extend(press(b)); }
                self.fingers.insert(id, Finger { on, from: at, at, far: 0.0 });
            }
            TouchPhase::Moved => if let Some(f) = self.fingers.get_mut(&id) {
                f.at = at;
                f.far = f.far.max(dist(f.from, at));
                if let On::Stick { mid } = &mut f.on {
                    // Past the rim, the middle follows the thumb.
                    let (dx, dy) = (at.0 - mid.0, at.1 - mid.1);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d > RIM { *mid = (at.0 - dx / d * RIM, at.1 - dy / d * RIM); }
                }
            },
            TouchPhase::Ended | TouchPhase::Cancelled => if let Some(mut f) = self.fingers.remove(&id) {
                f.far = f.far.max(dist(f.from, at));
                // A tap in a menu picks what is under it; anywhere, in a cutscene, goes on. A swipe
                // is the arrow its way (the sanctum's walk between keeper, shrine and smith).
                if phase == TouchPhase::Ended && f.on == On::Screen && matches!(ui, Ui::Menu | Ui::Tap) {
                    let (dx, dy) = (at.0 - f.from.0, at.1 - f.from.1);
                    if f.far <= STILL {
                        out.push(if ui == Ui::Menu { Out::Pick(f.from) } else { Out::Key('e') });
                    } else if dx.abs() >= SWIPE && dx.abs() >= 2.0 * dy.abs() {
                        out.push(Out::Key(if dx < 0.0 { 'a' } else { 'd' }));
                    } else if dy.abs() >= SWIPE && dy.abs() >= 2.0 * dx.abs() {
                        out.push(Out::Key(if dy < 0.0 { 'w' } else { 's' }));
                    }
                }
            },
        }
        out
    }

    /// What is held now; and a way newly held this tick is pressed, as a key would be (a lane
    /// stepped across on the road).
    pub fn tick(&mut self, out: &mut Vec<Out>) -> Held {
        let mut h = Held { guard: self.pressed(Btn::Guard), ..Held::default() };
        if let Some((mid, at)) = self.stick() {
            let (dx, dy) = (at.0 - mid.0, at.1 - mid.1);
            // The way the thumb leans most; aside only when pushed well aside and more aside than
            // half as much again up or down, so a push up and a little aside is still straight on.
            let (ax, ay) = (dx.abs(), dy.abs());
            h.left = dx < -SIDE && ax >= 0.75 * ay;
            h.right = dx > SIDE && ax >= 0.75 * ay;
            h.up = dy < -DEAD && ay >= 0.5 * ax;
            h.down = dy > DEAD && ay >= 0.5 * ax;
            h.run = (dx * dx + dy * dy).sqrt() >= RUN;
        }
        let now = [h.up, h.left, h.down, h.right];
        for (k, w) in ['w', 'a', 's', 'd'].into_iter().enumerate() {
            if now[k] && !self.ways[k] { out.push(Out::Key(w)); }
        }
        self.ways = now;
        h
    }

    /// Every finger let go of (the game left the screen, or a mode that takes none began).
    pub fn clear(&mut self) { self.fingers.clear(); self.ways = [false; 4]; }
}

fn press(b: Btn) -> Option<Out> {
    Some(match b {
        Btn::Light => Out::Tap(Tap::Light), Btn::Heavy => Out::Tap(Tap::Heavy), Btn::Roll => Out::Tap(Tap::Roll),
        Btn::Parry => Out::Tap(Tap::Parry), Btn::Flask => Out::Tap(Tap::Flask), Btn::Loose => Out::Tap(Tap::Loose),
        Btn::Cast => Out::Tap(Tap::Cast), Btn::Art => Out::Tap(Tap::Art), Btn::Jump => Out::Tap(Tap::Jump),
        Btn::Use => Out::Key('e'), Btn::Turn => Out::Key('t'), Btn::Map => Out::Key('m'), Btn::Pause | Btn::Back => Out::Key('q'),
        Btn::Guard => return None,
    })
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 { ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() }

#[cfg(test)]
mod tests {
    use super::*;

    const ROAD: Ui = Ui::Road { prompt: false, ranged: false, cast: false, art: false, turn: true };

    fn at(b: Btn, ui: Ui) -> (f32, f32) { layout(ui).into_iter().find(|x| x.btn == b).unwrap().at }

    #[test]
    fn a_thumb_steers_while_another_strikes_and_guards() {
        let mut p = Pad::default();
        let bs = layout(ROAD);
        let mut out = Vec::new();
        // The left thumb down, then pushed up: walking on, then a run.
        out.extend(p.feed(1, TouchPhase::Started, (700.0, 1500.0), ROAD, &bs));
        out.extend(p.feed(1, TouchPhase::Moved, (720.0, 1300.0), ROAD, &bs));
        let h = p.tick(&mut out);
        assert!(h.up && !h.run && !h.left && !h.right, "{h:?}");
        assert_eq!(out, vec![Out::Key('w')], "a way newly held is pressed once");
        out.clear();
        p.feed(1, TouchPhase::Moved, (720.0, 1150.0), ROAD, &bs);
        assert!(p.tick(&mut out).run && out.is_empty());
        // The right thumb strikes while the left still holds the way.
        out.extend(p.feed(2, TouchPhase::Started, at(Btn::Light, ROAD), ROAD, &bs));
        assert_eq!(out, vec![Out::Tap(Tap::Light)]);
        p.feed(2, TouchPhase::Ended, at(Btn::Light, ROAD), ROAD, &bs);
        // And guards, held.
        p.feed(3, TouchPhase::Started, at(Btn::Guard, ROAD), ROAD, &bs);
        let h = p.tick(&mut out);
        assert!(h.up && h.guard);
        p.feed(3, TouchPhase::Ended, at(Btn::Guard, ROAD), ROAD, &bs);
        p.feed(1, TouchPhase::Ended, (720.0, 1150.0), ROAD, &bs);
        assert_eq!(p.tick(&mut out), Held::default());
    }

    #[test]
    fn the_stick_reads_a_lane_once_and_its_middle_follows_a_thumb_past_the_rim() {
        let mut p = Pad::default();
        let bs = layout(ROAD);
        let mut out = Vec::new();
        p.feed(1, TouchPhase::Started, (900.0, 1600.0), ROAD, &bs);
        p.feed(1, TouchPhase::Moved, (600.0, 1620.0), ROAD, &bs);
        assert!(p.tick(&mut out).left);
        p.tick(&mut out);
        assert_eq!(out, vec![Out::Key('a')], "held is not pressed again");
        // Far left past the rim, then back a little right: the way turns at once.
        p.feed(1, TouchPhase::Moved, (100.0, 1600.0), ROAD, &bs);
        p.feed(1, TouchPhase::Moved, (100.0 + RIM + 2.0 * SIDE, 1600.0), ROAD, &bs);
        assert!(p.tick(&mut out).right);
    }

    #[test]
    fn a_firm_walk_neither_steps_aside_nor_runs() {
        let mut p = Pad::default();
        let bs = layout(ROAD);
        let mut out = Vec::new();
        // Up firmly and a fair way aside: a walk on, no lane, no run.
        p.feed(1, TouchPhase::Started, (900.0, 1600.0), ROAD, &bs);
        p.feed(1, TouchPhase::Moved, (900.0 + 150.0, 1600.0 - 300.0), ROAD, &bs);
        let held = p.tick(&mut out);
        assert!(held.up && !held.right && !held.run, "{held:?}");
        // To the rim: a run.
        p.feed(1, TouchPhase::Moved, (900.0, 1600.0 - RIM), ROAD, &bs);
        assert!(p.tick(&mut out).run);
        // Well aside: a lane.
        p.feed(1, TouchPhase::Moved, (900.0 + SIDE + 20.0, 1600.0 - 100.0), ROAD, &bs);
        assert!(p.tick(&mut out).right);
    }

    #[test]
    fn a_tap_picks_in_a_menu_goes_on_in_a_cutscene_and_a_drag_does_neither() {
        let mut p = Pad::default();
        p.feed(1, TouchPhase::Started, (2000.0, 1000.0), Ui::Menu, &[]);
        assert_eq!(p.feed(1, TouchPhase::Ended, (2010.0, 1005.0), Ui::Menu, &[]), vec![Out::Pick((2000.0, 1000.0))]);
        p.feed(2, TouchPhase::Started, (2000.0, 1000.0), Ui::Tap, &[]);
        assert_eq!(p.feed(2, TouchPhase::Ended, (2000.0, 1000.0), Ui::Tap, &[]), vec![Out::Key('e')]);
        p.feed(3, TouchPhase::Started, (2000.0, 1000.0), Ui::Menu, &[]);
        p.feed(3, TouchPhase::Moved, (2100.0, 1150.0), Ui::Menu, &[]);
        assert!(p.feed(3, TouchPhase::Ended, (2100.0, 1150.0), Ui::Menu, &[]).is_empty(), "a drag that is neither a tap nor a swipe");
        p.feed(5, TouchPhase::Started, (2000.0, 1000.0), Ui::Tap, &[]);
        assert_eq!(p.feed(5, TouchPhase::Ended, (1600.0, 1050.0), Ui::Tap, &[]), vec![Out::Key('a')], "a swipe left");
        let bs = layout(Ui::Menu);
        assert_eq!(p.feed(4, TouchPhase::Started, at(Btn::Back, Ui::Menu), Ui::Menu, &bs), vec![Out::Key('q')]);
    }

    #[test]
    fn the_yard_has_a_jump_and_the_road_its_use_only_when_asked() {
        let side = Ui::Side { prompt: false, ranged: true, cast: false, art: false };
        let has = |ui, b| layout(ui).iter().any(|x| x.btn == b);
        assert!(has(side, Btn::Jump) && has(side, Btn::Loose) && !has(side, Btn::Cast) && !has(side, Btn::Use));
        assert!(!has(ROAD, Btn::Use) && has(Ui::Road { prompt: true, ranged: false, cast: false, art: false, turn: true }, Btn::Use));
        assert!(!has(ROAD, Btn::Art) && has(Ui::Side { prompt: false, ranged: false, cast: false, art: true }, Btn::Art), "ART only with a forged weapon");
        // No two buttons overlap.
        for ui in [ROAD, side, Ui::Road { prompt: true, ranged: true, cast: true, art: true, turn: true }, Ui::Side { prompt: true, ranged: true, cast: true, art: true }] {
            let bs = layout(ui);
            for (i, a) in bs.iter().enumerate() {
                for b in &bs[i + 1..] { assert!(dist(a.at, b.at) > a.r + b.r, "{:?} overlaps {:?}", a.btn, b.btn); }
                assert!(a.at.0 + a.r <= 3840.0 && a.at.1 + a.r <= 2160.0 && a.at.0 - a.r >= 0.0 && a.at.1 - a.r >= 0.0, "{:?} off the screen", a.btn);
            }
        }
    }
}
