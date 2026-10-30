//! Cutscenes: shots directed over the live road. Each shot can cut the road to a journey stop,
//! move its camera (`PathForgePlugin::swing`: crane, push, orbit, tilt), set its hour and lay a
//! grade over it (`PathForgePlugin::grade`: saturation, vignette, grain, bloom, a lens's focus),
//! walk it, push the canvas camera in on a figure, play a boss's clip, and carry words, a title
//! card, music, sounds, flashes, shakes and slow motion, between letterbox bars, from black or
//! white and back. The game plays one shot at a time (`Pilgrimage::run_scene`); any key that backs
//! out or uses skips to the end.
//!
//! The opening, before a first pilgrimage; a boss's entrance, once the camera has swung round to
//! its yard; a last boss's fall; the trap at the Summit; and the pilgrimage's four ends.

use crate::audio::{Sfx, Tune};
use crate::finale::{Ending, Fates};

/// Ticks a second (the game runs at 60 Hz).
pub const TICKS: f32 = 60.0;
/// Seconds the letterbox bars take to come in and go out, and their height (share of the frame).
const BARS_IN: f32 = 0.8;
pub const BAR: f32 = 0.125;
/// Seconds words take to come and go.
const WORDS_IN: f32 = 0.7;
const TITLE_IN: f32 = 1.1;

/// Where a push of the canvas camera closes in: the frame's middle, the boss, the hero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Focus { Middle, Boss, Hero }

/// Something a shot does at a moment of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Cue {
    /// A camera move on the road (`Runtime::swing`'s JSON).
    Swing(String),
    /// A sound, at this much its level and pitch.
    Sound(Sfx, f32, f32),
    /// Colour over the frame for this many seconds.
    Flash([u8; 4], f32),
    /// The canvas shaking this many pixels for this many seconds.
    Shake(f32, f32),
    /// Slow motion: time at this share of real time for this many seconds.
    Slow(f32, f32),
}

/// Someone in a shot (`Shot::figure`): the folder of their sheet under `assets/pixellab`, the
/// clip they loop, where they stand as the shot starts (metres across, metres ahead of the
/// camera), how fast they walk on ahead (metres a second, as the road walks), and how tall they
/// stand (metres).
#[derive(Clone, Debug, PartialEq)]
pub struct Figure { pub sheet: &'static str, pub clip: &'static str, pub at: (f32, f32), pub pace: f32, pub tall: f32 }

