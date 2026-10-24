//! Cutscenes: shots directed over the live road. Each shot can cut the road to a journey stop,
//! move its camera (`PathForgePlugin::swing`: crane, push, orbit, tilt), set its hour and lay a
//! grade over it (`PathForgePlugin::grade`: saturation, vignette, grain, bloom, a lens's focus),
//! walk it, push the canvas camera in on a figure, play a boss's clip, and carry words, a title
//! card, music, sounds, flashes, shakes and slow motion, between letterbox bars, from black or
//! white and back. The game plays one shot at a time (`Pilgrimage::run_scene`); any key that backs
//! out or uses skips to the end.
//!
//! Three of them: the opening, before a first pilgrimage; a boss's entrance, once the camera has
//! swung round to its yard; and the Archangel's fall, then the pilgrimage's end.

use crate::audio::{Sfx, Tune};

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
/// The journey stops it cuts to, in order (for the road to get ready before it plays; the fall
/// adds the ending's, which follows it).
pub fn stops(&self) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = self.shots.iter().filter_map(|s| s.stop.map(|(stop, _)| stop)).collect();
    if self.name == "fall" { v.extend(Cutscene::ending().stops()); }
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
    pub fn hero(&self) -> bool { self.shot().is_some_and(|s| s.hero) }
    /// The music the shots so far asked for, if any did (None: leave it as it is).
    pub fn tune(&self) -> Option<Option<Tune>> { self.shots[..=self.i.min(self.shots.len().saturating_sub(1))].iter().rev().find_map(|s| s.tune) }

    /// Before a first pilgrimage: the Spire and what it keeps, the roads below gone wrong, the
    /// pilgrim's errand, and the title.
    pub fn opening() -> Cutscene {
        Cutscene::of("opening", vec![
            Shot::new(9.0).stop("spire_summit@sealed_slab", 2.0).hour(18.7).walk(0.35).black(1.8, 0.0).tune(Some(Tune::Title))
                .grade(r#"{"saturation":0.9,"exposure":1.15,"vignette":0.5,"grain":0.1,"bloom":0.6}"#)
                .camera(r#"{"eye_height":0.9,"horizon":0.64,"zoom":1.0}"#, r#"{"eye_height":2.2,"horizon":0.5,"zoom":1.22}"#)
                .caption("Above the world, at the head of the Celestial Spire, the Archangel keeps the Veil: the thin place between the living and the dead."),
            Shot::new(8.5).stop("hollowmere@gate", 6.0).hour(20.4).walk(0.6).black(0.9, 0.0)
                .grade(r#"{"saturation":0.6,"contrast":1.05,"exposure":1.3,"vignette":0.55,"grain":0.16}"#)
                .camera(r#"{"eye_height":9.0,"horizon":0.3}"#, r#"{"eye_height":3.0,"horizon":0.22}"#)
                .caption("The Veil has worn thin. What falls on the roads below no longer stays fallen."),
            Shot::new(8.5).stop("vale@vale_edge", 4.0).hour(5.6).walk(0.5).black(0.9, 0.0)
                .grade(r#"{"saturation":0.8,"vignette":0.5,"grain":0.12,"focus":{"enabled":true,"distance":9.0,"blur":0.008}}"#)
                .camera(r#"{"yaw":-32,"side":2.4,"eye_height":2.0,"horizon":0.34,"orbit":true}"#, r#"{"yaw":18,"side":-1.4,"eye_height":2.6,"horizon":0.3,"orbit":true}"#)
                .caption("So in every age one pilgrim walks the Hollow Road to the Spire, to ask the Archangel to mend it."),
            Shot::new(6.5).stop("warrens@guild_hall", 3.0).hour(21.5).walk(0.25).black(0.9, 1.6)
                .grade(r#"{"saturation":0.3,"exposure":0.8,"vignette":0.85,"grain":0.2}"#)
                .camera(r#"{"eye_height":1.4,"horizon":0.5,"zoom":1.0}"#, r#"{"eye_height":1.6,"horizon":0.48,"zoom":1.35}"#)
                .title("HOLLOW ROAD", "None who walked it has come back.")
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
        Cutscene::of("entrance", shots, true, false)
    }

    /// The Archangel falls: slow, a flash of white, the view closing on it, and white.
    pub fn fall() -> Cutscene {
        Cutscene::of("fall", vec![
            Shot::new(4.0).push(1.0, 1.3, Focus::Boss).tune(None).grade(r#"{"bloom":1.2,"exposure":1.2}"#)
                .at(0.0, Cue::Slow(0.35, 2.6)).at(0.0, Cue::Flash([255, 250, 235, 210], 0.7)).at(0.0, Cue::Shake(18.0, 0.8))
                .at(0.05, Cue::Sound(Sfx::Knell, 1.0, 0.45)),
            Shot::new(5.0).push(1.3, 1.6, Focus::Boss).white(0.0, 2.6)
                .caption("The Archangel falls, and the Veil, for a while, is whole."),
        ], false, false)
    }

    /// After the fall, out of the white: the sky over the Spire at dawn, the roads quiet, and the
    /// pilgrimage beginning again.
    pub fn ending() -> Cutscene {
        Cutscene::of("ending", vec![
            Shot::new(8.0).stop("spire_summit@sealed_slab", 20.0).hour(6.2).walk(0.2).white(2.4, 0.0).tune(Some(Tune::Shrine))
                .grade(r#"{"bloom":0.9,"exposure":1.12,"saturation":1.05,"vignette":0.3}"#)
                .camera(r#"{"eye_height":1.0,"horizon":0.7}"#, r#"{"eye_height":12.0,"horizon":0.82,"zoom":0.85}"#)
                .caption("Far below, the dead lie still, and the shrine fires burn low."),
            Shot::new(7.5).stop("warrens@guild_hall", 2.0).hour(6.8).walk(0.4).black(0.9, 1.4)
                .grade(r#"{"saturation":0.85,"vignette":0.45,"grain":0.12}"#)
                .camera(r#"{"eye_height":4.0,"horizon":0.3}"#, r#"{"eye_height":2.2,"horizon":0.26}"#)
                .caption("But the Veil wears thin again, as it always has. Somewhere a fire waits for the next pilgrim."),
            Shot::new(5.5).black(1.0, 1.6).title("THE PILGRIMAGE BEGINS AGAIN", "The dead return stronger.")
                .at(0.6, Cue::Sound(Sfx::Knell, 0.8, 0.5)),
        ], false, true)
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
        "The Hollowmere Screamer" => "The voice in the reeds",
        "The Pack Alpha" => "First of the ridge's wretched pack",
        "The Bandit Chief" => "Lord of the ridge camp",
        "The Order Agent" => "Sworn to an Order no one names",
        "The Hollow Magistrate" => "Judge of a city with no one left to judge",
        "The Wraith Lord" => "Crowned in the hollow sanctum",
        "The Rune Sentinel" => "Warden of the sealed vault",
        "The Butcher" => "Keeper of the husk pens",
        "The Angel Priest" => "Who sings to the Archangel still",
        "The Void Stalker" => "What the altars whispered to",
        "The Statue Archangel" => "The Spire's stone sentinel",
        "The Archangel" => "Keeper of the Veil",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Cutscene> {
        let mut v = vec![Cutscene::opening(), Cutscene::fall(), Cutscene::ending()];
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
