//! The game's sounds: what the fight does, the hero's feet on the road, and the menus. Every
//! sound is built into the binary (`assets/sfx`, credited in its CREDITS.txt), picked among its
//! takes at random and played a little higher or lower each time, so a repeated blow never
//! sounds stamped out.
//!
//! The glint of an attack committing has a sound of its own: timing a roll or a parry can be
//! done by ear as well as by eye.

use crate::fight::{Event, Move};
use quartz::*;

macro_rules! takes {
    ($($f:literal),+ $(,)?) => { &[$(include_bytes!(concat!("../assets/sfx/", $f)) as &[u8]),+] };
}

/// A sound, by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sfx {
    SwingLight, SwingHeavy, HitLight, HitHeavy, Block, BlockChip, Parry, GuardBreak, Deflect,
    Struck, StruckHeavy, Roll, Step, Glint, Felled, Knell, Twang, Frost, FlaskOpen, Glug,
    Coins, Pickup, LevelUp, Lesson, UiMove, UiConfirm, UiBack,
}

impl Sfx {
    /// Its takes, and how loud it plays of full.
    fn takes(self) -> (&'static [&'static [u8]], f32) {
        match self {
            Sfx::SwingLight => (takes!["swing_light.wav"], 0.32),
            Sfx::SwingHeavy => (takes!["swing_heavy.wav"], 0.42),
            Sfx::HitLight => (takes!["hit_light_0.ogg", "hit_light_1.ogg", "hit_light_2.ogg"], 0.6),
            Sfx::HitHeavy => (takes!["hit_heavy_0.ogg", "hit_heavy_1.ogg", "hit_heavy_2.ogg"], 0.75),
            Sfx::Block => (takes!["block_0.ogg", "block_1.ogg", "block_2.ogg"], 0.55),
            Sfx::BlockChip => (takes!["block_chip_0.ogg", "block_chip_1.ogg"], 0.65),
            Sfx::Parry => (takes!["parry.ogg"], 0.6),
            Sfx::GuardBreak => (takes!["guard_break.ogg"], 0.75),
            Sfx::Deflect => (takes!["deflect.ogg"], 0.5),
            Sfx::Struck => (takes!["struck_0.ogg", "struck_1.ogg", "struck_2.ogg"], 0.6),
            Sfx::StruckHeavy => (takes!["struck_heavy_0.ogg", "struck_heavy_1.ogg", "struck_heavy_2.ogg"], 0.75),
            Sfx::Roll => (takes!["cloth_0.ogg", "cloth_1.ogg", "cloth_2.ogg", "cloth_3.ogg"], 0.45),
            Sfx::Step => (takes!["step_0.ogg", "step_1.ogg", "step_2.ogg", "step_3.ogg", "step_4.ogg", "step_5.ogg"], 0.16),
            Sfx::Glint => (takes!["glint_0.ogg", "glint_1.ogg", "glint_2.ogg"], 0.3),
            Sfx::Felled => (takes!["felled.ogg"], 0.6),
            Sfx::Knell => (takes!["knell.ogg"], 0.55),
            Sfx::Twang => (takes!["bow_0.wav", "bow_1.wav", "bow_2.wav"], 0.34),
            Sfx::Frost => (takes!["cast_frost.wav"], 0.4),
            Sfx::FlaskOpen => (takes!["flask_open.ogg"], 0.4),
            Sfx::Glug => (takes!["glug.wav"], 0.35),
            Sfx::Coins => (takes!["coins.ogg"], 0.45),
            Sfx::Pickup => (takes!["pickup.ogg"], 0.5),
            Sfx::LevelUp => (takes!["levelup.ogg"], 0.45),
            Sfx::Lesson => (takes!["lesson.ogg"], 0.35),
            Sfx::UiMove => (takes!["ui_move.ogg"], 0.25),
            Sfx::UiConfirm => (takes!["ui_confirm.ogg"], 0.35),
            Sfx::UiBack => (takes!["ui_back.ogg"], 0.3),
        }
    }
}

/// A piece of music, by where it plays. Streamed from `assets/music` (credited in its
/// CREDITS.txt), not built in: together they are larger than the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tune { Title, Road, Dark, Shrine, Boss }