/// One shot: how long, and what it does.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Shot {
    pub secs: f32,
    /// Cut the road to this journey stop, metres in, as the shot starts.
    pub stop: Option<(&'static str, f32)>,
    /// The hour from here on (None inside: each scene's own).
    pub hour: Option<Option<f32>>,
    /// The grade from here on (JSON into the scenes' `post`; None inside: none).
    pub grade: Option<Option<&'static str>>,
    /// Metres a second the road walks.
    pub walk: f32,
    /// Whether the hero is drawn (on the road; a side fight draws its own).
    pub hero: bool,
    /// The canvas camera's zoom from and to over the shot, closing in on what.
    pub push: Option<(f32, f32, Focus)>,
    pub caption: Option<String>,
    /// A title card: the big line and the small one under it.
    pub title: Option<(String, String)>,
    /// Seconds from black at its start, and to black at its end (0: none). The same in white.
    pub black: (f32, f32),
    pub white: (f32, f32),
    /// The music from here on (None inside: silence).
    pub tune: Option<Option<Tune>>,
    /// The boss plays this clip from one second of the shot to another.
    pub pose: Option<(&'static str, f32, f32)>,
    /// What happens when: seconds into the shot.
    pub cues: Vec<(f32, Cue)>,
    /// Who stands on the road in it (the book's people, met in passing).
    pub figures: Vec<Figure>,
}

impl Shot {
    fn new(secs: f32) -> Shot { Shot { secs, ..Shot::default() } }
    fn stop(mut self, stop: &'static str, at: f32) -> Shot { self.stop = Some((stop, at)); self }
    fn hour(mut self, h: f32) -> Shot { self.hour = Some(Some(h)); self }
    fn grade(mut self, g: &'static str) -> Shot { self.grade = Some(Some(g)); self }
    fn walk(mut self, m: f32) -> Shot { self.walk = m; self }
    fn hero(mut self) -> Shot { self.hero = true; self }
    fn push(mut self, from: f32, to: f32, at: Focus) -> Shot { self.push = Some((from, to, at)); self }
    fn caption(mut self, words: &str) -> Shot { self.caption = Some(words.into()); self }
    fn title(mut self, big: &str, small: &str) -> Shot { self.title = Some((big.into(), small.into())); self }
    fn black(mut self, from: f32, to: f32) -> Shot { self.black = (from, to); self }
    fn white(mut self, from: f32, to: f32) -> Shot { self.white = (from, to); self }
    fn tune(mut self, t: Option<Tune>) -> Shot { self.tune = Some(t); self }
    fn pose(mut self, clip: &'static str, from: f32, to: f32) -> Shot { self.pose = Some((clip, from, to)); self }
    fn at(mut self, s: f32, cue: Cue) -> Shot { self.cues.push((s, cue)); self }
    fn figure(mut self, sheet: &'static str, clip: &'static str, at: (f32, f32), pace: f32, tall: f32) -> Shot {
        self.figures.push(Figure { sheet, clip, at, pace, tall });
        self
    }
    /// The camera set at once, then moved to `to` over the rest of the shot.
    fn camera(self, from: &str, to: &str) -> Shot {
        let secs = (self.secs - 0.1).max(0.1);
        self.at(0.0, Cue::Swing(with(from, r#""seconds":0.02"#))).at(0.05, Cue::Swing(with(to, &format!(r#""seconds":{secs:.2}"#))))
    }
    fn ticks(&self) -> u32 { (self.secs * TICKS).round().max(1.0) as u32 }
}

/// A swing's JSON with one more field.
fn with(json: &str, field: &str) -> String {
    let body = json.trim().trim_start_matches('{').trim_end_matches('}').trim();
    if body.is_empty() { format!("{{{field}}}") } else { format!("{{{body},{field}}}") }
}

fn smooth(x: f32) -> f32 { let x = x.clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) }

/// A cutscene being played: its shots, which one, and ticks into it.
#[derive(Clone, Debug, PartialEq)]
pub struct Cutscene {
    pub name: &'static str,
    shots: Vec<Shot>,
    pub i: usize,
    pub t: u32,
    /// Whether what it overlays stands still while it plays (a boss waits for its entrance; a
    /// fall plays out under it).
    pub freeze: bool,
    /// Whether it moved the road's camera, hour and grade, to be put back after.
    pub owns_road: bool,
}

impl Cutscene {
    fn of(name: &'static str, shots: Vec<Shot>, freeze: bool, owns_road: bool) -> Cutscene { Cutscene { name, shots, i: 0, t: 0, freeze, owns_road } }

    pub fn shot(&self) -> Option<&Shot> { self.shots.get(self.i) }
    pub fn done(&self) -> bool { self.i >= self.shots.len() }
/// The journey stops it cuts to, in order (for the road to get ready before it plays; a fall
/// into white adds the trap's, which follows it).
pub fn stops(&self) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = self.shots.iter().filter_map(|s| s.stop.map(|(stop, _)| stop)).collect();
    if self.ends_white() && self.name == "fall" { v.extend(Cutscene::trap(false, true).stops()); }
    v
}
    /// The shot just begun (its stop, hour, grade to apply).
    pub fn starting(&self) -> Option<&Shot> { self.shot().filter(|_| self.t == 0) }
    /// What happens on this tick.
    pub fn cues(&self) -> Vec<&Cue> {
        let Some(s) = self.shot() else { return Vec::new() };
        s.cues.iter().filter(|(at, _)| (at * TICKS).round() as u32 == self.t).map(|(_, c)| c).collect()
    }
    /// On a tick: false once it is over.
    pub fn advance(&mut self) -> bool {
        let Some(n) = self.shot().map(Shot::ticks) else { return false };
        self.t += 1;
        if self.t >= n { self.i += 1; self.t = 0; }
        !self.done()
    }
    /// To the end at once.
    pub fn skip(&mut self) { (self.i, self.t) = (self.shots.len(), 0); }
    /// Whether it ends on white (the next scene comes up out of it).
    pub fn ends_white(&self) -> bool { self.shots.last().is_some_and(|s| s.white.1 > 0.0) }

    fn secs(&self) -> f32 { self.t as f32 / TICKS }
    fn left(&self) -> f32 { self.shot().map_or(0.0, |s| s.secs - self.secs()) }

    /// How black the frame is (0..1): coming up from black at a shot's start, going to it at its end.
    pub fn black(&self) -> f32 { self.shot().map_or(0.0, |s| fade(s.black, self.secs(), self.left())) }
    pub fn white(&self) -> f32 { self.shot().map_or(0.0, |s| fade(s.white, self.secs(), self.left())) }
    /// The letterbox bars' height, a share of the frame each: in over the first shot's start, out
    /// over the last shot's end (unless it ends in black or white, which keep them).
    pub fn bars(&self) -> f32 {
        let Some(s) = self.shot() else { return 0.0 };
        let mut k: f32 = 1.0;
        if self.i == 0 { k = k.min(self.secs() / BARS_IN); }
        if self.i + 1 == self.shots.len() && s.black.1 <= 0.0 && s.white.1 <= 0.0 { k = k.min(self.left() / BARS_IN); }
        BAR * smooth(k)
    }
    /// The words under the picture, and how clear (0..1).
    pub fn caption(&self) -> Option<(&str, f32)> {
        let s = self.shot()?;
        let words = s.caption.as_deref()?;
        Some((words, smooth(((self.secs() - 0.3) / WORDS_IN).min(self.left() / WORDS_IN))))
    }
    pub fn title(&self) -> Option<(&str, &str, f32)> {
        let s = self.shot()?;
        let (big, small) = s.title.as_ref()?;
        Some((big, small, smooth(((self.secs() - 0.2) / TITLE_IN).min(self.left() / TITLE_IN))))
    }
    /// The canvas camera's zoom now, and what it closes on.
    pub fn push(&self) -> Option<(f32, Focus)> {
        let s = self.shot()?;
        let (from, to, at) = s.push?;
        Some((from + (to - from) * smooth(self.secs() / s.secs), at))
    }
    /// The boss's clip and how far through it (0..1), while the shot plays one.
    pub fn pose(&self) -> Option<(&'static str, f32)> {
        let (clip, from, to) = self.shot()?.pose?;
        let k = (self.secs() - from) / (to - from).max(0.01);
        (0.0..1.0).contains(&k).then_some((clip, k))
    }
    pub fn walk(&self) -> f32 { self.shot().map_or(0.0, |s| s.walk) }
    /// Who stands in the shot now: each figure, metres across and metres ahead of the camera (a
    /// figure walking as fast as the road keeps its distance).
    pub fn figures(&self) -> Vec<(&Figure, f32, f32)> {
        let Some(s) = self.shot() else { return Vec::new() };
        let secs = self.secs();
        s.figures.iter().map(|f| (f, f.at.0, f.at.1 + (f.pace - s.walk) * secs)).collect()
    }
    /// The sheets its figures are drawn from, each once.
    pub fn sheets(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for f in self.shots.iter().flat_map(|s| &s.figures) { if !out.contains(&f.sheet) { out.push(f.sheet); } }
        out
    }
    pub fn hero(&self) -> bool { self.shot().is_some_and(|s| s.hero) }
    /// The stop the shot under way is set on: its own, else the last one a shot before it cut to.
    pub fn road(&self) -> Option<&'static str> { self.shots[..(self.i + 1).min(self.shots.len())].iter().rev().find_map(|s| s.stop).map(|(stop, _)| stop) }
    /// The music the shots so far asked for, if any did (None: leave it as it is).
    pub fn tune(&self) -> Option<Option<Tune>> { self.shots[..=self.i.min(self.shots.len().saturating_sub(1))].iter().rev().find_map(|s| s.tune) }

    /// Before a first pilgrimage, as the Souls games open: the world's myth (the Veil of runes
    /// walling off the hunger beyond, the Eye that tore it, the three pieces and their guardians),
    /// why the world is failing now (the tear widening, the Taint), two places as rumour, the
    /// hero's condition (one of the few who rise whole) and where the road starts. The book's
    /// people and the Order are left for the road.
    pub fn opening() -> Cutscene {
        Cutscene::of("opening", vec![
            Shot::new(9.0).stop("spire_summit@sealed_slab", 2.0).hour(18.7).walk(0.35).black(1.8, 0.0).tune(Some(Tune::Title))
                .grade(r#"{"saturation":0.9,"exposure":1.15,"vignette":0.5,"grain":0.1,"bloom":0.6}"#)
                .camera(r#"{"eye_height":0.9,"horizon":0.64,"zoom":1.0}"#, r#"{"eye_height":2.2,"horizon":0.5,"zoom":1.22}"#)
                .caption("Before the Shatter, a Veil of runes walled the world off from the hunger beyond it. The old stones sang, and the Veil held."),
            Shot::new(8.0).stop("statue_court@angel_ruins", 6.0).hour(19.6).walk(0.3).black(0.9, 0.0)
                .grade(r#"{"saturation":0.7,"exposure":1.1,"vignette":0.6,"grain":0.12,"bloom":0.7}"#)
                .camera(r#"{"eye_height":6.0,"horizon":0.3}"#, r#"{"eye_height":3.6,"horizon":0.36,"zoom":1.05}"#)
                .caption("Then cloaked mortals turned the Eye that Sees Beyond upon the Veil, and tore it. Their names were not kept."),
            Shot::new(7.5).hour(19.6).walk(0.3)
                .camera(r#"{"eye_height":3.6,"horizon":0.36,"zoom":1.05}"#, r#"{"eye_height":2.4,"horizon":0.4,"zoom":1.12}"#)
                .caption("The angels broke the Eye in three, and set a guardian over every piece. They have kept their watch too long."),
            Shot::new(8.5).stop("vale@vale_edge", 4.0).hour(20.8).walk(0.5).black(0.9, 0.0)
                .grade(r#"{"saturation":0.55,"contrast":1.05,"exposure":1.25,"vignette":0.6,"grain":0.16}"#)
                .camera(r#"{"eye_height":7.0,"horizon":0.3}"#, r#"{"eye_height":2.6,"horizon":0.26}"#)
                .caption("The tear never closed. Through it seeps the Taint: mist on the roads, whispers in the mind, and the dead who will not stay fallen."),
            Shot::new(6.0).stop("hollowmere@gate", 26.0).hour(23.0).walk(0.25).black(0.7, 0.0)
                .grade(r#"{"saturation":0.45,"exposure":1.1,"vignette":0.7,"grain":0.18}"#)
                .camera(r#"{"eye_height":1.6,"horizon":0.46}"#, r#"{"eye_height":1.4,"horizon":0.48,"zoom":1.15}"#)
                .caption("In Hollowmere, something has come up the old well. The village hears it whispering.")
                .at(1.2, Cue::Sound(Sfx::Knell, 0.5, 0.4)),
            Shot::new(6.0).stop("cathedral@varnholt", 8.0).hour(22.4).walk(0.25).black(0.7, 0.0)
                .grade(r#"{"saturation":0.6,"exposure":1.05,"vignette":0.7,"grain":0.18}"#)
                .camera(r#"{"eye_height":1.2,"horizon":0.56}"#, r#"{"eye_height":1.5,"horizon":0.52,"zoom":1.12}"#)
                .caption("In Varnholt, the cathedral bells toll like a dirge, then stop, and the priest swears to guard what lies beneath."),
            Shot::new(8.0).stop(crate::atlas::START, 0.0).hour(4.8).black(1.2, 0.0).hero().push(1.0, 1.12, Focus::Hero)
                .grade(r#"{"saturation":0.7,"exposure":0.95,"vignette":0.7,"grain":0.14}"#)
                .camera(r#"{"eye_height":2.6,"horizon":0.34,"zoom":1.0}"#, r#"{"eye_height":2.2,"horizon":0.36,"zoom":1.05}"#)
                .caption("Most of those the Taint takes rise hollow. A few rise whole, with only scraps of who they were, and hear the old stones calling."),
            Shot::new(5.0).hour(4.8).hero().push(1.12, 1.3, Focus::Hero)
                .caption("You are one of these."),
            Shot::new(6.5).stop("warrens@warrens_lane", 6.0).hour(21.5).walk(0.3).black(0.9, 0.0)
                .grade(r#"{"saturation":0.4,"exposure":0.9,"vignette":0.75,"grain":0.18}"#)
                .camera(r#"{"eye_height":1.6,"horizon":0.48,"zoom":1.0}"#, r#"{"eye_height":1.5,"horizon":0.48,"zoom":1.1}"#)
                .caption("In the Ashen Warrens, the hunters' guild pays for proof of any kill. The road begins there."),
            Shot::new(7.0).stop("warrens@guild_hall", 3.0).hour(21.5).walk(0.25).black(0.9, 1.6)
                .grade(r#"{"saturation":0.3,"exposure":0.8,"vignette":0.85,"grain":0.2}"#)
                .camera(r#"{"eye_height":1.4,"horizon":0.5,"zoom":1.0}"#, r#"{"eye_height":1.6,"horizon":0.48,"zoom":1.35}"#)
                .title("HOLLOW ROAD", "Seek the stones that sang.")
                .at(1.0, Cue::Sound(Sfx::Knell, 0.9, 0.6)),
        ], false, true)
    }

    /// A boss's entrance in its yard: the bars come in while the camera swings round, the view
    /// closes on it as it shows what it is, its name and what it is called, and back out for the
    /// fight. A mini-boss's is shorter.
    pub fn entrance(name: &'static str, mini: bool) -> Cutscene {
        let k = if mini { 0.7 } else { 1.0 };
        let mut shots = vec![
            // The camera's swing round to the yard (`ARENA_IN`), the bars sliding in over it.
            Shot::new(140.0 / TICKS).grade(r#"{"vignette":0.55,"saturation":0.85}"#),
            Shot::new(2.2 * k).push(1.0, 1.42, Focus::Boss).pose("toll", 0.5 * k, 2.0 * k)
                .at(1.05 * k, Cue::Sound(Sfx::StruckHeavy, 0.9, 0.55))
                .at(1.1 * k, Cue::Shake(14.0, 0.45)),
            Shot::new(2.8 * k).push(1.42, 1.5, Focus::Boss).title(&name.to_uppercase(), epithet(name)),
            Shot::new(0.8).push(1.5, 1.0, Focus::Boss),
        ];
        if mini { shots[2].secs = 2.4; }
        // Those that speak into the head, as the book's do, say it as they show themselves.
        let breed = crate::atlas::PLACES.iter().filter_map(|p| p.boss).map(|(_, b)| b)
            .chain([&crate::order::VORYN_VESSEL, &crate::order::VORYN_WHOLE]).find(|b| b.name() == name).map(|b| b.breed);
        if let Some(words) = breed.and_then(crate::side::Breed::whisper) { shots[1] = shots[1].clone().caption(words); }
        Cutscene::of("entrance", shots, true, false)
    }

    /// A last boss falls: slow, a flash of white, the view closing on it, and its words; into
    /// white if what follows comes out of it.
    pub fn fall(words: &str, into_white: bool) -> Cutscene {
        let last = Shot::new(5.0).push(1.3, 1.6, Focus::Boss).caption(words);
        Cutscene::of("fall", vec![
            Shot::new(4.0).push(1.0, 1.3, Focus::Boss).tune(None).grade(r#"{"bloom":1.2,"exposure":1.2}"#)
                .at(0.0, Cue::Slow(0.35, 2.6)).at(0.0, Cue::Flash([255, 250, 235, 210], 0.7)).at(0.0, Cue::Shake(18.0, 0.8))
                .at(0.05, Cue::Sound(Sfx::Knell, 1.0, 0.45)),
            if into_white { last.white(0.0, 2.6) } else { last },
        ], false, false)
    }

    /// Exodus down, over his yard: the Eye caught as it rolls, and the Order's secret said (the
    /// Prophet came through the first tear and asked only for a door). Then the one the guardians
    /// named comes through: whole, through Unit 0 finished, if Fin-S was left on his plinth; else,
    /// with no door, into Exodus. What it rose in has its own entrance after.
    pub fn possession(through_fin_s: bool) -> Cutscene {
        let violet = [150, 80, 220, 200];
        let (offer, named, risen) = if through_fin_s {
            ("'The wooden one is finished. Unit 0, our door. Prophet, the door is open. Come through.'",
             "Down the corridor something wooden walks in, finished, every joint lit. Out of it speaks the one the guardians named: Voryn. 'All of me, at last.'",
             "The Eye flies from his hand to its chest, and sets there. What stands in the chamber is wood still, and nothing else it was.")
        } else {
            ("'The wooden one was to be his door, and you took it. Then I will be the door. Prophet, take me.'",
             "Out of the Eye speaks the one the guardians named: Voryn. 'Not all of me, then. Enough.'",
             "The Eye opens in his hand, and keeps opening. What gets up is not Exodus any more.")
        };
        Cutscene::of("possession", vec![
            Shot::new(3.5).push(1.0, 1.25, Focus::Boss).tune(None).grade(r#"{"saturation":0.8,"vignette":0.6}"#).at(0.0, Cue::Slow(0.4, 2.0))
                .caption("Exodus goes down on one knee. The Eye rolls from his hand, and he catches it."),
            Shot::new(6.0).push(1.25, 1.35, Focus::Boss)
                .caption("'The Prophet came to us through the first tear. He taught us harmony, as one. He asked only for a door.'"),
            Shot::new(5.0).push(1.35, 1.4, Focus::Boss).caption(offer),
            Shot::new(5.5).push(1.4, 1.5, Focus::Boss).caption(named)
                .at(0.6, Cue::Flash(violet, 0.9)).at(0.6, Cue::Shake(8.0, 0.7)).at(0.6, Cue::Sound(Sfx::Knell, 1.0, 0.35)),
            Shot::new(5.0).push(1.5, 1.0, Focus::Boss).grade(r#"{"saturation":0.6,"contrast":1.1,"vignette":0.75,"grain":0.18,"tint":[214,180,255]}"#)
                .caption(risen)
                .at(0.3, Cue::Flash([170, 90, 255, 235], 1.6)).at(0.3, Cue::Shake(20.0, 1.2)).at(0.4, Cue::Sound(Sfx::StruckHeavy, 1.0, 0.5)),
        ], false, false)
    }

    /// At the Summit, out of the Archangel's white: the Eye made whole, the three who made the Veil
    /// (and the hint that a sealed tear is heard), Olekeen arriving if he was cut free below (else
    /// still tied there), Exodus, the gas, Fin-S carried off if he came, and the hero waking by
    /// the fire with the Eye gone.
    pub fn trap(fin_s: bool, olekeen_free: bool) -> Cutscene {
        let stop = "spire_summit@sealed_slab";
        let grade = r#"{"bloom":1.0,"exposure":1.1,"saturation":0.95,"vignette":0.45}"#;
        let mut shots = vec![
            Shot::new(7.5).stop(stop, 24.0).hour(6.0).white(2.4, 0.0).tune(Some(Tune::Shrine)).grade(grade)
                .camera(r#"{"eye_height":1.2,"horizon":0.62}"#, r#"{"eye_height":2.0,"horizon":0.5,"zoom":1.1}"#)
                .caption("Three shards, brought together, close as a wound closes. The Eye that Sees Beyond is whole, and it looks back.")
                .at(1.0, Cue::Sound(Sfx::Knell, 0.7, 0.7)).at(1.0, Cue::Flash([255, 246, 220, 140], 0.8)),
            Shot::new(9.0).hour(6.0)
                .caption("Out of its light, three voices: Lirathar, Sylvara, Korath. 'We walled Voryn away, and called the wall the Veil. Where it is torn, it can be closed. Close enough of it, and we will hear you.'"),
            if olekeen_free {
                Shot::new(6.0).hour(6.0).figure("olekeen", "idle", (-0.7, 4.2), 0.0, 1.3)
                    .caption("Olekeen comes up the last stair, breathing hard, a bow in his hand again. 'Hunter! Olekeen late. Olekeen... not first.'")
            } else {
                Shot::new(6.0).hour(6.0)
                    .caption("Nobody comes up the stair behind you. Below, by the slab's fire, Olekeen is still tied where the Order left him.")
            },
            Shot::new(6.5).hour(6.0).figure("exodus", "idle", (0.8, 5.6), 0.0, 1.55)
                .caption("Exodus Lightfoot steps out of the stair's shadow. 'The Eye was never yours. It was lent. The Radiant Veil takes back what it lent.'"),
            Shot::new(5.0).hour(6.0).grade(r#"{"saturation":0.6,"exposure":0.9,"vignette":0.7,"grain":0.2}"#)
                .caption("Beaked masks in the doorway. Little pots break on the stone, and the air goes sweet as rot.")
                .at(0.4, Cue::Flash([150, 170, 120, 170], 1.6)).at(0.4, Cue::Shake(6.0, 0.6)).at(0.4, Cue::Sound(Sfx::Knell, 0.5, 1.4)),
        ];
        if fin_s { shots.push(Shot::new(5.0).caption("They lift the wooden one like cargo. 'Unit 0,' one says. 'Careful. It is not finished.'")); }
        shots.push(Shot::new(7.5).black(0.0, 2.2).caption("You wake by the fire. It was a death in all but name. The Eye is gone, and so is everyone. Drag marks lead on over the summit."));
        Cutscene::of("trap", shots, false, true)
    }

    /// The pilgrimage's end, as the Eye was chosen for: what the hero did, what it did to the
    /// world, what became of everyone the road left behind, its name, and the road begun again.
    pub fn ending(end: Ending, fates: &Fates) -> Cutscene {
        let (act, world, title) = Self::told_end(end);
        let mut shots = vec![
            Shot::new(8.0).stop("eye_chamber@penitent_cells", 34.0).hour(23.0).white(2.4, 0.0).tune(Some(Tune::Shrine))
                .grade(r#"{"bloom":1.1,"exposure":1.05,"vignette":0.55}"#)
                .camera(r#"{"eye_height":1.4,"horizon":0.5}"#, r#"{"eye_height":1.8,"horizon":0.46,"zoom":1.18}"#)
                .caption(act).at(1.2, Cue::Flash(Self::flash(end), 1.4)).at(1.2, Cue::Sound(Sfx::Knell, 0.9, 0.5)),
            world.caption(Self::told_world(end)),
        ];
        for (stop, at, words) in fates.told() {
            shots.push(Shot::new(6.0).stop(stop, at).hour(7.0).walk(0.3).black(0.8, 0.0)
                .grade(r#"{"saturation":0.8,"vignette":0.5,"grain":0.12}"#)
                .camera(r#"{"eye_height":2.4,"horizon":0.36}"#, r#"{"eye_height":2.0,"horizon":0.4,"zoom":1.06}"#)
                .caption(words));
        }
        shots.push(Shot::new(5.5).black(0.8, 0.0).title(title.0, title.1).at(0.6, Cue::Sound(Sfx::Knell, 0.8, 0.5)));
        shots.push(Shot::new(5.5).black(1.0, 1.6).title("THE PILGRIMAGE BEGINS AGAIN", "The dead return stronger.")
            .at(0.6, Cue::Sound(Sfx::Knell, 0.8, 0.5)));
        Cutscene::of("ending", shots, false, true)
    }

    /// What the hero did with the Eye, the shot of the world after it, and the ending's name.
    fn told_end(end: Ending) -> (&'static str, Shot, (&'static str, &'static str)) {
        let world = |stop, at, hour, grade| Shot::new(8.0).stop(stop, at).hour(hour).walk(0.3).black(1.0, 0.0).grade(grade)
            .camera(r#"{"eye_height":6.0,"horizon":0.3}"#, r#"{"eye_height":2.6,"horizon":0.3}"#);
        match end {
            Ending::Seal => ("Three voices answer out of the light. What came through of Voryn goes back, and the Veil closes after it like skin over a wound.",
                world("spire_summit@sealed_slab", 20.0, 6.2, r#"{"bloom":0.9,"exposure":1.12,"saturation":1.05,"vignette":0.3}"#),
                ("THE VEIL MENDED", "The stones sing again.")),
            Ending::Fall => ("The Eye opens on the Veil, as it once did for the cloaked ones. Beyond it, something vast and patient turns, and looks back.",
                world("vale@vale_edge", 4.0, 23.5, r#"{"saturation":0.5,"exposure":0.9,"vignette":0.8,"grain":0.2,"tint":[200,160,255]}"#),
                ("THE VEIL FALLS", "What waited beyond it is not waiting now.")),
            Ending::Order => ("The masked ones bow. 'Harmony,' they say, 'as one.' Across the world, all at once, every bell stops.",
                world("varnholt@violet_road", 6.0, 12.0, r#"{"saturation":0.35,"exposure":1.05,"vignette":0.5,"grain":0.1}"#),
                ("HARMONY, AS ONE", "The Radiant Veil keeps what it was given.")),
            Ending::Break => ("The Eye breaks on the stone, and does not mend. Whatever looked out of it is gone.",
                world("hollowmere@gate", 20.0, 6.6, r#"{"saturation":0.6,"exposure":1.0,"vignette":0.5,"grain":0.14}"#),
                ("THE EYE BROKEN", "The world endures, torn.")),
        }
    }

    /// What the world became.
    fn told_world(end: Ending) -> &'static str {
        match end {
            Ending::Seal => "The dead lie down, and stay down. The shrine fires burn low, and then go out, and are not needed.",
            Ending::Fall => "Voryn's hunger comes through. The mists go first, then the roads, then the names of things.",
            Ending::Order => "The Order keeps the Eye. The roads are quiet, and no one asks what it costs.",
            Ending::Break => "The tears stay as they are. So does the world: torn, and still here.",
        }
    }

    fn flash(end: Ending) -> [u8; 4] {
        match end { Ending::Seal => [255, 250, 235, 220], Ending::Fall => [150, 90, 220, 200], Ending::Order => [255, 214, 140, 180], Ending::Break => [230, 230, 240, 160] }
    }
}

/// How far into black or white (0..1): up from it over `fade.0` seconds at the start, down into it
/// over `fade.1` at the end.
fn fade(fade: (f32, f32), secs: f32, left: f32) -> f32 {
    let up = if fade.0 > 0.0 { 1.0 - smooth(secs / fade.0) } else { 0.0 };
    let down = if fade.1 > 0.0 { 1.0 - smooth(left / fade.1) } else { 0.0 };
    up.max(down)
}

/// What a boss is called on its title card.
pub fn epithet(name: &str) -> &'static str {
    match name {
        "The Debt Collector" => "What the Warrens owe, he takes",
        "The Bell Warden" => "Last keeper of Hollowmere's bells",
        "The Keening Cage" => "What came out of the eastern reeds",
        "The Mother of the Black Water" => "What keeps the drowned",
        "The Horror in the Hollow" => "What waited in the fog",
        "The Echo of Lurz" => "What the fog kept of a frightened man",
        "The Echo of Krix" => "Dead don't mean gone",
        "The Hollowmere Screamer" => "The voice in the well",
        "The Pack Alpha" => "First of the ridge's wretched pack",
        "The Bandit Chief" => "Lord of the ridge camp",
        "The Order Agent" => "Sworn to an Order no one names",
        "The Hollow Magistrate" => "Judge of a city with no one left to judge",
        "The Wraith Lord" => "Crowned in the hollow sanctum",
        "The Rune Sentinel" => "Warden of the sealed vault",
        "The Butcher" => "Keeper of the husk pens",
        "The Angel Priest" => "Who sings to the Archangel still",
        "The Void Stalker" => "What the altars whispered to",
        "The Statue Archangel" => "Eryndor's stone sentinel",
        "The Radiant Construct" => "Keeper of the pilgrims' road",
        "The Warped Drake" => "What nests where the sky broke",
        "The Echo of Bounty" => "What the wood kept of the fox",
        "The Archangel" => "Last guardian of the Eye",
        "The Crimson Captain" => "Who keeps the Order's gate",
        "Exodus Lightfoot" => "The Radiant Veil's open hand",
        "Voryn's Vessel" => "What the Order called its Prophet",
        "Voryn Made Whole" => "Unit 0, finished as the Order meant",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Cutscene> {
        let fates = crate::finale::Fates { atrael: Some(true), lost: Some(1), fin_s: crate::finale::Unit::Free, steel: crate::steel::Steel::Flesh, taint: 1, bounty: Some(true) };
        let mut v = vec![Cutscene::opening(), Cutscene::fall("", true), Cutscene::trap(true, true), Cutscene::possession(true), Cutscene::possession(false)];
        v.extend([Ending::Seal, Ending::Fall, Ending::Order, Ending::Break].map(|e| Cutscene::ending(e, &fates)));
        v.extend(bosses().map(|(name, mini)| Cutscene::entrance(name, mini)));
        v
    }

    /// Every boss some place has.
    fn bosses() -> impl Iterator<Item = (&'static str, bool)> { crate::atlas::PLACES.iter().filter_map(|p| p.boss).map(|(_, b)| (b.name(), b.mini)) }

    /// Every stop a shot cuts to is one the journey has; every swing and grade is JSON the road
    /// reads; every boss has its words.
    #[test]
    fn every_shot_cuts_to_a_real_stop_with_json_the_road_reads() {
        let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(crate::assets::path(crate::assets::JOURNEY)).unwrap()).unwrap();
        for c in all() {
            for s in &c.shots {
                if let Some((stop, _)) = s.stop { assert!(j["stops"].get(stop).is_some(), "{}: no stop {stop}", c.name); }
                if let Some(Some(g)) = s.grade { serde_json::from_str::<serde_json::Value>(g).unwrap_or_else(|e| panic!("{}: grade {g}: {e}", c.name)); }
                for (at, cue) in &s.cues {
                    assert!(*at < s.secs, "{}: a cue after its shot", c.name);
                    if let Cue::Swing(w) = cue { serde_json::from_str::<serde_json::Value>(w).unwrap_or_else(|e| panic!("{}: swing {w}: {e}", c.name)); }
                }
            }
        }
        for (name, _) in bosses() { assert!(!epithet(name).is_empty(), "{name} has no epithet"); }
    }

    /// The opening comes up from black and goes to black, its words fade in and out, and the bars
    /// come in; it plays to its end, and a skip ends it at once.
    #[test]
    fn a_cutscene_plays_shot_by_shot_from_black_to_black() {
        let mut c = Cutscene::opening();
        assert_eq!((c.black(), c.bars()), (1.0, 0.0));
        assert!(c.starting().is_some_and(|s| s.stop.is_some()));
        assert!(c.cues().iter().any(|q| matches!(q, Cue::Swing(_))));
        let mut ticks = 0;
        let (mut words, mut most_bars, mut end_black) = (0.0f32, 0.0f32, 0.0f32);
        while c.advance() {
            ticks += 1;
            words = words.max(c.caption().map_or(0.0, |w| w.1));
            most_bars = most_bars.max(c.bars());
            end_black = c.black();
        }
        let total: f32 = Cutscene::opening().shots.iter().map(|s| s.secs).sum();
        assert!((ticks as f32 / TICKS - total).abs() < 0.1, "{ticks} ticks for {total} s");
        assert!(words > 0.99 && (most_bars - BAR).abs() < 1e-4 && end_black > 0.95, "{words} {most_bars} {end_black}");
        let mut d = Cutscene::opening();
        d.advance();
        d.skip();
        assert!(d.done() && !d.advance());
    }

    /// Every figure is drawn from a sheet the game has, with the clip it plays; one walking with the
    /// road keeps its distance from the camera.
    #[test]
    fn every_figure_stands_on_a_sheet_with_its_clip() {
        for c in all() {
            for f in c.shots.iter().flat_map(|s| &s.figures) {
                let dir = crate::assets::path("pixellab").join(f.sheet).join(f.clip);
                assert!(std::fs::read_dir(&dir).is_ok_and(|mut d| d.next().is_some()), "{}: no frames in {}", c.name, dir.display());
            }
        }
        let walking = Shot::new(4.0).walk(0.9).figure("olekeen", "away", (0.2, 5.5), 0.9, 1.35).figure("exodus", "idle", (0.0, 8.0), 0.0, 1.95);
        let mut c = Cutscene::of("test", vec![walking], false, false);
        let near = c.figures()[0].2;
        for _ in 0..120 { c.advance(); }
        assert!((c.figures()[0].2 - near).abs() < 1e-3, "{near} -> {}", c.figures()[0].2);
        assert!((c.figures()[1].2 - (8.0 - 0.9 * 2.0)).abs() < 1e-3, "one standing still comes nearer");
        assert_eq!(c.sheets(), vec!["olekeen", "exodus"]);
    }

    /// A boss's entrance closes in on it while it tolls, names it, and opens back out to 1 before
    /// the fight.
    #[test]
    fn an_entrance_names_the_boss_and_opens_back_out() {
        let mut c = Cutscene::entrance("The Bell Warden", false);
        let (mut named, mut posed, mut most) = (false, false, 1.0f32);
        while c.advance() {
            named |= c.title().is_some_and(|t| t.0 == "THE BELL WARDEN" && t.2 > 0.9);
            posed |= c.pose().is_some_and(|p| p.0 == "toll");
            if let Some((z, Focus::Boss)) = c.push() { most = most.max(z); if c.i == 3 && c.t + 1 == c.shots[3].ticks() { assert!((z - 1.0).abs() < 0.01, "{z}"); } }
        }
        assert!(named && posed && most > 1.4);
    }
}