impl Tune {
    /// Its file, and how loud it plays under the fight's sounds.
    fn file(self) -> (&'static str, f32) {
        match self {
            Tune::Title => ("music/title.ogg", 0.45),
            Tune::Road => ("music/road.mp3", 0.3),
            // Mastered very quiet (peak 0.11 over the whole track): lifted to sit with the rest.
            Tune::Dark => ("music/dark.ogg", 1.6),
            Tune::Shrine => ("music/shrine.flac", 0.35),
            Tune::Boss => ("music/boss.flac", 0.4),
        }
    }
}

/// Scenes walked under a roof or below ground: the dungeon's drone instead of the road's tune.
const DARK_SCENES: &[&str] = &[
    "sewers.json", "sealed_slab.json", "wraith_sanctum.json", "lower_stairs.json", "spire_stair.json",
    "rune_vault.json", "vine_halls.json", "whisper_altars.json", "sunken_steps.json", "warrens.json",
];

impl Tune {
    /// What plays walking `scene`.
    pub fn of_scene(scene: &str) -> Tune { if DARK_SCENES.contains(&scene) { Tune::Dark } else { Tune::Road } }
}

/// Seconds one tune takes to give way to the next.
const CROSSFADE: f32 = 2.5;

/// Metres walked between footfalls, walking and running.
const STRIDE: f64 = 0.85;
const RUN_STRIDE: f64 = 1.25;

pub(crate) struct Audio {
    rng: u64,
    /// The road metre of the last footfall.
    stepped: f64,
    /// Off in the headless captures (they would play aloud on the machine running them).
    on: bool,
    /// The tune wanted now, and the one playing (none while muted).
    tune: Option<Tune>,
    playing: Option<SoundHandle>,
}

impl Audio {
    pub fn new() -> Audio {
        Audio { rng: 0x2545_F491_4F6C_DD1D, stepped: 0.0, on: !cfg!(test) && std::env::var_os("HR_MUTE").is_none(), tune: None, playing: None }
    }

    /// Play `want` from here on, looping: the tune before fades out as it fades in. `None` lets
    /// the music die away (the fall); the same tune again carries on where it is.
    pub fn music(&mut self, c: &Canvas, want: Option<Tune>) {
        if want == self.tune { return; }
        self.tune = want;
        if let Some(h) = self.playing.take() { h.fade_out(CROSSFADE); }
        let Some(t) = want.filter(|_| self.on) else { return };
        let (file, level) = t.file();
        let path = crate::assets::path(file);
        self.playing = Some(c.play_sound_with(&path.to_string_lossy(), SoundOptions::new().volume(level).looping(true).fade_in(CROSSFADE)));
    }

    fn roll(&mut self) -> f32 {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17;
        (self.rng % 10_000) as f32 / 10_000.0
    }

    /// `s` at `volume` of its own level and `pitch` of its own, nudged a few percent either way.
    pub fn play_at(&mut self, c: &Canvas, s: Sfx, volume: f32, pitch: f32) {
        if !self.on { return; }
        let (takes, level) = s.takes();
        let pick = takes[(self.roll() * takes.len() as f32) as usize % takes.len()];
        let wobble = 1.0 + (self.roll() - 0.5) * 0.1;
        c.play_sound_bytes_with(pick, SoundOptions::new().volume(level * volume).pitch(pitch * wobble));
    }
    pub fn play(&mut self, c: &Canvas, s: Sfx) { self.play_at(c, s, 1.0, 1.0); }

    /// What a tick of the fight (on the road or side-on) sounds like.
    pub fn on_events(&mut self, c: &Canvas, events: &[Event]) {
        for e in events {
            match *e {
                Event::Began(Move::Light) => self.play(c, Sfx::SwingLight),
                Event::Began(Move::Heavy | Move::Riposte) => self.play(c, Sfx::SwingHeavy),
                Event::Began(Move::Step(_) | Move::Tumble | Move::Backstep) => self.play(c, Sfx::Roll),
                Event::Began(Move::Lane(_)) => self.play_at(c, Sfx::Roll, 0.4, 1.2),
                Event::Began(Move::Flask) => self.play(c, Sfx::FlaskOpen),
                Event::Began(Move::Art) => self.play_at(c, Sfx::SwingHeavy, 1.0, 0.8),
                // The art going off: a deep ring and its blow; Sentence's falling like a knell.
                Event::Art { .. } => { self.play_at(c, Sfx::Parry, 0.9, 0.6); self.play_at(c, Sfx::GuardBreak, 0.7, 0.9); }
                Event::Sentenced { .. } => { self.play_at(c, Sfx::Knell, 0.8, 1.2); self.play(c, Sfx::HitHeavy); }
                Event::Caught => self.play_at(c, Sfx::Block, 0.5, 1.4),
                Event::Rung { .. } => { self.play_at(c, Sfx::Knell, 0.9, 0.8); self.play_at(c, Sfx::GuardBreak, 0.6, 0.6); }
                Event::Popped { .. } => { self.play_at(c, Sfx::Felled, 1.0, 0.6); self.play_at(c, Sfx::GuardBreak, 0.7, 0.7); }
                Event::Raised { .. } => self.play_at(c, Sfx::Knell, 0.5, 0.6),
                Event::Slam { heavy, .. } => self.play_at(c, Sfx::Felled, if heavy { 0.7 } else { 0.45 }, 0.75),
                // Blood drawn across the bow: a low hiss of it and a dull pulse.
                Event::Imbued => { self.play_at(c, Sfx::Frost, 0.6, 0.55); self.play_at(c, Sfx::StruckHeavy, 0.5, 0.7); }
                Event::Shot(r) => match r {
                    crate::gear::Ranged::Frost => self.play(c, Sfx::Frost),
                    // A crossbow's prod is shorter and stiffer than a bow's limbs.
                    crate::gear::Ranged::BloodCrossbow => self.play_at(c, Sfx::Twang, 1.0, 1.5),
                    _ => self.play(c, Sfx::Twang),
                },
                Event::Hit { mv: Move::Light, .. } => self.play(c, Sfx::HitLight),
                Event::Hit { mv: Move::Riposte, .. } => { self.play(c, Sfx::HitHeavy); self.play_at(c, Sfx::StruckHeavy, 0.7, 0.8); }
                Event::Hit { .. } => self.play(c, Sfx::HitHeavy),
                Event::Glanced { .. } | Event::Deflected { .. } => self.play(c, Sfx::Deflect),
                Event::Felled { .. } => self.play(c, Sfx::Felled),
                Event::Commits { .. } => self.play(c, Sfx::Glint),
                // Turning to the hero: a rustle as it squares up.
                Event::Noticed { .. } => self.play_at(c, Sfx::Roll, 0.5, 0.8),
                Event::Blocked { chip } if chip <= 0.0 => self.play(c, Sfx::Block),
                Event::Blocked { .. } => self.play(c, Sfx::BlockChip),
                Event::PerfectBlock => self.play_at(c, Sfx::Parry, 0.8, 0.85),
                Event::WardBurst { ward, .. } => match ward {
                    crate::gear::Ward::Bulwark => { self.play_at(c, Sfx::Parry, 1.0, 0.7); self.play(c, Sfx::GuardBreak); }
                    // A kite's bash: a dull slam of wood; a buckler's ring as the blow goes back; a parma's riposte, a bright deep cut.
                    crate::gear::Ward::Kite => { self.play_at(c, Sfx::Block, 1.0, 0.7); self.play_at(c, Sfx::Felled, 0.8, 0.9); }
                    crate::gear::Ward::Buckler => { self.play_at(c, Sfx::Parry, 1.0, 1.25); self.play(c, Sfx::HitHeavy); }
                    crate::gear::Ward::Parma => { self.play_at(c, Sfx::Parry, 0.8, 1.1); self.play_at(c, Sfx::StruckHeavy, 0.8, 0.9); }
                },
                Event::Parried { .. } => { self.play(c, Sfx::Parry); self.play_at(c, Sfx::Block, 0.6, 1.2); }
                Event::ParryMissed { .. } => self.play_at(c, Sfx::BlockChip, 0.8, 0.85),
                Event::GuardBroken { .. } => self.play(c, Sfx::GuardBreak),
                Event::JustDodge { .. } => self.play_at(c, Sfx::SwingLight, 0.8, 1.4),
                Event::Struck { heavy: false, .. } => self.play(c, Sfx::Struck),
                Event::Struck { heavy: true, .. } | Event::Staggered { .. } => self.play(c, Sfx::StruckHeavy),
                Event::KnockedDown { .. } => { self.play_at(c, Sfx::StruckHeavy, 1.0, 0.8); self.play_at(c, Sfx::Felled, 0.8, 0.9); }
                Event::Healed => self.play(c, Sfx::Glug),
                Event::Died => { self.play_at(c, Sfx::StruckHeavy, 1.0, 0.7); self.play_at(c, Sfx::Knell, 1.0, 0.5); }
                _ => {}
            }
        }
    }

    /// Footfalls while the road moves under the hero: one a stride, quicker and louder running.
    pub fn steps(&mut self, c: &Canvas, walked: f64, speed: f32) {
        if speed.abs() < 0.1 { self.stepped = walked; return; }
        let run = speed > crate::fight::WALK + 0.5;
        if (walked - self.stepped).abs() >= if run { RUN_STRIDE } else { STRIDE } {
            self.stepped = walked;
            self.play_at(c, Sfx::Step, if run { 1.3 } else { 1.0 }, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Sfx; 27] = [
        Sfx::SwingLight, Sfx::SwingHeavy, Sfx::HitLight, Sfx::HitHeavy, Sfx::Block, Sfx::BlockChip, Sfx::Parry, Sfx::GuardBreak, Sfx::Deflect,
        Sfx::Struck, Sfx::StruckHeavy, Sfx::Roll, Sfx::Step, Sfx::Glint, Sfx::Felled, Sfx::Knell, Sfx::Twang, Sfx::Frost, Sfx::FlaskOpen, Sfx::Glug,
        Sfx::Coins, Sfx::Pickup, Sfx::LevelUp, Sfx::Lesson, Sfx::UiMove, Sfx::UiConfirm, Sfx::UiBack,
    ];

    /// A take that cannot be decoded plays as silence and says nothing: every one is decoded
    /// here, by the library the engine plays them with, and must hold sound of some length.
    #[test]
    fn every_sound_decodes_and_is_not_silent() {
        for s in ALL {
            let (takes, level) = s.takes();
            assert!(level > 0.0 && level <= 1.0, "{s:?}");
            for (k, bytes) in takes.iter().enumerate() {
                let d = rodio::Decoder::new(std::io::Cursor::new(bytes.to_vec())).unwrap_or_else(|e| panic!("{s:?} take {k}: {e}"));
                let samples: Vec<f32> = d.collect();
                assert!(samples.len() > 400, "{s:?} take {k} is {} samples", samples.len());
                let peak = samples.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
                assert!(peak > 0.05, "{s:?} take {k} is near silent ({peak})");
            }
        }
    }

    /// The music is read from disk where the game looks for it: each tune is there, decodes,
    /// and is heard (as loud as it plays) in its first ten seconds.
    #[test]
    fn every_tune_is_found_and_decodes() {
        for t in [Tune::Title, Tune::Road, Tune::Dark, Tune::Shrine, Tune::Boss] {
            let (file, level) = t.file();
            let bytes = std::fs::read(crate::assets::path(file)).unwrap_or_else(|e| panic!("{t:?}: {e}"));
            let d = rodio::Decoder::new(std::io::Cursor::new(bytes)).unwrap_or_else(|e| panic!("{t:?}: {e}"));
            let rate = rodio::Source::sample_rate(&d) as usize * rodio::Source::channels(&d) as usize;
            let samples: Vec<f32> = d.take(rate * 10).collect();
            assert_eq!(samples.len(), rate * 10, "{t:?}");
            let peak = samples.iter().fold(0.0f32, |m, &x| m.max(x.abs())) * level;
            assert!(peak > 0.04, "{t:?} opens near silent ({peak})");
        }
        assert_eq!(Tune::of_scene("sewers.json"), Tune::Dark);
        assert_eq!(Tune::of_scene("road_to_hollowmere.json"), Tune::Road);
    }
}
